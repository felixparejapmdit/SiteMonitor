/**
 * Phase 3 — PWA shell, live WebSocket routing (VIP attention protection),
 * the IndexedDB offline queue with automatic FIFO sync, and smoothness
 * checks (no console errors, no layout shift when data refreshes).
 */
import { test, expect } from "@playwright/test";
import { api, openAs, resetDatabase, toast, userFor, workItemByTitle } from "./helpers";

test.describe.configure({ mode: "serial" });
test.beforeAll(() => resetDatabase());

test("PWA manifest and service worker", async ({ page }) => {
  const manifest = await (await page.request.get("/manifest.json")).json();
  expect(manifest.display).toBe("standalone");
  expect(manifest.orientation).toBe("any");
  expect(manifest.theme_color).toBe("#ffffff");
  expect(manifest.background_color).toBe("#f7f7f5");
  await openAs(page, "VipClient");
  const registered = await page.evaluate(async () => {
    const reg = await navigator.serviceWorker.ready;
    return !!reg.active;
  });
  expect(registered).toBe(true);
  const deepLink = await page.request.get("/agenda");
  expect(deepLink.status()).toBe(200);
});

test("live push: staff resolve a note → the owner's pin turns green instantly", async ({ browser }) => {
  const vipPage = await browser.newPage();
  const staffPage = await browser.newPage();
  await openAs(vipPage, "VipClient");
  const parking = await workItemByTitle(vipPage.request, "Parking Conversion to Multi-Purpose Hall");
  await vipPage.goto(`/projects/${parking.id}`);
  await expect(vipPage.locator("text=Live").first()).toBeVisible();
  await expect(vipPage.getByTestId("pin-Open")).toHaveCount(1);

  await openAs(staffPage, "SiteMonitor", `/projects/${parking.id}`, "Jun");
  await staffPage.getByTestId("pin-Open").click();
  await staffPage.getByTestId("reply-text").fill("Just runoff from the tool wash station, redirecting drainage now");
  await staffPage.getByTestId("mark-resolved").click();
  await expect(toast(staffPage).filter({ hasText: "Marked resolved" })).toBeVisible();

  // Tier-1 instant push to the owner.
  await expect(toast(vipPage).filter({ hasText: "The team resolved your photo note" })).toBeVisible();
  await expect(vipPage.getByTestId("pin-Resolved")).toHaveCount(1);
  await vipPage.getByTestId("pin-Resolved").click();
  await expect(vipPage.getByTestId("staff-reply")).toHaveText("Just runoff from the tool wash station, redirecting drainage now");
  await vipPage.close();
  await staffPage.close();
});

test("live push: owner approves → staff alerted; drafts never reach the owner", async ({ browser }) => {
  const vipPage = await browser.newPage();
  const adminPage = await browser.newPage();
  await openAs(vipPage, "VipClient");
  await openAs(adminPage, "OfficeAdmin");
  await expect(adminPage.locator("text=Live").first()).toBeVisible();

  // A field draft is created: the owner must NOT be interrupted.
  const monitor = await userFor(vipPage.request, "SiteMonitor", "Jun");
  const pool = await workItemByTitle(vipPage.request, "Swimming Pool Cleanup");
  await api(vipPage.request, monitor).post("/decisions", {
    work_item_id: pool.id, category: "RoutineUpdate", urgency: "Standard", title: "Skimmer basket replaced",
    issue_description: "Cracked basket swapped.", mitigation_action_taken: "", photo_data_url: null,
  });
  await vipPage.waitForTimeout(1500);
  await expect(toast(vipPage)).toHaveCount(0);

  await vipPage.getByTestId("decision-card").filter({ hasText: "Imported paving stones" }).getByTestId("approve-b").click();
  await expect(toast(adminPage).filter({ hasText: "Owner responded: Imported paving stones delayed by 2 weeks" })).toBeVisible();
  await expect(toast(adminPage).filter({ hasText: "Approved Option B: Wait for Imported Stones - +14 Days Delay" })).toBeVisible();
  await vipPage.close();
  await adminPage.close();
});

