/**
 * Side B — the team's "Engine": crew allocation & reshuffles, context-aware
 * equipment moves and breakdown reports, the weighted phase updater, the
 * draft-to-VIP pipeline, the Master Scheduler and the Map Studio.
 */
import { test, expect } from "@playwright/test";
import { api, openAs, pngBuffer, resetDatabase, toast, userFor, workItemByTitle } from "./helpers";

test.describe.configure({ mode: "serial" });
test.beforeAll(() => resetDatabase());

test("crew board: steppers and save write today's allocation", async ({ page }) => {
  await openAs(page, "OfficeAdmin", "/crew");
  await expect(page.getByRole("heading", { name: "Crew" })).toBeVisible();
  const row = page.getByTestId("crew-row").filter({ hasText: "Guest Villa 2 Roof Renovation" });
  await row.getByTestId("crew-plus-5").click();
  await row.getByTestId("crew-plus-1").click();
  await expect(row.getByTestId("crew-count")).toHaveText("6");
  await page.getByTestId("save-allocation").click();
  await expect(toast(page).filter({ hasText: "Today's crew is saved" })).toBeVisible();
  expect((await workItemByTitle(page.request, "Guest Villa 2 Roof Renovation")).active_worker_count).toBe(6);
});

test("mid-day reshuffle moves workers with a quick reason", async ({ page }) => {
  await openAs(page, "SiteMonitor", "/crew", "Jun");
  await page.getByTestId("open-move-workers").click();
  const parking = await workItemByTitle(page.request, "Parking Conversion to Multi-Purpose Hall");
  const suite = await workItemByTitle(page.request, "Master Suite Refit");
  await page.getByTestId("move-from").selectOption(parking.id);
  await page.getByTestId("move-to").selectOption(suite.id);
  await page.getByTestId("move-plus").click();
  await page.getByTestId("move-plus").click();
  await expect(page.getByTestId("move-count")).toHaveText("3");
  await page.getByRole("button", { name: "Materials just arrived" }).click();
  await page.getByTestId("confirm-move-workers").click();
  await expect(toast(page).filter({ hasText: "Moved 3 workers" })).toBeVisible();
  expect((await workItemByTitle(page.request, "Master Suite Refit")).active_worker_count).toBe(8);
});

test("equipment: zone context lists only that zone, then moves the crane", async ({ page }) => {
  await openAs(page, "SiteMonitor", "/equipment", "Jun");
  const map = await (await page.request.get("/api/estate/map")).json();
  const flat: any[] = [];
  const walk = (n: any) => { flat.push(n.area); n.children.forEach(walk); };
  map.areas.forEach(walk);
  const terrace = flat.find((a) => a.name === "Retaining Wall Terrace");
  const warehouse = flat.find((a) => a.name === "Contractor Warehouse");
  await page.getByTestId("zone-context").selectOption(terrace.id);
  await expect(page.getByTestId("asset-card")).toHaveCount(1);
  await expect(page.getByTestId("asset-card")).toContainText("2-Ton Mini Crane");

  await page.getByTestId("move-equipment").click();
  const picker = page.getByTestId("asset-picker");
  await expect(picker.locator("option")).toHaveCount(2); // placeholder + crane only
  await picker.selectOption({ label: "2-Ton Mini Crane" });
  await page.getByTestId("move-dest").selectOption(warehouse.id);
  await page.getByTestId("confirm-move").click();
  await expect(toast(page).filter({ hasText: "2-Ton Mini Crane moved to Contractor Warehouse" })).toBeVisible();
  await expect(page.getByTestId("asset-card")).toHaveCount(0);
});

