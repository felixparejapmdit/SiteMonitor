/**
 * Field-for-field parity with Service 5: the five-tab personnel profile, the
 * add / edit worker form, separation and re-employment, visit outcomes,
 * duty Clear Day, the attendance sheet generator, the records update forms,
 * visitation group transfers, and the importer keeping one group per name.
 * All people here are made up.
 */
import { readFileSync } from "node:fs";
import { Page, request as pwRequest } from "@playwright/test";
import { BASE_URL, expect, openAs, resetDatabase, setClinic, test, userFor } from "./helpers";

test.describe.configure({ mode: "serial" });
test.setTimeout(120_000);

const today = () => {
  const d = new Date();
  return new Date(d.getTime() - d.getTimezoneOffset() * 60_000).toISOString().slice(0, 10);
};
const BACKUP = {
  version: 11,
  workers: [
    { aNumber: 501, name: "Ramos, Elena", trade: "Mason", projectGroup: "Team P", department: "Job-site Construction", dailyStatus: "Present", visitationGroup: "Group A",
      visitationRole: "Member", visitationHousing: "Unit 1", birthday: "1980-03-04", civilStatus: "Married", projectRole: "Member", tradeRole: "Member", jOrder: "YES",
      specificTask: "Lay tiles", taskStartedAt: "2026-11-01T08:00:00Z", disableStaleAlerts: false,
      attendanceHistory: [
        { date: "2026-11-02", status: "Present", shift: "Day", verifiedBy: "Encoder" },
        { date: "2026-11-03", status: "Absent", reason: "Sick", verifiedBy: "Encoder" },
        { date: "2026-11-04", status: "Passes", reason: "Errand", verifiedBy: "Encoder" },
      ] },
    { aNumber: 502, name: "Santos, Paolo", trade: "Carpenter", projectGroup: "Team Q", dailyStatus: "Present", visitationGroup: "Group A", visitationHousing: "Unit 1", birthday: "1995-07-01" },
    { aNumber: 503, name: "Uy, Carmela", trade: "Painter", projectGroup: "Team Q", dailyStatus: "Passes", visitationHousing: "Unit 2", birthday: "1970-01-20" },
  ],
  // Two key generations of the same groups with different ids (as in a real service5.db).
  visitation_groups: [{ id: "vg_old_a", name: "Group A", housing: "Unit 1", createdAt: "2026-08-17T09:56:30Z" }, { id: "vg_old_b", name: "Group B", createdAt: "2026-08-17T09:56:30Z" }],
  wc_visitation_groups: [{ id: "vg_new_a", name: "Group A", housing: "Unit 1", prayerDay: "Thursday", createdAt: "2026-11-17T07:48:02Z" }, { id: "vg_new_b", name: "Group B", createdAt: "2026-11-17T07:48:02Z" }],
  duty_areas: [{ id: "duty_gate", name: "Gate", workersNeeded: 1, roster: [501], lastRotationIndex: -1, shiftTime: "20:00" }],
  duty_schedules: [{ dateStr: today(), assignments: [{ aNumber: 501, areaId: "duty_gate" }] }],
};

test.beforeAll(async () => {
  resetDatabase();
  await setClinic(true);
  const admin = await userFor(undefined as any, "OfficeAdmin");
  const ctx = await pwRequest.newContext({ baseURL: BASE_URL, extraHTTPHeaders: { authorization: `Bearer ${admin.token}` } });
  const res = await ctx.post("/api/workforce/import?mode=Replace&file_name=parity.json", { data: Buffer.from(JSON.stringify(BACKUP)), headers: { "content-type": "application/octet-stream" } });
  expect(res.ok(), await res.text()).toBeTruthy();
  await ctx.dispose();
});

const workers = async (request: any) => (await request.get("/api/workers")).json();
const byA = async (request: any, a: string) => (await workers(request)).find((w: any) => w.a_number === a);
const profile = async (request: any, a: string) => (await request.get(`/api/workers/${(await byA(request, a)).id}/profile`)).json();
const records = async (request: any, list: string) => (await request.get(`/api/workforce/records/${list}`)).json();
const openProfile = async (page: Page, name: string) => {
  await page.getByTestId("worker-row").filter({ hasText: name }).getByTestId("profile-worker").click();
  await expect(page.getByTestId("ptab-personal")).toBeVisible();
};

test("the importer keeps one visitation group per name, the newest copy", async ({ request }) => {
  const groups = await records(request, "visitation_groups");
  expect(groups.map((g: any) => g.name).sort()).toEqual(["Group A", "Group B"]);
  expect(groups.find((g: any) => g.name === "Group A").id).toBe("vg_new_a");
});

