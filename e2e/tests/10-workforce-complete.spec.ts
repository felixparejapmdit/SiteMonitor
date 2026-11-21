/**
 * Phases 7–10 of the Service 5 merge: offline workforce edits, area-to-zone
 * scoping for Site Monitors, photos, the profile editor, dashboard, tomorrow's
 * assignments, bulk update, pick lists, duty swaps, overtime from pasted names,
 * visitation groups and ministers, clinic boosters / medications / contact
 * conversion, and the new print kinds.
 */
import { Page, request as pwRequest } from "@playwright/test";
import { BASE_URL, expect, openAs, resetDatabase, test, userFor, setClinic } from "./helpers";

test.describe.configure({ mode: "serial" });
test.setTimeout(120_000);

const soon = new Date(Date.now() + 3 * 86_400_000).toISOString().slice(0, 10);
const BACKUP = {
  version: 11,
  workers: [
    { aNumber: "201", name: "Hall Worker One", area: "Hall Area", projectGroup: "Team H", groupRole: "Katiwala", trade: "Mason", dailyStatus: "Present", workResidence: "Bunk House 1", birthday: `1990${new Date().toISOString().slice(4, 8)}15` },
    { aNumber: "202", name: "Hall Worker Two", area: "Hall Area", projectGroup: "Team H", trade: "Mason", dailyStatus: "Present", workResidence: "Bunk House 1" },
    { aNumber: "203", name: "Garden Worker", area: "Garden Area", projectGroup: "Team G", trade: "Gardener", dailyStatus: "Present", workResidence: "Bunk House 2" },
  ],
  duty_areas: [{ id: "gate", name: "Main Gate", shiftTime: "20:00", workersNeeded: 1, roster: [201, 203], lastRotationIndex: -1 }],
  volunteer_vaccines: [{ id: "vx1", volunteerId: "202", vaccineName: "Flu", dateAdministered: "2026-01-01", boosterDueDate: soon }],
  contact_traces: [{ id: "ct1", contactWorkerId: "203", healthEventId: "x", contactType: "workplace", status: "monitoring" }],
};
const PNG = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

test.beforeAll(async () => {
  resetDatabase();
  await setClinic(true); // the clinic module is off by default
  const admin = await userFor(undefined as any, "OfficeAdmin");
  const ctx = await pwRequest.newContext({ baseURL: BASE_URL, extraHTTPHeaders: { authorization: `Bearer ${admin.token}` } });
  const res = await ctx.post("/api/workforce/import?file_name=seed.json", { data: Buffer.from(JSON.stringify(BACKUP)), headers: { "content-type": "application/octet-stream" } });
  expect(res.ok(), await res.text()).toBeTruthy();
  await ctx.dispose();
});

const workers = async (request: any, token?: string) =>
  (await request.get("/api/workers", token ? { headers: { authorization: `Bearer ${token}` } } : {})).json();
const records = async (request: any, list: string) => (await request.get(`/api/workforce/records/${list}`)).json();
// An init script, so the stub survives any reload (e.g. a service-worker update).

test("Site Monitors only see people working in their zones (via area mapping)", async ({ page, request }) => {
  const jun = await userFor(request, "SiteMonitor", "Jun"); // looks after the hall, the garden and the pool
  expect((await workers(request, jun.token)).length).toBe(0); // nothing mapped yet
  const map = await (await request.get("/api/estate/map")).json();
  const hall = map.areas.find((n: any) => n.area.name.startsWith("Multi-Purpose Hall")).area.id;

  await openAs(page, "OfficeAdmin", "/workforce");
  await page.getByTestId("tab-Areas & zones").click();
  await page.getByTestId("area-row").filter({ hasText: "Hall Area" }).getByTestId("area-zone").selectOption(hall);
  await expect(page.getByText("Area mapped").first()).toBeVisible();

  const seen = await workers(request, jun.token);
  expect(seen.map((w: any) => w.name).sort()).toEqual(["Hall Worker One", "Hall Worker Two"]);
  const garden = (await workers(request)).find((w: any) => w.a_number === "203");
  const r = await request.put(`/api/workers/${garden.id}/status`, { data: { status: "Absent", reason: "" }, headers: { authorization: `Bearer ${jun.token}` } });
  expect(r.status()).toBe(403);
});

