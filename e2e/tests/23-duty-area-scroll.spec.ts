import { expect, test } from "@playwright/test";

// Mock the read-only records so this regression never changes the live estate.
const areas = Array.from({ length: 24 }, (_, i) => ({
  id: `scroll-area-${i}`,
  name: `Scroll area ${String(i + 1).padStart(2, "0")}`,
  color: "#3b82f6",
  shiftTime: "22:00-06:00",
  workersNeeded: 1,
  roster: [],
}));

for (const viewport of [
  { width: 1280, height: 800 },
  { width: 1280, height: 520 },
  { width: 390, height: 735 },
]) {
  test(`duty area list scrolls to the last area at ${viewport.width}x${viewport.height} @safari`, async ({ page }) => {
    await page.setViewportSize(viewport);
    await page.route("**/api/**", async (route) => {
      expect(route.request().method()).toBe("GET");
      const path = new URL(route.request().url()).pathname;
      await route.fulfill({ json: path.endsWith("/duty_areas") ? areas : [] });
    });
    await page.goto("/print/s5view-duty.html");
    await expect(page.locator("body[data-ready='1']")).toBeAttached();

    const list = page.locator("#duty-areas-list");
    const last = list.locator(".duty-sidebar-area").last();
    const add = page.locator("#btn-add-duty-area");
    await expect(list.locator(".duty-sidebar-area")).toHaveCount(areas.length);
    const headerY = (await add.boundingBox())!.y;

    await list.hover();
    await page.mouse.wheel(0, 10_000);
    await expect.poll(() => list.evaluate((el) => el.scrollTop)).toBeGreaterThan(0);
    await expect.poll(() => last.evaluate((el) => {
      const row = el.getBoundingClientRect();
      const container = el.parentElement!.getBoundingClientRect();
      return row.top >= container.top && row.bottom <= container.bottom + 1;
    })).toBe(true);
    await expect(add).toBeInViewport();
    expect((await add.boundingBox())!.y).toBeCloseTo(headerY, 0);

    await last.click();
    await expect(last).toHaveClass(/selected/);
    await expect(page.locator("#duty-calendar-grid")).toBeVisible();
  });
}
