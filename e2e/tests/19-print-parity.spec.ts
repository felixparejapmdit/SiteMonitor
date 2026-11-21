/**
 * Print parity with Service 5: every print button in SiteMonitor produces
 * Service 5's own printout (its template, title, layout and paper set-up),
 * the Assignments page's "Generate Forms" opens Service 5's 4-up folio job
 * order view, and the prints Service 5 doesn't have are gone.
 * All people here are made up.
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
    { aNumber: 901, name: "Abad, Rico", trade: "Mason", projectGroup: "Team P", projectRole: "Project Leader", dailyStatus: "Present", area: "Hall", project: "Footing",
      specificTask: "Rebar tying for the east footing", visitationGroup: "VG Alpha", visitationRole: "Katiwala", visitationHousing: "Bldg 1", birthday: "1980-02-03",
      spouseName: "Abad, Mila", children: [{ name: "Abad, Jo", birthday: "2012-05-05" }], hourlyRate: 60, updatedAt: T0,
      attendanceHistory: [{ date: iso(-2), newStatus: "Present", area: "Hall" }, { date: iso(-1), newStatus: "Absent", reason: "Sick" }] },
    { aNumber: 902, name: "Bautista, Lea", trade: "Mason", projectGroup: "Team P", dailyStatus: "Present", area: "Hall", project: "Footing",
      visitationGroup: "VG Alpha", visitationRole: "Member", visitationHousing: "Bldg 1", updatedAt: T0 },
    { aNumber: 903, name: "Cruz, Nilo", trade: "Painter", projectGroup: "Team Q", dailyStatus: "Absent", absentReason: "Sick", area: "Gate", updatedAt: T0 },
    { aNumber: 904, name: "Diaz, Omar", trade: "Painter", projectGroup: "Team Q", dailyStatus: "Present", jOrder: "NO", area: "Gate", updatedAt: T0 },
  ],
  signatories: [{ id: "sig-p", name: "Engr. Sig P", projectGroups: ["Team P"] }],
  visitation_groups: [{ id: "vg-alpha", name: "VG Alpha", housing: "Bldg 1" }],
  visitations: [
    { id: "vis-1", ministerName: "Bro. Min", status: "scheduled", date: iso(2), workerName: "Abad, Rico", aNumber: "901", notes: "Home visit" },
    { id: "vis-2", ministerName: "Bro. Min", status: "completed", date: iso(-2), completedAt: iso(-2), workerName: "Bautista, Lea", aNumber: "902" },
  ],
  ot_entries: [
    { id: "ot-1", date: iso(0), volunteerId: "901", trade: "Mason", otArea: "Hall", otTask: "Footing", hours: 2, hourlyRate: 60, computedPay: 120 },
    { id: "ot-2", date: iso(0), volunteerId: "902", trade: "Mason", otArea: "Hall", otTask: "Footing", hours: 1, hourlyRate: 60, computedPay: 60 },
  ],
  duty_areas: [{ id: "da-gate", name: "Main Gate", needed: 2, shiftTime: "8:00 PM - 6:00 AM" }],
  duty_schedules: [{ id: "ds-today", dateStr: iso(0), assignments: [{ areaId: "da-gate", aNumber: "902" }] }],
  history: [{ id: "h1", dateStr: iso(-1), data: [{ aNumber: 901, name: "Abad, Rico", dailyStatus: "Present", area: "Hall", project: "Footing" }] }],
  assignment_log: [{ id: "al-1", date: iso(0), timestamp: Date.now(), aNumber: "902", workerName: "Bautista, Lea", field: "projectGroup", oldValue: "Team Q", newValue: "Team P", reason: "Short of masons" }],
};

test.beforeAll(async () => {
  resetDatabase();
  const admin = await userFor(undefined as any, "OfficeAdmin");
  const ctx = await pwRequest.newContext({ baseURL: BASE_URL, extraHTTPHeaders: { authorization: `Bearer ${admin.token}` } });
  const res = await ctx.post(`/api/workforce/import?mode=Replace&file_name=print.json`, {
    data: Buffer.from(JSON.stringify(BACKUP)), headers: { "content-type": "application/octet-stream" },
  });
  expect(res.ok(), await res.text()).toBeTruthy();
  await ctx.dispose();
});

/** Opens a page, then records every Service 5 document instead of printing it. */
async function open(page: Page, path: string) {
  await openAs(page, "OfficeAdmin", path);
  await page.waitForFunction(() => !!(window as any).S5Print);
  await page.evaluate(() => {
    const w = window as any;
    w.__s5 = [];
    w.safePrint = (html: string) => w.__s5.push(html);
    w.open = () => {
      let buf = "";
      const doc = { open() { buf = ""; }, write(h: string) { buf += h; }, close() { w.__s5.push(buf); buf = ""; }, images: [], title: "" };
      return { document: doc, print() {}, close() {}, focus() {} };
    };
  });
}
const docs = (page: Page) => page.evaluate(() => (window as any).__s5 as string[]);
const titleOf = (h: string) => (/<title>([^<]*)<\/title>/i.exec(h) || [])[1] || "";
/** Clicks a print button and returns the Service 5 document it produced. */
async function printOf(page: Page, click: () => Promise<void>, title: string) {
  const before = (await docs(page)).length;
  await click();
  await expect.poll(async () => (await docs(page)).slice(before).map(titleOf), { message: `printed "${title}"` }).toContainEqual(expect.stringContaining(title));
  const out = (await docs(page)).slice(before);
  return out.find((h) => titleOf(h).includes(title))!;
}
const by = (page: Page, id: string) => () => page.getByTestId(id).click();
const preview = (page: Page) => page.frameLocator("#s5-preview");

