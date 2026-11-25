#!/usr/bin/env python3
"""Prepare and deploy SiteMonitor to Synology Container Manager over SSH."""

from __future__ import annotations

import argparse
import base64
from datetime import datetime, timezone
import getpass
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import re
import secrets
import shlex
import shutil
import subprocess
import sys
import tarfile
import tempfile
import time
from typing import Any


ROOT = Path(__file__).resolve().parent
WEB_DIRECTORY = Path("target/dx/frontend/release/web/public")
PACKAGE_FILES = (
    "Dockerfile", "docker-compose.yml", ".dockerignore", ".env.nas.example",
    "Cargo.toml", "Cargo.lock", "backend/Cargo.toml", "shared/Cargo.toml",
    "frontend/Cargo.toml", "docker/entrypoint.sh",
)
SOURCE_DIRECTORIES = ("backend/src", "shared/src", "frontend/src")
WEB_EXTENSIONS = {
    ".html", ".js", ".wasm", ".json", ".css", ".woff", ".woff2", ".ttf",
    ".otf", ".png", ".jpg", ".jpeg", ".webp", ".svg", ".ico", ".txt",
    ".map", ".webmanifest", ".gif", ".avif",
}
PRIVATE_DIRECTORIES = {"data", "keys", "uploads", "backups", "node_modules"}
DOCKER_SYNOLOGY = "/var/packages/ContainerManager/target/usr/bin/docker"
COMPOSE_SYNOLOGY = "/var/packages/ContainerManager/target/usr/bin/docker-compose"
UPLOAD_CHUNK_SIZE = 192 * 1024
STATE_FILENAME = ".deployment-state.json"
ATTEMPT_FILENAME = ".deployment-attempt.json"
IMAGE_PATTERN = re.compile(r"[a-z0-9][a-z0-9._:/-]*\Z")


class DeploymentError(RuntimeError):
    """An actionable deployment failure that should terminate the wizard."""


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def allowed_package_path(name: str) -> bool:
    path = PurePosixPath(name)
    if not name or path.is_absolute() or ".." in path.parts or "\\" in name:
        return False
    if path.as_posix() != name:
        return False
    if name in PACKAGE_FILES or name == "release.json":
        return True
    if any(part.startswith(".") or part.lower() in PRIVATE_DIRECTORIES for part in path.parts):
        return False
    if any(name.startswith(directory + "/") for directory in SOURCE_DIRECTORIES):
        return path.suffix == ".rs"
    return path.parts[0] == "web" and path.suffix.lower() in WEB_EXTENSIONS


