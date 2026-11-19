# SiteMonitor

A two-sided Progressive Web App for running construction, maintenance and equipment across a large private estate.

- **The Showcase (estate owner):** a calm, document-style portal with an interactive estate map, decisions to approve, photo notes ("tap a spot, leave a note"), Quiet Days and handover sign-offs.
- **The Engine (office admin and site monitors):** a fast field tool for daily crew allocation, equipment moves and breakdown reports, phase-by-phase progress with 16:9 proof photos, and a review queue that turns field reports into owner decisions. It keeps working without signal.

Everything is Rust: an **Axum + SeaORM + PostgreSQL** backend and a **Dioxus 0.7 WebAssembly** frontend, sharing one `shared` crate of domain models.

```
Cargo.toml        workspace (shared, backend, frontend)
shared/           domain models, API contracts, WebSocket events, plain-language labels
backend/          Axum API, SeaORM entities + SQL migrations, seed, automation crons, WebSocket hub
frontend/         Dioxus PWA (Tailwind design tokens, IndexedDB offline queue, 16:9 canvas cropper)
e2e/              Playwright end-to-end tests
scripts/          cross-platform build + icon rendering
```

## Ports

| What | Port |
| --- | --- |
| App (UI, REST API `/api`, WebSocket `/ws`) | **8737** |
| PostgreSQL (development) | **54329** |

## Requirements

- Rust (stable) with the WASM target: `rustup target add wasm32-unknown-unknown`
- Dioxus CLI 0.7: `cargo install dioxus-cli@0.7.10 --locked`
- Node.js 18+ (Tailwind build and Playwright only)
- PostgreSQL 14+ reachable at `DATABASE_URL` (see `.env.example`)

Works on macOS and Windows. TLS uses `rustls`, so there is no OpenSSL dependency.

### Setting up the development database

Any PostgreSQL works. To match the defaults:

```sql
CREATE ROLE sitemonitor LOGIN PASSWORD 'sitemonitor_dev';
CREATE DATABASE sitemonitor OWNER sitemonitor;
```

Run it on port 54329, or set `DATABASE_URL` to wherever it is.

## Build and run

```bash
node scripts/build.mjs              # Tailwind CSS → Dioxus WASM bundle → release backend
cargo run -p backend --release      # migrates, seeds a demo estate on first start
```

Open **http://localhost:8737** and sign in.

### Signing in

- With `SITEMONITOR_DEMO=1` (the Windows start script sets it) the sign-in page lists the demo accounts. They all use the password `SiteMonitor-demo`. Turn demo mode off (`SITEMONITOR_DEMO=0`) and change those passwords before real use.
- Passwords are stored as Argon2 hashes. Sessions are server-side and last 14 days from last use. Browsers get an HttpOnly, SameSite=Lax cookie; API clients can send `Authorization: Bearer <token>` from the sign-in response. Behind HTTPS, set `SITEMONITOR_SECURE_COOKIES=1`.
- After 5 wrong passwords for one email, sign-in pauses for 15 minutes.
- The Office Admin adds people with a first password and can reset a password from **People**. Either way, the person must choose their own password at their next sign-in, and a reset signs them out everywhere.
- Locked out? `cargo run -p backend --release -- set-password <email> <new password>`

### Access

**Access** (Office Admin) is a matrix of 18 permissions × people. Everyone starts with their role's defaults. The admin can add or remove single permissions for one person, review the changes, and save. Every change is logged, the server checks the matrix on every action, nobody can remove their own "Change access", and at least one active person always keeps it.

On Windows you can also start the database and the app in the background with:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/start-windows.ps1
```

To reset the demo data at any time:

```bash
cargo run -p backend --release -- reset
```

## Tests

```bash
cargo test --workspace                       # unit tests (progress, 99% lock, cropper maths, conflicts…)
cargo check --workspace                      # compiles shared, backend and frontend
cd e2e && npm install && npx playwright install chromium
npx playwright test                          # 43 end-to-end tests (resets the database itself; needs SITEMONITOR_DEMO=1)
```

The end-to-end suite covers every phase of `PLAN.md`:

- the seed and API rules: weighted progress, the 99% handover lock, the Quiet Day engine, the manpower ledger, deadline shifts from breakdowns, cron jobs and role permissions;
- the owner portal: Action Center, map drill-down and layers, documents, photo pins, agenda and sign-off;
- the staff portal: crew board, equipment, phase updater, decision pipeline, Master Scheduler and Map Studio;
- sign-in, first-password change, sign-out and the access matrix;
- real-time pushes, offline queue and sync, the PWA manifest and service worker, and smoothness checks (no console errors, no layout shift on refresh).

## Data privacy

Real operational and personnel data never goes into Git.

- **Encrypted at rest.** Every personnel record (workers and volunteers, all workforce lists including the clinic, and workforce snapshots) is stored encrypted with AES-256-GCM. The key is `data/keys/personnel.key` (created on first start) or the `SITEMONITOR_DATA_KEY` environment variable. **Back the key up** somewhere safe and private: without it the personnel data cannot be read. `backend audit-data` reports any personnel data still readable without the key (all counts should be 0). Data stored before encryption was on is encrypted automatically at start-up.
- **Login accounts are separate.** Sign-in accounts live in their own table, are not encrypted, and are never wiped by Load sample data, Clear data or a restore, so a fresh clone is usable straight away.
- **Exports are encrypted.** Workforce backups (Settings → Workforce data) and whole-database packages (Settings → Backup & restore) are always encrypted with a passphrase (PBKDF2-HMAC-SHA256, 600,000 rounds, AES-256-GCM).
- **Sharing a database.** To move SiteMonitor to another machine, make a package in Settings → Backup & restore, send the `.smpkg` file and its passphrase to the other person separately (never through GitHub), and restore it there. Estate, projects, personnel, clinic records and photos come across; their local sign-in accounts are kept.
- **On the device.** Saves queued while offline are encrypted in the browser (WebCrypto AES-256-GCM, key kept on the device and not exportable). Browsers only offer WebCrypto over HTTPS or on `localhost`; on a plain-HTTP LAN address the queue is stored unencrypted, so serve SiteMonitor over HTTPS (for example behind a reverse proxy) when phones use it.
- **Git hygiene.** `.gitignore` excludes `data/` (database files, uploads, keys, backups), every database and dump format, backup and package files, snapshots, uploads, keys and `.env` files.

## Automations

These jobs run in the estate's local time:

| Time | Job |
| --- | --- |
| 00:00 | Clears yesterday's crew numbers (no "ghost workers") and queues the morning roster prompt. |
| 07:00 | Deadline monitor: staff warnings 7 days and 48 hours before each deadline, plus a "coming up" card for the owner at 48 hours. |
| 07:30 | Shows the "Allocate Today's Crew" prompt to the office admin. |
| 17:00 | The owner's daily digest. |

An Office Admin can also run the three jobs on demand through `POST /api/admin/jobs/{midnight-roster-reset|deadline-monitor|vip-daily-digest}/run`.
