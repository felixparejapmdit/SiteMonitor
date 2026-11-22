/**
 * Service 5 page parity: Assignments sits in Workforce records after the
 * Dashboard; Departments, Overnight Duty, OT and Signatories are Service 5's
 * own pages (saving to SiteMonitor); Visitations, Project Groups, Org Chart,
 * History, Dependability, Reassign and Update Forms follow Service 5's
 * layout; Ctrl+K quick search works on every page. All people are made up.
 */
import { Page, request as pwRequest } from "@playwright/test";
import { BASE_URL, expect, openAs, resetDatabase, test, userFor } from "./helpers";

test.describe.configure({ mode: "serial" });
test.setTimeout(120_000);

const iso = (offset: number) => {
  const d = new Date(Date.now() + offset * 86_400_000);
  return new Date(d.getTime() - d.getTimezoneOffset() * 60_000).toISOString().slice(0, 10);
};
const T0 = "2026-11-01T08:00:00.000Z";
const BACKUP = {
  version: 11,
  workers: [
    { aNumber: 801, name: "Abad, Rico", trade: "Mason", projectGroup: "Hall | Masons", projectRole: "Project Leader", tradeGroup: "Masonry Group", tradeRole: "Katiwala",
      dailyStatus: "Present", area: "Hall", project: "Footing", specificTask: "Rebar", updatedAt: T0,
      attendanceHistory: [{ date: iso(-2), newStatus: "Present", area: "Hall", specificTask: "Rebar" }, { date: iso(-1), newStatus: "Absent" }] },
    { aNumber: 802, name: "Bautista, Lea", trade: "Mason", projectGroup: "Hall | Masons", tradeGroup: "Masonry Group", tradeRole: "Kalihim", dailyStatus: "Present", area: "Hall", updatedAt: T0 },
    { aNumber: 803, name: "Cruz, Nilo", trade: "Painter", projectGroup: "Gate | Painters", tradeGroup: "Masonry Group", dailyStatus: "Absent", area: "Gate", updatedAt: T0 },
  ],
  signatories: [{ id: "sig-1", name: "Engr. Sig One", projectGroups: ["Hall | Masons"] }],
  ministers: [{ id: "min-1", name: "Bro. Min" }],
  visitations: [{ id: "vis-1", ministerName: "Bro. Min", status: "scheduled", date: iso(2), workerName: "Abad, Rico", aNumber: "801" }],
  duty_areas: [{ id: "da-gate", name: "Main Gate", workersNeeded: 2, roster: [801, 802], shiftTime: "22:00-06:00", color: "#3b82f6" }],
  ot_entries: [{ id: "ot-1", date: iso(0), volunteerId: "801", trade: "Mason", otArea: "Hall", otTask: "Footing", hours: 2, hourlyRate: 60, computedPay: 120 }],
  history: [{ id: "h1", dateStr: iso(-1), timestamp: Date.now() - 86_400_000, activeCount: 1, totalCount: 2,
    data: [{ aNumber: 801, name: "Abad, Rico", dailyStatus: "Present", area: "Hall", specificTask: "Rebar" }, { aNumber: 803, name: "Cruz, Nilo", dailyStatus: "Absent", absentReason: "Sick" }] }],
};

test.beforeAll(async () => {
  resetDatabase();
  const admin = await userFor(undefined as any, "OfficeAdmin");
  const ctx = await pwRequest.newContext({ baseURL: BASE_URL, extraHTTPHeaders: { authorization: `Bearer ${admin.token}` } });
  const res = await ctx.post(`/api/workforce/import?mode=Replace&file_name=pages.json`, {
    data: Buffer.from(JSON.stringify(BACKUP)), headers: { "content-type": "application/octet-stream" },
  });
  expect(res.ok(), await res.text()).toBeTruthy();
  await ctx.dispose();
});

const frame = (page: Page, name: string) => page.frameLocator(`[data-testid="s5-page-${name}"]`);
const ready = async (page: Page, name: string) => expect(frame(page, name).locator("body[data-ready='1']")).toBeAttached({ timeout: 20_000 });

test("sidebar: Assignments is in Workforce records, right after the Dashboard", async ({ page }) => {
  await openAs(page, "OfficeAdmin", "/workforce");
  const tabs = await page.locator("[data-testid^=tab-]").allTextContents();
  expect(tabs.slice(0, 2).map((t) => t.trim())).toEqual(["Dashboard", "Assignments"]);
  await page.getByTestId("tab-Assignments").click();
  await expect(page.getByText("Daily assignments")).toBeVisible();
});

test("Departments, Overnight Duty, OT and Signatories are Service 5's own pages", async ({ page }) => {
  await openAs(page, "OfficeAdmin", "/workforce");
  await page.getByTestId("tab-Departments").click();
  await ready(page, "departments");
  await expect(frame(page, "departments").getByText("Job-Site Deployability & Capacity")).toBeVisible();
  await expect(frame(page, "departments").getByText("Work Residences & Housing")).toBeVisible();

  await page.goto("/operations");
  await page.getByTestId("tab-Overnight Duty").click();
  await ready(page, "duty");
  const duty = frame(page, "duty");
  for (const b of ["Gen Week", "Clear Day", "Clear Week", "Print", "Week", "Month", "Roster"]) await expect(duty.getByRole("button", { name: b, exact: true }).first()).toBeVisible();
  await expect(duty.locator("#duty-areas-list")).toContainText("Main Gate");

  await page.getByTestId("tab-OT Entries").click();
  await ready(page, "ot-entries");
  await expect(frame(page, "ot-entries").getByText("ABAD, Rico").first()).toBeVisible();
  await page.getByTestId("tab-OT Calculator").click();
  await ready(page, "ot-calc");

  await page.getByTestId("tab-Signatories").click();
  await ready(page, "signatories");
  await expect(frame(page, "signatories").locator("input[value=\"Engr. Sig One\"]").first()).toBeAttached();
  await expect(frame(page, "signatories").locator("body")).toContainText("Hall | Masons");
});