test("Assignments: Service 5's Print Reports menu and Generate Forms (4-up folio job orders)", async ({ page }) => {
  await open(page, "/assignments");
  await page.getByTestId("assign-reports").click();
  await expect(page.getByTestId("assign-reports-menu").locator("button")).toHaveText([
    "📋 Volunteer & Family Roster", "📋 List & Attendance Generator", "📝 Semi-Annual Records Update Forms", "By Trade", "By Project Group", "By Housing",
  ]);
  expect(await printOf(page, by(page, "assign-report-by-trade"), "Personnel Directory - By Trade")).toContain("Mason");
  for (const [k, t] of [["group", "By Project Group"], ["housing", "By Housing Group"]]) {
    await page.getByTestId("assign-reports").click();
    await printOf(page, by(page, `assign-report-by-${k}`), `Personnel Directory - ${t}`);
  }
  await page.getByTestId("assign-reports").click();
  expect(await printOf(page, by(page, "assign-report-family"), "Volunteer &amp; Family Directory Roster")).toContain("Abad, Mila");

  // Generate Forms for everyone ticked; Service 5 skips the absent and "no job order" people.
  for (const n of ["Abad, Rico", "Bautista, Lea", "Cruz, Nilo", "Diaz, Omar"]) {
    await page.getByRole("row").filter({ hasText: n }).locator("input[type=checkbox]").first().check();
  }
  await page.getByTestId("assign-forms").click();
  await expect(page.getByTestId("jo-toolbar")).toBeVisible();
  await expect(page.getByTestId("print-date")).toHaveValue(iso(1));
  const f = preview(page);
  await expect(f.locator(".worker-form")).toHaveCount(2);
  await expect(f.locator(".print-summary-feedback")).toContainText("Generated 2 Job Orders (2 skipped");
  await expect(f.locator(".worker-form").first()).toContainText("JOB ORDER");
  await expect(f.locator(".worker-form").first()).toContainText("Note: Makikisuyo po na isumite po ito sa JOBSITE Office bago po umuwi.");
  await expect(f.locator(".worker-form").first()).toContainText("Rebar tying for the east footing");
  await expect(f.locator(".sig-name-input").first()).toHaveValue("Engr. Sig P");
  const pageSpec = await page.evaluate(() => (document.getElementById("s5-preview") as HTMLIFrameElement).contentDocument!.getElementById("global-page-spec")!.textContent);
  expect(pageSpec).toMatch(/size: 8\.5in 13\.0in;\s*margin: 0;/);
  expect(pageSpec).toContain("grid-template-rows: repeat(4, 1fr)");
  // Printed: 4 slips per folio page, none cut off.
  await page.emulateMedia({ media: "print" });
  const clipped = await page.evaluate(() => {
    const d = (document.getElementById("s5-preview") as HTMLIFrameElement).contentDocument!;
    return [...d.querySelectorAll(".worker-form")].filter((el) => el.scrollHeight > (el as HTMLElement).offsetHeight + 1).length;
  });
  await page.emulateMedia({ media: "screen" });
  expect(clipped).toBe(0);
  // Service 5's batch modes.
  await page.getByTestId("print-by").selectOption("by-signatory");
  await expect(f.getByText("Signatory Batch: Engr. Sig P")).toBeVisible();
  await page.getByTestId("print-by").selectOption("by-project-group");
  await expect(f.getByText("Project Group: Team P")).toBeVisible();
  await expect(page.getByTestId("s5-print-now")).toBeVisible();
});

