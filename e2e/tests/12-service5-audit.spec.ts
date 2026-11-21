/**
 * Gaps found by the screen-by-screen audit against Service 5 DB: staged
 * assignments and Start New Day, the directory's filters / columns / bulk
 * actions / archive reasons / delete, job-order eligibility, the records update
 * forms and list sheet, visitations (schedule, done, coverage), visitation
 * group members, overtime copy and summary, project group management,
 * signatory bulk assignment, dossiers, project history and selective backups.
 */
import { createDecipheriv, pbkdf2Sync } from "node:crypto";
import { Page, request as pwRequest } from "@playwright/test";
import { BASE_URL, expect, openAs, resetDatabase, test, userFor, setClinic } from "./helpers";

test.describe.configure({ mode: "serial" });
test.setTimeout(120_000);

const iso = (offset: number) => {
  const d = new Date(Date.now() + offset * 86_400_000);
  return new Date(d.getTime() - d.getTimezoneOffset() * 60_000).toISOString().slice(0, 10);
};
const BACKUP = {
  version: 11,
  workers: [
    { aNumber: "301", name: "Ana Present", trade: "Mason", projectGroup: "Team X", area: "Hall Area", department: "Job-site Construction", dailyStatus: "Present",
      medical: "Hypertension", jOrder: "YES", visitationHousing: "Bunk 1", hourlyRate: 80,
      tomorrowAssignment: { specificTask: "Lay blocks", area: "Hall Area", project: "Hall", date: iso(0), isAssigned: true, dailyStatus: "Present" } },
    { aNumber: "302", name: "Ben NoJO", trade: "Mason", projectGroup: "Team X", dailyStatus: "Present", jOrder: "NO", visitationHousing: "Bunk 1" },
    { aNumber: "303", name: "Cara Pass", trade: "Painter", projectGroup: "Team Y", dailyStatus: "Passes", visitationHousing: "Bunk 2",
      tomorrowAssignment: { specificTask: "Prime walls", date: iso(1), isAssigned: true } },
    { aNumber: "304", name: "Dino Extra", trade: "Painter", projectGroup: "Team Y", dailyStatus: "Present" },
  ],
  signatories: [{ id: "sig1", name: "Sig One", projectGroups: ["Team X"] }],
  visitation_groups: [{ id: "vg1", name: "Dalaw 1", housing: "Bunk 1" }],
  ministers: [{ id: "m1", name: "Bro. Cruz" }],
  ot_entries: [{ id: "ot1", date: iso(-1), volunteerId: "301", hours: 2, hourlyRate: 80, computedPay: 160, title: "Pour" }],
};

test.beforeAll(async () => {
  resetDatabase();
  await setClinic(true); // the clinic module is off by default
  const admin = await userFor(undefined as any, "OfficeAdmin");
  const ctx = await pwRequest.newContext({ baseURL: BASE_URL, extraHTTPHeaders: { authorization: `Bearer ${admin.token}` } });
  const res = await ctx.post("/api/workforce/import?file_name=seed.json", { data: Buffer.from(JSON.stringify(BACKUP)), headers: { "content-type": "application/octet-stream" } });
  expect(res.ok(), await res.text()).toBeTruthy();
  await ctx.dispose();
});

const workers = async (request: any, token?: string) => (await request.get("/api/workers", token ? { headers: { authorization: `Bearer ${token}` } } : {})).json();
const byA = async (request: any, a: string) => (await workers(request)).find((w: any) => w.a_number === a);
const records = async (request: any, list: string) => (await request.get(`/api/workforce/records/${list}`)).json();
/** Opens an encrypted (.agwcz) download, as the importer does. */
function open(env: any, pass: string) {
  const key = pbkdf2Sync(pass, Buffer.from(env.salt, "base64"), env.iterations, 32, "sha256");
  const data = Buffer.from(env.data, "base64");
  const d = createDecipheriv("aes-256-gcm", key, Buffer.from(env.iv, "base64"));
  d.setAuthTag(data.subarray(data.length - 16));
  return JSON.parse(Buffer.concat([d.update(data.subarray(0, data.length - 16)), d.final()]).toString("utf8"));
}
const row = (page: Page, name: string) => page.getByTestId("worker-row").filter({ hasText: name });

