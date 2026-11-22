/**
 * Phase 1 of the Service 5 DB merge: importing real legacy data (service5.db,
 * a v4 daily snapshot, an encrypted .agwcz backup), anti-wipe, snapshots and
 * the Personnel directory.
 *
 * The real files are read from the sibling project at run time and never copied here.
 */
import { createCipheriv, createHash, pbkdf2Sync, randomBytes } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { api, expect, openAs, resetDatabase, test, userFor } from "./helpers";

test.describe.configure({ mode: "serial" });
test.setTimeout(180_000);
test.beforeAll(() => resetDatabase());

const SIBLING = join(__dirname, "..", "..", "..", "01_Rust_WC_DB", "data");
const DB_FILE = join(SIBLING, "service5.db");
const SNAPSHOT = join(SIBLING, "snapshots", "daily_snapshot_2026-11-19.json");

const b64 = (buf: Buffer | string) => Buffer.from(buf).toString("base64");
/** Uploads like the Settings screen: the file is the raw body, options in the query. */
const importFile = (request: any, file_name: string, content: Buffer | string, opts: { mode?: string; passphrase?: string; token?: string } = {}) => {
  const headers: Record<string, string> = { "content-type": "application/octet-stream" };
  if (opts.passphrase) headers["x-backup-passphrase"] = Buffer.from(opts.passphrase, "utf8").toString("hex");
  if (opts.token) headers.authorization = `Bearer ${opts.token}`;
  const q = new URLSearchParams({ file_name, mode: opts.mode ?? "Merge" });
  return request.post(`/api/workforce/import?${q}`, { data: Buffer.from(content), headers, timeout: 170_000 });
};
const workers = async (request: any) => (await request.get("/api/workers")).json();

/** Encrypts exactly like Service 5's js/crypto.js `encryptBackupArchive`. */
function agwcz(obj: object, passphrase: string) {
  const json = JSON.stringify(obj);
  const salt = randomBytes(16), iv = randomBytes(12);
  const key = pbkdf2Sync(passphrase, salt, 600_000, 32, "sha256");
  const c = createCipheriv("aes-256-gcm", key, iv);
  const data = Buffer.concat([c.update(json, "utf8"), c.final(), c.getAuthTag()]);
  return JSON.stringify({
    _isAgwcEncrypted: true, format: "agwcz", version: 1, algo: "AES-GCM", kdf: "PBKDF2-HMAC-SHA256", iterations: 600_000,
    salt: b64(salt), iv: b64(iv), sha256: createHash("sha256").update(json).digest("hex"), data: b64(data),
  });
}

test("only workforce managers can import; monitors can read the list, the owner can't", async ({ request }) => {
  const jun = await userFor(request, "SiteMonitor", "Jun");
  expect((await importFile(request, "x.json", "[]")).status()).toBe(400); // empty file
  expect((await importFile(request, "x.json", "[]", { token: jun.token })).status()).toBe(403);
  expect((await request.get("/api/workers", { headers: { authorization: `Bearer ${jun.token}` } })).status()).toBe(200);
  const vip = await userFor(request, "VipClient");
  expect((await request.get("/api/workers", { headers: { authorization: `Bearer ${vip.token}` } })).status()).toBe(403);
});

test("the real service5.db imports completely, and importing again changes nothing", async ({ request }) => {
  test.skip(!existsSync(DB_FILE), "../01_Rust_WC_DB/data/service5.db is not on this machine");
  const res = await importFile(request, "service5.db", readFileSync(DB_FILE));
  expect(res.ok(), await res.text()).toBeTruthy();
  const report = await res.json();
  expect(report.detected).toContain("service5.db");
  expect(report.workers).toBe(174);
  const got = Object.fromEntries(report.collections);
  // Both key generations (plain and wc_) merged into one collection each.
  expect(got.attendance_log).toBe(10791);
  expect(got.assignment_log).toBe(1605);
  expect(got.volunteer_health_logs).toBeGreaterThanOrEqual(151);
  expect(got.duty_schedules).toBeGreaterThanOrEqual(161);
  for (const c of ["ot_entries", "signatories", "visitations", "health_events", "duty_areas", "project_groups_meta"]) expect(got[c], c).toBeGreaterThan(0);

  const list = await workers(request);
  expect(list.length).toBe(174);
  expect(list.some((w: any) => w.status === "OnPass")).toBeTruthy(); // "Passes" in Service 5
  expect(new Set(list.map((w: any) => w.id)).size).toBe(174);

  // Service 5's own daily snapshots inside service5.db become restorable snapshots (once).
  const snaps = await (await request.get("/api/workforce/snapshots")).json();
  expect(snaps.filter((s: any) => s.reason.startsWith("Service 5 daily snapshot")).length).toBeGreaterThan(0);
  const again = await (await importFile(request, "service5.db", readFileSync(DB_FILE))).json();
  expect(again.workers).toBe(174);
  expect((await workers(request)).length).toBe(174);
});

test("a v4 daily snapshot JSON (41 MB) imports and matches the same people", async ({ request }) => {
  test.skip(!existsSync(SNAPSHOT), "the Service 5 daily snapshot is not on this machine");
  const res = await importFile(request, "daily_snapshot.json", readFileSync(SNAPSHOT));
  expect(res.ok(), await res.text()).toBeTruthy();
  const report = await res.json();
  expect(report.detected).toBe("Service 5 backup (v4)");
  expect(report.workers).toBe(174);
  expect((await workers(request)).length).toBe(174);
});