test("ID cards page: Print Selected (info cards) and Generate List", async ({ page }) => {
  await open(page, "/workforce");
  await page.getByTestId("tab-ID Cards").click();
  await page.getByTestId("card-pick").first().check();
  await printOf(page, by(page, "print-now"), "Personnel Info Cards");
  await page.getByTestId("cards-group-name").fill("Pour team");
  expect(await printOf(page, by(page, "print-make-list"), "Pour team")).toContain("Volunteer List");
});

test("Personnel: Print Reports menu and Print & Export", async ({ page }) => {
  await open(page, "/personnel");
  await page.getByTestId("personnel-reports").click();
  await printOf(page, by(page, "personnel-report-by-trade"), "Personnel Directory - By Trade");
  await page.locator("[data-testid=worker-row] input[type=checkbox]").first().check();
  await page.getByTestId("print-export-open").click();
  await printOf(page, by(page, "pe-print"), "Volunteer &amp; Family Directory Roster");
});

test("Profile: family info, attendance dossier, update form and assignment history", async ({ page }) => {
  await open(page, "/personnel");
  await page.getByTestId("worker-row").filter({ hasText: "Abad, Rico" }).getByTestId("profile-worker").click();
  await page.getByTestId("ptab-family").click();
  expect(await printOf(page, by(page, "family-print"), "Family Information")).toContain("Abad, Jo");
  await page.getByTestId("ptab-attendance").click();
  expect(await printOf(page, by(page, "att-print-dossier"), "Attendance Dossier : Abad, Rico")).toContain("Fit for Full Duty");
  await printOf(page, by(page, "att-print-update"), "Semi-Annual Volunteer Records Update Forms");
  await printOf(page, by(page, "att-assignment-history"), "Assignment History");
});

test("Attendance generator and records update forms", async ({ page }) => {
  await open(page, "/workforce");
  await page.getByTestId("tab-Attendance Gen").click();
  await page.getByTestId("ag-select-all").click();
  expect(await printOf(page, by(page, "ag-print"), "Attendance Sheet")).toContain("Abad, Rico");
  await page.getByTestId("tab-Update Forms").click();
  expect(await printOf(page, by(page, "vru-print"), "Semi-Annual Volunteer Records Update Forms")).toContain("8.5in 13in");
});