test("Service 5 staged assignments show as tasks, and Start New Day applies the ones due today", async ({ page, request }) => {
  expect((await byA(request, "301")).tomorrow).toBe("Lay blocks"); // stored as an object, read as its task
  expect((await byA(request, "303")).tomorrow).toBe("Prime walls");
  await openAs(page, "OfficeAdmin", "/personnel");
  await page.getByTestId("start-day").click();
  await page.getByTestId("start-day-confirm").click();
  await expect(page.getByText("New day started: 1 staged assignment activated").first()).toBeVisible();
  const ana = await byA(request, "301");
  expect([ana.task, ana.project, ana.tomorrow, ana.assigned_today]).toEqual(["Lay blocks", "Hall", "", true]);
  expect((await byA(request, "303")).tomorrow).toBe("Prime walls"); // dated tomorrow: still staged
  const snaps = await (await request.get("/api/workforce/snapshots")).json();
  expect(snaps.some((s: any) => s.reason === "before Start New Day")).toBeTruthy();
});

test("directory: filters, columns, bulk status, archive with a reason, restore, delete, CSV", async ({ page, request }) => {
  await openAs(page, "OfficeAdmin", "/personnel");
  await page.getByTestId("f-trade").selectOption("Painter");
  await expect(page.getByTestId("worker-row")).toHaveCount(2);
  await page.getByTestId("f-trade").selectOption("");
  await page.getByTestId("hide-assigned").check();
  await expect(row(page, "Ana Present")).toHaveCount(0); // assigned today by Start New Day
  await page.getByTestId("hide-assigned").uncheck();

  await page.getByTestId("columns").click();
  await page.getByTestId("col-medical").check();
  await expect(row(page, "Ana Present").getByTestId("worker-extras")).toContainText("Medical: Hypertension");
  await expect(row(page, "Ana Present").getByTestId("worker-extras")).toContainText("Task: Lay blocks");
  await expect(row(page, "Ben NoJO")).toContainText("No job order");

  const download = page.waitForEvent("download");
  await page.getByTestId("export-csv").click();
  const csv = await (await (await download).createReadStream()).toArray();
  expect(Buffer.concat(csv).toString().split("\n")[0]).toContain("A#,Name,Status,Trade");

  await row(page, "Ben NoJO").getByTestId("select-worker").check();
  await row(page, "Dino Extra").getByTestId("select-worker").check();
  await expect(page.getByTestId("bulk-bar")).toContainText("2 selected");
  await page.getByTestId("bulk-absent").click();
  await page.getByTestId("bulk-confirm").click();
  await expect(page.getByText("2 people updated").first()).toBeVisible();
  expect((await byA(request, "302")).status).toBe("Absent");
  expect((await byA(request, "304")).status).toBe("Absent");

  await row(page, "Dino Extra").getByTestId("select-worker").check();
  await page.getByTestId("bulk-archive").click();
  await page.getByTestId("bulk-category").selectOption("Retired");
  await page.getByTestId("bulk-confirm").click();
  await expect.poll(async () => (await byA(request, "304")).archive_category).toBe("Retired");
  await page.getByTestId("count-Archived").click();
  await page.getByTestId("f-cat").selectOption("Retired");
  await expect(row(page, "Dino Extra")).toContainText("Retired");

  await row(page, "Dino Extra").getByTestId("select-worker").check();
  await page.getByTestId("bulk-restore").click();
  await page.getByTestId("bulk-confirm").click();
  await expect.poll(async () => (await byA(request, "304")).archived).toBe(false);

  await page.getByTestId("count-Total active").click();
  await row(page, "Dino Extra").getByTestId("select-worker").check();
  await page.getByTestId("bulk-delete").click();
  await page.getByTestId("bulk-confirm").click();
  await expect(page.getByText("1 person deleted (a snapshot was kept)").first()).toBeVisible();
  expect(await byA(request, "304")).toBeUndefined();
  const snaps = await (await request.get("/api/workforce/snapshots")).json();
  expect(snaps.some((s: any) => s.reason === "before deleting 1 person")).toBeTruthy();
});

