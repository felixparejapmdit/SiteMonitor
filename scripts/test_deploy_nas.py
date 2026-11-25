"""Offline regression tests for release packaging and NAS deployment boundaries."""

from __future__ import annotations

import argparse
import base64
import importlib.util
import io
import json
from pathlib import Path
import shlex
import tarfile
import tempfile
import unittest
from unittest.mock import Mock, patch


SPEC = importlib.util.spec_from_file_location("deploy_nas", Path(__file__).resolve().parents[1] / "deploy-to-nas.py")
assert SPEC is not None and SPEC.loader is not None
nas = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(nas)


class PackagingTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        for name in nas.PACKAGE_FILES:
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("build input\n", encoding="utf-8")
        for directory in nas.SOURCE_DIRECTORIES:
            source = self.root / directory / "main.rs"
            source.parent.mkdir(parents=True, exist_ok=True)
            source.write_text("fn main() {}\n", encoding="utf-8")
        self.frontend = self.root / "target" / "frontend-output"
        self.frontend.mkdir(parents=True)
        (self.frontend / "index.html").write_text("<!doctype html><html><title>SiteMonitor</title></html>", encoding="utf-8")
        (self.frontend / "app.wasm").write_bytes(b"\0asm")

    def package(self):
        return nas.prepare_package(self.root, self.frontend, "test-release")

    def test_package_excludes_runtime_secrets_and_compilation_cache(self):
        private = (
            ".env", ".env.nas", "data/keys/personnel.key", "data/uploads/private.jpg",
            "target/debug/backend.exe", "backend/src/.env", "shared/src/keys/private.rs",
            "backend/src/private.db", "frontend/src/node_modules/private.rs",
        )
        for name in private:
            source = self.root / name
            source.parent.mkdir(parents=True, exist_ok=True)
            source.write_text("DO_NOT_PACKAGE_THIS_SECRET", encoding="utf-8")
        (self.frontend / ".env").write_text("DO_NOT_PACKAGE_THIS_SECRET", encoding="utf-8")
        (self.frontend / "private.pem").write_text("DO_NOT_PACKAGE_THIS_SECRET", encoding="utf-8")
        _, archive, manifest = self.package()
        with tarfile.open(archive) as bundle:
            self.assertIn("web/app.wasm", bundle.getnames())
            self.assertIn("backend/src/main.rs", bundle.getnames())
            for member in bundle.getmembers():
                self.assertTrue(nas.allowed_package_path(member.name), member.name)
                self.assertNotIn(b"DO_NOT_PACKAGE_THIS_SECRET", bundle.extractfile(member).read())
        self.assertNotIn(".env", manifest["files"])
        self.assertEqual(nas.validate_archive(archive)["release_id"], "test-release")

    def test_packaged_shell_entrypoint_has_linux_line_endings(self):
        (self.root / "docker/entrypoint.sh").write_bytes(b"#!/bin/sh\r\nset -eu\r\n")
        package, _, _ = self.package()
        self.assertEqual((package / "docker/entrypoint.sh").read_bytes(), b"#!/bin/sh\nset -eu\n")

    def dioxus_manifest(self, bundled_paths):
        assets = {f"source-{index}": [{"bundled_path": value}] for index, value in enumerate(bundled_paths)}
        (self.frontend.parent / ".manifest.json").write_text(json.dumps({"assets": assets}), encoding="utf-8")

    def test_dioxus_manifest_excludes_stale_hashed_assets(self):
        assets = self.frontend / "assets"
        assets.mkdir()
        (assets / "current.wasm").write_bytes(b"\0asm-current")
        (assets / "current.js").write_text("// current", encoding="utf-8")
        (assets / "stale.wasm").write_bytes(b"\0asm-stale")
        (assets / "stale.js").write_text("// stale", encoding="utf-8")
        font = self.frontend / "fonts/inter.woff2"
        font.parent.mkdir()
        font.write_bytes(b"font")
        self.dioxus_manifest(["current.wasm", "current.js", "../fonts\\inter.woff2"])
        _, _, manifest = self.package()
        self.assertIn("web/assets/current.wasm", manifest["files"])
        self.assertIn("web/assets/current.js", manifest["files"])
        self.assertIn("web/fonts/inter.woff2", manifest["files"])
        self.assertNotIn("web/assets/stale.wasm", manifest["files"])
        self.assertNotIn("web/assets/stale.js", manifest["files"])

    def test_dioxus_manifest_missing_current_asset_fails(self):
        self.dioxus_manifest(["missing.wasm"])
        with self.assertRaisesRegex(nas.DeploymentError, "current asset is missing"):
            self.package()

    def test_dioxus_manifest_rejects_escaping_bundled_paths(self):
        for name in ("../../.env", "/etc/passwd", "C:\\private.key", "nested/../../private.js"):
            with self.subTest(name=name):
                self.dioxus_manifest([name])
                with self.assertRaisesRegex(nas.DeploymentError, "unsafe bundled path"):
                    self.package()

    def test_missing_frontend_fails_before_replacing_package(self):
        (self.frontend / "app.wasm").unlink()
        with self.assertRaisesRegex(nas.DeploymentError, "Frontend bundle is missing"):
            self.package()
        self.assertFalse((self.root / "dist/nas").exists())

    def test_checksum_detects_changed_release_contents(self):
        package, archive, _ = self.package()
        (package / "backend/src/main.rs").write_text("tampered", encoding="utf-8")
        with tarfile.open(archive, "w:gz") as bundle:
            for source in package.rglob("*"):
                if source.is_file():
                    bundle.add(source, arcname=source.relative_to(package).as_posix())
        with self.assertRaisesRegex(nas.DeploymentError, "checksum mismatch"):
            nas.validate_archive(archive)

    def test_archive_rejects_traversal_and_links(self):
        for name, member_type in (("../.env", tarfile.REGTYPE), ("web/link.js", tarfile.SYMTYPE)):
            with self.subTest(name=name):
                archive = self.root / "unsafe.tar.gz"
                with tarfile.open(archive, "w:gz") as bundle:
                    member = tarfile.TarInfo(name)
                    member.type = member_type
                    member.linkname = "/etc/passwd"
                    bundle.addfile(member, io.BytesIO(b""))
                with self.assertRaisesRegex(nas.DeploymentError, "unsafe archive member"):
                    nas.validate_archive(archive)

    def test_path_allowlist_rejects_private_and_ambiguous_names(self):
        for name in ("/web/app.js", "web/../.env", "web\\app.js", "web//app.js", "web/.env",
                     "web/keys/private.json", "backend/src/key.pem", "data/personnel.db"):
            with self.subTest(name=name):
                self.assertFalse(nas.allowed_package_path(name))


