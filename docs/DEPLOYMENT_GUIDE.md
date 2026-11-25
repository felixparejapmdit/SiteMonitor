# SiteMonitor on the Synology NAS

This deployment follows `01_Rust_WC_DB`'s Synology DS423+ setup at `10.45.1.3`. SiteMonitor runs as its own Container Manager project at `/volume1/docker/sitemonitor`, on port `8737`. The existing Service 5 application at port `8105` keeps its own containers and storage.

SiteMonitor's application server is compiled Rust. The server provides the API, WebSocket endpoint and Dioxus WebAssembly frontend; PostgreSQL runs in a second container.

```text
Browser / installed PWA
        |
        | HTTPS through DSM reverse proxy
        v
Synology DS423+ / Container Manager
  sitemonitor app :8737  ---->  PostgreSQL :5432
       |                       |
       | /app/data             | database storage
       v                       v
  /volume1/docker/sitemonitor/data/
  /volume1/docker/sitemonitor/postgres/

  /volume1/docker/sitemonitor/.env   passwords and personnel encryption key
```

The database port is only available on the Compose network. The application port is configurable.

## Prerequisites

On the development PC:

- Python 3.11 or newer, Rust, the `wasm32-unknown-unknown` target, Dioxus CLI `0.7.10`, and Node.js, as described in the [project requirements](../README.md#what-you-need).
- Access to the NAS over the office network.
- Paramiko for the SSH wizard: `python -m pip install -r scripts/requirements-nas.txt`. Preparing a package alone does not require Paramiko.

On the NAS:

- Synology Container Manager, a running Docker daemon, and the Compose plugin or `docker-compose` command.
- SSH enabled under **Control Panel > Terminal & SNMP**, with a DSM account allowed to use Docker and write to `/volume1/docker/sitemonitor`.
- Internet access for the Rust and PostgreSQL container images and Cargo dependencies during the first build.
- Enough free disk space and memory for a Rust release build. The Dockerfile limits build concurrency to one job and disables LTO to reduce memory use. A first build can take a while on the DS423+; leave the deployment terminal open.

The frontend is built on the PC. The Linux backend is built inside Docker on the NAS, so neither Docker Desktop nor WSL is required on the PC. Do not upload the Windows `backend.exe` as a NAS executable.

## Deploy from Windows

From the SiteMonitor project folder:

```powershell
python -m pip install -r scripts/requirements-nas.txt
python deploy-to-nas.py
```

You can also double-click `deploy-to-nas.bat`. On macOS/Linux, run `bash deploy-to-nas.sh`.

The wizard defaults to `10.45.1.3`, SSH port `22`, application port `8737`, and `/volume1/docker/sitemonitor`. Use `python deploy-to-nas.py --help` for overrides.

The wizard:

1. Builds a fresh production frontend with `node scripts/build.mjs --skip-backend`.
2. Creates an allow-listed package of source code and generated web files. When available, Dioxus's release manifest selects the current hashed assets, excluding bundles left by older builds. Local databases, uploads, personnel keys, `.env` files, Git metadata, dependencies and build caches are excluded.
3. Connects over SSH and checks the server's host key. Verify a new fingerprint against the NAS before accepting it.
4. Uploads and verifies the package in a new `releases/<release-id>` directory. Transfers use SSH commands rather than requiring the NAS's SFTP subsystem.
5. Creates the private NAS `.env` on first deployment, with generated database and administrator passwords and a personnel encryption key. An existing `.env` is retained.
6. Builds the Linux application image before replacing the running application, then starts the `sitemonitor` Compose project with persistent storage under the base directory.
7. Checks database readiness and the application page through the established SSH connection, inside the app container. This also works when only DSM's HTTPS proxy is publicly accessible. A failed build or failed verification exits with an error; staff-facing HTTPS access is checked separately.

The deployment never copies your PC's development database. A new NAS database starts with an empty estate. To move existing data, use SiteMonitor's encrypted backup/restore flow described below.

### First sign-in

For initial LAN setup, open `http://10.45.1.3:8737`. Sign in as **`marco`**, using the generated `SITEMONITOR_ADMIN_PASSWORD` in the NAS's private `.env`. Change the bootstrap password when prompted, then create your staff accounts or activate the accounts you need.

Production mode activates only the initial Office Admin. The initial owner and monitor accounts remain inactive. `SITEMONITOR_ADMIN_PASSWORD` initializes an empty database; editing it later does **not** reset an existing account password. Keep it configured so subsequent starts pass production configuration validation.

Before staff use, configure HTTPS as described below. The direct HTTP address is for initial setup and troubleshooting.

## Prepare a package without connecting to the NAS

```powershell
python deploy-to-nas.py --prepare-only
```

Outputs:

- `dist/nas/`: a self-contained Docker build directory.
- `dist/sitemonitor-nas.tar.gz`: the same files as a transfer archive, without an enclosing directory.

This command builds the frontend and performs no SSH connection or NAS changes. It produces no passwords or keys. `--skip-frontend-build` explicitly reuses an existing production bundle; use it only when that bundle matches the source revision you intend to deploy.

Do not add `.env`, database files, uploads or encryption keys to the prepared package. Secret configuration is created on the NAS separately.

## Manual upload and launch

This path is useful when you want to inspect the prepared package or use File Station for the upload.

1. Run `python deploy-to-nas.py --prepare-only` on the PC.
2. In DSM File Station, create `/docker/sitemonitor` and upload the **contents** of `dist/nas/` there.
3. Copy `.env.nas.example` to `.env` in that directory. Replace every secret placeholder. Generate independent values on a trusted PC with:

   ```powershell
   python -c "import secrets; print('POSTGRES_PASSWORD=' + secrets.token_urlsafe(32)); print('SITEMONITOR_ADMIN_PASSWORD=' + secrets.token_urlsafe(24)); print('SITEMONITOR_DATA_KEY=' + secrets.token_hex(32))"
   ```

   Use a URL-safe database password because it becomes part of `DATABASE_URL`. The administrator password must have at least 16 characters; the generated value satisfies the application's password rules. The encryption key must be exactly 64 hexadecimal characters. Keep `.env` private and outside Git.

4. SSH into the NAS and build the image before starting the project:

   ```sh
   cd /volume1/docker/sitemonitor
   chmod 600 .env
   DOCKER=/var/packages/ContainerManager/target/usr/bin/docker
   "$DOCKER" build --tag sitemonitor:local .
   "$DOCKER" compose --project-name sitemonitor --env-file .env config --quiet
   "$DOCKER" compose --project-name sitemonitor --env-file .env up -d
   ```

   Keep `SITEMONITOR_IMAGE=sitemonitor:local` for these commands. If Docker is on `PATH`, use `docker` in place of the full path. If the NAS has the standalone Compose command, replace `"$DOCKER" compose` with its `docker-compose` executable.

5. Check the containers and endpoints:

   ```sh
   "$DOCKER" compose --project-name sitemonitor --env-file .env ps
   curl --fail http://127.0.0.1:8737/api/ready
   curl --fail http://127.0.0.1:8737/api/health
   ```

   `/api/ready` checks PostgreSQL connectivity and returns HTTP 503 if unavailable. `/api/health` reports HTTP liveness and the application version.

You can manage this manual deployment in DSM **Container Manager > Project > Create**, selecting `/docker/sitemonitor` and its existing `docker-compose.yml`. Build `sitemonitor:local` first: the Compose project starts that image; the GUI does not create the Rust application image for you.

## Configuration and persistent storage

The NAS uses `.env.nas.example` as its configuration template. The development `.env.example` is for the local Windows/macOS/Linux launcher.

| Setting | Purpose |
| --- | --- |
| `POSTGRES_PASSWORD` | Password for the `sitemonitor` PostgreSQL role. Required; no shared development password is used. |
| `SITEMONITOR_ADMIN_PASSWORD` | Bootstrap password for `marco` on an empty production database. |
| `SITEMONITOR_DATA_KEY` | Stable 64-character hexadecimal personnel encryption key. Preserve it across deployments and restores. |
| `SITEMONITOR_IMAGE` | Application image tag. The wizard selects the newly built release image. |
| `SITEMONITOR_PORT` | NAS application port, normally `8737`. |
| `SITEMONITOR_BIND_IP` | NAS address on which the application port is published. Use `127.0.0.1` when DSM reverse proxy is the only access path. |
| `SITEMONITOR_SECURE_COOKIES` | Initially `0` for LAN setup; set `1` when using HTTPS. |
| `TZ` | Container timezone, default `Asia/Singapore`. |

Compose enables `SITEMONITOR_PRODUCTION=1`, disables demo mode, and supplies the internal PostgreSQL connection URL. Production startup validates its settings before touching the database. It also refuses the `reset-sample` command.

| NAS location | Contents | Preserve during updates |
| --- | --- | --- |
| `.env` | Database password, bootstrap secret and personnel encryption key | Yes |
| `postgres/` | PostgreSQL database files | Yes |
| `data/` | Uploaded photos, maps and any file-based application keys | Yes |
| `releases/` | Uploaded source and web bundles | Keep the current and previous release for recovery |
| `.deployment-state.json` | Wizard's last successful release/image and previous image | Yes, for deployment operations |
| `.deployment-attempt.json` | Incomplete update and last successful fallback, if an update failed | Keep until successful deployment or rollback clears it |

The wizard's Compose commands use the base directory as `--project-directory`, so staged releases share the same `postgres/` and `data/` storage. Source uploads do not overwrite either directory.

Changing `POSTGRES_PASSWORD` in `.env` does not change the password inside an already initialized PostgreSQL database. Change the database role's password and `.env` together during maintenance. Changing `SITEMONITOR_DATA_KEY` makes existing encrypted personnel records unreadable; restore the original key instead of generating a new one.

## HTTPS and WebSocket access

Set up a DNS name that resolves to the NAS and a certificate trusted by staff devices. In DSM 7:

1. Open **Control Panel > Login Portal > Advanced > Reverse Proxy** and create a SiteMonitor rule.
2. Set the source to HTTPS, your SiteMonitor hostname, and port `443`.
3. Set the destination to HTTP, `127.0.0.1`, and port `8737`.
4. Under **Custom Header**, choose **Create > WebSocket**. SiteMonitor uses `/ws` for live updates; preserve the request paths through the proxy.
5. Assign the certificate to the SiteMonitor hostname under **Control Panel > Security > Certificate**.
6. Set `SITEMONITOR_SECURE_COOKIES=1` in `/volume1/docker/sitemonitor/.env` and recreate the app service using the same release's Compose command.

These DSM controls are described in [Synology's reverse proxy documentation](https://kb.synology.com/en-global/DSM/help/DSM/AdminCenter/system_login_portal_advanced).

Use the HTTPS hostname for staff bookmarks and PWA installation. Service workers and browser encryption require a secure context on other devices; an HTTP LAN IP does not provide one. If you set `SITEMONITOR_BIND_IP=127.0.0.1`, the direct LAN port will no longer be accessible, and staff use the reverse proxy hostname.

Keep PostgreSQL internal to the project. Publish only the application access required by your office network.

## Update and rollback

Back up the database, encryption key and uploads before an update. Run the deployment wizard again from the updated source. It stages and builds a new image before updating the application, retains the NAS `.env`, and leaves persistent storage in place. The deployment state records the previous image for an application rollback.

To restore the last successful release after an incomplete update, or revert a completed deployment to the previous recorded release:

```powershell
python deploy-to-nas.py --rollback
```

The wizard uses `.deployment-attempt.json` to select the last successful fallback after a failed update; otherwise it selects the previous completed release. It does not rebuild the frontend and does not revert database migrations. Check schema compatibility and keep the matching database backup before using it.

To run maintenance commands against a wizard deployment, read `/volume1/docker/sitemonitor/.deployment-state.json` for the successful `compose_file`, then define a shell helper using that exact path:

```sh
BASE=/volume1/docker/sitemonitor
DOCKER=/var/packages/ContainerManager/target/usr/bin/docker
COMPOSE_FILE="$BASE/releases/REPLACE_WITH_RELEASE_ID/docker-compose.yml"
export SITEMONITOR_IMAGE=REPLACE_WITH_CURRENT_IMAGE_TAG
compose() {
    "$DOCKER" compose --project-name sitemonitor \
        --project-directory "$BASE" --env-file "$BASE/.env" \
        --file "$COMPOSE_FILE" "$@"
}
```

Take both the Compose file and current image tag from `.deployment-state.json`. For a manual deployment, use `COMPOSE_FILE="$BASE/docker-compose.yml"` and `SITEMONITOR_IMAGE=sitemonitor:local`. If necessary, adapt the helper to the standalone `docker-compose` executable. Keep the same Compose project name and base directory for every maintenance command.

The `SITEMONITOR_IMAGE` shell variable can override the image in `.env`. To manually roll back a completed update, first confirm the previous image is present, set its exact tag from the deployment state, then recreate only the app:

```sh
export SITEMONITOR_IMAGE=REPLACE_WITH_PREVIOUS_IMAGE_TAG
"$DOCKER" image inspect "$SITEMONITOR_IMAGE" >/dev/null
compose up -d --no-deps app
curl --fail http://127.0.0.1:8737/api/ready
```

Keep that image selection configured until you intentionally update again. Do not prune the last working image.

Database migrations run automatically at application startup. An older image may not support a newer database schema; reverting the image alone is not a database rollback. When schema compatibility is uncertain, restore the matching pre-update database backup, original encryption key and uploads in a maintenance window. Do not remove the data directories or run `reset` to roll back.

## Backups and restoring data

For normal exports and moving the local estate to the NAS, use **Settings > Backup DB > Download encrypted package**. Restore it through the NAS application's corresponding restore screen, using its passphrase. This transfers application records and uploads without copying the PC's development PostgreSQL files or overwriting the NAS encryption key.

After restoring existing accounts, review their active status and passwords before granting staff access. Production bootstrap initializes an empty database; it does not rewrite restored accounts.

For disaster recovery, preserve all three pieces together:

- A PostgreSQL logical dump.
- The NAS `.env`, including the original `SITEMONITOR_DATA_KEY`.
- The matching contents of `data/`, including uploads and any file-based keys.

Store these backups privately and copy them to a separate backup destination. A database dump alone cannot decrypt personnel records without the original key. Copying a live `postgres/` directory is not a substitute for a logical dump or a properly configured PostgreSQL-aware snapshot.

After defining the `compose` helper above, the following creates a consistent maintenance backup. The application is stopped while the database and uploads are captured and restarted when the block exits:

```sh
(
    set -eu
    STAMP=$(date +%Y%m%d-%H%M%S)
    BACKUP="$BASE/backups/$STAMP"
    umask 077
    mkdir -p "$BACKUP"
    compose stop app
    trap 'compose start app' EXIT
    compose exec -T db pg_dump --username=sitemonitor \
        --dbname=sitemonitor --format=custom > "$BACKUP/database.dump"
    cp "$BASE/.env" "$BACKUP/environment.env"
    if [ -f "$BASE/.deployment-state.json" ]; then
        cp "$BASE/.deployment-state.json" "$BACKUP/deployment-state.json"
    fi
    compose run --rm --no-deps -T --user 10001:10001 --entrypoint tar \
        app -czf - -C /app/data . > "$BACKUP/app-data.tar.gz"
)
```

Keep the successful release metadata and its image tag with the backup. Synology Hyper Backup or a scheduled DSM task can copy the resulting backup directory to your chosen backup destination.

For a full restore, first use an isolated recovery environment or a replacement installation with empty PostgreSQL storage and no running SiteMonitor application. Restore the matching `.env` privately, select the matching release image and Compose file, then:

1. Start only the database with `compose up -d db` and wait until it is healthy.
2. Restore the logical dump into the empty database:

   ```sh
   compose exec -T db pg_restore --exit-on-error --no-owner --no-privileges \
       --username=sitemonitor --dbname=sitemonitor < /path/to/database.dump
   ```

3. Restore uploads and file-based keys before starting the application:

   ```sh
   compose run --rm --no-deps -T --user 0 --entrypoint sh app -c \
       'tar -xzf - -C /app/data && chown -R 10001:10001 /app/data' \
       < /path/to/app-data.tar.gz
   ```

4. Start the application with `compose up -d app`, verify readiness, sign in, and open a restored photo and personnel record.

Do not restore over the running production database. When rehearsing a restore on the same NAS, isolate its project name, published port and storage from the live project.

## Deployment checks

Before sharing the NAS address with staff, verify:

- `app` and `db` are healthy in Container Manager.
- `/api/ready` and the homepage work through the HTTPS hostname.
- The bootstrap administrator can sign in and complete the password change; staff access has been configured.
- Upload a photo, restart the app, and confirm the photo and encrypted personnel records remain readable.
- Live changes reach another signed-in browser through WebSocket.
- An encrypted backup can be downloaded and restored in a separate test environment.
- A NAS/container restart brings the services back with the same records, uploads and key.

The repository provides the deployment package and verification tools. A successful local build or package preparation does not establish that a particular NAS is running the release; verify the actual NAS deployment using these checks.

The GitHub Actions workflow `.github/workflows/nas-container.yml` builds and smoke-tests the Linux container in an isolated environment. It exercises production bootstrap, database readiness and account persistence. This CI check is separate from verifying the NAS's network, proxy and storage configuration; inspect the workflow result for the release you deploy.

## Troubleshooting

| Problem | Check |
| --- | --- |
| SSH authentication or Docker permission failure | Enable SSH, verify the DSM account and host fingerprint, and grant only the Docker/directory access needed for deployment. |
| SFTP unavailable | The wizard uploads through SSH commands; no SFTP service is required. |
| Application port already in use | Choose a different `SITEMONITOR_PORT`; keep Service 5's `8105` unchanged. |
| Build exits with an out-of-memory error | Stop the build, free NAS resources or use a machine with more memory to build the Linux image, then transfer the image with Docker save/load. |
| `db` is unhealthy | Check `compose logs db`, storage permissions, free space and existing database credentials. |
| `app` is unhealthy or `/api/ready` returns 503 | Check `compose logs app` and confirm PostgreSQL is healthy. |
| Initial administrator cannot sign in | On a fresh production database use `marco` and the NAS bootstrap password. On an existing database the current account password is retained. |
| Sign-in stops working after enabling secure cookies | Use the HTTPS hostname; secure cookies are not sent over the direct HTTP LAN URL. |
| Live changes do not reach other browsers | Enable WebSocket headers on the DSM reverse proxy and verify `/ws` in the browser network panel. |
| Personnel records cannot be decrypted | Restore the original `SITEMONITOR_DATA_KEY` or matching personnel key file. Do not generate another key. |
| Records disappear after redeployment | Confirm the Compose project directory is the original base directory, and check the bind paths. Do not initialize a replacement database over the original storage. |
