/**
 * Service 5's daily encoder (group slip, grid, card; today and tomorrow), the
 * OT screens (filters, rate column, Load from yesterday, Clear), the grouped
 * Print & Export picker, the merge review (differential sync with collision
 * choices), and every legacy backup generation importing cleanly.
 * All people here are made up.
 */
import { readFileSync } from "node:fs";
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
    { aNumber: 701, name: "Abad, Rico", trade: "Mason", skillLevel: "3", projectGroup: "Team E", dailyStatus: "Present", hourlyRate: 60, updatedAt: T0,
      personalInfo: { contact: "0917 000 0701", updatedAt: T0 } },
    { aNumber: 702, name: "Bautista, Lea", trade: "Mason", projectGroup: "Team E", dailyStatus: "Present", updatedAt: T0, personalInfo: { contact: "0917 000 0702", updatedAt: T0 } },
    { aNumber: 703, name: "Cruz, Nilo", trade: "Painter", projectGroup: "Team F", dailyStatus: "Present", updatedAt: T0 },
  ],
  signatories: [{ id: "sig-e", name: "Sig E", projectGroups: ["Team E"] }],
  ot_entries: [
    { id: "ot-y1", date: iso(-1), volunteerId: "701", trade: "Mason", otArea: "Hall", otTask: "Footing", hours: 2, hourlyRate: 60, computedPay: 120 },
    { id: "ot-y2", date: iso(-1), volunteerId: "703", trade: "Painter", otArea: "Gate", otTask: "Primer", hours: 1, hourlyRate: 50, computedPay: 50 },
  ],
};

async function adminApi() {
  const admin = await userFor(undefined as any, "OfficeAdmin");
  return pwRequest.newContext({ baseURL: BASE_URL, extraHTTPHeaders: { authorization: `Bearer ${admin.token}` } });
}
const importJson = async (ctx: any, name: string, body: unknown, mode = "Merge") =>
  ctx.post(`/api/workforce/import?mode=${mode}&file_name=${name}`, { data: Buffer.from(JSON.stringify(body)), headers: { "content-type": "application/octet-stream" } });

test.beforeAll(async () => {
  resetDatabase();
  const ctx = await adminApi();
  const res = await importJson(ctx, "enc.json", BACKUP, "Replace");
  expect(res.ok(), await res.text()).toBeTruthy();
  await ctx.dispose();
});

const workers = async (request: any) => (await request.get("/api/workers")).json();
const byA = async (request: any, a: string) => (await workers(request)).find((w: any) => w.a_number === a);
const records = async (request: any, list: string) => (await request.get(`/api/workforce/records/${list}`)).json();
const openEncoder = async (page: Page, mode: "today" | "tomorrow") => {
  await openAs(page, "OfficeAdmin", "/operations");
  await page.getByTestId("tab-Encoder").click();
  await page.getByTestId(`encoder-${mode}`).click();
};

test("encoder: group slip with quick-fill, status pill, signatory and progress", async ({ page, request }) => {
  await openEncoder(page, "today");
  await expect(page.getByTestId("enc-slip-item")).toHaveCount(2);
  await expect(page.getByTestId("enc-slip")).toContainText("Team E");
  await expect(page.getByTestId("enc-slip")).toContainText("Signatory: Sig E");
  await page.getByTestId("enc-master-area").fill("Hall");
  await page.getByTestId("enc-master-project").fill("Main Hall");
  await page.getByTestId("enc-master-task").fill("Lay blocks");
  await page.getByTestId("enc-fill").click();
  await expect.poll(async () => (await workers(request)).filter((w: any) => w.task === "Lay blocks" && w.area === "Hall" && w.project === "Main Hall").length).toBe(2);
  await expect(page.getByTestId("enc-updated")).toContainText("Updated: 2");
  await expect.poll(async () => (await records(request, "assignment_log")).filter((e: any) => e.field === "specificTask" && e.newValue === "Lay blocks").length).toBe(2);
  expect((await byA(request, "701")).assigned_today).toBe(true);

  const lea = page.getByTestId("enc-slip-row").filter({ hasText: "Bautista, Lea" });
  await lea.getByTestId("enc-status").click();
  await expect.poll(async () => (await byA(request, "702")).status).toBe("Absent");
  await expect(lea.getByTestId("enc-area")).toBeDisabled();

  await page.getByTestId("enc-slip-next").click();
  await expect(page.getByTestId("enc-slip")).toContainText("Team F");
  await page.getByTestId("enc-print-this").click();
  await expect(page.getByTestId("enc-print")).toBeVisible();
  // Service 5's "print this group": its print view, by project group, for today.
  await expect(page.frameLocator("#s5-preview").getByText("Project Group: Team F")).toBeVisible();
  await expect(page.frameLocator("#s5-preview").locator(".worker-form")).toHaveCount(1);
});

