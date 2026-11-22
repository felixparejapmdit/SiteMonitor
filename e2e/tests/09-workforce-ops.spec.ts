/**
 * Phases 2–6 of the Service 5 DB merge: attendance and reassignment (logged),
 * groups linked to jobs feeding the Crew page, leaders in the team picker,
 * overnight duty, overtime, the clinic ledger (office only, audited), the
 * unified sidebar per role, live refresh across tabs, and print set-ups.
 */
import { Page, request as pwRequest } from "@playwright/test";
import { BASE_URL, expect, openAs, resetDatabase, test, userFor, workItemByTitle, setClinic } from "./helpers";

test.describe.configure({ mode: "serial" });
test.setTimeout(120_000);

/** A small Service 5 backup (v11 shape) so this spec doesn't need real files. */
const BACKUP = {
  version: 11,
  workers: [
    { aNumber: "101", name: "Pedro Leader", groupRole: "Katiwala", projectGroup: "Team A", trade: "Mason", phone: "0917 000 0101", dailyStatus: "Present" },
    { aNumber: "102", name: "Rosa Member", groupRole: "Member", projectGroup: "Team A", trade: "Mason", dailyStatus: "Present" },
    { aNumber: "103", name: "Ben Member", groupRole: "Member", projectGroup: "Team A", trade: "Painter", dailyStatus: "Absent" },
    { aNumber: "104", name: "Lito Night", groupRole: "Member", projectGroup: "Team B", trade: "Guard", dailyStatus: "Present" },
  ],
  duty_areas: [{ id: "gate", name: "Main Gate", color: "#064e3b", shiftTime: "20:00", workersNeeded: 1, roster: [104, 102], lastRotationIndex: -1 }],
  ministers: [{ id: "m1", name: "Bro. Santos" }],
  project_groups_meta: { "Team A": "Site A", "Team B": "Site B" },
};


test.beforeAll(async () => {
  resetDatabase();
  await setClinic(true); // the clinic module is off by default
  const admin = await userFor(undefined as any, "OfficeAdmin");
  const ctx = await pwRequest.newContext({ baseURL: BASE_URL, extraHTTPHeaders: { authorization: `Bearer ${admin.token}` } });
  const res = await ctx.post("/api/workforce/import?file_name=seed.json&mode=Merge", {
    data: Buffer.from(JSON.stringify(BACKUP)), headers: { "content-type": "application/octet-stream" },
  });
  expect(res.ok(), await res.text()).toBeTruthy();
  await ctx.dispose();
});

const workers = async (request: any) => (await request.get("/api/workers")).json();
const records = async (request: any, list: string) => (await request.get(`/api/workforce/records/${list}`)).json();

test("attendance taps and Shift+R reassignments are logged like Service 5", async ({ page, request }) => {
  await openAs(page, "OfficeAdmin", "/personnel");
  const rosa = page.getByTestId("worker-row").filter({ hasText: "Rosa Member" });
  await rosa.getByTestId("mark-Absent").click();
  await expect(rosa.getByTestId("mark-Absent")).toHaveAttribute("aria-pressed", "true");
  const att = await records(request, "attendance_log");
  expect(att.some((e: any) => e.aNumber === "102" && e.oldStatus === "Present" && e.newStatus === "Absent")).toBeTruthy();

  await rosa.focus();
  await page.keyboard.press("Shift+R");
  await page.getByTestId("ra-group").fill("Team B");
  await page.getByTestId("ra-save").click();
  await expect(page.getByText("Rosa Member reassigned").first()).toBeVisible();
  const asg = await records(request, "assignment_log");
  expect(asg.some((e: any) => e.aNumber === "102" && e.field === "projectGroup" && e.oldValue === "Team A" && e.newValue === "Team B")).toBeTruthy();
  // Put Rosa back for the next tests.
  await request.put(`/api/workers/${(await workers(request)).find((w: any) => w.a_number === "102").id}/reassign`, { data: { project_group: "Team A", area: "", department: "", reason: "" } });
  await request.put(`/api/workers/${(await workers(request)).find((w: any) => w.a_number === "102").id}/status`, { data: { status: "Present", reason: "" } });
});

test("a group linked to a job feeds the Crew page roster", async ({ page, request }) => {
  const parking = await workItemByTitle(request, "Parking Conversion to Multi-Purpose Hall");
  await openAs(page, "OfficeAdmin", "/workforce");
  await page.getByTestId("tab-Project Groups").click();
  const team = page.getByTestId("group-row").filter({ hasText: "Team A" });
  await team.getByTestId("group-job").selectOption(parking.id);
  await expect(page.getByText("Group linked").first()).toBeVisible();

  await page.goto("/crew");
  await expect(page.getByTestId("workforce-today")).toContainText("3 Present");
  const row = page.getByTestId("crew-row").filter({ hasText: "Parking Conversion" });
  await expect(row.getByTestId("crew-roster")).toContainText("Roster: 2 present");
  await row.getByTestId("crew-use-roster").click();
  await expect(row.getByTestId("crew-count")).toHaveText("2");
});