test("attendance taps made offline are kept and sent when the signal returns", async ({ browser, request }) => {
  const context = await browser.newContext();
  const page = await context.newPage();
  await openAs(page, "OfficeAdmin", "/personnel");
  await context.setOffline(true);
  await page.getByTestId("worker-row").filter({ hasText: "Hall Worker Two" }).getByTestId("mark-OnPass").click();
  await expect(page.getByText("Saved on this device").first()).toBeVisible();
  await context.setOffline(false);
  await expect.poll(async () => (await workers(request)).find((w: any) => w.a_number === "202").status, { timeout: 20_000 }).toBe("OnPass");
  await context.close();
});

test("profile editor: personal details, health fields and a photo", async ({ page, request }) => {
  await openAs(page, "OfficeAdmin", "/personnel");
  const row = page.getByTestId("worker-row").filter({ hasText: "Hall Worker One" });
  await row.getByTestId("profile-worker").click();
  await page.getByTestId("ptab-family").click();
  await page.getByTestId("pf-spouseName").fill("Maria One");
  await page.getByTestId("ptab-personal").click();
  await page.getByTestId("pf-emergencyPhone").fill("0917 111 2222");
  await page.getByTestId("pf-bloodType").selectOption("O+"); // Office Admins have clinic access
  await page.getByTestId("profile-save").click();
  await expect(page.getByText("Saved").first()).toBeVisible();
  const w = (await workers(request)).find((x: any) => x.a_number === "201");
  const prof = await (await request.get(`/api/workers/${w.id}/profile`)).json();
  expect([prof.spouseName, prof.emergencyPhone, prof.bloodType]).toEqual(["Maria One", "0917 111 2222", "O+"]);

  // A photo is stored with the person and served on its own.
  expect((await request.put(`/api/workers/${w.id}/profile`, { data: { photoUrl: PNG } })).ok()).toBeTruthy();
  const photo = await request.get(`/api/workers/${w.id}/photo`);
  expect(photo.headers()["content-type"]).toBe("image/png");
  await page.reload();
  await expect(page.getByTestId("worker-row").filter({ hasText: "Hall Worker One" }).getByTestId("worker-photo")).toBeVisible();

  // Site Monitors can't open personal details.
  const jun = await userFor(request, "SiteMonitor", "Jun");
  expect((await request.get(`/api/workers/${w.id}/profile`, { headers: { authorization: `Bearer ${jun.token}` } })).status()).toBe(403);
});

test("dashboard, tomorrow's assignments, and bulk update by pasted names", async ({ page, request }) => {
  await openAs(page, "OfficeAdmin", "/workforce");
  await expect(page.getByTestId("wf-dashboard")).toContainText("Total active");
  await expect(page.getByText("Hall Worker One").first()).toBeVisible(); // this month's birthday

  await page.goto("/operations");
  await page.getByTestId("tab-Encoder").click();
  await page.getByTestId("encoder-tomorrow").click();
  await page.getByTestId("enc-slip-item").filter({ hasText: "Team H" }).click();
  await page.getByTestId("enc-master-task").fill("Pour hall footing");
  await page.getByTestId("enc-fill").click();
  await expect.poll(async () => (await workers(request)).filter((w: any) => w.tomorrow === "Pour hall footing").length).toBe(2);

  await page.getByTestId("tab-Bulk Update").click();
  await page.getByTestId("bulk-text").fill("203\nNobody Here");
  await page.getByTestId("bulk-action").selectOption("Absent");
  await page.getByTestId("bulk-apply").click();
  await expect(page.getByTestId("bulk-result")).toContainText("1 person updated");
  await expect(page.getByTestId("bulk-result")).toContainText("Nobody Here");
});