test("encoder: tomorrow's schedule from the grid and card views", async ({ page, request }) => {
  await openEncoder(page, "tomorrow");
  await page.getByTestId("enc-view-grid").click();
  await expect(page.getByTestId("enc-grid-row")).toHaveCount(3);
  const nilo = page.getByTestId("enc-grid-row").filter({ hasText: "Cruz, Nilo" });
  await nilo.getByTestId("enc-specificTask").fill("Paint gate");
  await nilo.getByTestId("enc-specificTask").press("Tab");
  await expect.poll(async () => (await byA(request, "703")).tomorrow).toBe("Paint gate");
  expect((await byA(request, "703")).task).not.toBe("Paint gate");

  await page.getByTestId("enc-view-card").click();
  await expect(page.getByTestId("enc-card")).toContainText("Abad, Rico");
  await page.getByTestId("enc-card-next").click();
  await expect(page.getByTestId("enc-card")).toContainText("Bautista, Lea");
  // Lea is absent (marked on the slip above), so her fields are locked, as in Service 5.
  await expect(page.getByTestId("enc-card").getByTestId("enc-area")).toBeDisabled();
  await page.getByTestId("enc-card-next").click();
  await expect(page.getByTestId("enc-card")).toContainText("Cruz, Nilo");
  await page.getByTestId("enc-card").getByTestId("enc-area").fill("Annex");
  await page.getByTestId("enc-card").getByTestId("enc-area").press("Tab");
  await expect.poll(async () => (await byA(request, "703")).tomorrow_area).toBe("Annex");
  expect((await byA(request, "703")).tomorrow).toBe("Paint gate"); // the earlier field is kept
});

test("OT: the original Service 5 entries page, Load Yesterday saved to SiteMonitor", async ({ page, request }) => {
  await openAs(page, "OfficeAdmin", "/operations");
  await page.getByTestId("tab-OT Entries").click();
  const ote = page.frameLocator("[data-testid=s5-page-ot-entries]");
  await expect(ote.locator("body[data-ready=\"1\"]")).toBeAttached({ timeout: 20_000 });
  await ote.getByRole("button", { name: "Load Yesterday" }).click();
  await expect.poll(async () => (await records(request, "ot_entries")).filter((e: any) => e.date === iso(0)).length).toBe(2);
  const copied = (await records(request, "ot_entries")).filter((e: any) => e.date === iso(0));
  expect(copied.every((e: any) => e.hours === 0 && e.otArea && e.status === "Planned")).toBeTruthy();

});

test("print & export: grouped field picker, layouts, CSV and Excel", async ({ page }) => {
  await openAs(page, "OfficeAdmin", "/personnel");
  await page.getByTestId("print-export-open").click();
  await expect(page.getByTestId("pe-tree")).toContainText("Personal info");
  await expect(page.getByTestId("pe-field-phone")).toBeChecked();
  await expect(page.getByTestId("pe-field-spouseName")).not.toBeChecked();
  await expect(page.getByTestId("pe-field-name")).toBeDisabled();
  await page.getByTestId("pe-cat-family").check();
  await expect(page.getByTestId("pe-field-spouseName")).toBeChecked();
  await page.getByTestId("pe-layout-table").check();
  // Service 5's table print, in its new tab.
  await page.evaluate(() => {
    const w = window as any; w.__s5 = [];
    w.open = () => { let buf = ""; return { document: { open() { buf = ""; }, write(h: string) { buf += h; }, close() { w.__s5.push(buf); buf = ""; }, images: [] }, print() {}, close() {}, focus() {} }; };
  });
  await page.getByTestId("pe-print").click();
  const doc = () => page.evaluate(() => (window as any).__s5.join(""));
  await expect.poll(doc).toContain("Personnel Details Report");
  expect(await doc()).toContain("Spouse Name");
  for (const n of ["Abad, Rico", "Bautista, Lea", "Cruz, Nilo"]) expect(await doc()).toContain(n);

  await page.getByTestId("pe-tab-export").click();
  await expect(page.getByTestId("pe-field-spouseName")).toBeChecked();
  await page.getByTestId("pe-clear-all").click();
  await page.getByTestId("pe-field-aNumber").check();
  const [download] = await Promise.all([page.waitForEvent("download"), page.getByTestId("pe-csv").click()]);
  const csv = readFileSync(await download.path(), "utf8");
  expect(csv.split("\n")[0]).toBe("Name,A#");
  expect(csv).toContain("Abad, Rico");
  const [xls] = await Promise.all([page.waitForEvent("download"), page.getByTestId("pe-xlsx").click()]);
  expect(readFileSync(await xls.path(), "utf8")).toContain("urn:schemas-microsoft-com:office:spreadsheet");
});