class UploadTests(unittest.TestCase):
    def test_empty_file_is_created_and_shell_path_is_quoted(self):
        remote = "/volume1/docker/site monitor's/empty"
        with patch.object(nas, "run_ssh") as execute:
            nas.upload_stream(object(), io.BytesIO(b""), remote)
        execute.assert_called_once()
        self.assertEqual(execute.call_args.args[1], "umask 077; : > " + shlex.quote(remote))

    def test_upload_streams_bounded_chunks_on_stdin(self):
        payload = b"password-is-never-a-shell-argument"
        with patch.object(nas, "UPLOAD_CHUNK_SIZE", 5), patch.object(nas, "run_ssh") as execute:
            nas.upload_stream(object(), io.BytesIO(payload), "/volume1/docker/sitemonitor/.env.new")
        chunks = [call.kwargs["input_bytes"] for call in execute.call_args_list[1:]]
        self.assertEqual(b"".join(base64.b64decode(chunk) for chunk in chunks), payload)
        self.assertTrue(all(len(base64.b64decode(chunk)) <= 5 for chunk in chunks))
        self.assertTrue(all(payload.decode() not in call.args[1] for call in execute.call_args_list))

    def test_upload_failure_stops_subsequent_chunks(self):
        with patch.object(nas, "UPLOAD_CHUNK_SIZE", 2), patch.object(
                nas, "run_ssh", side_effect=[(0, "", ""), nas.DeploymentError("disk full")]) as execute:
            with self.assertRaisesRegex(nas.DeploymentError, "disk full"):
                nas.upload_stream(object(), io.BytesIO(b"abcdef"), "/release")
        self.assertEqual(execute.call_count, 2)

    def test_uploaded_archive_requires_matching_checksum(self):
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary) / "release.tar.gz"
            source.write_bytes(b"archive")
            with patch.object(nas, "upload_stream"), patch.object(nas, "run_ssh", return_value=(0, "0000  /release", "")):
                with self.assertRaisesRegex(nas.DeploymentError, "checksum did not match"):
                    nas.upload_file(object(), source, "/release")


class FakeChannel:
    def __init__(self, code=0):
        self.output = [b"build output\n"]
        self.errors = [b"diagnostic\n"]
        self.code = code
        self.closed = False

    def shutdown_write(self):
        pass

    def recv_ready(self):
        return bool(self.output)

    def recv(self, _):
        return self.output.pop(0)

    def recv_stderr_ready(self):
        return bool(self.errors)

    def recv_stderr(self, _):
        return self.errors.pop(0)

    def exit_status_ready(self):
        return not self.output and not self.errors

    def recv_exit_status(self):
        if self.output or self.errors:
            raise AssertionError("Exit status requested before output was drained")
        return self.code

    def close(self):
        self.closed = True