test("Visitations: Service 5's metrics and table columns", async ({ page }) => {
  await openAs(page, "OfficeAdmin", "/workforce");
  await page.getByTestId("tab-Visitations").click();
  await expect(page.getByTestId("metric-upcoming-visits")).toContainText("1");
  await expect(page.getByTestId("metric-completed-month")).toContainText("Done This Month");
  await expect(page.getByTestId("metric-total-ministers")).toContainText("1");
  await expect(page.getByTestId("visit-schedule").locator("th")).toHaveText(["DATE", "VOLUNTEER", "HOUSING", "MINISTER", "STATUS", "ACTIONS"]);
});

test("Project Groups: stats, areas, member photos with role and trade", async ({ page }) => {
  await openAs(page, "OfficeAdmin", "/workforce");
  await page.getByTestId("tab-Project Groups").click();
  await expect(page.getByTestId("pg-stats")).toContainText("Total Groups");
  await expect(page.getByTestId("pg-area-pills")).toContainText("Hall (1)");
  const card = page.getByTestId("group-row").filter({ hasText: "Hall | Masons" });
  await expect(card).toContainText("Signatory: Engr. Sig One");
  await expect(card.getByTestId("group-member").first()).toContainText("Leader");
  await expect(card.getByTestId("group-member").first()).toContainText("Mason");
});

test("Org Chart: trade groups with Katiwala, II Katiwala, Kalihim and members", async ({ page, request }) => {
  await openAs(page, "OfficeAdmin", "/operations");
  await page.getByTestId("tab-Org Chart").click();
  const g = page.getByTestId("org-group").filter({ hasText: "MASONRY GROUP" });
  await expect(g).toContainText("Katiwala");
  await expect(g).toContainText("Abad, Rico");
  await expect(g).toContainText("Members (1)");
  await g.getByTestId("org-member").filter({ hasText: "Cruz, Nilo" }).getByTestId("org-edit").click();
  await page.getByTestId("org-edit-role").selectOption("II Katiwala");
  await page.getByTestId("org-edit-save").click();
  await expect.poll(async () => ((await (await request.get("/api/workers")).json()) as any[]).find((w) => w.a_number === "803").trade_role).toBe("II Katiwala");
});

test("History: snapshots on the left, the chosen one on the right; Project Roster History opens Dependability", async ({ page }) => {
  await openAs(page, "OfficeAdmin", "/operations");
  await page.getByTestId("tab-History Log").click();
  await expect(page.getByTestId("history-viewer")).toContainText("Select a snapshot");
  await page.getByTestId("history-item").first().click();
  await expect(page.getByTestId("history-viewer")).toContainText("Snapshot:");
  await expect(page.getByTestId("history-area")).toContainText("Abad, Rico");
  await expect(page.getByTestId("history-away")).toContainText("Absent - Sick");
  await page.getByTestId("history-project-roster").click();
  await expect(page.getByTestId("analytics-tab-project")).toHaveClass(/btn-primary/);
});

test("Dependability: Service 5's filter bar, KPIs, timeline and roster with photos", async ({ page }) => {
  await openAs(page, "OfficeAdmin", "/operations");
  await page.getByTestId("tab-Dependability").click();
  await page.getByTestId("dep-period").selectOption("0");
  await expect(page.getByTestId("dep-kpis")).toContainText("Reliability Index");
  await expect(page.getByTestId("dep-kpis")).toContainText("Versatility Score");
  await page.getByTestId("dep-row").filter({ hasText: "Abad, Rico" }).click();
  await expect(page.getByTestId("dep-banner")).toContainText("Abad, Rico");
  await expect(page.getByTestId("dep-kpis")).toContainText("50%");
  await expect(page.getByTestId("dep-timeline")).toContainText("Rebar");
});

test("Reassign Volunteers and Update Forms use Service 5's fields", async ({ page }) => {
  await openAs(page, "OfficeAdmin", "/workforce");
  await page.getByTestId("tab-Reassign Volunteers").click();
  for (const l of ["Target Project Group *", "Target Area / Site *", "New Tasking / Specific Task", "Reallocation Note / Reason (Optional)", "Add another volunteer to queue:"]) {
    await expect(page.getByText(l, { exact: true })).toBeVisible();
  }
  await page.getByTestId("tab-Update Forms").click();
  await expect(page.getByTestId("vru-notice")).toContainText("Bi-Annual Census / Update Cycle:");
  await expect(page.getByTestId("vru-count-preview")).toContainText("personalized update form(s)");
  await expect(page.getByTestId("vru-photo")).toBeChecked();
});

test("Ctrl+K quick search on any page, including inside Service 5's pages", async ({ page }) => {
  await openAs(page, "OfficeAdmin", "/");
  await page.keyboard.press("Control+k");
  await page.getByTestId("quick-search-input").fill("signat");
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(/\/operations$/);
  await ready(page, "signatories");
  await frame(page, "signatories").locator("body").click({ position: { x: 5, y: 5 } });
  await page.keyboard.press("Control+k");
  await expect(page.getByTestId("quick-search")).toBeVisible();
  await page.getByTestId("quick-search-input").fill("Abad");
  await expect(page.getByTestId("quick-search-hit").filter({ hasText: "Abad, Rico" })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("quick-search")).toHaveCount(0);
});
