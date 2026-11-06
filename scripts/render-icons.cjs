// Renders frontend/public/icons/icon.svg to the PNG sizes listed in manifest.json.
// Usage (from the e2e folder, which has Playwright installed): node ../scripts/render-icons.cjs
const { chromium } = require("@playwright/test");
const fs = require("fs");
const path = require("path");

(async () => {
  const dir = path.join(__dirname, "..", "frontend", "public", "icons");
  const svg = fs.readFileSync(path.join(dir, "icon.svg"), "utf8");
  const browser = await chromium.launch();
  for (const size of [192, 512]) {
    const page = await browser.newPage({ viewport: { width: size, height: size } });
    const sized = svg.replace('width="512" height="512"', `width="${size}" height="${size}"`);
    await page.setContent(`<html><body style="margin:0;background:#37352f">${sized}</body></html>`);
    await page.screenshot({ path: path.join(dir, `icon-${size}.png`) });
    await page.close();
  }
  await browser.close();
  console.log("icons rendered");
})();
