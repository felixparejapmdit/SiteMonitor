/**
 * Every page at desktop, tablet and phone sizes with no console or runtime
 * errors; the phone's slide-out menu and the bottom bar's "More" sheet (same
 * categories as the sidebar); and "Load sample data" bringing in a workforce
 * that is linked to the sample estate.
 */
import { Page } from "@playwright/test";
import { Role, expect, openAs, resetDatabase, signInPage, test, workItemByTitle } from "./helpers";

test.describe.configure({ mode: "serial" });
test.setTimeout(10 * 60_000);
test.beforeAll(() => resetDatabase());

const SIZES = { desktop: { width: 1440, height: 900 }, tablet: { width: 820, height: 1180 }, phone: { width: 390, height: 844 } };

function watch(page: Page) {
  const problems: string[] = [];
  page.on("pageerror", (e) => problems.push(`pageerror: ${e.message}`));
  page.on("console", (m) => { if (m.type() === "error") problems.push(`console: ${m.text()}`); });
  page.on("response", (r) => { if (r.url().includes("/api/") && r.status() >= 400 && r.status() !== 401) problems.push(`api ${r.status()}: ${r.url()}`); });
  return problems;
}

async function pagesFor(page: Page, role: Role): Promise<string[]> {
  const parking = (await workItemByTitle(page.request, "Parking Conversion to Multi-Purpose Hall")).id;
  const shared = ["/", "/agenda", "/archive", `/projects/${parking}`];
  if (role === "VipClient") return shared;
  const staff = [...shared, "/crew", "/equipment", "/decisions", "/personnel"];
  return role === "OfficeAdmin" ? [...staff, "/map-studio", "/access", "/people", "/settings", "/workforce", "/operations", "/clinic"] : staff;
}

for (const [size, viewport] of Object.entries(SIZES)) {
  for (const role of ["OfficeAdmin", "SiteMonitor", "VipClient"] as Role[]) {
    test(`${role} on ${size}: every page loads with no errors`, async ({ browser }) => {
      const ctx = await browser.newContext({ viewport });
      const page = await ctx.newPage();
      await signInPage(page, "OfficeAdmin");
      const paths = await pagesFor(page, role);
      const problems = watch(page);
      await openAs(page, role, "/", role === "SiteMonitor" ? "Jun" : undefined);
      for (const p of paths) {
        await page.goto(p);
        await expect(page.locator("body.app-ready")).toHaveCount(1, { timeout: 30_000 });
        await expect(page.locator("main")).not.toBeEmpty();
        await page.waitForLoadState("networkidle");
        // The menu is reachable at every size: the sidebar on wide screens, the menu button on phones.
        if (size === "phone") {
          await expect(page.getByTestId("sidebar")).toBeHidden();
          await expect(page.getByTestId("menu-open")).toBeVisible();
          await expect(page.getByTestId("bottom-more")).toBeVisible();
        } else {
          await expect(page.getByTestId("sidebar")).toBeVisible();
          await expect(page.getByTestId("bottom-bar")).toBeHidden();
        }
        // Nothing spills sideways.
        expect(await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth), `${p} scrolls sideways`).toBeLessThanOrEqual(1);
      }
      expect(problems, problems.join("\n")).toEqual([]);
      await ctx.close();
    });
  }
}

