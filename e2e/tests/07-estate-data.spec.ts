/**
 * Settings → Estate data: clear everything, or load the sample estate.
 * People, their sign-ins and settings always survive.
 */
import { BASE_URL, api, expect, openAs, resetDatabase, resetDatabaseEmpty, test, userFor } from "./helpers";

test.describe.configure({ mode: "serial" });
test.beforeAll(() => resetDatabase());

const json = async (r: { json(): Promise<any> }) => r.json();
let sampleProjects = 0;

test("only people who may change settings can clear or replace the estate", async ({ request }) => {
  const jun = await userFor(request, "SiteMonitor", "Jun");
  expect((await api(request, jun).post("/settings/clear-data")).status()).toBe(403);
  expect((await api(request, jun).post("/settings/sample-data")).status()).toBe(403);
  sampleProjects = (await json(await request.get("/api/work-items"))).length;
  expect(sampleProjects).toBeGreaterThan(5);
});

test("clear data leaves an empty estate on a blank map, and everyone keeps their account", async ({ page, request }) => {
  const before = await json(await request.get("/api/users"));
  // Some workforce data too (people, OT, signatories, duty), to see it cleared with the estate.
  const backup = { version: 11, workers: [{ aNumber: "1", name: "Clear Test" }], ot_entries: [{ id: "o1", date: "2026-11-01", hours: 1 }],
    signatories: [{ id: "s1", name: "Sig" }], duty_schedules: [{ dateStr: "2026-11-01", assignments: [] }] };
  expect((await request.post("/api/workforce/import?file_name=x.json", { data: Buffer.from(JSON.stringify(backup)), headers: { "content-type": "application/octet-stream" } })).ok()).toBeTruthy();
  expect((await json(await request.get("/api/workers"))).length).toBe(1);
  await openAs(page, "OfficeAdmin", "/settings");
  await page.getByTestId("tab-Estate data").click();
  await page.getByTestId("clear-data").click();
  const confirm = page.getByTestId("reset-confirm");
  await expect(confirm).toBeDisabled();
  await page.getByTestId("reset-word").fill("SAMPLE");
  await expect(confirm).toBeDisabled();
  await page.getByTestId("reset-word").fill("CLEAR");
  await confirm.click();
  await expect(page.getByText("All data cleared").first()).toBeVisible();
  await expect(page).toHaveURL(/\/$/);
  await expect(page.getByTestId("signed-in-as")).toBeVisible();

  expect(await json(await request.get("/api/work-items"))).toEqual([]);
  const map = await request.get("/api/estate/map");
  expect(map.ok()).toBeTruthy();
  expect((await map.json()).areas).toEqual([]);
  const after = await json(await request.get("/api/users"));
  expect(after.map((u: any) => u.id).sort()).toEqual(before.map((u: any) => u.id).sort());
  expect(after.every((u: any) => u.assigned_area_ids.length === 0)).toBeTruthy();
  // The workforce goes too: people, every list, and snapshots.
  expect(await json(await request.get("/api/workers"))).toEqual([]);
  for (const list of ["duty_schedules", "ot_entries", "signatories", "visitations", "attendance_log"]) {
    expect(await json(await request.get(`/api/workforce/records/${list}`)), list).toEqual([]);
  }
  expect(await json(await request.get("/api/workforce/snapshots"))).toEqual([]);
});

test("load sample data brings the sample estate back, worked by the people already here", async ({ page, request }) => {
  const before = await json(await request.get("/api/users"));
  await openAs(page, "OfficeAdmin", "/settings");
  await page.getByTestId("tab-Estate data").click();
  await page.getByTestId("load-sample").click();
  await page.getByTestId("reset-word").fill("sample");
  await page.getByTestId("reset-confirm").click();
  await expect(page.getByText("Sample data loaded").first()).toBeVisible();

  const items = await json(await request.get("/api/work-items"));
  expect(items.length).toBe(sampleProjects);
  const users = await json(await request.get("/api/users"));
  expect(users.map((u: any) => u.id).sort()).toEqual(before.map((u: any) => u.id).sort()); // no new accounts
  const names = new Set(users.map((u: any) => u.name));
  for (const w of items) {
    for (const n of [w.personnel.group_leader_name, w.personnel.site_qc_officer_name, w.personnel.admin_coordinator_name]) {
      expect(names.has(n), `${w.title}: ${n} should be someone in People`).toBeTruthy();
    }
  }
  for (const m of users.filter((u: any) => u.role === "SiteMonitor")) expect(m.assigned_area_ids.length).toBe(3);
});

test("a fresh install starts empty: only the sign-in accounts, no estate, no workforce", async ({ playwright }) => {
  resetDatabaseEmpty();
  // Sign in again: the reset ended every session.
  const admin = await userFor(undefined as any, "OfficeAdmin");
  const request = await playwright.request.newContext({ baseURL: BASE_URL, extraHTTPHeaders: { authorization: `Bearer ${admin.token}` } });
  const users = await json(await request.get("/api/users"));
  expect(users.length).toBe(4);
  expect(await json(await request.get("/api/work-items"))).toEqual([]);
  expect((await json(await request.get("/api/estate/map"))).areas).toEqual([]);
  expect(await json(await request.get("/api/workers"))).toEqual([]);
  expect((await json(await request.get("/api/assets"))).assets).toEqual([]);
});