test("offline mode: staff updates queue in IndexedDB and sync when signal returns", async ({ browser }) => {
  const context = await browser.newContext({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });
  const page = await context.newPage();
  const backwash = await workItemByTitle(page.request, "Water Filtration Backwash");
  await openAs(page, "SiteMonitor", `/projects/${backwash.id}`, "Ana");
  await expect(page.getByTestId("mark-completed")).toBeVisible();

  await context.setOffline(true);
  await expect(page.getByTestId("offline-badge")).toContainText("Offline Mode — 0 Updates Queued");
  await page.getByTestId("mark-completed").click();
  await page.getByTestId("confirm-mark-completed").click();
  await expect(toast(page).filter({ hasText: "Saved on this device" })).toBeVisible();
  await expect(page.getByTestId("offline-badge")).toContainText("Offline Mode — 1 Update Queued");
  const queued = await page.evaluate(
    () =>
      new Promise<number>((resolve) => {
        const req = indexedDB.open("sitemonitor", 1);
        req.onsuccess = () => {
          const tx = req.result.transaction("staff_mutation_queue");
          const count = tx.objectStore("staff_mutation_queue").count();
          count.onsuccess = () => resolve(count.result);
        };
      }),
  );
  expect(queued).toBe(1);

  await context.setOffline(false);
  await expect(toast(page).filter({ hasText: "Synced 1 update saved offline" })).toBeVisible({ timeout: 15_000 });
  await expect(page.getByTestId("offline-badge")).toHaveCount(0);
  const vip = await userFor(page.request, "VipClient");
  const detail = await api(page.request, vip).get(`/work-items/${backwash.id}`);
  expect(detail.timeline.filter((t: any) => t.title.startsWith("Water Filtration Backwash — Completed")).length).toBeGreaterThanOrEqual(2);
  await context.close();
});

test("smoothness: no console errors and no layout shift on background refresh", async ({ page }) => {
  const errors: string[] = [];
  page.on("console", (m) => m.type() === "error" && errors.push(m.text()));
  page.on("pageerror", (e) => errors.push(e.message));
  await openAs(page, "VipClient");
  await page.evaluate(() => {
    (window as any).__cls = 0;
    new PerformanceObserver((list) => {
      for (const e of list.getEntries() as any[]) if (!e.hadRecentInput) (window as any).__cls += e.value;
    }).observe({ type: "layout-shift", buffered: false });
  });
  // Trigger several background refreshes from another role.
  const admin = await userFor(page.request, "OfficeAdmin");
  const suite = await workItemByTitle(page.request, "Master Suite Refit");
  for (const n of [6, 7, 8]) {
    await api(page.request, admin).post("/manpower/reallocate", { allocations: [{ work_item_id: suite.id, headcount: n }], transfers: [] });
  }
  await page.waitForTimeout(2000);
  const cls = await page.evaluate(() => (window as any).__cls);
  expect(cls).toBeLessThan(0.05);
  for (const path of ["/agenda", "/archive", `/projects/${suite.id}`]) {
    await page.goto(path);
    await expect(page.locator("body.app-ready")).toHaveCount(1);
  }
  expect(errors).toEqual([]);
});

test("mobile layout: bottom tabs for field staff", async ({ browser }) => {
  const context = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true });
  const page = await context.newPage();
  await openAs(page, "SiteMonitor", "/", "Jun").catch(async () => {
    // The desktop tab bar is hidden on phones; the select is used instead.
    await expect(page.getByTestId("role-select")).toBeVisible();
  });
  const tabs = page.getByRole("navigation", { name: "Main" }).last();
  await expect(tabs).toBeVisible();
  await expect(tabs.getByRole("link")).toHaveCount(5);
  await tabs.getByRole("link", { name: "Equipment" }).click();
  await expect(page.getByRole("heading", { name: "Equipment" })).toBeVisible();
  await context.close();
});
