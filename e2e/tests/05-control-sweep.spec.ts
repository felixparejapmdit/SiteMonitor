/**
 * Control sweep: for every page × role, click every visible button, link,
 * select and checkbox one at a time (fresh page each time) and fail on any
 * WASM panic, console error, failed API call or blank screen.
 */
import { Page } from "@playwright/test";
import { Role, expect, openAs, resetDatabase, signInPage, test, workItemByTitle } from "./helpers";

test.describe.configure({ mode: "serial" });
test.setTimeout(40 * 60_000); // hundreds of controls, each clicked on a fresh page
test.beforeAll(() => resetDatabase());

const CONTROLS = "main button:visible, main a[href]:visible, main select:visible, main input[type=checkbox]:visible, [role=dialog] button:visible";

async function areaId(page: Page, name: string): Promise<string> {
  const map = await (await page.request.get("/api/estate/map")).json();
  const flat: any[] = [];
  const walk = (n: any) => { flat.push(n.area); n.children.forEach(walk); };
  map.areas.forEach(walk);
  return flat.find((a) => a.name === name).id;
}

async function pagesFor(page: Page, role: Role): Promise<string[]> {
  await signInPage(page, "OfficeAdmin");
  const parking = (await workItemByTitle(page.request, "Parking Conversion to Multi-Purpose Hall")).id;
  const pool = (await workItemByTitle(page.request, "Swimming Pool Cleanup")).id;
  const gazebo = (await workItemByTitle(page.request, "Rose Garden Gazebo Build")).id;
  const hall = await areaId(page, "Multi-Purpose Hall (Parking Conversion)");
  const shared = ["/", "/agenda", "/archive", `/projects/${parking}`, `/projects/${pool}`, `/zones/${hall}`, `/handover/${gazebo}`];
  if (role === "VipClient") return shared;
  const staff = [...shared, "/crew", "/equipment", "/decisions", "/assignments", "/personnel"];
  return role === "OfficeAdmin" ? [...staff, "/map-studio", "/access", "/people", "/settings", "/workforce", "/operations", "/clinic"] : staff;
}

/** Collects problems (panics, console errors, failed API calls) while a page is open. */
function watch(page: Page) {
  const problems: string[] = [];
  page.on("pageerror", (e) => problems.push(`pageerror: ${e.message}`));
  page.on("console", (m) => { if (m.type() === "error") problems.push(`console: ${m.text()}`); });
  page.on("response", (r) => { if (r.url().includes("/api/") && r.status() >= 500) problems.push(`api ${r.status()}: ${r.url()}`); });
  return problems;
}

const findings: string[] = [];

for (const [role, who] of [["VipClient", undefined], ["OfficeAdmin", undefined], ["SiteMonitor", "Jun"]] as [Role, string | undefined][]) {
  test(`every control works for ${role}`, async ({ browser }) => {
    const probe = await browser.newPage();
    const paths = await pagesFor(probe, role);
    await probe.close();
    let clicked = 0;

    for (const path of paths) {
      const first = await browser.newPage();
      await openAs(first, role, path, who);
      await first.waitForTimeout(600);
      const total = await first.locator(CONTROLS).count();
      await first.close();

      for (let i = 0; i < total; i++) {
        const page = await browser.newPage();
        const problems = watch(page);
        await openAs(page, role, path, who);
        await page.waitForTimeout(400);
        const el = page.locator(CONTROLS).nth(i);
        if (!(await el.count()) || !(await el.isEnabled().catch(() => false))) { await page.close(); continue; }
        const tag = await el.evaluate((e) => e.tagName.toLowerCase());
        const label = ((await el.innerText().catch(() => "")) || (await el.getAttribute("aria-label")) || (await el.getAttribute("data-testid")) || tag).trim().slice(0, 40);
        const href = tag === "a" ? await el.getAttribute("href") : null;
        try {
          if (href && /^(tel:|sms:|mailto:)/.test(href)) {
            expect(href.length).toBeGreaterThan(5); // native dialer links: only check they exist
          } else if (tag === "select") {
            const values = await el.locator("option").evaluateAll((os) => os.map((o) => (o as HTMLOptionElement).value).filter(Boolean));
            if (values.length) await el.selectOption(values[values.length - 1]);
          } else {
            await el.click({ timeout: 3000 });
          }
          await page.waitForTimeout(700);
          // Some controls reload the page on purpose (e.g. switching a module): let it finish first.
          await page.waitForLoadState("load").catch(() => {});
          const isAlive = async () => (await page.locator("main").count()) > 0 || (await page.locator("[role=dialog]").count()) > 0;
          let alive = await isAlive();
          for (let i = 0; !alive && i < 6; i++) {
            await page.waitForTimeout(500);
            alive = await isAlive();
          }
          if (!alive) problems.push("app disappeared after click");
        } catch (e: any) {
          problems.push(`click failed: ${String(e.message).split("\n")[0]}`);
        }
        clicked++;
        for (const p of problems) findings.push(`[${role}] ${path} → "${label}": ${p}`);
        await page.close();
      }
    }
    console.log(`${role}: clicked ${clicked} controls`);
  });
}

test.afterAll(() => {
  if (findings.length) console.log("FINDINGS:\n" + findings.join("\n"));
  expect(findings, findings.join("\n")).toEqual([]);
});
