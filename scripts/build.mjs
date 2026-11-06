#!/usr/bin/env node
// Builds SiteMonitor end to end (works the same on macOS and Windows):
//   1. Tailwind CSS          -> frontend/assets/tailwind.css
//   2. Dioxus WASM bundle    -> target/dx/frontend/release/web/public
//   3. Axum backend (release)-> target/release/backend[.exe]
//
// Usage: node scripts/build.mjs [--skip-backend] [--skip-frontend]
import { execSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const frontend = join(root, "frontend");
const args = new Set(process.argv.slice(2));
const run = (cmd, cwd = root) => {
  console.log(`\n> ${cmd}`);
  execSync(cmd, { cwd, stdio: "inherit", shell: true });
};

if (!args.has("--skip-frontend")) {
  run("npm install --no-audit --no-fund", frontend);
  run("npm run fonts", frontend);
  run("node scripts/generate-seed-art.mjs", frontend);
  run("npm run css", frontend);
  run("dx bundle --platform web --release -p frontend", frontend);
}
if (!args.has("--skip-backend")) {
  run("cargo build -p backend --release");
}
console.log("\nBuild complete. Start with: cargo run -p backend --release   (http://localhost:8737)");
