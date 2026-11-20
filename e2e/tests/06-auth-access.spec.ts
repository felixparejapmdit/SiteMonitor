/**
 * Sign-in, first-time password change, and the access matrix.
 */
import { api, expect, openAs, resetDatabase, test, userFor } from "./helpers";

test.beforeAll(() => resetDatabase());

test("sign-in: wrong password is refused, demo sign-in works, sign out ends the session", async ({ browser, request }) => {
  const admin = await userFor(request, "OfficeAdmin");
  const context = await browser.newContext();
  const page = await context.newPage();
  await page.goto("/map-studio");
  await expect(page.getByTestId("login-form")).toBeVisible();
  expect((await page.request.get("/api/estate/map")).status()).toBe(401);

  await page.getByTestId("login-email").fill(admin.email);
  await page.getByTestId("login-password").fill("not-the-password");
  await page.getByTestId("login-submit").click();
  await expect(page.getByTestId("login-error")).toContainText("don't match");

  await page.getByTestId("demo-OfficeAdmin").click();
  await expect(page.getByTestId("signed-in-as")).toHaveText(admin.name);
  // Deep link survives sign-in.
  await expect(page.getByRole("heading", { name: "Map Studio" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Access" })).toBeVisible();

  await page.getByTestId("sign-out").first().click();
  await expect(page.getByTestId("login-form")).toBeVisible();
  expect(await (await page.request.get("/api/auth/me")).json()).toBeNull();
  await context.close();
});

test("a new person must choose their own password; staff without access can't open admin pages", async ({ browser, request }) => {
  const admin = await userFor(request, "OfficeAdmin");
  const email = `liza.${Date.now()}@ashford-estate.example`;
  const created = await api(request, admin).post("/users", {
    name: "Liza Marquez", role: "SiteMonitor", phone: "", email, assigned_area_ids: [], is_active: true, initial_password: "welcome-to-the-estate",
  });
  expect(created.ok(), await created.text()).toBeTruthy();

  const context = await browser.newContext();
  const page = await context.newPage();
  await page.goto("/");
  await page.getByTestId("login-email").fill(email);
  await page.getByTestId("login-password").fill("welcome-to-the-estate");
  await page.getByTestId("login-submit").click();
  await expect(page.getByTestId("forced-password-change")).toBeVisible();
  await page.getByTestId("pw-current").fill("welcome-to-the-estate");
  await page.getByTestId("pw-new").fill("short");
  await expect(page.getByTestId("pw-save")).toBeDisabled();
  await page.getByTestId("pw-new").fill("liza-own-secret-2026");
  await page.getByTestId("pw-confirm").fill("liza-own-secret-2026");
  await page.getByTestId("pw-save").click();
  await expect(page.locator("main")).toBeVisible();
  await expect(page.getByTestId("signed-in-as")).toHaveText("Liza Marquez");

  await expect(page.getByRole("link", { name: "Access" })).toHaveCount(0);
  await page.goto("/access");
  await expect(page.getByText("You don't have access to the access matrix")).toBeVisible();
  await context.close();
});

test("access matrix: grant a permission, review, save — it applies at once and is logged", async ({ page, request }) => {
  await openAs(page, "OfficeAdmin", "/access");
  await expect(page.getByTestId("access-matrix")).toBeVisible();

  // You can't remove your own access to this page.
  const admin = await userFor(request, "OfficeAdmin");
  await expect(page.locator(`[data-testid=cell][aria-label="${admin.name} — Change access (this matrix)"]`)).toBeDisabled();

  const cell = page.locator('[data-testid=cell][aria-label="Ana Santos — Edit the estate map"]');
  await expect(cell).toHaveAttribute("aria-checked", "false");
  await cell.click();
  await expect(cell).toHaveAttribute("aria-checked", "true");
  await expect(page.getByTestId("access-pending")).toContainText("1 change");
  await page.getByTestId("review-access").click();
  await expect(page.getByRole("dialog")).toContainText("+ Edit the estate map");
  await page.getByTestId("confirm-access").click();
  await expect(page.getByTestId("access-pending")).toHaveCount(0);
  await expect(page.getByTestId("access-audit")).toContainText("Granted: Edit the estate map");

  // Ana now has Map Studio, and the server agrees.
  const ana = await userFor(request, "SiteMonitor", "Ana");
  expect(ana.permissions).toContain("ManageMap");
  const anaPage = await page.context().browser()!.newPage();
  await openAs(anaPage, "SiteMonitor", "/", "Ana");
  await expect(anaPage.getByRole("link", { name: "Map Studio" })).toBeVisible();
  await anaPage.close();
});