test("merge review: provenance, counts, collisions and their resolutions", async ({ page, request }) => {
  const incoming = {
    version: 4,
    syncMetadata: { sourceDeviceId: "Station-North", exportTimestamp: "2026-11-02T09:00:00.000Z" },
    workers: [
      { aNumber: 701, name: "Abad, Rico", trade: "Mason", projectGroup: "Team E", updatedAt: "2026-11-01T08:00:30.000Z", personalInfo: { contact: "0917 999 9999", updatedAt: "2026-11-01T08:00:30.000Z" } },
      { aNumber: 704, name: "Diaz, Ona", trade: "Welder", projectGroup: "Team F", dailyStatus: "Present" },
    ],
  };
  await openAs(page, "OfficeAdmin", "/settings");
  await page.getByTestId("tab-Restore DB").click();
  await page.getByTestId("workforce-file").setInputFiles({ name: "station-north.json", mimeType: "application/json", buffer: Buffer.from(JSON.stringify(incoming)) });
  await expect(page.getByTestId("sync-review")).toBeVisible();
  await expect(page.getByTestId("sync-source")).toHaveText("Station-North");
  await expect(page.getByTestId("sync-new")).toHaveText("+1");
  await expect(page.getByTestId("sync-conflicts")).toHaveText("1");
  await expect(page.getByTestId("sync-conflict")).toContainText("Abad, Rico");
  await expect(page.getByTestId("sync-conflict")).toContainText("Phone: 0917 000 0701");
  await expect(page.getByTestId("sync-res-accept_incoming")).toBeChecked();
  await page.getByTestId("sync-all-local").click();
  await expect(page.getByTestId("sync-res-keep_local")).toBeChecked();
  await page.getByTestId("sync-apply").click();
  await expect(page.getByTestId("import-report")).toBeVisible();
  await expect.poll(async () => (await workers(request)).length).toBe(4);
  const ctx = await adminApi();
  const rico = await byA(request, "701");
  expect((await (await ctx.get(`/api/workers/${rico.id}/profile`)).json()).personalInfo.contact).toBe("0917 000 0701");
  await ctx.dispose();
});

test("every legacy Service 5 backup generation restores", async ({ request }) => {
  const ctx = await adminApi();
  const person = (a: number, name: string) => ({ aNumber: a, name, trade: "Mason", dailyStatus: "Present" });
  const generations: [string, unknown][] = [
    ["v1.json", [person(801, "Legacy, One")]],
    ["v2.json", { version: 2, workersData: [person(802, "Legacy, Two")], attendanceLog: [{ id: "lg-att-2", aNumber: 802, date: "2026-01-02", newStatus: "Absent" }] }],
    ["v3.json", { version: "3.0", workers: [person(803, "Legacy, Three")], vaccines: [{ id: "lg-vx", workerId: 803, vaccineName: "Flu", dateGiven: "2026-01-01", nextDueDate: "2026-01-31" }] }],
    ["v4.json", { version: 4, schemaVersion: 2, syncMetadata: { sourceDeviceId: "Old-PC" }, workers: [person(804, "Legacy, Four")], duty_areas: [{ id: "lg-duty", name: "Old Gate" }] }],
    ["v11.json", { version: 11, workers: [person(805, "Legacy, Five")], wc_ministers_data: [{ id: "lg-min", name: "Bro. Legacy" }] }],
    ["snapshot.json", { version: 1.0, exportedAt: "2026-11-19T23:59:00Z", workers: [person(806, "Legacy, Six")], history: [{ id: "lg-h", dateStr: "Nov 19" }] }],
  ];
  for (const [name, body] of generations) {
    const res = await importJson(ctx, name, body);
    expect(res.ok(), `${name}: ${await res.text()}`).toBeTruthy();
  }
  const names = (await workers(request)).map((w: any) => w.name);
  for (const n of ["Legacy, One", "Legacy, Two", "Legacy, Three", "Legacy, Four", "Legacy, Five", "Legacy, Six"]) expect(names).toContain(n);
  expect((await records(request, "attendance_log")).some((e: any) => e.id === "lg-att-2")).toBeTruthy();
  expect((await records(request, "duty_areas")).some((e: any) => e.id === "lg-duty")).toBeTruthy();
  expect((await records(request, "ministers")).some((e: any) => e.id === "lg-min")).toBeTruthy();
  const enc = await ctx.get("/api/workforce/export", { headers: { "x-backup-passphrase": Buffer.from("legacy pass 1").toString("hex") } });
  const sealed = await enc.text();
  expect((await ctx.post("/api/workforce/import?mode=Merge&file_name=b.agwcz", { data: Buffer.from(sealed), headers: { "content-type": "application/octet-stream" } })).status()).toBe(400);
  const ok = await ctx.post("/api/workforce/import?mode=Merge&file_name=b.agwcz", { data: Buffer.from(sealed), headers: { "content-type": "application/octet-stream", "x-backup-passphrase": Buffer.from("legacy pass 1").toString("hex") } });
  expect(ok.ok(), await ok.text()).toBeTruthy();
  await ctx.dispose();
});