test("job orders skip people who are off or turned off; update forms and the list sheet", async ({ page }) => {
  await openAs(page, "OfficeAdmin", "/operations");
  await page.getByTestId("tab-Print").click();
  // Service 5's print view: only people who can get a job order; its banner counts the rest.
  const f = page.frameLocator("#s5-preview");
  await expect(f.locator(".worker-form")).toHaveCount(1);
  await expect(f.locator(".worker-form")).toContainText("Ana Present");
  await expect(f.locator(".worker-form")).toContainText("Lay blocks");
  await expect(f.locator(".print-summary-feedback")).toContainText("Generated 1 Job Order (2 skipped");

  // Service 5's update forms and list sheet (captured instead of printed).
  await page.evaluate(() => { const w = window as any; w.__s5 = []; w.safePrint = (h: string) => w.__s5.push(h); });
  const last = () => page.evaluate(() => { const a = (window as any).__s5; return a[a.length - 1] || ""; });
  await page.goto("/workforce");
  await page.evaluate(() => { const w = window as any; w.__s5 = []; w.safePrint = (h: string) => w.__s5.push(h); });
  await page.getByTestId("tab-Update Forms").click();
  await page.getByTestId("vru-print").click();
  await expect.poll(last).toContain("Semi-Annual Volunteer Records Update Forms");
  expect(await last()).toContain("Hypertension");
  await page.getByTestId("tab-Attendance Gen").click();
  await page.getByTestId("ag-title").fill("Pour team");
  await page.getByTestId("ag-select-all").click();
  await page.getByTestId("ag-print").click();
  await expect.poll(last).toContain("Pour team");
  for (const n of ["Ana Present", "Ben NoJO", "Cara Pass"]) expect(await last()).toContain(n);
});

test("visitations: schedule, mark done, coverage; visitation group members and the monthly sheet", async ({ page, request }) => {
  await openAs(page, "OfficeAdmin", "/workforce");
  await page.getByTestId("tab-Visitations").click();
  await page.getByTestId("visit-add").click();
  await page.getByTestId("rf-aNumber").selectOption("301");
  await page.getByTestId("rf-ministerName").selectOption("Bro. Cruz");
  await page.getByTestId("rf-save").click();
  await expect(page.getByTestId("visit-row")).toHaveCount(1);
  await page.getByTestId("visit-done").click();
  await expect(page.getByText("Visit marked done").first()).toBeVisible();
  await page.getByTestId("visit-view-History").click();
  await expect(page.getByTestId("record-row").filter({ hasText: "Ana Present" })).toContainText("completed");
  await page.getByTestId("visit-view-Coverage").click();
  await expect(page.getByTestId("visit-unvisited")).toContainText("Ben NoJO");
  await expect(page.getByTestId("visit-unvisited")).not.toContainText("Ana Present");

  await page.getByTestId("visit-view-Groups").click();
  await page.getByText(/^Add members/).click();
  await page.locator("label", { hasText: "Ben NoJO" }).getByTestId("vgroup-unassigned").check();
  await page.getByTestId("vgroup-assign").click();
  await expect(page.getByTestId("vgroup-member").filter({ hasText: "Ben NoJO" })).toBeVisible();
  await page.getByTestId("vgroup-member").filter({ hasText: "Ben NoJO" }).getByTestId("vgroup-role").selectOption("Katiwala");
  await expect.poll(async () => (await byA(request, "302")).visit_role).toBe("Katiwala");
  await page.evaluate(() => { const w = window as any; w.__s5 = []; w.safePrint = (h: string) => w.__s5.push(h); });
  await page.getByTestId("vgroup-print-month").click();
  // Service 5's monthly sheet: landscape, with the group's officers.
  const sheet = () => page.evaluate(() => (window as any).__s5[0] || "");
  await expect.poll(sheet).toContain("TALAAN NG PRESENSYA");
  expect(await sheet()).toMatch(/size:\s*landscape/);
  expect(await sheet()).toContain("Ben NoJO");
});