class SshExecutionTests(unittest.TestCase):
    def fake_ssh(self, channel):
        ssh = Mock()
        output = Mock(channel=channel)
        ssh.exec_command.return_value = (io.BytesIO(), output, Mock())
        return ssh

    def test_both_output_streams_are_drained_before_exit_status(self):
        channel = FakeChannel()
        result = nas.run_ssh(self.fake_ssh(channel), "docker build .")
        self.assertEqual(result, (0, "build output", "diagnostic"))
        self.assertTrue(channel.closed)

    def test_remote_nonzero_exit_is_a_failure(self):
        channel = FakeChannel(code=7)
        with self.assertRaisesRegex(nas.DeploymentError, "exit 7"):
            nas.run_ssh(self.fake_ssh(channel), "docker build .")
        self.assertTrue(channel.closed)

    def test_readiness_uses_container_probe_for_loopback_only_deployments(self):
        with patch.object(nas, "run_ssh", side_effect=[
                (0, '{"ok":true,"version":"1.0.0"}', ""),
                (0, "<!doctype html><html><title>SiteMonitor</title></html>", "")]) as execute:
            nas.wait_until_ready(object(), "compose -p sitemonitor", 3)
        for call in execute.call_args_list:
            self.assertIn("exec -T app curl", call.args[1])
            self.assertIn("http://127.0.0.1:8737/", call.args[1])


class ConfigurationTests(unittest.TestCase):
    def test_existing_environment_is_never_replaced_or_regenerated(self):
        with patch.object(nas, "run_ssh", return_value=(0, "", "")), patch.object(nas, "upload_stream") as upload:
            self.assertFalse(nas.ensure_remote_environment(object(), "/volume1/docker/sitemonitor", Mock(), "sitemonitor:new"))
        upload.assert_not_called()

    def test_generated_secrets_are_distinct_and_url_safe(self):
        args = argparse.Namespace(app_port=8737, bind_ip="0.0.0.0", secure_cookies=False)
        environments = [dict(line.split("=", 1) for line in nas.generated_environment(args, "sitemonitor:test").splitlines()
                             if line and not line.startswith("#")) for _ in range(2)]
        for key, minimum in (("POSTGRES_PASSWORD", 32), ("SITEMONITOR_ADMIN_PASSWORD", 24)):
            self.assertGreaterEqual(len(environments[0][key]), minimum)
            self.assertRegex(environments[0][key], r"^[A-Za-z0-9_-]+$")
            self.assertNotEqual(environments[0][key], environments[1][key])
        self.assertRegex(environments[0]["SITEMONITOR_DATA_KEY"], r"^[0-9a-f]{64}$")

    def test_project_directory_preserves_existing_service5_and_prevents_traversal(self):
        self.assertEqual(nas.remote_directory("/volume1/docker/site monitor"), "/volume1/docker/site monitor")
        for name in ("/volume1/docker/service5_db", "/volume1/docker", "/volume1/docker/sitemonitor/../service5_db", "relative"):
            with self.subTest(name=name), self.assertRaises(argparse.ArgumentTypeError):
                nas.remote_directory(name)

    def test_compose_command_quotes_paths_and_constrains_rollback_release(self):
        base = "/volume1/docker/site monitor's"
        filename = base + "/releases/one/docker-compose.yml"
        command = nas.compose_command("docker compose", base, filename, "sitemonitor:release-one")
        self.assertIn("--project-directory " + shlex.quote(base), command)
        self.assertIn("--env-file " + shlex.quote(base + "/.env"), command)
        self.assertIn("-p sitemonitor", command)
        with self.assertRaises(nas.DeploymentError):
            nas.compose_command("docker compose", base, base + "/releases/../../service5_db/docker-compose.yml", "sitemonitor:old")

    def test_readiness_rejects_login_html_and_malformed_health(self):
        html = b"<!doctype html><html><title>SiteMonitor</title></html>"
        self.assertTrue(nas.health_response_valid(b'{"ok":true}', html))
        self.assertFalse(nas.health_response_valid(b'{"ok":false}', html))
        self.assertFalse(nas.health_response_valid(b'{"ok":true}', b"<html>unrelated NAS login</html>"))
        self.assertFalse(nas.health_response_valid(b"<html>not json</html>", html))

    def test_failed_attempt_rolls_back_to_last_good_not_its_predecessor(self):
        args = nas.parser().parse_args(["--rollback", "--user", "test"])
        base = args.directory
        last_good = {"image": "sitemonitor:release-b", "previous_image": "sitemonitor:release-a",
                     "compose_file": base + "/releases/b/docker-compose.yml"}
        failed = {"attempted_image": "sitemonitor:release-c", "fallback_image": last_good["image"],
                  "fallback_compose_file": last_good["compose_file"]}
        with patch.object(nas, "connect_ssh", return_value=Mock()), \
                patch.object(nas, "discover_docker", return_value=("docker", "docker compose")), \
                patch.object(nas, "read_state", side_effect=[last_good, failed]), \
                patch.object(nas, "run_ssh", return_value=(0, "", "")) as execute, \
                patch.object(nas, "wait_until_ready"), patch.object(nas, "write_state") as write:
            nas.deploy(args, None, None)
        launch = next(call.args[1] for call in execute.call_args_list if "up -d app" in call.args[1])
        self.assertIn("SITEMONITOR_IMAGE=sitemonitor:release-b", launch)
        self.assertNotIn("release-a", launch)
        write.assert_not_called()


if __name__ == "__main__":
    unittest.main()
