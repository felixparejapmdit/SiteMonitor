/**
 * Every Service 5 sidebar page has its SiteMonitor counterpart: Assignments
 * (the big table), Signatories (cards), Reassign Volunteers (with the notice),
 * the dashboard rosters, duty overrides, the OT encoder, org chart roles,
 * Archived Personnel, ID card trade filter, and Backup / Restore / Daily
 * Snapshot. Seeded with "Load sample data" (16 people with duty, OT, etc.).
 */
import { Page } from "@playwright/test";
import { expect, openAs, resetDatabase, test } from "./helpers";

test.describe.configure({ mode: "serial" });
test.setTimeout(120_000);

test.beforeAll(async ({ browser }) => {
  resetDatabase();
  const page = await browser.newPage();
  await openAs(page, "OfficeAdmin", "/");
  const res = await page.request.post("/api/settings/sample-data");
  expect(res.ok(), await res.text()).toBeTruthy();
  await page.close();
});

const workers = async (request: any) => (await request.get("/api/workers")).json();
const records = async (request: any, list: string) => (await request.get(`/api/workforce/records/${list}`)).json();
const watch = (page: Page) => {
  const problems: string[] = [];
  page.on("pageerror", (e) => problems.push(e.message));
  page.on("console", (m) => { if (m.type() === "error") problems.push(m.text()); });
  return problems;
};

test("Assignments: three views, filters, sorting, paging, bulk edit, generate forms", async ({ page, request }) => {
  const problems = watch(page);
  await openAs(page, "OfficeAdmin", "/assignments");
  await expect(page.getByTestId("assign-count")).toContainText("of 16 entries");
  await expect(page.getByTestId("assign-row")).toHaveCount(15);
  await page.getByTestId("assign-next").click();
  await expect(page.getByTestId("assign-row")).toHaveCount(1);
  await page.getByTestId("assign-per-page").selectOption("25");
  await expect(page.getByTestId("assign-row")).toHaveCount(16);

  await page.getByTestId("assign-trade").selectOption("Mason");
  await expect(page.getByTestId("assign-row")).toHaveCount(3);
  await page.getByTestId("assign-sort-name").click(); // descending
  await expect(page.getByTestId("assign-row").first()).toContainText("Ramon Dela Cruz");
  await page.getByTestId("assign-view-Personnel directory").click();
  await expect(page.getByRole("columnheader", { name: /Spouse/ })).toBeVisible();
  await page.getByTestId("assign-view-Task assignments").click();
  await expect(page.getByRole("columnheader", { name: /Specific task/ })).toBeVisible();

  await page.getByTestId("assign-select-page").check();
  await page.getByTestId("assign-bulk-edit").click();
  await page.getByTestId("bulk-field").selectOption("specificTask");
  await page.getByTestId("bulk-value").fill("Pour the slab");
  await page.getByTestId("bulk-confirm").click();
  await expect(page.getByText("3 people updated").first()).toBeVisible();
  expect((await workers(request)).filter((w: any) => w.task === "Pour the slab").length).toBe(3);

  await page.getByTestId("assign-select-page").check();
  await page.getByTestId("assign-forms").click();
  await expect(page).toHaveURL(/\/operations$/);
  // Service 5's "Generate Forms": the job order print view for the ticked people.
  await expect(page.getByTestId("jo-toolbar")).toBeVisible();
  await expect(page.frameLocator("#s5-preview").locator(".worker-form")).toHaveCount(3);

  // A past day shows the statuses of that day, read-only.
  await page.goto("/assignments");
  const yesterday = new Date(Date.now() - 86_400_000);
  await page.getByTestId("assign-day").fill(new Date(yesterday.getTime() - yesterday.getTimezoneOffset() * 60_000).toISOString().slice(0, 10));
  await expect(page.getByTestId("assign-past")).toBeVisible();
  await page.getByTestId("assign-cards").click();
  await expect(page.getByTestId("assign-card").first()).toBeVisible();
  expect(problems, problems.join("\n")).toEqual([]);
});