test("profile: five tabs save every Service 5 personnel field", async ({ page, request }) => {
  await openAs(page, "OfficeAdmin", "/personnel");
  await openProfile(page, "Ramos, Elena");
  for (const t of ["personal", "family", "residence", "trade", "attendance"]) await expect(page.getByTestId(`ptab-${t}`)).toBeVisible();
  await expect(page.getByTestId("pf-age")).not.toHaveValue("");

  await page.getByTestId("ptab-family").click();
  await page.getByTestId("pf-spouseChurchDuty").fill("Choir");
  await page.getByTestId("pf-spouseKasalSaIglesia").selectOption("Oo");
  await page.getByTestId("add-children").click();
  await page.getByTestId("children-0-name").fill("Lia Ramos");
  await page.getByTestId("children-0-birthday").fill("2010-05-06");
  await page.getByTestId("children-0-isStudent").check();
  await page.getByTestId("children-0-course").fill("Grade 10");

  await page.getByTestId("ptab-residence").click();
  await page.getByTestId("pf-projectRole").selectOption("Project Leader");
  await page.getByTestId("pf-ca-Station 1").check();
  await page.getByTestId("pf-workResidence").selectOption("Tagumpay I");

  await page.getByTestId("ptab-trade").click();
  await page.getByTestId("pf-tradeRole").selectOption("Katiwala");
  await page.getByTestId("pf-tradeGroup").fill("Masons A");
  await page.getByTestId("pf-subDepartment").selectOption("__ADD_NEW_SHOP__");
  await page.getByTestId("new-shop").fill("Glass Shop");
  await page.getByTestId("add-shop").click();
  await page.getByTestId("pf-jOrder").uncheck();

  await page.getByTestId("ptab-personal").click();
  await page.getByTestId("add-maintenanceMeds").click();
  await page.getByTestId("maintenanceMeds-0-name").fill("Amlodipine");
  await page.getByTestId("maintenanceMeds-0-dosage").fill("5mg daily");
  await page.getByTestId("profile-save").click();
  await expect(page.getByText("Saved").first()).toBeVisible();

  const p = await profile(request, "501");
  expect(p.spouseChurchDuty).toBe("Choir");
  expect(p.spouseKasalSaIglesia).toBe("Oo");
  expect(p.children[0]).toMatchObject({ name: "Lia Ramos", birthday: "2010-05-06", isStudent: true, course: "Grade 10" });
  expect(typeof p.children[0].age).toBe("number");
  expect(p.projectRole).toBe("Project Leader");
  expect(p.caEligible).toEqual(["Station 1"]);
  expect(p.workResidence).toBe("Tagumpay I");
  expect([p.tradeRole, p.tradeGroup, p.subDepartment, p.jOrder]).toEqual(["Katiwala", "Masons A", "Glass Shop", "NO"]);
  expect(p.maintenanceMeds[0]).toMatchObject({ name: "Amlodipine", dosage: "5mg daily" });
  expect((await records(request, "custom_datalists"))[0].woodlandShops).toContain("Glass Shop");
});

test("profile: attendance tab, previous / next and save-and-next", async ({ page, request }) => {
  await openAs(page, "OfficeAdmin", "/personnel");
  await openProfile(page, "Ramos, Elena");
  await page.getByTestId("ptab-attendance").click();
  await expect(page.getByTestId("profile-att-kpis")).toContainText("Total duty");
  await expect(page.getByTestId("profile-att-table").locator("tbody tr")).toHaveCount(3);
  await page.getByTestId("att-filter-absent").click();
  await expect(page.getByTestId("profile-att-table").locator("tbody tr")).toHaveCount(1);
  await expect(page.getByTestId("profile-att-table")).toContainText("Sick");
  await page.getByTestId("att-filter-all").click();
  await page.getByTestId("att-search").fill("Errand");
  await expect(page.getByTestId("profile-att-table").locator("tbody tr")).toHaveCount(1);

  await page.getByTestId("ptab-personal").click();
  await page.getByTestId("profile-next").click();
  await expect(page.getByTestId("pf-name")).toHaveValue("Santos, Paolo");
  await page.getByTestId("pf-language").fill("Ilocano");
  await page.getByTestId("profile-save-next").click();
  await expect(page.getByTestId("pf-name")).toHaveValue("Uy, Carmela");
  expect((await profile(request, "502")).language).toBe("Ilocano");
  await page.getByTestId("profile-prev").click();
  await expect(page.getByTestId("pf-name")).toHaveValue("Santos, Paolo");
});