test("pick lists, visitation groups and ministers, duty swaps", async ({ page, request }) => {
  await openAs(page, "OfficeAdmin", "/operations");
  await page.getByTestId("tab-Pick Lists").click();
  await page.getByTestId("picklist-woodlandShops").fill("Electrical Shop\nCarpentry Shop");
  await page.getByTestId("picklist-save").click();
  await expect(page.getByText("Pick lists saved").first()).toBeVisible();
  const lists = (await records(request, "custom_datalists"))[0];
  expect(lists.woodlandShops).toEqual(["Electrical Shop", "Carpentry Shop"]);

  await page.goto("/workforce");
  await page.getByTestId("tab-Visitations").click();
  await page.getByTestId("visit-view-Groups").click();
  await page.getByTestId("minister-add").click();
  await page.getByTestId("rf-name").fill("Bro. Reyes");
  await page.getByTestId("rf-save").click();
  await page.getByTestId("vgroup-add").click();
  await page.getByTestId("rf-name").fill("Bunk House Group");
  await page.getByTestId("rf-save").click();
  await expect(page.getByTestId("record-row").filter({ hasText: "Bunk House Group" })).toBeVisible();
  await page.getByTestId("visit-view-Schedule").click();
  await page.getByTestId("visit-add").click();
  await expect(page.getByTestId("rf-ministerName").locator("option", { hasText: "Bro. Reyes" })).toHaveCount(1);
  await page.keyboard.press("Escape");

  // Only present people can be swapped in; 202 went on pass in the offline test.
  const two = (await workers(request)).find((w: any) => w.a_number === "202");
  expect((await request.put(`/api/workers/${two.id}/status`, { data: { status: "Present", reason: "" } })).ok()).toBeTruthy();
  await page.goto("/operations");
  await page.getByTestId("tab-Overnight Duty").click();
  await page.getByTestId("duty-generate").click();
  await expect(page.getByText("Duty roster made").first()).toBeVisible();
  await page.getByTestId("tab-Duty History").click();
  await page.getByTestId("duty-swap").first().selectOption("202");
  await expect(page.getByText("Swapped").first()).toBeVisible();
  const day = await page.evaluate(() => { const d = new Date(); return new Date(d.getTime() - d.getTimezoneOffset() * 60_000).toISOString().slice(0, 10); });
  const today = (await records(request, "duty_schedules")).find((d: any) => d.dateStr === day);
  expect(today.assignments[0].aNumber).toBe("202");
});

test("overtime from a pasted list of names", async ({ page, request }) => {
  await openAs(page, "OfficeAdmin", "/operations");
  await page.getByTestId("tab-OT Entries").click();
  await page.getByTestId("ot-paste-open").click();
  await page.getByTestId("otp-title").fill("Night pour");
  await page.getByTestId("otp-names").fill("hall worker one\nWorker Two Hall\nStranger");
  await page.getByTestId("otp-hours").fill("3");
  await page.getByTestId("otp-rate").fill("50");
  await expect(page.getByTestId("otp-matches")).toContainText("Stranger → not found");
  await expect(page.getByTestId("otp-pay")).toContainText("2 people · ₱300.00");
  await page.getByTestId("otp-save").click();
  await expect.poll(async () => (await records(request, "ot_entries")).find((e: any) => e.title === "Night pour")?.computedPay).toBe(300);
  const entry = (await records(request, "ot_entries")).find((e: any) => e.title === "Night pour");
  expect(entry.totalCount).toBe(2);
});

test("clinic: booster reminders, medications, contact becomes an episode", async ({ page, request }) => {
  await openAs(page, "OfficeAdmin", "/clinic");
  await page.getByTestId("tab-Vaccines").click();
  await expect(page.getByTestId("boosters")).toContainText("Hall Worker Two");
  await page.getByTestId("tab-Medications").click();
  await page.getByTestId("simple-add").click();
  await page.getByTestId("rf-volunteerId").selectOption("202");
  await page.getByTestId("rf-medicationName").fill("Paracetamol");
  await page.getByTestId("rf-save").click();
  await expect(page.getByTestId("record-row").filter({ hasText: "Paracetamol" })).toBeVisible();
  await page.getByTestId("tab-Contact tracing").click();
  await page.getByTestId("contact-convert").click();
  await expect(page.getByText("Episode opened").first()).toBeVisible();
  const trace = (await records(request, "contact_traces")).find((t: any) => t.id === "ct1");
  expect(trace.status).toBe("converted");
  const ep = (await records(request, "health_events")).find((e: any) => e.id === trace.convertedToEventId);
  expect(ep.workerId).toBe("203");
});

test("tomorrow's job orders print with tomorrow's task (Service 5's print view) @safari", async ({ page, request }) => {
  // Runs alone in WebKit, so set its data up itself (same values the earlier tests save).
  for (const w of await workers(request)) {
    if (w.project_group !== "Team H") continue;
    const data: any = { tomorrowAssignment: { specificTask: "Pour hall footing" } };
    if (w.a_number === "201") data.spouseName = "Maria One";
    expect((await request.put(`/api/workers/${w.id}/profile`, { data })).ok()).toBeTruthy();
  }
  await openAs(page, "OfficeAdmin", "/operations");
  await page.getByTestId("tab-Print").click();
  // Service 5's print view starts on tomorrow and uses the staged task.
  const f = page.frameLocator("#s5-preview");
  await expect(f.locator(".worker-form").filter({ hasText: "Pour hall footing" })).toHaveCount(2);
  await expect(f.locator(".print-page")).toHaveCount(1);
});