test("Signatories: the original Service 5 page, coverage and unbind saved to SiteMonitor", async ({ page, request }) => {
  const problems = watch(page);
  await openAs(page, "OfficeAdmin", "/operations");
  await page.getByTestId("tab-Signatories").click();
  const sig = page.frameLocator("[data-testid=s5-page-signatories]");
  await expect(sig.locator("body[data-ready=\"1\"]")).toBeAttached({ timeout: 20_000 });
  await expect(sig.locator("body")).toContainText("ACTIVE SIGNATORIES", { ignoreCase: true });
  await expect(sig.locator("body")).toContainText("Hall | Masons");
  // Unbind a group with its chip button; Service 5 saves, SiteMonitor keeps it.
  await sig.locator("button.sig-chip-remove[aria-label=\"Deselect Utility | Plumbers\"]").first().click();
  await expect.poll(async () => ((await records(request, "signatories")) as any[]).find((s) => s.name === "Engr. Carlos Medina").projectGroups.includes("Utility | Plumbers")).toBe(false);
  expect(problems, problems.join("\n")).toEqual([]);
});

test("Reassign Volunteers: several people to a project, logged, with the notice", async ({ page, request }) => {
  const problems = watch(page);
  await openAs(page, "OfficeAdmin", "/workforce");
  await page.getByTestId("tab-Reassign Volunteers").click();
  for (const name of ["Mario Reyes", "Danilo Ramos"]) {
    await page.getByTestId("realloc-search").fill(name.split(" ")[0]);
    await page.getByTestId("realloc-match").filter({ hasText: name }).click();
  }
  await expect(page.getByTestId("realloc-chip")).toHaveCount(2);
  await page.getByTestId("realloc-project").fill("Gazebo");
  await page.getByTestId("realloc-area").fill("Garden Area");
  await page.getByTestId("realloc-reason").fill("Roof push");
  await page.getByTestId("realloc-apply").click();
  await expect(page.getByTestId("realloc-notice")).toContainText("Workforce reallocation notice");
  await expect(page.getByTestId("realloc-notice")).toContainText("Mario Reyes");
  const moved = (await workers(request)).filter((w: any) => w.project === "Gazebo");
  expect(moved.map((w: any) => w.name).sort()).toEqual(["Danilo Ramos", "Mario Reyes"]);
  expect((await records(request, "assignment_log")).some((e: any) => e.field === "projectGroup" && e.newValue === "Gazebo" && e.reason === "Roof push")).toBeTruthy();
  expect(problems, problems.join("\n")).toEqual([]);
});

test("dashboard rosters, org chart trade roles, archived personnel", async ({ page, request }) => {
  const problems = watch(page);
  await openAs(page, "OfficeAdmin", "/workforce");
  await page.getByTestId("wf-donut-Absent").click();
  await expect(page.getByTestId("roster-modal")).toContainText("Paolo Garcia");
  await page.keyboard.press("Escape");

  // Service 5 org chart: edit a person trade role.
  await page.goto("/operations");
  await page.getByTestId("tab-Org Chart").click();
  await page.getByTestId("org-member").filter({ hasText: "Paolo Garcia" }).first().getByTestId("org-edit").click();
  await page.getByTestId("org-edit-role").selectOption("Kalihim");
  await page.getByTestId("org-edit-save").click();
  await expect.poll(async () => (await workers(request)).find((w: any) => w.a_number === "104").trade_role).toBe("Kalihim");

  await page.goto("/workforce");
  await page.getByTestId("tab-Archived Personnel").click();
  await expect(page.getByTestId("assign-status")).toHaveValue("archived");
  expect(problems, problems.join("\n")).toEqual([]);
});

test("ID card trade filter, visit prints, and the Backup / Restore / Daily Snapshot tabs", async ({ page }) => {
  const problems = watch(page);
  await openAs(page, "OfficeAdmin", "/workforce");
  await page.getByTestId("tab-ID Cards").click();
  await page.getByTestId("print-trade").selectOption("Plumber");
  await expect(page.locator(".id-card")).toHaveCount(2);
  // Service 5 has no minister summary print (its visit sheets are per visit, scheduled, or the last 7 days).
  await page.getByTestId("tab-Visitations").click();
  await expect(page.getByTestId("visit-print-ministers")).toHaveCount(0);
  await expect(page.getByTestId("visit-print-scheduled")).toBeVisible();

  await page.goto("/settings");
  await page.getByTestId("tab-Backup DB").click();
  await expect(page.getByTestId("pkg-make")).toBeVisible();
  await expect(page.getByTestId("workforce-export")).toBeVisible();
  await page.getByTestId("tab-Restore DB").click();
  await expect(page.getByTestId("pkg-restore")).toBeVisible();
  await expect(page.getByTestId("workforce-import")).toBeVisible();
  await page.getByTestId("tab-Daily Snapshot").click();
  await expect(page.getByTestId("take-snapshot")).toBeVisible();
  await expect(page.getByTestId("snapshot-row").first()).toBeVisible();
  expect(problems, problems.join("\n")).toEqual([]);
});