test("anti-wipe: an empty workers list never clears people, even in Replace mode", async ({ request }) => {
  const before = (await workers(request)).length;
  expect((await importFile(request, "empty.json", JSON.stringify({ version: 11, workers: [] }))).status()).toBe(400);
  const res = await importFile(request, "partial.json", JSON.stringify({ version: 11, workers: [], ministers: [{ id: "m-test", name: "Test Minister" }] }), { mode: "Replace" });
  expect(res.ok()).toBeTruthy();
  const report = await res.json();
  expect(report.kept_existing).toContain("workers");
  expect((await workers(request)).length).toBe(before);
});

test("encrypted .agwcz backups: the right passphrase imports, a wrong one changes nothing", async ({ request }) => {
  const backup = { version: 11, workers: [{ aNumber: "E-1", name: "Encrypted Test Worker", dailyStatus: "Absent" }] };
  const file = agwcz(backup, "estate passphrase");
  const before = (await workers(request)).length;
  const wrong = await importFile(request, "b.agwcz", file, { passphrase: "nope" });
  expect(wrong.status()).toBe(400);
  expect((await wrong.json()).message).toContain("Wrong passphrase");
  expect((await importFile(request, "b.agwcz", file)).status()).toBe(400); // passphrase required
  const ok = await importFile(request, "b.agwcz", file, { passphrase: "estate passphrase" });
  expect(ok.ok(), await ok.text()).toBeTruthy();
  expect((await ok.json()).detected).toBe("Encrypted Service 5 backup (v11)");
  const list = await workers(request);
  expect(list.length).toBe(before + 1);
  expect(list.find((w: any) => w.a_number === "E-1").status).toBe("Absent");
});

test("Personnel page: counts, search, Ctrl+K, add and edit a worker", async ({ page }) => {
  await openAs(page, "OfficeAdmin", "/");
  // Ctrl+K quick search, then the Personnel page.
  await page.keyboard.press("Control+k");
  await page.getByTestId("quick-search-input").fill("Personnel");
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(/\/personnel$/);
  const total = Number(await page.getByTestId("count-Total active").innerText());
  expect(total).toBeGreaterThan(0);

  await page.getByTestId("add-worker").click();
  await page.getByTestId("worker-name").fill("Playwright Volunteer");
  await page.getByTestId("worker-a-number").fill("PW-001");
  await page.getByTestId("worker-trade").fill("Carpenter");
  await page.getByTestId("worker-status-OnPass").click();
  await page.getByTestId("save-worker").click();
  await expect(page.getByText("Playwright Volunteer saved").first()).toBeVisible();

  await page.getByTestId("worker-search").fill("PW-001");
  await expect(page.getByTestId("worker-row")).toHaveCount(1);
  await expect(page.getByTestId("worker-row")).toContainText("On pass");
  await page.getByTestId("edit-worker").click();
  await page.getByTestId("worker-a-number").fill("PW-002");
  await page.getByTestId("save-worker").click();
  await page.getByTestId("worker-search").fill("PW-002");
  await expect(page.getByTestId("worker-row")).toHaveCount(1);
});

test("snapshots: take one, change something, restore it back", async ({ page, request }) => {
  await openAs(page, "OfficeAdmin", "/settings");
  await page.getByTestId("tab-Daily Snapshot").click();
  await page.getByTestId("take-snapshot").click();
  await expect(page.getByText("Snapshot saved").first()).toBeVisible();
  const target = (await workers(request)).find((w: any) => w.a_number === "PW-002");
  await request.put(`/api/workers/${target.id}`, { data: { ...target, status: "Present", name: "Renamed Volunteer" } });
  expect((await workers(request)).find((w: any) => w.a_number === "PW-002").name).toBe("Renamed Volunteer");

  await page.reload();
  await page.getByTestId("tab-Daily Snapshot").click();
  const newest = page.getByTestId("snapshot-row").filter({ hasText: "manual" }).first();
  await newest.getByTestId("restore-snapshot").click();
  await newest.getByTestId("restore-confirm").click();
  await expect(page.getByText(/^Restored:/).first()).toBeVisible();
  expect((await workers(request)).find((w: any) => w.a_number === "PW-002").name).toBe("Playwright Volunteer");
});

test("the downloaded backup is encrypted and imports back with its passphrase", async ({ request }) => {
  expect((await request.get("/api/workforce/export")).status()).toBe(400); // personnel data never leaves in the clear
  const headers = { "x-backup-passphrase": Buffer.from("team passphrase", "utf8").toString("hex") };
  const file = await (await request.get("/api/workforce/export", { headers })).text();
  expect(JSON.parse(file)._isAgwcEncrypted).toBe(true);
  expect(file).not.toContain("Playwright Volunteer");
  const res = await importFile(request, "back.agwcz", file, { passphrase: "team passphrase" });
  expect(res.ok(), await res.text()).toBeTruthy();
  const report = await res.json();
  expect(report.detected).toBe("Encrypted SiteMonitor workforce backup");
  expect(report.workers).toBe((await workers(request)).length);
});
