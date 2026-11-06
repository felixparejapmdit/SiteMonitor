/**
 * Side A — the owner's "Showcase": Action Center, estate map drill-down,
 * layer toggles & executive summaries, project document, tap-to-pin
 * call-outs, the Estate Agenda with Quiet Day, and the handover sign-off.
 */
import { test, expect } from "@playwright/test";
import { api, openAs, resetDatabase, toast, userFor, workItemByTitle } from "./helpers";

test.describe.configure({ mode: "serial" });
test.beforeAll(() => resetDatabase());

test("action center: approve Option A folds the card away", async ({ page }) => {
  await openAs(page, "VipClient");
  await expect(page.getByRole("heading", { name: /Good (morning|afternoon|evening), Victoria/ })).toBeVisible();
  const center = page.getByTestId("action-center");
  await expect(center).toBeVisible();
  const card = page.getByTestId("decision-card").filter({ hasText: "Imported paving stones delayed by 2 weeks" });
  await expect(card.getByTestId("approve-a")).toHaveText("Approve Option A: Approve Local Paving Supplier - No Delay");
  await card.getByTestId("approve-a").click();
  await expect(card).toContainText("Approved");
  await expect(page.getByTestId("decision-card").filter({ hasText: "Imported paving stones" })).toHaveCount(0, { timeout: 8_000 });
});

test("action center: acknowledge receipt clears updates and collapses when empty", async ({ page }) => {
  await openAs(page, "VipClient");
  await page.getByTestId("decision-card").filter({ hasText: "Pool tiles" }).getByTestId("acknowledge").click();
  await expect(page.getByTestId("decision-card")).toHaveCount(0, { timeout: 8_000 });
  // Clear every photo update.
  while ((await page.getByTestId("acknowledge-photo").count()) > 0) {
    const before = await page.getByTestId("acknowledge-photo").count();
    await page.getByTestId("acknowledge-photo").first().click();
    await expect(page.getByTestId("acknowledge-photo")).toHaveCount(before - 1, { timeout: 8_000 });
  }
  // Only the gazebo sign-off remains.
  await expect(page.getByTestId("signoff-card")).toHaveCount(1);
});

test("estate map: drill down with breadcrumbs, layers and executive summary", async ({ page }) => {
  await openAs(page, "VipClient");
  const map = page.getByTestId("estate-map");
  await expect(map).toBeVisible();
  await map.locator('[data-testid="zone-Main Residence 1"]').click({ force: true });
  await expect(page.getByRole("navigation", { name: "Map location" })).toContainText("Entire Estate/Main Residence 1");
  await page.waitForTimeout(800); // let the zoom animation settle
  await map.locator('[data-testid="zone-South Wing"]').click({ force: true });
  await expect(page.getByRole("navigation", { name: "Map location" })).toContainText("South Wing");
  await expect(map.locator('[data-testid="zone-Master Suite"]')).toHaveCount(1);
  // Project list is filtered to the selected hierarchy.
  await expect(page.getByText("Showing South Wing and everything inside it.")).toBeVisible();
  await expect(page.getByTestId("capital-card")).toHaveCount(1);
  await expect(page.getByTestId("capital-card")).toContainText("Master Suite Refit");
  await page.getByTestId("crumb-root").click();
  await expect(map.locator('[data-testid="zone-Power House"]')).toHaveCount(0);

  await page.getByTestId("layer-infra").click();
  await page.waitForTimeout(800);
  await map.locator('[data-testid="zone-Power House"]').click({ force: true });
  await expect(page.getByTestId("executive-summary")).toContainText("Battery array: 3 of 4 batteries healthy");
});

test("project document shows leaders, crew delta, assets and audit trail", async ({ page }) => {
  await openAs(page, "VipClient");
  await page.getByTestId("capital-card").filter({ hasText: "Parking Conversion" }).click();
  await expect(page.getByTestId("project-title")).toHaveText("Parking Conversion to Multi-Purpose Hall");
  await expect(page.getByText("Current Target Deadline:")).toBeVisible();
  await expect(page.getByText(/moved from .* \(\+3 days\)/)).toBeVisible();
  const crew = page.getByTestId("manpower-block");
  await expect(crew.getByTestId("headcount")).toHaveText("14 Active Personnel");
  await expect(crew).toContainText("+6 transferred from Guest Villa 2 Roof Renovation");
  for (const role of ["Construction Group Leader", "On-Site Quality Control (QC) Officer", "Admin Office Coordinator"]) {
    await expect(crew).toContainText(role);
  }
  await expect(crew.getByRole("link", { name: /Call Ramon Dizon/ })).toHaveAttribute("href", "tel:+639175550142");
  await expect(page.getByTestId("asset-block")).toContainText("Deployed Heavy Machinery");
  const timeline = page.getByTestId("timeline");
  await expect(timeline).toContainText("Weather · +2 days");
  await expect(timeline).toContainText("Site Conditions · +1 day");
});