def copy_package_file(source: Path, destination: Path) -> None:
    if source.is_symlink() or not source.is_file():
        raise DeploymentError(f"Expected an ordinary file: {source}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    if source.suffix == ".sh":
        destination.write_bytes(source.read_bytes().replace(b"\r\n", b"\n"))
    else:
        shutil.copyfile(source, destination)


def current_dioxus_assets(frontend: Path) -> set[str] | None:
    """Use Dioxus's release manifest to omit hashed assets left by older builds."""
    manifest_path = frontend.parent / ".manifest.json"
    if not manifest_path.is_file():
        return None
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        assets = manifest["assets"]
        if not isinstance(assets, dict):
            raise ValueError("assets must be an object")
        current: set[str] = set()
        for variants in assets.values():
            if not isinstance(variants, list):
                raise ValueError("asset variants must be lists")
            for variant in variants:
                bundled = variant["bundled_path"].replace("\\", "/")
                public = bundled.startswith("../")
                relative = bundled[3:] if public else bundled
                path = PurePosixPath(relative)
                if (not relative or path.is_absolute() or ".." in path.parts or ":" in relative
                        or path.as_posix() != relative or "\x00" in relative):
                    raise ValueError(f"unsafe bundled path: {bundled}")
                relative = relative if public else "assets/" + relative
                if not allowed_package_path("web/" + relative):
                    raise ValueError(f"unexpected public asset: {relative}")
                if not (frontend / relative).is_file():
                    raise ValueError(f"current asset is missing: {relative}")
                if not public:
                    current.add(relative)
        if not current:
            raise ValueError("release manifest lists no bundled assets")
        return current
    except (KeyError, TypeError, AttributeError, ValueError) as error:
        raise DeploymentError(f"Invalid Dioxus release manifest: {error}") from error


def prepare_package(root: Path, frontend: Path, release_id: str) -> tuple[Path, Path, dict]:
    """Copy only build sources and public assets; never copy local runtime data."""
    root = root.resolve()
    frontend = frontend.resolve()
    if not (frontend / "index.html").is_file() or not any(frontend.rglob("*.wasm")):
        raise DeploymentError("Frontend bundle is missing. Run node scripts/build.mjs --skip-backend first.")
    current_assets = current_dioxus_assets(frontend)
    dist = root / "dist"
    if dist.is_symlink():
        raise DeploymentError("Refusing to write through a symlinked dist directory.")
    dist.mkdir(exist_ok=True)
    package = dist / "nas"
    if package.is_symlink():
        raise DeploymentError("Refusing to replace a symlinked dist/nas directory.")
    with tempfile.TemporaryDirectory(prefix=".nas-", dir=dist) as temporary:
        staging = Path(temporary)
        for name in PACKAGE_FILES:
            copy_package_file(root / name, staging / name)
        for directory in SOURCE_DIRECTORIES:
            for source in sorted((root / directory).rglob("*")):
                if source.is_symlink():
                    raise DeploymentError(f"Symlinks are not allowed in deployment sources: {source}")
                relative = source.relative_to(root).as_posix()
                if source.is_file() and allowed_package_path(relative):
                    copy_package_file(source, staging / relative)
        for source in sorted(frontend.rglob("*")):
            if source.is_symlink():
                raise DeploymentError(f"Symlinks are not allowed in the frontend bundle: {source}")
            web_path = source.relative_to(frontend).as_posix()
            if current_assets is not None and web_path.startswith("assets/") and web_path not in current_assets:
                continue
            relative = "web/" + web_path
            if source.is_file() and allowed_package_path(relative):
                copy_package_file(source, staging / relative)
        files = {
            source.relative_to(staging).as_posix(): file_sha256(source)
            for source in sorted(staging.rglob("*")) if source.is_file()
        }
        manifest = {"release_id": release_id, "files": files}
        (staging / "release.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
        if package.exists():
            if package.resolve() != root / "dist" / "nas":
                raise DeploymentError("Refusing to replace a package outside this project's dist/nas.")
            shutil.rmtree(package)
        shutil.copytree(staging, package)
    archive = dist / "sitemonitor-nas.tar.gz"
    if archive.is_symlink():
        raise DeploymentError("Refusing to replace a symlinked release archive.")
    with tarfile.open(archive, "w:gz") as bundle:
        for source in sorted(package.rglob("*")):
            if source.is_file():
                info = bundle.gettarinfo(str(source), arcname=source.relative_to(package).as_posix())
                info.mode = 0o755 if info.name.endswith(".sh") else 0o644
                info.uid = info.gid = 0
                info.uname = info.gname = ""
                with source.open("rb") as content:
                    bundle.addfile(info, content)
    validate_archive(archive)
    return package, archive, manifest


def validate_archive(archive: Path) -> dict:
    """Reject unexpected paths, links, missing files, and changed file contents."""
    with tarfile.open(archive, "r:gz") as bundle:
        members = bundle.getmembers()
        names: set[str] = set()
        for member in members:
            if not member.isfile() or not allowed_package_path(member.name) or member.name in names:
                raise DeploymentError(f"Unexpected or unsafe archive member: {member.name}")
            names.add(member.name)
        if "release.json" not in names or not set(PACKAGE_FILES).issubset(names):
            raise DeploymentError("Release archive is missing its manifest or required build files.")
        manifest_file = bundle.extractfile("release.json")
        assert manifest_file is not None
        manifest = json.loads(manifest_file.read(2 * 1024 * 1024))
        expected = manifest.get("files")
        if not isinstance(expected, dict) or set(expected) != names - {"release.json"}:
            raise DeploymentError("Release manifest does not match the archive file list.")
        for name, checksum in expected.items():
            source = bundle.extractfile(name)
            assert source is not None
            digest = hashlib.sha256()
            for chunk in iter(lambda: source.read(1024 * 1024), b""):
                digest.update(chunk)
            if digest.hexdigest() != checksum:
                raise DeploymentError(f"Release checksum mismatch: {name}")
    return manifest


def remote_directory(value: str) -> str:
    path = PurePosixPath(value)
    if (not path.is_absolute() or ".." in path.parts or path.as_posix() != value
            or len(path.parts) != 4 or not re.fullmatch(r"volume[1-9][0-9]*", path.parts[1])
            or path.parts[2] != "docker" or path.parts[3] in {"service5_db", ".", ".."}
            or any(character in value for character in "\x00\r\n")):
        raise argparse.ArgumentTypeError("Use a dedicated /volumeN/docker/<project> directory; service5_db is reserved.")
    return value


def port_number(value: str) -> int:
    try:
        port = int(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError("Port must be a number.") from error
    if not 1 <= port <= 65535:
        raise argparse.ArgumentTypeError("Port must be between 1 and 65535.")
    return port


def run_ssh(ssh: Any, command: str, *, timeout: float = 60, input_bytes: bytes | None = None,
            stream: bool = False, check: bool = True) -> tuple[int, str, str]:
    """Drain both SSH output streams before waiting for the exit status."""
    stdin, stdout, _ = ssh.exec_command(command, timeout=timeout)
    channel = stdout.channel
    if input_bytes is not None:
        stdin.write(input_bytes)
        stdin.flush()
    channel.shutdown_write()
    deadline = time.monotonic() + timeout
    output = bytearray()
    errors = bytearray()
    try:
        while True:
            for ready, receive, destination in (
                (channel.recv_ready, channel.recv, output),
                (channel.recv_stderr_ready, channel.recv_stderr, errors),
            ):
                while ready():
                    chunk = receive(32768)
                    if not chunk:
                        break
                    destination.extend(chunk)
                    if len(destination) > 256 * 1024:
                        del destination[:-256 * 1024]
                    if stream:
                        print(chunk.decode("utf-8", errors="replace"), end="", flush=True)
            if channel.exit_status_ready() and not channel.recv_ready() and not channel.recv_stderr_ready():
                break
            if time.monotonic() >= deadline:
                raise DeploymentError(f"NAS command timed out after {timeout:g} seconds.")
            time.sleep(0.02)
        code = channel.recv_exit_status()
    finally:
        channel.close()
    out = output.decode("utf-8", errors="replace").strip()
    err = errors.decode("utf-8", errors="replace").strip()
    if check and code != 0:
        raise DeploymentError(f"NAS command failed (exit {code}): {(err or out)[-2000:]}")
    return code, out, err


def upload_stream(ssh: Any, source: Any, remote_path: str) -> None:
    quoted = shlex.quote(remote_path)
    run_ssh(ssh, f"umask 077; : > {quoted}")
    for chunk in iter(lambda: source.read(UPLOAD_CHUNK_SIZE), b""):
        run_ssh(ssh, f"base64 -d >> {quoted}", input_bytes=base64.b64encode(chunk))


def upload_file(ssh: Any, source: Path, remote_path: str) -> None:
    with source.open("rb") as content:
        upload_stream(ssh, content, remote_path)
    _, checksum, _ = run_ssh(ssh, f"sha256sum {shlex.quote(remote_path)}")
    if not checksum.split() or checksum.split()[0] != file_sha256(source):
        raise DeploymentError("Uploaded archive checksum did not match; the current app was not replaced.")


def generated_environment(args: argparse.Namespace, image: str) -> str:
    values = {
        "POSTGRES_PASSWORD": secrets.token_urlsafe(36),
        "SITEMONITOR_ADMIN_PASSWORD": secrets.token_urlsafe(24),
        "SITEMONITOR_DATA_KEY": secrets.token_hex(32),
        "SITEMONITOR_IMAGE": image,
        "SITEMONITOR_PORT": str(args.app_port),
        "SITEMONITOR_BIND_IP": args.bind_ip,
        "SITEMONITOR_SECURE_COOKIES": "1" if args.secure_cookies else "0",
        "TZ": "Asia/Singapore",
    }
    return "# Generated once by the SiteMonitor NAS wizard. Keep private and back up.\n" + "".join(
        f"{name}={value}\n" for name, value in values.items()
    )


def ensure_remote_environment(ssh: Any, base: str, args: argparse.Namespace, image: str) -> bool:
    environment = base + "/.env"
    code, _, _ = run_ssh(ssh, f"test -f {shlex.quote(environment)}", check=False)
    if code == 0:
        print("Keeping the existing NAS .env and encryption key.")
        return False
    if code != 1:
        raise DeploymentError("Could not check the NAS environment file.")
    temporary = base + "/.env.new-" + secrets.token_hex(6)
    upload_stream(ssh, io.BytesIO(generated_environment(args, image).encode("utf-8")), temporary)
    quoted_env = shlex.quote(environment)
    quoted_temp = shlex.quote(temporary)
    run_ssh(ssh, f"chmod 600 {quoted_temp}; if test -e {quoted_env}; then rm -f {quoted_temp}; "
            f"else ln {quoted_temp} {quoted_env} && rm -f {quoted_temp}; fi")
    print("Created private NAS .env. Initial admin password is stored there; it is not printed or uploaded from this PC.")
    return True


def discover_docker(ssh: Any) -> tuple[str, str]:
    _, docker, _ = run_ssh(ssh, f"if test -x {shlex.quote(DOCKER_SYNOLOGY)}; then "
                          f"printf '%s' {shlex.quote(DOCKER_SYNOLOGY)}; else command -v docker; fi")
    docker = docker.strip()
    if not docker or "\n" in docker:
        raise DeploymentError("Docker was not found. Install and start Synology Container Manager.")
    quoted = shlex.quote(docker)
    code, platform, _ = run_ssh(ssh, f"{quoted} info --format '{{{{.OSType}}}}/{{{{.Architecture}}}}'", check=False)
    if code != 0:
        raise DeploymentError("The SSH user cannot access Docker. Grant this account Container Manager/Docker access, "
                              "or run the documented commands through an authorized NAS administrator. No sudo was attempted.")
    if not platform.startswith("linux/"):
        raise DeploymentError(f"Expected a Linux Docker server; received {platform!r}.")
    print(f"NAS Docker platform: {platform}")
    code, _, _ = run_ssh(ssh, f"{quoted} compose version", check=False)
    if code == 0:
        return quoted, quoted + " compose"
    _, compose, _ = run_ssh(ssh, f"if test -x {shlex.quote(COMPOSE_SYNOLOGY)}; then "
                           f"printf '%s' {shlex.quote(COMPOSE_SYNOLOGY)}; else command -v docker-compose; fi")
    if not compose or "\n" in compose:
        raise DeploymentError("Docker Compose was not found in Container Manager.")
    run_ssh(ssh, shlex.quote(compose) + " version")
    return quoted, shlex.quote(compose)


def compose_command(compose: str, base: str, compose_file: str, image: str) -> str:
    if not IMAGE_PATTERN.fullmatch(image):
        raise DeploymentError("Invalid image name in deployment state.")
    path = PurePosixPath(compose_file)
    prefix = base + "/releases/"
    if (not compose_file.startswith(prefix) or path.as_posix() != compose_file
            or ".." in path.parts or path.name != "docker-compose.yml"
            or len(PurePosixPath(compose_file[len(prefix):]).parts) != 2):
        raise DeploymentError("Compose file must belong to a staged release in this deployment directory.")
    return (f"SITEMONITOR_IMAGE={shlex.quote(image)} {compose} -p sitemonitor "
            f"--project-directory {shlex.quote(base)} --env-file {shlex.quote(base + '/.env')} "
            f"-f {shlex.quote(compose_file)}")


def read_state(ssh: Any, base: str, filename: str = STATE_FILENAME) -> dict:
    state_path = shlex.quote(base + "/" + filename)
    _, content, _ = run_ssh(ssh, f"if test -f {state_path}; then cat {state_path}; fi")
    if not content:
        return {}
    try:
        state = json.loads(content)
    except json.JSONDecodeError as error:
        raise DeploymentError("NAS deployment state is invalid; refusing to replace its rollback record.") from error
    if not isinstance(state, dict):
        raise DeploymentError("NAS deployment state must be a JSON object.")
    return state


def write_state(ssh: Any, base: str, state: dict, filename: str = STATE_FILENAME) -> None:
    temporary = base + "/" + filename + ".new"
    upload_stream(ssh, io.BytesIO((json.dumps(state, indent=2) + "\n").encode()), temporary)
    run_ssh(ssh, f"mv -f {shlex.quote(temporary)} {shlex.quote(base + '/' + filename)}")


def health_response_valid(ready: bytes, html: bytes) -> bool:
    try:
        health = json.loads(ready)
    except (ValueError, UnicodeDecodeError):
        return False
    document = html.decode("utf-8", errors="replace").lower()
    return (isinstance(health, dict) and health.get("ok") is True
            and "sitemonitor" in document and ("<html" in document or "<!doctype html" in document))


def wait_until_ready(ssh: Any, command: str, timeout: float) -> None:
    """Probe inside the app container, including when DSM owns the public HTTPS endpoint."""
    probe = command + " exec -T app curl --fail --silent --show-error --max-time 5 "
    deadline = time.monotonic() + timeout
    while True:
        try:
            ready_code, ready, _ = run_ssh(ssh, probe + "http://127.0.0.1:8737/api/ready", timeout=15, check=False)
            html_code, html, _ = run_ssh(ssh, probe + "http://127.0.0.1:8737/", timeout=15, check=False)
            if ready_code == html_code == 0 and health_response_valid(ready.encode(), html.encode()):
                print("SiteMonitor HTML and database readiness are healthy on the NAS.")
                return
        except (OSError, DeploymentError):
            pass
        if time.monotonic() >= deadline:
            raise DeploymentError("NAS readiness verification failed: expected healthy database readiness and SiteMonitor HTML.")
        print("Waiting for the NAS app and database...", flush=True)
        time.sleep(min(3, max(0, deadline - time.monotonic())))


def connect_ssh(args: argparse.Namespace) -> Any:
    try:
        import paramiko
    except ImportError as error:
        raise DeploymentError("SSH deployment needs Paramiko. Install it with: "
                              "python -m pip install -r scripts/requirements-nas.txt") from error
    username = args.user
    if not username:
        if not sys.stdin.isatty():
            raise DeploymentError("Supply --user for noninteractive SSH deployment.")
        username = input("NAS SSH username [admin]: ").strip() or "admin"
    known_hosts = Path.home() / ".ssh" / "known_hosts"
    ssh = paramiko.SSHClient()
    ssh.load_system_host_keys()
    if known_hosts.exists():
        ssh.load_host_keys(str(known_hosts))

    class VerifyNewHost(paramiko.MissingHostKeyPolicy):
        def missing_host_key(self, client: Any, hostname: str, key: Any) -> None:
            fingerprint = "SHA256:" + base64.b64encode(hashlib.sha256(key.asbytes()).digest()).decode().rstrip("=")
            if args.host_key_fingerprint:
                trusted = secrets.compare_digest(args.host_key_fingerprint, fingerprint)
            elif args.accept_new_host_key:
                trusted = True
                print(f"Accepting new NAS host key: {fingerprint}")
            elif sys.stdin.isatty():
                print(f"New SSH host {hostname}: {key.get_name()} {fingerprint}")
                trusted = input("Compare this fingerprint with DSM. Trust and save this key? [y/N]: ").strip().lower() == "y"
            else:
                trusted = False
            if not trusted:
                raise DeploymentError(f"Untrusted SSH host key {fingerprint}. Verify it through DSM and use "
                                      "--host-key-fingerprint SHA256:... or explicitly --accept-new-host-key.")
            known_hosts.parent.mkdir(parents=True, exist_ok=True)
            client.get_host_keys().add(hostname, key.get_name(), key)
            client.save_host_keys(str(known_hosts))

    ssh.set_missing_host_key_policy(VerifyNewHost())
    options: dict[str, Any] = {
        "hostname": args.host, "port": args.ssh_port, "username": username,
        "timeout": 20, "auth_timeout": 30, "banner_timeout": 30,
    }
    if args.key:
        options["key_filename"] = str(args.key.expanduser())
    try:
        try:
            ssh.connect(**options)
        except paramiko.SSHException as error:
            authentication_failure = isinstance(error, (paramiko.AuthenticationException, paramiko.PasswordRequiredException))
            if not authentication_failure and str(error) != "No authentication methods available":
                raise
            if not sys.stdin.isatty():
                raise DeploymentError("SSH authentication failed. Configure an SSH key/agent or run interactively.")
            password = getpass.getpass(f"SSH password for {username}@{args.host}: ")
            ssh.connect(**options, password=password, allow_agent=False, look_for_keys=False)
        transport = ssh.get_transport()
        if transport is not None:
            transport.set_keepalive(20)
        return ssh
    except Exception as error:
        ssh.close()
        if isinstance(error, DeploymentError):
            raise
        raise DeploymentError(f"SSH connection failed: {error}") from error


def deploy(args: argparse.Namespace, archive: Path | None, release_id: str | None) -> None:
    ssh = connect_ssh(args)
    base = args.directory
    try:
        docker, compose = discover_docker(ssh)
        quoted_base = shlex.quote(base)
        run_ssh(ssh, f"test ! -L {quoted_base} && mkdir -p {quoted_base} && "
                f"test \"$(cd {quoted_base} && pwd -P)\" = {quoted_base}")
        previous = read_state(ssh, base)
        if args.rollback:
            attempt = read_state(ssh, base, ATTEMPT_FILENAME)
            image = attempt.get("fallback_image") if attempt else previous.get("previous_image")
            compose_file = attempt.get("fallback_compose_file") if attempt else previous.get("previous_compose_file")
            if not image or not compose_file or not compose_file.startswith(base + "/releases/"):
                raise DeploymentError("No previous successful release was recorded for rollback.")
            print("Restoring the previous app image. Database migrations and data will not be rolled back.")
            command = compose_command(compose, base, compose_file, image)
            run_ssh(ssh, command + " up -d app", timeout=300, stream=True)
            wait_until_ready(ssh, command, args.health_timeout)
            if not attempt:
                write_state(ssh, base, {"release_id": previous.get("previous_release_id"), "image": image,
                            "compose_file": compose_file, "previous_image": previous.get("image"),
                            "previous_compose_file": previous.get("compose_file"),
                            "previous_release_id": previous.get("release_id")})
            run_ssh(ssh, "rm -f " + shlex.quote(base + "/" + ATTEMPT_FILENAME))
            return
        assert archive is not None and release_id is not None
        validate_archive(archive)
        release = base + "/releases/" + release_id
        remote_archive = base + "/releases/" + release_id + ".tar.gz"
        image = "sitemonitor:release-" + release_id
        run_ssh(ssh, f"mkdir -p {shlex.quote(base + '/releases')} && mkdir {shlex.quote(release)}")
        print(f"Uploading {archive.stat().st_size / (1024 * 1024):.1f} MiB release; local database and keys are excluded.")
        upload_file(ssh, archive, remote_archive)
        run_ssh(ssh, f"tar -xzf {shlex.quote(remote_archive)} -C {shlex.quote(release)}", timeout=120)
        ensure_remote_environment(ssh, base, args, image)
        compose_file = release + "/docker-compose.yml"
        command = compose_command(compose, base, compose_file, image)
        run_ssh(ssh, command + " config --quiet")
        print("Building the Linux Rust image on the NAS. The existing app stays running during this step.")
        run_ssh(ssh, f"{docker} build --tag {shlex.quote(image)} {shlex.quote(release)}",
                timeout=args.build_timeout, stream=True)
        print("Starting SiteMonitor and PostgreSQL in the independent sitemonitor project.")
        write_state(ssh, base, {"attempted_image": image, "attempted_compose_file": compose_file,
                    "fallback_image": previous.get("image"), "fallback_compose_file": previous.get("compose_file")},
                    ATTEMPT_FILENAME)
        try:
            run_ssh(ssh, command + " up -d", timeout=600, stream=True)
            wait_until_ready(ssh, command, args.health_timeout)
        except DeploymentError:
            if previous.get("image"):
                print("Previous successful image and database volumes were kept. Use --rollback only after checking migration compatibility.")
            raise
        write_state(ssh, base, {"release_id": release_id, "image": image, "compose_file": compose_file,
                    "previous_image": previous.get("image"), "previous_compose_file": previous.get("compose_file"),
                    "previous_release_id": previous.get("release_id")})
        run_ssh(ssh, "rm -f " + shlex.quote(base + "/" + ATTEMPT_FILENAME))
        print(f"Deployment recorded in {base}/{STATE_FILENAME}. Service 5 was not modified.")
    finally:
        ssh.close()


def parser() -> argparse.ArgumentParser:
    arguments = argparse.ArgumentParser(description=__doc__)
    mode = arguments.add_mutually_exclusive_group()
    mode.add_argument("--prepare-only", action="store_true", help="Create dist/nas and its archive without SSH or Paramiko.")
    mode.add_argument("--rollback", action="store_true", help="Restore the previous app image; database changes are not reverted.")
    arguments.add_argument("--skip-frontend-build", action="store_true", help="Package the existing frontend bundle explicitly.")
    arguments.add_argument("--frontend-dir", type=Path, default=WEB_DIRECTORY, help="Existing release web bundle path.")
    arguments.add_argument("--host", default="10.45.1.3")
    arguments.add_argument("--ssh-port", type=port_number, default=22)
    arguments.add_argument("--user", help="NAS SSH username; prompted interactively if omitted.")
    arguments.add_argument("--key", type=Path, help="SSH private key path; passwords are only read using a hidden prompt.")
    arguments.add_argument("--directory", type=remote_directory, default="/volume1/docker/sitemonitor")
    arguments.add_argument("--app-port", type=port_number, default=8737, help="Published app port on the first deployment.")
    arguments.add_argument("--bind-ip", default="0.0.0.0", help="Published bind IP on the first deployment.")
    arguments.add_argument("--secure-cookies", action="store_true", help="Enable Secure cookies in a newly created NAS .env (requires HTTPS).")
    arguments.add_argument("--accept-new-host-key", action="store_true", help="Explicitly trust and save a previously unknown SSH host key.")
    arguments.add_argument("--host-key-fingerprint", help="Expected SHA256 SSH fingerprint for a new host.")
    arguments.add_argument("--build-timeout", type=int, default=3600, help="Maximum NAS image build seconds.")
    arguments.add_argument("--health-timeout", type=int, default=180, help="Maximum app readiness wait seconds.")
    return arguments


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    if not re.fullmatch(r"[a-zA-Z0-9._:-]+", args.host):
        raise DeploymentError("Host must be a NAS hostname or IP address, without a URL or path.")
    if not re.fullmatch(r"[0-9a-fA-F:.]+", args.bind_ip):
        raise DeploymentError("Bind IP must be an IPv4 or IPv6 address.")
    if args.build_timeout <= 0 or args.health_timeout <= 0:
        raise DeploymentError("Timeouts must be positive.")
    if args.rollback:
        deploy(args, None, None)
        return 0
    if not args.skip_frontend_build:
        print("Building the release frontend locally...", flush=True)
        subprocess.run(["node", str(ROOT / "scripts/build.mjs"), "--skip-backend"], cwd=ROOT, check=True)
    revision = subprocess.run(["git", "rev-parse", "--short=12", "HEAD"], cwd=ROOT, text=True,
                              capture_output=True, check=False).stdout.strip()
    if not re.fullmatch(r"[0-9a-f]{7,40}", revision):
        revision = "source"
    release_id = datetime.now(timezone.utc).strftime("%Y%m%d%H%M%S%f") + "-" + revision
    frontend = args.frontend_dir if args.frontend_dir.is_absolute() else ROOT / args.frontend_dir
    package, archive, manifest = prepare_package(ROOT, frontend, release_id)
    print(f"Prepared {len(manifest['files'])} public/build files: {package}")
    print(f"Archive: {archive}\nSHA256: {file_sha256(archive)}")
    if not args.prepare_only:
        deploy(args, archive, release_id)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        print("\nDeployment cancelled.", file=sys.stderr)
        sys.exit(130)
    except (DeploymentError, OSError, subprocess.CalledProcessError, tarfile.TarError, ValueError) as error:
        print(f"Deployment failed: {error}", file=sys.stderr)
        sys.exit(1)
