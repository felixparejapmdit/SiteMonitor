# SiteMonitor

A Progressive Web App for running construction, maintenance, equipment and the workforce across a large private estate.

- **The Showcase (estate owner):** a calm, document-style portal with an interactive estate map, decisions to approve, photo notes ("tap a spot, leave a note"), Quiet Days and handover sign-offs.
- **The Engine (office admin and site monitors):** a fast field tool for daily crew allocation, equipment moves and breakdown reports, phase-by-phase progress with 16:9 proof photos, and a review queue that turns field reports into owner decisions. It keeps working without signal.
- **Workforce (merged from Service 5 DB):** assignments, personnel, project groups, visitations, overnight duty, overtime, signatories, ID cards and job orders, attendance, update forms, history and dependability, and an optional clinic ledger.

Everything is Rust: an **Axum + SeaORM + PostgreSQL** backend and a **Dioxus 0.7 WebAssembly** frontend, sharing one `shared` crate of domain models.

---

## Contents

1. [Quick start](#quick-start)
2. [What you need](#what-you-need)
3. [One-time database setup](#one-time-database-setup)
4. [Build](#build)
5. [Start, stop and update](#start-stop-and-update)
6. [Sign in](#sign-in)
7. [Data: empty, sample, clear](#data-empty-sample-clear)
8. [Using it on phones and tablets](#using-it-on-phones-and-tablets)
9. [Configuration](#configuration)
10. [Before real use](#before-real-use)
11. [Access and modules](#access-and-modules)
12. [Printing](#printing)
13. [Data privacy and backups](#data-privacy-and-backups)
14. [Tests](#tests)
15. [Automations](#automations)
16. [Troubleshooting](#troubleshooting)
17. [Project layout](#project-layout)

---

## Quick start

Once the [requirements](#what-you-need) and the [database](#one-time-database-setup) are in place:

**Windows**

```powershell
node scripts/build.mjs
powershell -ExecutionPolicy Bypass -File scripts/start-windows.ps1
```

**macOS / Linux**

```bash
node scripts/build.mjs
cargo run -p backend --release
```

Open **http://localhost:8737**, sign in as the Office Admin (see [Sign in](#sign-in)), and go to **Settings → Estate data → Load sample data** if you want something to look at.

| What | Address / port |
| --- | --- |
| App (UI, REST API `/api`, WebSocket `/ws`) | http://localhost:**8737** |
| PostgreSQL (development) | localhost:**54329** |

---

## What you need

| Tool | Version | Install |
| --- | --- | --- |
| Rust | stable | https://rustup.rs |
| WASM target | — | `rustup target add wasm32-unknown-unknown` |
| Dioxus CLI | 0.7.10 | `cargo install dioxus-cli@0.7.10 --locked` |
| Node.js | 18 or newer | https://nodejs.org (used for Tailwind CSS and the tests) |
| PostgreSQL | 14 or newer | see below |

Works on Windows, macOS and Linux. TLS uses `rustls`, so there is no OpenSSL dependency.

Check the tools:

```bash
cargo --version && dx --version && node --version
```

---

## One-time database setup

SiteMonitor needs a PostgreSQL database named `sitemonitor`, owned by a role `sitemonitor` with password `sitemonitor_dev`, on port **54329**. Any PostgreSQL works; the app creates its tables itself on first start. (To use another address, set `DATABASE_URL`, see [Configuration](#configuration).)

### Windows (portable PostgreSQL, what the start script expects)

1. Download the PostgreSQL **zip archive** (binaries) for Windows from https://www.enterprisedb.com/download-postgresql-binaries and unzip it so that `pg_ctl.exe` is at `%USERPROFILE%\tools\pgsql\bin\pg_ctl.exe`.
2. Create the data folder and the database (PowerShell):

   ```powershell
   $pg = "$env:USERPROFILE\tools\pgsql\bin"
   & "$pg\initdb.exe" -D "$env:USERPROFILE\tools\sitemonitor-pgdata" -U postgres -A trust -E UTF8
   & "$pg\pg_ctl.exe" -D "$env:USERPROFILE\tools\sitemonitor-pgdata" -o "-p 54329" -l "$env:USERPROFILE\tools\sitemonitor-pg.log" start
   & "$pg\psql.exe" -p 54329 -U postgres -c "CREATE ROLE sitemonitor LOGIN PASSWORD 'sitemonitor_dev';"
   & "$pg\psql.exe" -p 54329 -U postgres -c "CREATE DATABASE sitemonitor OWNER sitemonitor;"
   ```

3. Make the port permanent: open `%USERPROFILE%\tools\sitemonitor-pgdata\postgresql.conf` and set `port = 54329`.

The start script (`scripts/start-windows.ps1`) starts this PostgreSQL for you whenever it is not running. If your PostgreSQL lives elsewhere, set `SITEMONITOR_PG_BIN` (the `bin` folder) and `SITEMONITOR_PG_DATA` (the data folder) before running it.

### macOS (Homebrew)

```bash
brew install postgresql@16
brew services start postgresql@16           # runs on 5432 by default
psql postgres -c "CREATE ROLE sitemonitor LOGIN PASSWORD 'sitemonitor_dev';"
psql postgres -c "CREATE DATABASE sitemonitor OWNER sitemonitor;"
export DATABASE_URL=postgres://sitemonitor:sitemonitor_dev@localhost:5432/sitemonitor
```

### Linux (Debian / Ubuntu)

```bash
sudo apt install postgresql
sudo -u postgres psql -c "CREATE ROLE sitemonitor LOGIN PASSWORD 'sitemonitor_dev';"
sudo -u postgres psql -c "CREATE DATABASE sitemonitor OWNER sitemonitor;"
export DATABASE_URL=postgres://sitemonitor:sitemonitor_dev@localhost:5432/sitemonitor
```

Tip: put `DATABASE_URL` (and any other setting) in a `.env` file in the project folder instead of exporting it each time; copy `.env.example` to start.

---

## Build

```bash
node scripts/build.mjs
```

This runs three steps: Tailwind CSS → the Dioxus WebAssembly bundle → the release backend. The first build takes several minutes; later ones are much faster.

| Option | Use when |
| --- | --- |
| `node scripts/build.mjs --skip-backend` | you only changed the frontend (`frontend/`) |
| `node scripts/build.mjs --skip-frontend` | you only changed the backend (`backend/`, `shared/`) |

On Windows, **stop the running app before building the backend** (the running `backend.exe` locks the file):

```powershell
Get-Process backend -ErrorAction SilentlyContinue | Stop-Process -Force
```

---

## Start, stop and update

### Start

**Windows** (starts PostgreSQL if needed, then the app, both in the background):

```powershell
powershell -ExecutionPolicy Bypass -File scripts/start-windows.ps1
```

If the app was never built, the script builds it first. Logs go to `data\backend.log` and `data\backend.err.log`.

**macOS / Linux** (in a terminal; `Ctrl+C` stops it):

```bash
cargo run -p backend --release
```

The app runs in **live mode** (the sign-in page does not list any accounts). Add `SITEMONITOR_DEMO=1` in front of the command, or set it before the Windows start script, for demo mode, which lists the accounts on the sign-in page; the end-to-end tests need it.

On every start the app applies database migrations, creates the sign-in accounts on a brand-new database, and encrypts any personnel data that is not yet encrypted.

### Check it is running

```bash
curl http://localhost:8737/api/health      # {"ok":true,...}
```

### Stop

**Windows**

```powershell
Get-Process backend -ErrorAction SilentlyContinue | Stop-Process -Force
& "$env:USERPROFILE\tools\pgsql\bin\pg_ctl.exe" -D "$env:USERPROFILE\tools\sitemonitor-pgdata" stop   # optional: stop PostgreSQL too
```

**macOS / Linux:** `Ctrl+C` in the terminal running the app.

### Update to the latest version

```bash
git pull
node scripts/build.mjs          # stop the app first on Windows
```

Then start it again. Database changes are applied automatically on start. After an update, browsers pick up the new version on their next load; if a page looks stale, reload it once.

---

## Sign in

People sign in with a **username** and password. A new database has four accounts:

| Person | Role | Username |
| --- | --- | --- |
| Victoria Ashford | Estate owner (VIP client) | `victoria` |
| Marco Villanueva | Office Admin | `marco` |
| Jun Dela Cruz | Site Monitor | `jun` |
| Ana Santos | Site Monitor | `ana` |

On a brand-new database they all start with the password **`SiteMonitor-demo`**; change them right away (**People → Reset password**, or `set-password` below). Usernames are 3 to 40 letters, digits, dots, dashes or underscores; passwords need at least 8 characters. In demo mode (`SITEMONITOR_DEMO=1`) the sign-in page lists the accounts.

- Passwords are stored as Argon2 hashes. Sessions are server-side and last 14 days from last use. Browsers get an HttpOnly, SameSite=Lax cookie; API clients can send `Authorization: Bearer <token>` from the sign-in response. Behind HTTPS, set `SITEMONITOR_SECURE_COOKIES=1`.
- After 5 wrong passwords for one username, sign-in pauses for 15 minutes.
- The Office Admin adds people with a first password and can reset a password from **People**. Either way, the person must choose their own password at their next sign-in, and a reset signs them out everywhere.
- Locked out? Set a new password from the command line:

  ```bash
  cargo run -p backend --release -- set-password <username> <new password>
  ```

---

## Data: empty, sample, clear

A fresh install starts **empty**: only the sign-in accounts, a blank map and no workforce.

In the app, as the Office Admin, **Settings → Estate data**:

- **Load sample data** fills every module at once: the estate (map, zones, projects, photos, equipment, schedule, decisions) and a linked workforce (people, project groups tied to the sample jobs, duty rosters, attendance, overtime, visitations, signatories, pick lists and clinic records). A snapshot of the previous workforce is kept first.
- **Clear data** empties every module again (estate and workforce, including snapshots), keeping the sign-in accounts and settings.

Both ask you to type a confirmation word first.

From the command line (stop the app first; **this replaces everything, sign-in accounts included**, and recreates the four accounts above):

```bash
cargo run -p backend --release -- reset          # empty, like a fresh install
cargo run -p backend --release -- reset-sample   # with the sample estate (the tests use it)
```

Other command-line tools:

```bash
cargo run -p backend --release -- audit-data     # counts personnel data readable without the key (all should be 0)
cargo run -p backend --release -- parity-check path/to/service5.db   # is every record and field of that file held here?
```

Bringing in real Service 5 data: **Settings → Restore DB → Import a workforce backup** accepts any Service 5 JSON backup (any version), its encrypted `.agwcz` backups, and its `service5.db` file (close Service 5 first). A snapshot is taken first, so an import can be undone in **Settings → Daily Snapshot**. Afterwards, `parity-check` compares the file with the database, collection by collection and field by field, and ends with `PARITY OK` when nothing was dropped or changed (it prints names and counts only, never personal details). Service 5's per-device screen state (dismissed dashboard alerts) and its sync bookkeeping (`sync_audit_log`) are not imported.

Every Service 5 backup generation restores: v1 (a bare list of workers), v2 (camelCase stores such as `workersData`), v3 (string versions, the old vaccine shape), v4 (with `syncMetadata`), v11 and `service5.db` (the `wc_` storage keys), its daily snapshot files, and encrypted `.agwcz` backups (with their passphrase).

**Merge review.** Importing without "Replace everything" first shows Service 5's differential review: where the file came from, how many people and attendance entries it adds, how many sections are newer, and any true collisions (the same section changed here and in the file within a minute). For each collision choose keep local, accept incoming (the default) or smart merge, or set them all at once. People are then merged section by section (personal, assignment, health), newest first, with attendance and assignment histories combined, never shortened. A snapshot is taken first; **Download checkpoint** also saves an encrypted copy.

Backups and database files stay on each PC: `data/`, `*.dump`, `*.db`, `*.agwcz`, `*.smpkg` and keys are git-ignored and never committed.

---

## Using it on phones and tablets

- On the same network, open `http://<this computer's IP address>:8737` (find the IP with `ipconfig` on Windows or `ipconfig getifaddr en0` on macOS). Allow port 8737 through the firewall if asked.
- Phones get a bottom bar (four pages + **More**) and a menu button that opens the full sidebar. Add the app to the home screen from the browser menu to use it like an app; it keeps working offline and sends saved changes when the signal returns.
- **Use HTTPS for phones in real use.** Browsers only allow the offline queue's encryption over HTTPS (or on `localhost`); on a plain `http://` network address, changes saved offline are kept unencrypted on the phone. Put SiteMonitor behind a reverse proxy with a certificate (for example Caddy or nginx) and set `SITEMONITOR_SECURE_COOKIES=1`.

---

## Configuration

Settings are read from environment variables or a `.env` file in the project folder (copy `.env.example`). All have working defaults.

| Variable | Default | What it does |
| --- | --- | --- |
| `DATABASE_URL` | `postgres://sitemonitor:sitemonitor_dev@localhost:54329/sitemonitor` | Where the database is |
| `PORT` | `8737` | The port the app listens on |
| `STATIC_DIR` | `target/dx/frontend/release/web/public` | The built web app |
| `UPLOAD_DIR` | `data/uploads` | Uploaded photos and map renders |
| `DATA_KEY_FILE` | `data/keys/personnel.key` | The personnel encryption key (created on first start) |
| `SITEMONITOR_DATA_KEY` | — | The key itself (64 hex characters); used instead of the key file |
| `SITEMONITOR_DEMO` | off | `1` lists the accounts on the sign-in page (demo mode; the tests need it) |
| `SITEMONITOR_SECURE_COOKIES` | off | Secure cookies; turn on behind HTTPS |
| `RUST_LOG` | `backend=info,tower_http=warn` | Log detail |
| `SITEMONITOR_PG_BIN`, `SITEMONITOR_PG_DATA` | `%USERPROFILE%\tools\pgsql\bin`, `%USERPROFILE%\tools\sitemonitor-pgdata` | Where the Windows start script finds PostgreSQL |

---

## Before real use

1. Keep demo mode off (the default): don't set `SITEMONITOR_DEMO=1`.
2. Sign in as the Office Admin and change every account's starting password (**People → Reset password**), or deactivate the ones you don't need and add your own people.
3. **Back up the personnel key** `data/keys/personnel.key` somewhere safe and private (not Git). Without it, personnel data cannot be read.
4. Change the database password from `sitemonitor_dev` and update `DATABASE_URL`.
5. Serve it over HTTPS and set `SITEMONITOR_SECURE_COOKIES=1`.
6. Keep regular backups: **Settings → Backup DB → Download encrypted package**.

---

## Access and modules

**Access** (Office Admin) is a matrix of 27 permissions × people. Everyone starts with their role's defaults. The admin can add or remove single permissions for one person, review the changes, and save. Every change is logged, the server checks the matrix on every action, nobody can remove their own "Change access", and at least one active person always keeps it.

The modules merged from Service 5 DB each have their own permission (group "Workforce (Service 5)"): Assignments, personnel & project groups; Workforce reports; Overnight duty; Overtime; Visitations; Signatories; Print centre; Backup, restore & snapshots; and Clinic ledger. A page or tab only shows for people who hold its permission, and the server checks the same permission.

The **Clinic module is switched off** for everyone by default (**Access → Modules**). While it is off nobody has clinic access, whatever their permissions say, and the Clinic ledger page is hidden; its code, routes and records are kept, and each person's clinic permission is preserved. **Access → Modules → Switch on** brings it all back.

---

## Printing

Every printout from the workforce pages is Service 5's own: the same template, title, paper size, margins, tables, signatory lines, fonts and print CSS. The templates are copied verbatim from `../01_Rust_WC_DB/js` into `frontend/public/print/s5print.js`, together with Service 5's `print.css` (`s5print.css`), and fed SiteMonitor's records, which are stored in Service 5's own shape. They print the way Service 5 does: some in a new tab with its own Print button, the rest through a hidden frame straight to the print dialog.

| Where | Prints |
|---|---|
| Assignments → **Generate Forms** (ticked people), Operations → **Print**, Encoder print buttons, Org Chart → **Print Job Orders** | Job orders, 4 per folio page: all workers, by project group or by signatory, for a chosen date (tomorrow by default), with Service 5's eligibility banner. The slips can be edited on screen before **Print now**. |
| Assignments / Personnel → **Print Reports** | Volunteer & Family Roster; List & Attendance Generator; Records Update Forms; Personnel Directory by Trade, by Project Group, by Housing |
| Personnel → **Print & export…** | Family roster (cards or table), table, or CV / biodata (one per page) with the chosen fields |
| ID Cards | Print Selected (info cards), Generate List |
| Profile | Family information, attendance dossier, update form, assignment history |
| Attendance Gen, Update Forms | The attendance / meeting / roster sheet; prefilled or blank update forms |
| Visitations | Scheduled visits, last 7 days, one visit's form; group roster, all rosters, monthly attendance (one group or all) |
| Project Groups | One group, Print All, group change report |
| OT Calculator / OT Entries | OT schedule by area, timesheet matrix, daily breakdown |
| Overnight Duty, Dependability, Analytics | Night duty roster, dependability roster, worker dossier, project dossier |
| Workforce dashboard → Present | Present volunteers attendance sheet |

Two differences from Service 5, both because its own code fails there: its attendance dossier refers to `clearance` and `restrictions`, which it never defines, so that print stops with an error in Service 5; here it shows Service 5's clearance label and the person's restrictions list. The monthly attendance sheets print without the marks typed into Service 5's sheet screen, which SiteMonitor does not import.

When Service 5's templates change, regenerate the kit with `python scripts/s5print/build.py` (Service 5 is only read), then rebuild.

### Service 5 pages inside SiteMonitor

Five pages are Service 5's own screens, shown in their tab: **Departments**, **Overnight Duty**, **OT Calculator**, **OT Entries** and **Signatories** (`frontend/public/print/s5view-*.html`, built by the same script from Service 5's page markup, styles and code). They read SiteMonitor's records, and their changes save to SiteMonitor through the normal API (each changed duty schedule, duty area, OT entry or signatory is saved on its own, with the server's usual permission checks). Service 5's copies in the browser's local storage are switched off, so no personnel data is kept in the browser.

Departments shows Service 5's department cards exactly, including one of its quirks: each card's "/ N present" total stays at 0, because Service 5 never counts it.

### Quick search (Ctrl+K)

Ctrl+K (⌘K on a Mac) on any page, including inside the Service 5 pages, opens quick search: every page and tab you can open, and people by name, A#, trade or project group. Arrow keys move, Enter opens (a person opens their profile), Esc closes.

---

## Data privacy and backups

Real operational and personnel data never goes into Git.

- **Encrypted at rest.** Every personnel record (workers and volunteers, all workforce lists including the clinic, and workforce snapshots) is stored encrypted with AES-256-GCM. The key is `data/keys/personnel.key` (created on first start) or the `SITEMONITOR_DATA_KEY` environment variable. **Back the key up** somewhere safe and private: without it the personnel data cannot be read. `backend audit-data` reports any personnel data still readable without the key (all counts should be 0). Data stored before encryption was on is encrypted automatically at start-up.
- **Login accounts are separate.** Sign-in accounts live in their own table, are not encrypted, and are never wiped by Load sample data, Clear data or a restore, so a fresh clone is usable straight away.
- **Exports are encrypted.** Workforce backups and whole-database packages (**Settings → Backup DB**) are always encrypted with a passphrase (PBKDF2-HMAC-SHA256, 600,000 rounds, AES-256-GCM).
- **Sharing a database.** To move SiteMonitor to another machine, make a package in **Settings → Backup DB**, send the `.smpkg` file and its passphrase to the other person separately (never through GitHub), and restore it there in **Settings → Restore DB**. Estate, projects, personnel, clinic records and photos come across; their local sign-in accounts are kept.
- **Snapshots.** The workforce is saved every night and before every import, restore, Start New Day and delete; **Settings → Daily Snapshot** goes back to any of them.
- **On the device.** Saves queued while offline are encrypted in the browser (WebCrypto AES-256-GCM, key kept on the device and not exportable), when served over HTTPS or on `localhost`.
- **Git hygiene.** `.gitignore` excludes `data/` (database files, uploads, keys, backups), every database and dump format, backup and package files, snapshots, uploads, keys and `.env` files.

---

## Tests

```bash
cargo test --workspace                       # unit tests
cargo check --workspace                      # compiles shared, backend and frontend

cd e2e
npm install
npx playwright install chromium webkit       # once
npx playwright test                          # 107 end-to-end tests in 16 files
```

The end-to-end tests need the app running in demo mode: on Windows `$env:SITEMONITOR_DEMO="1"` before the start script, elsewhere `SITEMONITOR_DEMO=1 cargo run -p backend --release`. **They reset the database to the sample estate**, so back up anything you want to keep first (**Settings → Backup DB**, or `pg_dump`). A full run takes about 30 minutes; run one file with `npx playwright test tests/07-estate-data.spec.ts`.

They cover the API rules and permissions, the owner portal, the staff portal, sign-in and the access matrix, real-time updates, the offline queue and PWA, every Service 5 page and module (import of real Service 5 backups, assignments, signatories, duty, overtime, visitations, print centre, clinic), desktop / tablet / phone layouts, sample and clear data, encryption and backups, and a sweep that clicks every control on every page for each role with zero console errors allowed. Print checks also run in WebKit (Safari's engine).

---

## Automations

These jobs run in the estate's local time (times can be changed in **Settings → General**):

| Time | Job |
| --- | --- |
| 00:00 | Clears yesterday's crew numbers (no "ghost workers"), queues the morning roster prompt, and takes the nightly workforce snapshot. |
| 07:00 | Deadline monitor: staff warnings 7 days and 48 hours before each deadline, plus a "coming up" card for the owner at 48 hours. |
| 07:30 | Shows the "Allocate Today's Crew" prompt to the office admin. |
| 17:00 | The owner's daily digest. |

An Office Admin can also run the three jobs on demand through `POST /api/admin/jobs/{midnight-roster-reset|deadline-monitor|vip-daily-digest}/run`.

---

## Troubleshooting

| Problem | Fix |
| --- | --- |
| "Can't reach the SiteMonitor server" in the app | The app is not running. Start it (see [Start](#start)); check `data/backend.log`. |
| Log says `pool timed out while waiting for an open connection` | PostgreSQL is not running or `DATABASE_URL` is wrong. Start PostgreSQL (the Windows start script does it) and check the port. |
| Build fails with "Access is denied" / file in use (Windows) | The app is still running: stop `backend.exe` first, then build. |
| `dx: command not found` | Install the Dioxus CLI: `cargo install dioxus-cli@0.7.10 --locked`. |
| Port 8737 already in use | Another copy is running; stop it, or set `PORT`. |
| Page looks old after an update | Reload the page once (the app checks for a new version on each load). |
| "Personnel data could not be decrypted" | The server is using a different key than the data was saved with. Put back the right `data/keys/personnel.key` (or `SITEMONITOR_DATA_KEY`). |
| Forgot the admin password | `cargo run -p backend --release -- set-password <username> <new password>` |
| The Clinic ledger page is missing | The Clinic module is off by default: **Access → Modules → Switch on**. |
| A workforce tab is missing for someone | They lack that module's permission: grant it in **Access**. |

---

## Project layout

```
Cargo.toml        workspace (shared, backend, frontend)
shared/           domain models, API contracts, access matrix, WebSocket events
backend/          Axum API, SeaORM entities + migrations, seed and sample data,
                  Service 5 importer, encryption (vault), backups, automations, WebSocket hub
frontend/         Dioxus PWA (Tailwind design tokens, IndexedDB offline queue, 16:9 cropper)
e2e/              Playwright end-to-end tests
scripts/          build.mjs (build), start-windows.ps1 (start on Windows), icon rendering, s5print/ (builds the Service 5 print kit)
docs/merge/       how Service 5 DB was merged, page by page
data/             runtime data: uploads, keys, logs, backups (never committed)
```
