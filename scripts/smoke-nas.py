#!/usr/bin/env python3
"""Smoke-test an isolated NAS container stack (never a production database)."""

import argparse
import json
import time
import urllib.error
import urllib.request
from pathlib import Path


def request(base, path, payload=None, token=None):
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = "Bearer " + token
    req = urllib.request.Request(
        base.rstrip("/") + path,
        data=json.dumps(payload).encode() if payload is not None else None,
        headers=headers,
    )
    try:
        with urllib.request.urlopen(req, timeout=10) as reply:
            return reply.status, reply.headers, reply.read()
    except urllib.error.HTTPError as error:
        return error.code, error.headers, error.read()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url", required=True)
    parser.add_argument("--env-file", type=Path)
    parser.add_argument("--web-dir", type=Path)
    parser.add_argument("--after-restart", action="store_true")
    parser.add_argument("--expect-database-down", action="store_true")
    args = parser.parse_args()

    if args.expect_database_down:
        assert request(args.url, "/api/health")[0] == 200, "Liveness must remain available"
        status, _, body = request(args.url, "/api/ready")
        assert status == 503 and json.loads(body)["ok"] is False, "Readiness must detect database failure"
        print("Database failure correctly returns 503 readiness and 200 liveness")
        return

    if not args.env_file or not args.web_dir:
        parser.error("--env-file and --web-dir are required for bootstrap checks")
    for attempt in range(60):
        try:
            status, _, body = request(args.url, "/api/ready")
            if status == 200 and json.loads(body).get("ok") is True:
                break
        except (OSError, ValueError):
            pass
        time.sleep(2)
    else:
        raise RuntimeError("The isolated deployment never became ready")

    status, headers, body = request(args.url, "/")
    assert status == 200 and "text/html" in headers.get("Content-Type", "")
    assert b"SiteMonitor" in body, "The compiled frontend must be served"
    wasm = next(args.web_dir.rglob("*.wasm"))
    status, headers, body = request(args.url, "/" + wasm.relative_to(args.web_dir).as_posix())
    assert status == 200 and body.startswith(b"\x00asm"), "The real WebAssembly bundle must be served"
    assert "application/wasm" in headers.get("Content-Type", "")
    options = json.loads(request(args.url, "/api/auth/options")[2])
    assert not options.get("demo_accounts") and not options.get("demo_password"), "Demo accounts must stay hidden"

    env = dict(
        line.split("=", 1)
        for line in args.env_file.read_text().splitlines()
        if line.strip() and not line.lstrip().startswith("#")
    )
    bootstrap = env["SITEMONITOR_ADMIN_PASSWORD"]
    replacement = "ci-changed-" + bootstrap
    password = replacement if args.after_restart else bootstrap
    status, _, body = request(args.url, "/api/auth/login", {"username": "marco", "password": password})
    assert status == 200, "The isolated administrator must be able to sign in"
    session = json.loads(body)
    if args.after_restart:
        assert not session["user"]["must_change_password"], "Password change must persist after recreation"
        status, _, _ = request(args.url, "/api/auth/login", {"username": "marco", "password": bootstrap})
        assert status == 401, "Bootstrap credentials must never overwrite an existing account"
    else:
        assert session["user"]["must_change_password"], "Initial administrator must choose a new password"
        status, _, _ = request(
            args.url,
            "/api/auth/password",
            {"current_password": bootstrap, "new_password": replacement},
            token=session["token"],
        )
        assert status == 200, "Initial administrator must be able to change the bootstrap password"
    print("Rust server, database, WebAssembly, and production account checks passed")


if __name__ == "__main__":
    main()