test("tap-to-pin: the owner drops an amber pin with a note", async ({ page }) => {
  await openAs(page, "VipClient");
  await page.getByTestId("capital-card").filter({ hasText: "Master Suite Refit" }).click();
  const stage = page.getByTestId("photo-stage").first();
  await expect(stage).toBeVisible();
  const box = (await stage.boundingBox())!;
  await stage.click({ position: { x: box.width * 0.25, y: box.height * 0.5 } });
  await page.getByTestId("callout-text").fill("Can the window frame match the walnut?");
  await page.getByTestId("submit-callout").click();
  await expect(toast(page).filter({ hasText: "Note sent to the site team" })).toBeVisible();
  const pin = page.getByTestId("pin-Open");
  await expect(pin).toHaveCount(1);
  const style = await pin.getAttribute("style");
  expect(style).toMatch(/left:\s*2[45]\.\d+%/);
  expect(style).toMatch(/top:\s*(49|50)\.\d+%/);
});

test("estate agenda hides routine silent work and books a Quiet Day", async ({ page }) => {
  await openAs(page, "VipClient", "/agenda");
  await expect(page.getByRole("heading", { name: "Estate Agenda" })).toBeVisible();
  await expect(page.getByTestId("agenda-item").filter({ hasText: "Deepwell Descaling" })).toContainText("Water / power interruption");
  await expect(page.getByTestId("agenda-item").filter({ hasText: "Swimming Pool Cleanup" })).toHaveCount(0);
  await expect(page.getByTestId("agenda-item").filter({ hasText: "Garden detailing" })).toHaveCount(0);

  await page.getByTestId("schedule-quiet-day").click();
  await page.getByTestId("qd-title").fill("Weekend Estate Gala");
  // Cover the day of the crane lifts and compaction.
  const agenda = await (await page.request.get("/api/agenda")).json();
  const compaction = agenda.items.find((i: any) => i.title === "Paving base compaction");
  const d = new Date(compaction.start_time);
  const pad = (n: number) => String(n).padStart(2, "0");
  const day = `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
  await page.getByTestId("qd-start").fill(`${day}T06:00`);
  await page.getByTestId("qd-end").fill(`${day}T22:00`);
  await page.getByTestId("qd-submit").click();
  const confirm = page.getByTestId("quiet-day-confirmation");
  await expect(confirm).toContainText("Confirmed. All heavy machinery (cranes, backhoes) and utility interruptions are automatically halted. Silent grounds detailing (pool & gardens) will proceed prior to your event.");
  await expect(confirm).toContainText("Paving base compaction");
  await expect(page.getByTestId("agenda-item").filter({ hasText: "Weekend Estate Gala" })).toContainText("Your occasion");
});

test("handover report: before/after slider, delay audit and sign-off to 100%", async ({ page }) => {
  await openAs(page, "VipClient");
  await page.getByTestId("open-handover").click();
  const report = page.getByTestId("handover-report");
  await expect(report).toContainText("Executive Handover Report");
  await expect(page.getByTestId("before-after")).toBeVisible();
  const slider = page.getByTestId("before-after").locator("input[type=range]");
  await slider.fill("20");
  await expect(page.getByTestId("before-after").locator("img").nth(1)).toHaveAttribute("style", /inset\(0 80% 0 0\)/);
  await expect(page.getByTestId("delay-audit")).toContainText("+2 Days: Material Shortage");
  await expect(report).toContainText("Garden Lighting Transformer");

  // Hold at 99% with a final call-out first.
  await page.getByTestId("add-final-callout").click();
  const stage = report.getByTestId("photo-stage");
  await stage.click({ position: { x: 100, y: 120 } });
  await report.getByTestId("callout-text").fill("Move this debris bin out of the corner");
  await report.getByTestId("submit-callout").click();
  await expect(page.getByTestId("accept-archive")).toBeDisabled();
  await expect(report).toContainText("stays at 99%");
});

test("handover: once the team resolves the final note, Accept & Archive closes at 100%", async ({ page }) => {
  const admin = await userFor(page.request, "OfficeAdmin");
  const open = await api(page.request, admin).get("/callouts/open");
  for (const n of open.filter((o: any) => o.work_item_title === "Rose Garden Gazebo Build")) {
    await api(page.request, admin).put(`/callouts/${n.callout.id}/resolve`, { reply: "Bin moved behind the hedge." });
  }
  const gazebo = await workItemByTitle(page.request, "Rose Garden Gazebo Build");
  await openAs(page, "VipClient", `/handover/${gazebo.id}`);
  await expect(page.getByTestId("accept-archive")).toBeEnabled();
  await page.getByTestId("accept-archive").click();
  await expect(toast(page).filter({ hasText: "Project accepted and archived at 100%" })).toBeVisible();
  const done = await workItemByTitle(page.request, "Rose Garden Gazebo Build");
  expect(done.calculated_progress).toBe(100);
  expect(done.status).toBe("CompletedAndArchived");
  await expect(page.getByTestId("signoff-card")).toHaveCount(0);
});