test("Visitations: scheduled, one visit's form; groups: rosters and monthly sheets", async ({ page }) => {
  await open(page, "/workforce");
  await page.getByTestId("tab-Visitations").click();
  await expect(page.getByTestId("visit-print-ministers")).toHaveCount(0);
  await page.getByTestId("visit-print-scheduled").click();
  await expect(preview(page).locator(".visitation-print-page")).toHaveCount(1);
  await expect(preview(page).locator(".visitation-print-page")).toContainText("BRO. MIN");
  await page.keyboard.press("Escape");
  await page.getByTestId("visit-print-form").first().click();
  await expect(preview(page).locator(".visitation-print-page")).toHaveCount(1);
  await page.keyboard.press("Escape");
  await page.getByTestId("visit-view-Groups").click();
  await printOf(page, by(page, "vgroup-print-all-rosters"), "All Visitation Groups Summary");
  await printOf(page, by(page, "vgroup-print-roster"), "VG Alpha - Visitation Roster");
  await printOf(page, by(page, "vgroup-print-month"), "Monthly Attendance - VG Alpha");
  const before = (await docs(page)).length;
  await page.getByTestId("vgroup-print-all").click();
  await expect.poll(async () => (await docs(page)).length).toBe(before + 1);
  expect((await docs(page))[before]).toContain("VG Alpha");
});

test("Operations: overtime, duty, dependability, org chart job orders", async ({ page }) => {
  await open(page, "/operations");
  await page.getByTestId("tab-OT Calculator").click();
  await printOf(page, by(page, "ots-print"), "OVERTIME SCHEDULE");
  await printOf(page, by(page, "ots-print-matrix"), "Overtime Compensation Breakdown");
  await printOf(page, by(page, "ots-print-daily"), "Daily OT Breakdown");
  await page.getByTestId("tab-OT Entries").click();
  await printOf(page, by(page, "ot-schedule-print"), "OVERTIME SCHEDULE");
  await printOf(page, by(page, "ot-matrix-print"), "Overtime Compensation Breakdown");
  await page.getByTestId("tab-Overnight Duty").click();
  expect(await printOf(page, by(page, "duty-print"), "Night Duty Roster")).toContain("Main Gate");
  await page.getByTestId("tab-Dependability").click();
  await printOf(page, by(page, "print-tab"), "Dependability Roster");
  await page.getByTestId("tab-Monthly Attendance").click();
  await expect(page.getByTestId("print-attendance")).toHaveCount(0);
  await page.getByTestId("tab-Org Chart").click();
  await page.getByTestId("org-print-jo").click();
  await expect(preview(page).locator(".worker-form")).toHaveCount(2);
});

test("Project groups: one group, Print All, and the group change report", async ({ page }) => {
  await open(page, "/workforce");
  await page.getByTestId("tab-Project Groups").click();
  await printOf(page, () => page.getByTestId("group-row").filter({ hasText: "Team P" }).getByTestId("group-print").click(), "Project Group - Team P");
  await printOf(page, by(page, "groups-print"), "All Project Groups");
  await page.getByTestId("groups-log").click();
  expect(await printOf(page, by(page, "group-log-print"), "Group Change Report")).toContain("Short of masons");
});

test("Dashboard: only the Present roster prints (Service 5's Print All Present)", async ({ page }) => {
  await open(page, "/workforce");
  await page.getByTestId("tab-Dashboard").click();
  await page.getByTestId("wf-area").first().click();
  await expect(page.getByTestId("roster-modal")).toBeVisible();
  await expect(page.getByTestId("roster-print")).toHaveCount(0);
  await page.keyboard.press("Escape");
  await page.locator("[data-testid^=wf-donut-]").filter({ hasText: "Present" }).first().click();
  await printOf(page, by(page, "roster-print"), "Present Volunteers Attendance Sheet");
});

test("prints Service 5 doesn't have are gone", async ({ page }) => {
  await open(page, "/workforce");
  await page.getByTestId("tab-Reassign Volunteers").click();
  await expect(page.getByTestId("realloc-print")).toHaveCount(0);
  await open(page, "/operations");
  await page.getByTestId("tab-Duty History").click();
  await expect(page.getByTestId("duty-month-print")).toHaveCount(0);
  await page.getByTestId("tab-Print").click();
  await expect(page.getByTestId("print-kind")).toHaveCount(0);
  await expect(page.getByTestId("jo-toolbar")).toBeVisible();
});
