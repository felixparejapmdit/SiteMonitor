/**
 * Access for the merged Service 5 modules: each module has its own permission
 * on the Access page (and the server enforces it), and the Clinic module is
 * off for everyone by default, switchable back on without losing its records.
 */
import { expect, openAs, resetDatabase, setClinic, test, userFor } from "./helpers";

test.describe.configure({ mode: "serial" });
test.setTimeout(120_000);

test.beforeAll(async () => {
  resetDatabase();
  await setClinic(false); // the default; the running server may still hold an earlier spec's choice
});

const auth = (token: string) => ({ headers: { authorization: `Bearer ${token}` } });

test("the clinic is off for everyone by default, and nothing of it shows", async ({ page, request }) => {
  const admin = await userFor(request, "OfficeAdmin");
  const me = await (await request.get("/api/auth/me", auth(admin.token))).json();
  expect(me.permissions ?? me.user?.permissions).not.toContain("ManageClinic");
  expect((await request.get("/api/workforce/records/health_events", auth(admin.token))).status()).toBe(403);

  await openAs(page, "OfficeAdmin", "/access");
  await expect(page.getByTestId("module-clinic")).toHaveAttribute("aria-checked", "false");
  await expect(page.getByText("Module off").first()).toBeVisible();
  for (const label of ["Overnight duty", "Overtime (OT entries & calculator)", "Visitations", "Signatories", "Print centre (ID cards, job orders, forms)", "Workforce reports", "Backup, restore & snapshots", "Assignments, personnel & project groups"]) {
    await expect(page.locator("th").filter({ hasText: label }).first()).toBeVisible();
  }
  await expect(page.getByTestId("sidebar")).not.toContainText("Clinic ledger");
});

test("a module permission opens just that module, and the server holds to it", async ({ page, request }) => {
  const matrix = await (await request.get("/api/access")).json();
  const jun = matrix.users.find((u: any) => u.name.startsWith("Jun"));
  const save = await request.put("/api/access", { data: { changes: [{ user_id: jun.id, access: { granted: ["ManageOvertime"], revoked: [] } }] } });
  expect(save.ok(), await save.text()).toBeTruthy();

  const token = (await userFor(request, "SiteMonitor", "Jun")).token;
  expect((await request.put("/api/workforce/records/ot_entries/ot-test", { ...auth(token), data: { date: "2026-11-20", hours: 1 } })).ok()).toBeTruthy();
  expect((await request.put("/api/workforce/records/duty_areas/d-test", { ...auth(token), data: { name: "x" } })).status()).toBe(403);
  expect((await request.get("/api/workforce/export", auth(token))).status()).toBe(403);

  await openAs(page, "SiteMonitor", "/operations", "Jun");
  await expect(page.getByTestId("tab-OT Entries")).toBeVisible();
  await expect(page.getByTestId("tab-OT Calculator")).toBeVisible();
  await expect(page.getByTestId("tab-Overnight Duty")).toHaveCount(0);
  await expect(page.getByTestId("tab-Signatories")).toHaveCount(0);
});

test("switching the clinic on from Access brings it back for admins; off again keeps the records", async ({ page, request }) => {
  await openAs(page, "OfficeAdmin", "/access");
  await page.getByTestId("module-clinic").click();
  await expect(page.getByTestId("module-clinic")).toHaveAttribute("aria-checked", "true", { timeout: 15_000 });
  await expect(page.getByTestId("sidebar")).toContainText("Clinic ledger");
  const admin = await userFor(request, "OfficeAdmin");
  expect((await request.put("/api/workforce/records/health_events/he-test", { ...auth(admin.token), data: { workerId: "1", diagnosis: "Test", status: "active" } })).ok()).toBeTruthy();

  await page.getByTestId("module-clinic").click();
  await expect(page.getByTestId("module-clinic")).toHaveAttribute("aria-checked", "false", { timeout: 15_000 });
  await expect(page.getByTestId("sidebar")).not.toContainText("Clinic ledger");
  expect((await request.get("/api/workforce/records/health_events", auth(admin.token))).status()).toBe(403);

  // Saving access while the module is off must not strip anyone's clinic permission.
  const matrix = await (await request.get("/api/access")).json();
  const office = matrix.users.find((u: any) => u.role === "OfficeAdmin");
  await request.put("/api/access", { data: { changes: [{ user_id: office.id, access: { granted: [], revoked: ["ManageClinic", "ManageMap"] } }] } });
  await setClinic(true);
  const back = (await (await request.get("/api/access")).json()).users.find((u: any) => u.id === office.id);
  expect(back.permissions).toContain("ManageClinic");
  expect(back.permissions).not.toContain("ManageMap");
  const events = await (await request.get("/api/workforce/records/health_events", auth(admin.token))).json();
  expect(events.some((e: any) => e.id === "he-test")).toBeTruthy(); // the records were kept
  await setClinic(false);
});