test("profile: a visit logged from the profile keeps its outcome where Service 5 does", async ({ page, request }) => {
  await openAs(page, "OfficeAdmin", "/personnel");
  await openProfile(page, "Uy, Carmela");
  await page.getByTestId("ptab-residence").click();
  await page.getByTestId("profile-past-visit").click();
  await page.getByTestId("rf-ministerName").fill("Bro. Lim");
  await page.getByTestId("rf-completionDetails.members").fill("Spouse and two children");
  await page.getByTestId("rf-completionDetails.concerns").fill("Needs a ride on Sundays");
  await page.getByTestId("rf-save").click();
  await expect.poll(async () => (await records(request, "visitations")).length).toBe(1);
  const v = (await records(request, "visitations"))[0];
  expect(v.status).toBe("completed");
  expect(v.aNumber).toBe("503");
  expect(v.completionDetails).toMatchObject({ members: "Spouse and two children", concerns: "Needs a ride on Sundays" });
});

test("separation and re-employment do what Service 5 does, and are logged", async ({ request }) => {
  const w = await byA(request, "503");
  const bad = await request.post(`/api/workers/${w.id}/archive`, { data: { category: "Fired", date: "2026-11-01" } });
  expect(bad.status()).toBe(400);
  expect((await request.post(`/api/workers/${w.id}/archive`, { data: { category: "Retired", date: "2026-11-01", remarks: "Went home" } })).ok()).toBeTruthy();
  let p = await profile(request, "503");
  expect([p.isArchived, p.archiveCategory, p.archivedAt, p.archivedRemarks, p.dailyStatus, p.jOrder, p.specificTask]).toEqual([true, "Retired", "2026-11-01", "Went home", "Absent", "NO", ""]);
  expect((await byA(request, "503")).archived).toBe(true);

  expect((await request.post(`/api/workers/${w.id}/reemploy`, { data: { date: "2026-11-15", status: "Passes", project_group: "Team R", remarks: "Back" } })).ok()).toBeTruthy();
  p = await profile(request, "503");
  expect([p.isArchived, p.archiveCategory, p.reemployedAt, p.dailyStatus, p.projectGroup, p.jOrder]).toEqual([false, "", "2026-11-15", "Passes", "Team R", ""]);
  const log = await records(request, "assignment_log");
  expect(log.filter((e: any) => e.aNumber === "503").map((e: any) => e.action).sort()).toEqual(["archive", "reemploy"]);
});

test("archive from the directory asks for Service 5's category and effective date", async ({ page, request }) => {
  await openAs(page, "OfficeAdmin", "/personnel");
  await page.getByTestId("worker-row").filter({ hasText: "Santos, Paolo" }).getByRole("checkbox").first().check();
  await page.getByTestId("bulk-archive").click();
  await expect(page.getByTestId("bulk-category")).toContainText("Dismissed (eligible for future re-employment)");
  await page.getByTestId("bulk-category").selectOption("Dismissed");
  await page.getByTestId("bulk-date").fill("2026-11-10");
  await page.getByTestId("bulk-confirm").click();
  await expect.poll(async () => (await profile(request, "502")).archivedAt).toBe("2026-11-10");
  // Put Paolo back for the tests after this one.
  const w = await byA(request, "502");
  expect((await request.post(`/api/workers/${w.id}/reemploy`, { data: { status: "Present" } })).ok()).toBeTruthy();
});

test("duty: Clear Day empties the night's roster", async ({ page, request }) => {
  await openAs(page, "OfficeAdmin", "/operations");
  await page.getByTestId("tab-Overnight Duty").click();
  await page.getByTestId("duty-clear-day").click();
  await page.getByTestId("duty-clear-confirm").click();
  await expect.poll(async () => (await records(request, "duty_schedules")).find((d: any) => d.dateStr === today()).assignments.length).toBe(0);
});

test("attendance sheet generator: formats, grouping, filters and selection", async ({ page }) => {
  await openAs(page, "OfficeAdmin", "/workforce");
  await page.getByTestId("tab-Attendance Gen").click();
  await page.getByTestId("ag-select-all").click();
  await expect(page.getByTestId("ag-print")).toContainText("(3)");
  await page.getByTestId("ag-group-by").selectOption("projectGroup");
  await page.getByTestId("ag-format").selectOption("meeting");
  // Service 5's own sheet, grouped by project group (Team P, Team Q, and Team R from the re-employment above).
  await page.evaluate(() => { const w = window as any; w.__s5 = []; w.safePrint = (h: string) => w.__s5.push(h); });
  await page.getByTestId("ag-print").click();
  const sheet = () => page.evaluate(() => (window as any).__s5[0] || "");
  await expect.poll(sheet).toContain("Project Group");
  for (const g of ["Team P", "Team Q", "Team R", "Ramos, Elena"]) expect(await sheet()).toContain(g);
  await page.getByTestId("ag-clear").click();
  await page.getByTestId("ag-filter-status").selectOption("passes");
  await page.getByTestId("ag-select-all").click();
  await expect(page.getByTestId("ag-print")).toContainText("(1)");
  await expect(page.getByTestId("ag-print")).toContainText("Generate & print (1)");
});