test("phone: the slide-out menu has the sidebar's categories and tabs, and closes after a pick", async ({ browser }) => {
  const ctx = await browser.newContext({ viewport: SIZES.phone });
  const page = await ctx.newPage();
  const problems = watch(page);
  await openAs(page, "OfficeAdmin", "/");
  await page.getByTestId("menu-open").click();
  const drawer = page.getByTestId("mobile-drawer");
  await expect(drawer).toBeVisible();
  for (const g of ["Estate command", "Workforce & crew", "Operations & duty", "System & admin"]) {
    await expect(drawer.getByTestId("nav-group").filter({ hasText: g })).toBeVisible();
  }
  await page.screenshot({ path: "test-results/phone-drawer.png" });
  await page.keyboard.press("Escape");
  await expect(drawer).toHaveCount(0);

  await page.getByTestId("menu-open").click();
  await drawer.getByRole("link", { name: "Personnel" }).click();
  await expect(page).toHaveURL(/\/personnel$/);
  await expect(drawer).toHaveCount(0);

  // A tab picked in the menu opens that tab on the page.
  await page.getByTestId("menu-open").click();
  const ops = drawer.locator("li", { has: page.getByRole("link", { name: "Operations", exact: true }) }).first();
  await ops.getByTestId("nav-expand").click();
  await ops.getByTestId("nav-tab-OT Entries").click();
  await expect(page).toHaveURL(/\/operations$/);
  await expect(page.getByTestId("tab-OT Entries")).toHaveAttribute("aria-selected", "true");
  await expect(drawer).toHaveCount(0);
  expect(problems, problems.join("\n")).toEqual([]);
  await ctx.close();
});

test("phone: the bottom bar ends with More, which lists every page by category", async ({ browser }) => {
  const ctx = await browser.newContext({ viewport: SIZES.phone });
  const page = await ctx.newPage();
  const problems = watch(page);
  await openAs(page, "OfficeAdmin", "/");
  const bar = page.getByTestId("bottom-bar");
  await expect(bar.locator("a, button")).toHaveCount(5);
  await expect(bar.locator("a, button").last()).toHaveText("More");
  await page.getByTestId("bottom-more").click();
  const sheet = page.getByTestId("more-sheet");
  await expect(sheet).toBeVisible();
  const groups = await sheet.getByTestId("more-group").locator("p").allInnerTexts();
  expect(groups.map((g) => g.trim().toLowerCase())).toEqual(["start", "estate command", "workforce & crew", "operations & duty", "system & admin"]);
  await page.screenshot({ path: "test-results/phone-more.png" });
  await sheet.getByRole("link", { name: "Settings" }).click();
  await expect(page).toHaveURL(/\/settings$/);
  await expect(sheet).toHaveCount(0);
  await expect(page.getByTestId("bottom-more")).toHaveClass(/active/); // Settings lives under More
  expect(problems, problems.join("\n")).toEqual([]);
  await ctx.close();
});

test("Load sample data brings a workforce linked to the sample estate", async ({ page, request }) => {
  await openAs(page, "OfficeAdmin", "/settings");
  await page.getByTestId("tab-Estate data").click();
  await page.getByTestId("load-sample").click();
  await page.getByTestId("reset-word").fill("sample");
  await page.getByTestId("reset-confirm").click();
  await expect(page.getByText("Sample data loaded").first()).toBeVisible({ timeout: 60_000 });

  const workers = await (await request.get("/api/workers")).json();
  expect(workers.length).toBe(16);
  const jobs = await (await request.get("/api/work-items")).json();
  const links = await (await request.get("/api/workforce/records/group_links")).json();
  expect(links.length).toBe(4);
  for (const l of links) expect(jobs.some((j: any) => j.id === l.work_item_id), l.id).toBeTruthy();
  const map = await (await request.get("/api/estate/map")).json();
  const flat: any[] = [];
  const walk = (n: any) => { flat.push(n.area); n.children.forEach(walk); };
  map.areas.forEach(walk);
  for (const z of await (await request.get("/api/workforce/records/area_zones")).json()) expect(flat.some((a) => a.id === z.zone_id), z.id).toBeTruthy();
  for (const c of ["duty_schedules", "ot_entries", "visitations", "signatories", "health_events", "attendance_log"]) {
    expect((await (await request.get(`/api/workforce/records/${c}`)).json()).length, c).toBeGreaterThan(0);
  }
  const snaps = await (await request.get("/api/workforce/snapshots")).json();
  expect(snaps.some((s: any) => s.reason === "before loading sample data")).toBeTruthy();

  // Every screen tells the same story: the Crew page shows the linked group's roster.
  await page.goto("/crew");
  await expect(page.getByTestId("crew-row").filter({ hasText: "Parking Conversion" }).getByTestId("crew-roster")).toContainText("present");
  await page.goto("/workforce");
  await expect(page.getByTestId("wf-stale")).toContainText("days");
});
