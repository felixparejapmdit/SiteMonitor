/**
 * Settings → Estate data: clear everything, or load the sample estate.
 * People, their sign-ins and settings always survive.
 */
import { api, expect, openAs, resetDatabase, test, userFor } from "./helpers";

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
  await openAs(page, "OfficeAdmin", "/settings");
  await page.getByTestId("clear-data").click();
  const confirm = page.getByTestId("reset-confirm");
  await expect(confirm).toBeDisabled();
  await page.getByTestId("reset-word").fill("SAMPLE");
  await expect(confirm).toBeDisabled();
  await page.getByTestId("reset-word").fill("CLEAR");
  await confirm.click();
  await expect(page.getByText("All estate data cleared").first()).toBeVisible();
  await expect(page).toHaveURL(/\/$/);
  await expect(page.getByTestId("signed-in-as")).toBeVisible();

  expect(await json(await request.get("/api/work-items"))).toEqual([]);
  const map = await request.get("/api/estate/map");
  expect(map.ok()).toBeTruthy();
  expect((await map.json()).areas).toEqual([]);
  const after = await json(await request.get("/api/users"));
  expect(after.map((u: any) => u.id).sort()).toEqual(before.map((u: any) => u.id).sort());
  expect(after.every((u: any) => u.assigned_area_ids.length === 0)).toBeTruthy();
});

test("load sample data brings the sample estate back, worked by the people already here", async ({ page, request }) => {
  const before = await json(await request.get("/api/users"));
  await openAs(page, "OfficeAdmin", "/settings");
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