test("overtime: copy the day before, then the calculator summary", async ({ page }) => {
  await openAs(page, "OfficeAdmin", "/operations");
  await page.getByTestId("tab-OT Entries").click();
  await expect(page.getByTestId("ot-total")).toContainText("0 entries");
  await page.getByTestId("ot-copy-yesterday").click();
  await expect(page.getByText(/1 entry copied from/).first()).toBeVisible();
  await expect(page.getByTestId("ot-total")).toContainText("1 entries");
  await page.getByTestId("ot-prev").click();
  await expect(page.getByTestId("ot-total")).toContainText("₱160.00");

  await page.getByTestId("tab-OT Calculator").click();
  await page.getByTestId("ots-from").fill(iso(-3));
  await expect(page.getByTestId("ots-total")).toContainText("₱160.00");
  await expect(page.getByTestId("record-row").filter({ hasText: "Ana Present" })).toContainText("₱160.00");
  await page.getByTestId("ots-view-Matrix").click();
  await expect(page.getByTestId("ots-matrix")).toContainText("Ana Present");
});

test("project groups: create with members and a signatory, change log; signatory bulk assignment", async ({ page, request }) => {
  await openAs(page, "OfficeAdmin", "/workforce");
  await page.getByTestId("tab-Project Groups").click();
  await page.getByTestId("group-create").click();
  await page.getByTestId("gm-name").fill("Team Z");
  await page.getByTestId("gm-area").fill("Garden");
  await page.locator("label", { hasText: "Ben NoJO" }).getByTestId("gm-member").check();
  await page.getByTestId("gm-sig").selectOption("sig1");
  await page.getByTestId("gm-reason").fill("New crew");
  await page.getByTestId("gm-save").click();
  await expect(page.getByText("Group saved").first()).toBeVisible();
  expect((await byA(request, "302")).project_group).toBe("Team Z");
  expect((await records(request, "signatories")).find((s: any) => s.id === "sig1").projectGroups).toEqual(["Team X", "Team Z"]);
  await expect(page.getByTestId("group-row").filter({ hasText: "Team Z" })).toContainText("Garden");
  await page.getByTestId("groups-log").click();
  await expect(page.getByTestId("group-log")).toContainText("New crew");
  await page.keyboard.press("Escape");

  await page.goto("/operations");
  await page.getByTestId("tab-Signatories").click();
  await page.getByTestId("sig-assign-open").click();
  await page.getByTestId("sig-assign-who").selectOption("sig1");
  await page.locator("label", { hasText: "Team X" }).getByTestId("sig-assign-group").uncheck();
  await page.getByTestId("sig-assign-save").click();
  await expect(page.getByText("Groups assigned").first()).toBeVisible();
  expect((await records(request, "signatories")).find((s: any) => s.id === "sig1").projectGroups).toEqual(["Team Z"]);
});

test("dossier, project history, selective backup, and the medical column stays with the clinic", async ({ page, request }) => {
  await openAs(page, "OfficeAdmin", "/operations");
  await page.getByTestId("tab-History Log").click();
  await page.getByTestId("history-view-Person dossier").click();
  await page.getByTestId("dossier-who").selectOption("302");
  await expect(page.getByTestId("dossier")).toContainText("Ben NoJO");
  await expect(page.getByTestId("dossier")).toContainText("Present → Absent");
  await page.getByTestId("history-view-Project roster history").click();
  await expect(page.getByTestId("ph-project")).toHaveValue("Hall");
  await expect(page.getByTestId("record-row").first()).toContainText("Ana Present");

  const headers = { "x-backup-passphrase": Buffer.from("audit passphrase", "utf8").toString("hex") };
  const partial = open(await (await request.get("/api/workforce/export?collections=workers", { headers })).json(), "audit passphrase");
  expect(Object.keys(partial).sort()).toEqual(["exportedAt", "format", "version", "workers"]);

  // A Site Monitor (no clinic access) who can see Ana does not see her medical conditions.
  const map = await (await request.get("/api/estate/map")).json();
  const hall = map.areas.find((n: any) => n.area.name.startsWith("Multi-Purpose Hall")).area.id;
  expect((await request.put("/api/workforce/records/area_zones/Hall%20Area", { data: { zone_id: hall } })).ok()).toBeTruthy();
  const jun = await userFor(request, "SiteMonitor", "Jun");
  const seen = (await workers(request, jun.token)).find((w: any) => w.a_number === "301");
  expect(seen.medical).toBe("");
  expect((await byA(request, "301")).medical).toBe("Hypertension");
});