test("breakdown report with deadline impact shifts the project", async ({ page }) => {
  await openAs(page, "SiteMonitor", "/equipment", "Jun");
  const parking = await workItemByTitle(page.request, "Parking Conversion to Multi-Purpose Hall");
  await page.getByTestId("zone-context").selectOption({ label: "Multi-Purpose Hall (Parking Conversion)" });
  await page.getByTestId("report-breakdown").click();
  const form = page.getByTestId("breakdown-form");
  await form.getByTestId("asset-picker").selectOption({ label: "Backhoe" });
  await form.getByRole("button", { name: /Slows work down/ }).click();
  await page.getByTestId("diag-note").fill("Track tension failing; mechanic called, using plate compactor meanwhile.");
  await form.getByRole("button", { name: "Yes" }).click();
  await page.getByTestId("affected-project").selectOption(parking.id);
  await page.getByTestId("delay-plus").click();
  await expect(page.getByTestId("delay-days")).toHaveText("2");
  await page.getByTestId("submit-breakdown").click();
  await expect(toast(page).filter({ hasText: "Backhoe reported" })).toBeVisible();
  const after = await workItemByTitle(page.request, "Parking Conversion to Multi-Purpose Hall");
  expect(new Date(after.current_target_deadline).getTime() - new Date(parking.current_target_deadline).getTime()).toBe(2 * 86_400_000);
});

test("weighted phase updater with a 16:9 proof photo", async ({ page }) => {
  await openAs(page, "SiteMonitor", "/", "Jun");
  const suite = await workItemByTitle(page.request, "Master Suite Refit");
  await page.goto(`/projects/${suite.id}`);
  await expect(page.getByTestId("progress-updater")).toContainText("3. Joinery (35%)");
  await page.getByTestId("complete-phase-3").click();
  await page.getByTestId("photo-input").setInputFiles({ name: "joinery.png", mimeType: "image/png", buffer: pngBuffer(800, 600) });
  const preview = page.getByAltText("Photo preview (cropped to 16:9)");
  await expect(preview).toBeVisible();
  const ratio = await preview.evaluate((img: HTMLImageElement) => img.naturalWidth / img.naturalHeight);
  expect(Math.abs(ratio - 16 / 9)).toBeLessThan(0.01);
  await page.getByTestId("confirm-complete-phase").click();
  await expect(toast(page).filter({ hasText: "Joinery completed — progress is now 80%" })).toBeVisible();
  await expect(page.getByTestId("progress-value")).toHaveText("80%");
});

test("routine maintenance one-tap completion", async ({ page }) => {
  await openAs(page, "SiteMonitor", "/", "Jun");
  const pool = await workItemByTitle(page.request, "Swimming Pool Cleanup");
  await page.goto(`/projects/${pool.id}`);
  await page.getByTestId("mark-completed").click();
  await page.getByTestId("confirm-mark-completed").click();
  await expect(toast(page).filter({ hasText: "Marked completed" })).toBeVisible();
  await expect(page.getByTestId("timeline")).toContainText("Swimming Pool Cleanup — Completed");
});

test("draft-to-VIP: monitor reports, admin polishes and publishes", async ({ page, browser }) => {
  await openAs(page, "SiteMonitor", "/decisions", "Ana");
  const suite = await workItemByTitle(page.request, "Master Suite Refit");
  await page.getByTestId("report-project").selectOption(suite.id);
  await page.getByRole("button", { name: "Material delay" }).click();
  await page.getByTestId("report-title").fill("Imported stone basin delayed by 10 days");
  await page.getByTestId("report-issue").fill("Supplier says the basin is still at the port.");
  await page.getByTestId("submit-report").click();
  await expect(toast(page).filter({ hasText: "Sent to the office for review" })).toBeVisible();

  const admin = await browser.newPage();
  await openAs(admin, "OfficeAdmin", "/decisions");
  const editor = admin.getByTestId("publish-editor").filter({ hasText: "Imported stone basin delayed by 10 days" });
  await expect(editor).toHaveCount(1);
  await editor.getByTestId("publish-summary").fill("The stone basin for the master bathroom is held at the port for about 10 more days.");
  await editor.getByTestId("option-a").fill("Install a local marble basin - No Delay");
  await editor.getByTestId("option-b").fill("Wait for the imported basin - +10 Days Delay");
  await editor.getByTestId("publish-to-vip").click();
  await expect(toast(admin).filter({ hasText: "Published — the owner was notified instantly" })).toBeVisible();
  await admin.close();

  const vip = await userFor(page.request, "VipClient");
  const list = await api(page.request, vip).get("/decisions");
  expect(list.some((d: any) => d.title === "Imported stone basin delayed by 10 days" && d.status === "PublishedPendingClient")).toBe(true);
});

