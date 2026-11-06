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

Open **http://localhost:8737**. Use the dark bar at the top to switch between the **Estate Owner**, **Office Admin** and **Site Monitor** views.

On Windows you can also start the database and the app in the background with:

```powershell
powershell -ExecutionPolicy Bypass -File scriptsstart-windows.ps1
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
npx playwright test                          # 37 end-to-end tests (resets the database itself)
```

The end-to-end suite covers every phase of `PLAN.md`:

- the seed and API rules: weighted progress, the 99% handover lock, the Quiet Day engine, the manpower ledger, deadline shifts from breakdowns, cron jobs and role permissions;
- the owner portal: Action Center, map drill-down and layers, documents, photo pins, agenda and sign-off;
- the staff portal: crew board, equipment, phase updater, decision pipeline, Master Scheduler and Map Studio;
- real-time pushes, offline queue and sync, the PWA manifest and service worker, and smoothness checks (no console errors, no layout shift on refresh).

## Automations

These jobs run in the estate's local time:

| Time | Job |
| --- | --- |
| 00:00 | Clears yesterday's crew numbers (no "ghost workers") and queues the morning roster prompt. |
| 07:00 | Deadline monitor: staff warnings 7 days and 48 hours before each deadline, plus a "coming up" card for the owner at 48 hours. |
| 07:30 | Shows the "Allocate Today's Crew" prompt to the office admin. |
| 17:00 | The owner's daily digest. |

An Office Admin can also run the three jobs on demand through `POST /api/admin/jobs/{midnight-roster-reset|deadline-monitor|vip-daily-digest}/run`.
