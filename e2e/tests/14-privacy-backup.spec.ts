/**
 * Privacy: personnel data is encrypted at rest (database and the browser's
 * offline queue), only leaves the server encrypted, and whole databases move
 * between machines as passphrase-encrypted packages that keep local sign-ins.
 */
import { request as pwRequest } from "@playwright/test";
import { BASE_URL, auditData, expect, openAs, resetDatabase, test, userFor } from "./helpers";

test.describe.configure({ mode: "serial" });
test.setTimeout(180_000);

const PASS = "package passphrase 1";
const hexPass = (p: string) => ({ "x-backup-passphrase": Buffer.from(p, "utf8").toString("hex") });

test.beforeAll(async () => {
  resetDatabase();
  const admin = await userFor(undefined as any, "OfficeAdmin");
  const ctx = await pwRequest.newContext({ baseURL: BASE_URL, extraHTTPHeaders: { authorization: `Bearer ${admin.token}` } });
  const backup = {
    version: 11,
    workers: [
      { aNumber: "901", name: "Privacy Test Person", trade: "Mason", projectGroup: "Team P", dailyStatus: "Present", medical: "Asthma", phone: "0917 999 0001" },
      { aNumber: "902", name: "Second Private Person", trade: "Painter", projectGroup: "Team P", dailyStatus: "Present" },
    ],
    health_events: [{ id: "he1", workerId: "901", diagnosis: "Private diagnosis", dayZero: "2026-11-01", status: "active" }],
  };
  const res = await ctx.post("/api/workforce/import?file_name=seed.json", { data: Buffer.from(JSON.stringify(backup)), headers: { "content-type": "application/octet-stream" } });
  expect(res.ok(), await res.text()).toBeTruthy();
  await ctx.dispose();
});

test("personnel data is encrypted at rest; the app still reads it", async ({ request }) => {
  const a = auditData();
  expect(a.workers).toBe(2);
  expect([a.workers_plain, a.records_plain, a.snapshots_plain]).toEqual([0, 0, 0]);
  expect(a.records).toBeGreaterThan(0);
  const workers = await (await request.get("/api/workers")).json();
  expect(workers.map((w: any) => w.name).sort()).toEqual(["Privacy Test Person", "Second Private Person"]);
  const events = await (await request.get("/api/workforce/records/health_events")).json();
  expect(events[0].diagnosis).toBe("Private diagnosis");
});

test("a database package is encrypted, needs its passphrase, and restores everything", async ({ request }) => {
  expect((await request.post("/api/settings/backup")).status()).toBe(400);
  const res = await request.post("/api/settings/backup", { headers: hexPass(PASS) });
  expect(res.ok(), await res.text()).toBeTruthy();
  const file = await res.text();
  const env = JSON.parse(file);
  expect(env._isAgwcEncrypted).toBe(true);
  for (const secret of ["Privacy Test Person", "Asthma", "Private diagnosis", "Parking Conversion"]) expect(file).not.toContain(secret);

  // Change things after the package was made…
  const jobsBefore = await (await request.get("/api/work-items")).json();
  const usersBefore = await (await request.get("/api/users")).json();
  const ana = (await (await request.get("/api/workers")).json()).find((w: any) => w.a_number === "901");
  await request.post("/api/workers/delete", { data: { ids: [ana.id] } });
  await request.post("/api/settings/clear-data");
  expect((await (await request.get("/api/work-items")).json()).length).toBe(0);

  // …a wrong passphrase changes nothing…
  const upload = (pass: string) => request.post("/api/settings/restore", { data: Buffer.from(file), headers: { "content-type": "application/octet-stream", ...hexPass(pass) } });
  const wrong = await upload("not the passphrase");
  expect(wrong.status()).toBe(400);
  expect((await wrong.json()).message).toContain("Wrong passphrase");

  // …and the right one puts the estate and the workforce back, keeping every sign-in.
  const ok = await upload(PASS);
  expect(ok.ok(), await ok.text()).toBeTruthy();
  const report = await ok.json();
  expect(report.workers).toBe(2);
  expect(report.accounts_added).toBe(0);
  expect((await (await request.get("/api/work-items")).json()).length).toBe(jobsBefore.length);
  expect((await (await request.get("/api/workers")).json()).length).toBe(2);
  expect((await (await request.get("/api/users")).json()).length).toBe(usersBefore.length);
  expect(auditData().workers_plain).toBe(0); // restored personnel is encrypted again
});

test("only someone with settings, workforce and clinic access can make or restore packages", async ({ request }) => {
  const jun = await userFor(request, "SiteMonitor", "Jun");
  const r = await request.post("/api/settings/backup", { headers: { authorization: `Bearer ${jun.token}`, ...hexPass(PASS) } });
  expect(r.status()).toBe(403);
});

test("Settings → Backup & restore downloads an encrypted package", async ({ page }) => {
  await openAs(page, "OfficeAdmin", "/settings");
  await page.getByTestId("tab-Backup DB").click();
  await expect(page.getByTestId("pkg-make")).toBeDisabled();
  await page.getByTestId("pkg-pass").fill(PASS);
  await page.getByTestId("pkg-pass2").fill(PASS);
  const download = page.waitForEvent("download");
  await page.getByTestId("pkg-make").click();
  const d = await download;
  expect(d.suggestedFilename()).toMatch(/^sitemonitor-.*\.smpkg$/);
  const text = Buffer.concat(await (await d.createReadStream()).toArray()).toString();
  expect(JSON.parse(text)._isAgwcEncrypted).toBe(true);
  await page.getByTestId("tab-Restore DB").click();
  await expect(page.getByTestId("pkg-restore")).toBeDisabled(); // needs a file, its passphrase and RESTORE
});

test("attendance queued offline is stored encrypted on the device", async ({ browser, request }) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await openAs(page, "OfficeAdmin", "/personnel");
  await context.setOffline(true);
  await page.getByTestId("worker-row").filter({ hasText: "Second Private Person" }).getByTestId("mark-Absent").click();
  await expect(page.getByText("Saved on this device").first()).toBeVisible();
  const stored: string[] = await page.evaluate(
    () =>
      new Promise((ok) => {
        const open = indexedDB.open("sitemonitor");
        open.onsuccess = () => {
          const all = open.result.transaction("staff_mutation_queue").objectStore("staff_mutation_queue").getAll();
          all.onsuccess = () => ok(all.result as string[]);
        };
      }),
  );
  expect(stored.length).toBeGreaterThan(0);
  for (const s of stored) {
    expect(s.startsWith("smq1:")).toBeTruthy();
    expect(s).not.toContain("Second Private Person");
  }
  await context.setOffline(false);
  await expect.poll(async () => (await (await request.get("/api/workers")).json()).find((w: any) => w.a_number === "902").status, { timeout: 20_000 }).toBe("Absent");
  await context.close();
});