test("group leaders can be picked as a project's team", async ({ page, request }) => {
  const parking = await workItemByTitle(request, "Parking Conversion to Multi-Purpose Hall");
  await openAs(page, "OfficeAdmin", `/projects/${parking.id}`);
  await page.getByTestId("edit-project").click();
  const lead = page.getByTestId("person-lead");
  await expect(lead.locator("optgroup")).toHaveAttribute("label", "Group leaders (Personnel)");
  await lead.selectOption({ label: "Pedro Leader · Katiwala, Team A" });
  await expect(page.getByText("Phone 0917 000 0101")).toBeVisible();
});

test("overnight duty: the original Service 5 screen generates the week from the roster", async ({ page, request }) => {
  await openAs(page, "OfficeAdmin", "/operations");
  await page.getByTestId("tab-Overnight Duty").click();
  const duty = page.frameLocator("[data-testid=s5-page-duty]");
  await expect(duty.locator("body[data-ready=\"1\"]")).toBeAttached({ timeout: 20_000 });
  await expect(duty.locator("#duty-areas-list")).toContainText("Main Gate");
  await duty.getByRole("button", { name: "Gen Week", exact: true }).click();
  await duty.locator("#confirm-modal.active").waitFor();
  await duty.locator("#confirm-modal-ok").click();
  await expect.poll(async () => ((await records(request, "duty_schedules")) as any[]).some((d) => d.assignments?.length > 0), { timeout: 20_000 }).toBe(true);
});

test("the clinic ledger is office-only and every change is audited", async ({ page, request }) => {
  const jun = await userFor(request, "SiteMonitor", "Jun");
  const vip = await userFor(request, "VipClient");
  for (const u of [jun, vip]) {
    expect((await request.get("/api/workforce/records/health_events", { headers: { authorization: `Bearer ${u.token}` } })).status()).toBe(403);
  }
  await openAs(page, "OfficeAdmin", "/clinic");
  await page.getByTestId("episode-add").click();
  await page.getByTestId("rf-workerId").selectOption("103");
  await page.getByTestId("rf-diagnosis").fill("Influenza");
  await page.getByTestId("rf-save").click();
  await expect(page.getByTestId("episodes-open")).toContainText("1 open");
  const audit = await records(request, "medical_audit_logs");
  expect(audit.some((a: any) => a.details === "health_events" && a.action === "saved")).toBeTruthy();
  // The audit trail itself can't be edited.
  expect((await request.put("/api/workforce/records/medical_audit_logs/x", { data: {} })).status()).toBe(400);
});

test("one unified sidebar: each role sees only its sections", async ({ browser }) => {
  const sidebarText = async (role: any, who?: string) => {
    const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
    const p = await ctx.newPage();
    await openAs(p, role, "/", who);
    const t = await p.getByTestId("sidebar").innerText();
    await ctx.close();
    return t.toLowerCase();
  };
  const admin = await sidebarText("OfficeAdmin");
  for (const s of ["estate command", "workforce & crew", "operations & duty", "system & admin", "personnel", "clinic ledger", "operations"]) expect(admin, s).toContain(s);
  const vip = await sidebarText("VipClient");
  for (const s of ["personnel", "clinic", "operations", "workforce", "settings"]) expect(vip, s).not.toContain(s);
  const monitor = await sidebarText("SiteMonitor", "Jun");
  expect(monitor).toContain("personnel");
  for (const s of ["clinic", "workforce records", "signatories", "settings"]) expect(monitor, s).not.toContain(s);
});

test("live refresh: another open screen sees an attendance change without reloading", async ({ browser }) => {
  const a = await browser.newContext(), b = await browser.newContext();
  const pa = await a.newPage(), pb = await b.newPage();
  await openAs(pa, "OfficeAdmin", "/personnel");
  await openAs(pb, "OfficeAdmin", "/personnel");
  // Both screens must be connected to live updates before the change is made.
  for (const p of [pa, pb]) await expect(p.locator('[title="Live updates are on"]')).not.toHaveCount(0, { timeout: 15_000 });
  await pa.getByTestId("worker-row").filter({ hasText: "Lito Night" }).getByTestId("mark-OnPass").click();
  await expect(pb.getByTestId("worker-row").filter({ hasText: "Lito Night" }).getByTestId("mark-OnPass")).toHaveAttribute("aria-pressed", "true", { timeout: 10_000 });
  await a.close();
  await b.close();
});

test("print set-ups: Service 5's 4-up folio job orders @safari", async ({ page }) => {
  await openAs(page, "OfficeAdmin", "/operations");
  await page.getByTestId("tab-Monthly Attendance").click();
  await expect(page.getByTestId("attendance-sheet")).toContainText("Pedro Leader");
  // Service 5 has no print for this grid.
  await expect(page.getByTestId("print-attendance")).toHaveCount(0);

  await page.getByTestId("tab-Print").click();
  // Job orders go only to people who can get one (Service 5's eligibility); its banner counts the rest.
  const f = page.frameLocator("#s5-preview");
  await expect(f.locator(".worker-form").filter({ hasText: "Pedro Leader" })).toHaveCount(1);
  await expect(f.locator(".worker-form").filter({ hasText: "Ben Member" })).toHaveCount(0);
  await expect(f.locator(".print-summary-feedback")).toContainText("skipped due to settings, pass, day-off, or absence");
  await page.evaluate(() => {
    const fr = document.getElementById("s5-preview") as HTMLIFrameElement;
    (fr.contentWindow as any).print = () => { (window as any).__printed = true; };
  });
  await page.getByTestId("s5-print-now").click();
  expect(await page.evaluate(() => (window as any).__printed)).toBe(true);
});
