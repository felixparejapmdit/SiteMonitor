/**
 * Sidebar dropdowns and page tabs: categories open and close like dropdowns,
 * and a tabbed page's tabs appear as sub-items under it. Picking a tab in
 * either place changes both.
 */
import { expect, openAs, resetDatabase, test } from "./helpers";

test.describe.configure({ mode: "serial" });
test.beforeAll(() => resetDatabase());

const shots = (name: string) => `test-results/nav-${name}.png`;

test("a tabbed page's tabs are sidebar sub-items, and the two stay in step", async ({ page }) => {
  await openAs(page, "OfficeAdmin", "/operations");
  const sidebar = page.getByTestId("sidebar");
  // The open page's tabs are listed under it, with the open tab marked.
  await expect(sidebar.getByTestId("nav-tab-Overnight Duty")).toHaveClass(/active/);
  await page.screenshot({ path: shots("operations"), clip: { x: 0, y: 0, width: 1440, height: 900 } });

  // Sidebar → page.
  await sidebar.getByTestId("nav-tab-OT Entries").click();
  await expect(page.getByTestId("tab-OT Entries")).toHaveAttribute("aria-selected", "true");
  await expect(page.getByTestId("ot-total")).toBeVisible();

  // Page → sidebar.
  await page.getByTestId("tab-Signatories").click();
  await expect(sidebar.getByTestId("nav-tab-Signatories")).toHaveClass(/active/);
  await expect(sidebar.getByTestId("nav-tab-OT Entries")).not.toHaveClass(/active/);

  // Another page's tabs, straight from the sidebar.
  const settings = sidebar.locator("li", { has: page.getByRole("link", { name: "Settings" }) }).first();
  await settings.getByTestId("nav-expand").click();
  await expect(settings.getByTestId("nav-expand")).toHaveAttribute("aria-expanded", "true");
  await settings.getByTestId("nav-tab-Estate data").click();
  await expect(page).toHaveURL(/\/settings$/);
  await expect(page.getByTestId("tab-Estate data")).toHaveAttribute("aria-selected", "true");
  await expect(page.getByTestId("load-sample")).toBeVisible();
  await page.screenshot({ path: shots("settings"), clip: { x: 0, y: 0, width: 1440, height: 900 } });
});

test("categories open and close like dropdowns, and stay as left", async ({ page }) => {
  await openAs(page, "OfficeAdmin", "/");
  const sidebar = page.getByTestId("sidebar");
  const admin = sidebar.getByTestId("nav-group").filter({ hasText: "System & admin" });
  await expect(admin).toHaveAttribute("aria-expanded", "true");
  await expect(sidebar.getByRole("link", { name: "Access" })).toBeVisible();
  await admin.click();
  await expect(admin).toHaveAttribute("aria-expanded", "false");
  await expect(sidebar.getByRole("link", { name: "Access" })).toHaveCount(0);
  await page.reload();
  await expect(sidebar.getByTestId("nav-group").filter({ hasText: "System & admin" })).toHaveAttribute("aria-expanded", "false");
  await page.screenshot({ path: shots("collapsed"), clip: { x: 0, y: 0, width: 1440, height: 900 } });
  // Going to a page inside a closed category opens it.
  await page.goto("/access");
  await expect(sidebar.getByTestId("nav-group").filter({ hasText: "System & admin" })).toHaveAttribute("aria-expanded", "true");
  await expect(sidebar.getByRole("link", { name: "Access" })).toHaveClass(/active/);
});