test("master scheduler warns about the double-booked crane and adds an item", async ({ page }) => {
  await openAs(page, "OfficeAdmin", "/agenda");
  await expect(page.getByTestId("equipment-conflicts")).toContainText("2-Ton Mini Crane");
  await page.getByTestId("add-schedule").click();
  await page.getByTestId("sched-title").fill("Crane lift — gazebo lights");
  await page.getByRole("button", { name: "2-Ton Mini Crane", exact: true }).click();
  await page.getByTestId("sched-save").click();
  await expect(toast(page).filter({ hasText: /Added/ })).toBeVisible();
});

test("quiet day conflicts are moved with one click", async ({ page }) => {
  const vip = await userFor(page.request, "VipClient");
  const agenda = await (await page.request.get("/api/agenda")).json();
  const descale = agenda.items.find((i: any) => i.title === "Deepwell Descaling");
  const s = new Date(new Date(descale.start_time).getTime() - 3600_000);
  const e = new Date(new Date(descale.end_time).getTime() + 3600_000);
  await api(page.request, vip).post("/agenda/quiet-day", { title: "Garden Lunch", description: "", start_time: s.toISOString(), end_time: e.toISOString() });

  await openAs(page, "OfficeAdmin", "/agenda");
  const queue = page.getByTestId("conflict-queue");
  await expect(queue).toContainText("Deepwell Descaling");
  await queue.getByTestId("conflict-item").filter({ hasText: "Deepwell Descaling" }).getByTestId("move-next-workday").click();
  await expect(toast(page).filter({ hasText: "Deepwell Descaling moved to" })).toBeVisible();
  await expect(page.getByTestId("conflict-item").filter({ hasText: "Deepwell Descaling" })).toHaveCount(0);
});

test("map studio: draw a nested zone, reshape and archive it", async ({ page }) => {
  await openAs(page, "OfficeAdmin", "/map-studio");
  await page.getByTestId("draw-zone").click();
  const canvas = page.getByTestId("studio-canvas");
  const box = (await canvas.boundingBox())!;
  for (const [x, y] of [[0.38, 0.40], [0.47, 0.40], [0.47, 0.49], [0.38, 0.49]]) {
    await canvas.click({ position: { x: box.width * x, y: box.height * y } });
  }
  await page.getByTestId("zone-name").fill("Koi Pond");
  const map = await (await page.request.get("/api/estate/map")).json();
  const garden = map.areas.find((n: any) => n.area.name === "Grand Garden").area.id;
  await page.getByTestId("zone-parent").selectOption(garden);
  await page.getByTestId("save-zone").click();
  await expect(toast(page).filter({ hasText: "Koi Pond added to the map" })).toBeVisible();

  const panel = page.getByTestId("edit-zone-panel");
  await expect(panel).toContainText("Koi Pond");
  const vertex = page.getByTestId("vertex").first();
  const vb = (await vertex.boundingBox())!;
  await page.mouse.move(vb.x + vb.width / 2, vb.y + vb.height / 2);
  await page.mouse.down();
  await page.mouse.move(vb.x - 30, vb.y - 20, { steps: 5 });
  await page.mouse.up();
  await page.getByTestId("save-shape").click();
  await expect(toast(page).filter({ hasText: "New shape saved" })).toBeVisible();

  await page.getByTestId("archive-zone").click();
  await expect(toast(page).filter({ hasText: "Zone archived" })).toBeVisible();
  const after = await (await page.request.get("/api/estate/map")).json();
  const koi = after.areas.find((n: any) => n.area.name === "Grand Garden").children.find((c: any) => c.area.name === "Koi Pond");
  expect(koi.area.is_archived).toBe(true);
  expect(koi.area.parent_id).toBe(garden);
});

test("history search finds work in archived locations", async ({ page }) => {
  await openAs(page, "VipClient", "/archive");
  await page.getByTestId("history-search").fill("tool shed");
  const results = page.getByTestId("history-results");
  await expect(results).toContainText("Old Tool Shed Clear-Out");
  await expect(results).toContainText("Archived Locations");
});