test("records update forms: blank copies, single volunteer, cycle banner", async ({ page }) => {
  await openAs(page, "OfficeAdmin", "/workforce");
  await page.evaluate(() => { const w = window as any; w.__s5 = []; w.safePrint = (h: string) => w.__s5.push(h); });
  const last = () => page.evaluate(() => { const a = (window as any).__s5; return a[a.length - 1] || ""; });
  const sheets = async () => ((await last()).match(/class="vru-sheet"/g) || []).length;
  await page.getByTestId("tab-Update Forms").click();
  await expect(page.getByTestId("vru-print")).toContainText("(3)");
  await page.getByTestId("vru-cycle").fill("Semi-Annual Update 2026 (2nd Half)");
  await page.getByTestId("vru-scope").selectOption("single");
  await page.getByTestId("vru-single").selectOption({ label: "Ramos, Elena" });
  await expect(page.getByTestId("vru-print")).toContainText("(1)");
  await page.getByTestId("vru-print").click();
  await expect.poll(sheets).toBe(1);
  expect(await last()).toContain("Semi-Annual Update 2026 (2nd Half)");
  expect(await last()).toContain("Project Leader");
  await page.getByTestId("vru-mode").selectOption("blank");
  await page.getByTestId("vru-copies").fill("4");
  await expect(page.getByTestId("vru-print")).toContainText("(4)");
  await page.getByTestId("vru-print").click();
  await expect.poll(sheets).toBe(4);
  expect(await last()).not.toContain("Ramos");
});

test("visitation groups: move a member to another group", async ({ page, request }) => {
  await openAs(page, "OfficeAdmin", "/workforce");
  await page.getByTestId("tab-Visitations").click();
  await page.getByTestId("visit-view-Groups").click();
  await page.getByTestId("vgroup-pick").selectOption("Group A");
  const paolo = page.getByTestId("vgroup-member").filter({ hasText: "Santos, Paolo" });
  await paolo.getByTestId("vgroup-transfer").selectOption("Group B");
  await expect.poll(async () => (await byA(request, "502")).visit_group).toBe("Group B");
  await page.getByText(/^Add members/).click();
  await page.getByTestId("vgroup-pool-other").click();
  await expect(page.getByTestId("vgroup-unassigned").first()).toBeVisible();
});

test("personnel: Excel export and CV / biodata profiles", async ({ page }) => {
  await openAs(page, "OfficeAdmin", "/personnel");
  const [download] = await Promise.all([page.waitForEvent("download"), page.getByTestId("export-xlsx").click()]);
  expect(download.suggestedFilename()).toMatch(/^personnel-.*\.xls$/);
  const body = readFileSync(await download.path(), "utf8");
  expect(body).toContain("urn:schemas-microsoft-com:office:spreadsheet");
  expect(body).toContain("Ramos, Elena");

  // CV / biodata: Service 5's Print & Export "profile" layout, one person per page.
  await page.goto("/personnel");
  await page.evaluate(() => {
    const w = window as any; w.__s5 = [];
    w.open = () => { let buf = ""; return { document: { open() { buf = ""; }, write(h: string) { buf += h; }, close() { w.__s5.push(buf); buf = ""; }, images: [] }, print() {}, close() {}, focus() {} }; };
  });
  await page.getByTestId("print-export-open").click();
  await page.getByTestId("pe-layout-profile").check();
  await page.getByTestId("pe-individual").check();
  await page.getByTestId("pe-print").click();
  const cv = () => page.evaluate(() => (window as any).__s5.join(""));
  await expect.poll(cv).toContain("Personnel Biodata / CV");
  expect(await cv()).toContain("Ramos, Elena");
});

test("dashboard: stale task alerts can mark everyone flagged at once", async ({ page, request }) => {
  await openAs(page, "OfficeAdmin", "/workforce");
  await expect(page.getByTestId("wf-stale")).toContainText("Ramos, Elena");
  await page.getByTestId("stale-all-OnPass").click();
  await expect.poll(async () => (await byA(request, "501")).status).toBe("OnPass");
});
