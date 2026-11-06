// Generates the illustrative seed artwork used by the demo estate:
// top-down base-map renders (16:10), 16:9 site "photos" and the app icon.
// Run: node scripts/generate-seed-art.mjs   (from the frontend folder)
import { mkdirSync, writeFileSync } from "node:fs";

const out = (p, s) => { writeFileSync(new URL(`../public/${p}`, import.meta.url), s.trim() + "\n"); };
mkdirSync(new URL("../public/seed/photos", import.meta.url), { recursive: true });
mkdirSync(new URL("../public/icons", import.meta.url), { recursive: true });

// ----------------------------------------------------------------- helpers
const W = 1600, H = 1000; // base map: 16:10, coordinates are % of these
const X = (p) => (p * W) / 100, Y = (p) => (p * H) / 100;
const rect = (x1, y1, x2, y2, fill, extra = "") =>
  `<rect x="${X(x1)}" y="${Y(y1)}" width="${X(x2 - x1)}" height="${Y(y2 - y1)}" fill="${fill}" ${extra}/>`;
const poly = (pts, fill, extra = "") =>
  `<polygon points="${pts.map(([x, y]) => `${X(x)},${Y(y)}`).join(" ")}" fill="${fill}" ${extra}/>`;
let seed = 7;
const rnd = () => ((seed = (seed * 9301 + 49297) % 233280) / 233280);
const trees = (n, x1, y1, x2, y2, r = 14) => {
  let s = "";
  for (let i = 0; i < n; i++) {
    const cx = X(x1 + rnd() * (x2 - x1)), cy = Y(y1 + rnd() * (y2 - y1)), rr = r * (0.7 + rnd() * 0.6);
    s += `<circle cx="${cx + 4}" cy="${cy + 5}" r="${rr}" fill="#000" opacity=".12"/><circle cx="${cx}" cy="${cy}" r="${rr}" fill="${rnd() > .5 ? "#7f9c6b" : "#6d8c5c"}"/><circle cx="${cx - rr / 3}" cy="${cy - rr / 3}" r="${rr / 2.2}" fill="#9db886" opacity=".7"/>`;
  }
  return s;
};
const roof = (x1, y1, x2, y2, color = "#c9c1b3") =>
  `<g filter="url(#shadow)">${rect(x1, y1, x2, y2, color, 'rx="6"')}</g>` +
  `<line x1="${X(x1)}" y1="${Y((y1 + y2) / 2)}" x2="${X(x2)}" y2="${Y((y1 + y2) / 2)}" stroke="#b3aa9a" stroke-width="3"/>`;

function baseMap(version) {
  seed = version === 1 ? 7 : 3;
  const v1 = version === 1;
  return `
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${W} ${H}" width="${W}" height="${H}">
  <defs>
    <filter id="shadow" x="-10%" y="-10%" width="130%" height="140%"><feDropShadow dx="6" dy="8" stdDeviation="5" flood-color="#2f3a26" flood-opacity=".28"/></filter>
    <pattern id="grass" width="24" height="24" patternUnits="userSpaceOnUse"><rect width="24" height="24" fill="#d7e2c8"/><path d="M3 20l2-6M14 9l1-5M19 22l2-5" stroke="#c6d4b3" stroke-width="1.4"/></pattern>
    <pattern id="paving" width="20" height="20" patternUnits="userSpaceOnUse"><rect width="20" height="20" fill="#ece5d6"/><path d="M0 10h20M10 0v20" stroke="#ddd3bf" stroke-width="1"/></pattern>
    <linearGradient id="water" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#7fc4d8"/><stop offset="1" stop-color="#4f9fbf"/></linearGradient>
  </defs>
  <rect width="${W}" height="${H}" fill="url(#grass)"/>
  <!-- driveways & paths -->
  <path d="M${X(50)} ${H} L${X(50)} ${Y(66)} M${X(4)} ${Y(33)} H${X(96)} M${X(4)} ${Y(74)} H${X(96)} M${X(33)} ${Y(33)} V${Y(74)} M${X(67)} ${Y(33)} V${Y(74)}" stroke="#efe8da" stroke-width="26" fill="none" stroke-linecap="round"/>
  <path d="M${X(50)} ${H} L${X(50)} ${Y(66)} M${X(4)} ${Y(33)} H${X(96)} M${X(4)} ${Y(74)} H${X(96)}" stroke="#e2d9c6" stroke-width="2" stroke-dasharray="14 12" fill="none"/>
  ${trees(26, 1, 1, 99, 5, 16)}${trees(10, 1, 50, 5, 72, 13)}${trees(10, 95, 50, 99, 72, 13)}${trees(14, 31.5, 8, 35.5, 30, 12)}${trees(14, 64.5, 8, 68.5, 30, 12)}
  <!-- main residences -->
  <g>${rect(36, 5, 64, 34, "url(#paving)", 'rx="10"')}</g>
  ${roof(38, 7, 62, 17)}${roof(38, 18, 53, 29.5, "#cdbfa8")}${roof(54, 18, 62, 29.5, "#d3c7b3")}
  ${rect(46, 30, 54, 33, "#b9ae9b", 'rx="3"')}
  ${roof(7.5, 8.5, 29.5, 27.5)}${rect(10, 22, 18, 26.5, "url(#water)", 'rx="4" opacity=".9"')}
  ${roof(70.5, 8.5, 92.5, 27.5, "#c4bcae")}
  <!-- guest villas -->
  ${roof(6, 37, 16, 47, "#d2c3a8")}${roof(20, 37, 30, 47, "#c9b493")}${roof(70, 37, 80, 47, "#d2c3a8")}${roof(84, 37, 94, 47, "#d2c3a8")}
  <!-- grand garden -->
  ${poly([[35, 37], [65, 37], [66, 52], [63, 66], [37, 66], [34, 52]], "#c4d6aa")}
  ${rect(37.5, 39.5, 47.5, 49.5, "#e8f0e4", 'rx="6" stroke="#b9cbb0" stroke-width="3"')}
  ${[0, 1, 2].map((i) => rect(38.5, 41 + i * 2.8, 46.5, 42.4 + i * 2.8, "#9cbf86", 'rx="3"')).join("")}
  ${rect(52.5, 39.5, 62.5, 49.5, "#b9cf9c", 'rx="6"')}
  ${[0, 1, 2, 3].map((i) => `<circle cx="${X(54 + i * 2.3)}" cy="${Y(42)}" r="9" fill="#d98a9a"/><circle cx="${X(54 + i * 2.3)}" cy="${Y(47)}" r="9" fill="#e3a4b0"/>`).join("")}
  ${v1 ? `<g filter="url(#shadow)"><polygon points="${X(57.5)},${Y(43.2)} ${X(59.6)},${Y(44.6)} ${X(59.6)},${Y(47.6)} ${X(57.5)},${Y(49)} ${X(55.4)},${Y(47.6)} ${X(55.4)},${Y(44.6)}" fill="#8a6f52"/></g>` : ""}
  ${rect(38, 54, 62, 64, "#cfc6b2", 'rx="4"')}${rect(38, 54, 62, 56, "#a99f8c")}
  ${trees(8, 36, 50.5, 64, 53.5, 10)}
  <!-- pool -->
  ${rect(7, 54, 30, 70, "url(#paving)", 'rx="8"')}
  ${rect(9.5, 57, 27.5, 67, "url(#water)", 'rx="10" stroke="#e9f6fb" stroke-width="6"')}
  <path d="M${X(11)} ${Y(60)} q20 -8 40 0 t40 0 t40 0 t40 0" stroke="#bfe6f2" stroke-width="3" fill="none" opacity=".7"/>
  <!-- multi-purpose hall (parking conversion) -->
  ${rect(70, 54, 93, 70, v1 ? "#d9d3c7" : "#bdb6a8", 'rx="6"')}
  ${v1 ? roof(71.5, 55.5, 85, 68.5, "#c7bfb0") + rect(86, 56, 92, 68, "url(#paving)", 'rx="4"') : [0, 1, 2, 3, 4, 5].map((i) => rect(71 + i * 3.6, 56, 73.8 + i * 3.6, 68, "#a8a091", 'rx="2"')).join("")}
  <!-- utilities -->
  ${roof(5.5, 79, 19.5, 93, "#b8b6b0")}${[0, 1, 2, 3].map((i) => rect(7 + i * 3, 80.5, 9.2 + i * 3, 83, "#3d4b5c", 'rx="2"')).join("")}
  ${roof(23.5, 79, 35.5, 93, "#bfbdb6")}${`<circle cx="${X(27)}" cy="${Y(87)}" r="22" fill="#9cc7d6" stroke="#8a8a84" stroke-width="5"/><circle cx="${X(32)}" cy="${Y(87)}" r="22" fill="#9cc7d6" stroke="#8a8a84" stroke-width="5"/>`}
  ${poly([[40, 80], [49, 80], [51, 87], [49, 94], [40, 94], [38, 87]], "#c9c6bd")}<circle cx="${X(44.5)}" cy="${Y(87)}" r="24" fill="#8a8f96" stroke="#6c7178" stroke-width="5"/>
  <!-- contractor yard -->
  ${rect(54, 76, 97, 96, "#ddd6c6", 'rx="6"')}
  ${roof(56.5, 78.5, 66.5, 93.5, "#c2b8a6")}${roof(70.5, 78.5, 81.5, 93.5, "#b5ad9e")}${roof(84.5, 78.5, 94.5, 93.5, "#b9b1a2")}
  ${v1 ? "" : roof(84.5, 30.5, 91.5, 33.5, "#a99f8f")}
  ${trees(10, 1, 95, 99, 99, 12)}
</svg>`;
}

out("seed/estate-map-v1.svg", baseMap(1));
out("seed/estate-map-v0.svg", baseMap(0));

// ----------------------------------------------------------------- photos (1600x900)
const sky = (top = "#cfe3f1", bottom = "#f4efe6") =>
  `<defs><linearGradient id="sky" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${top}"/><stop offset="1" stop-color="${bottom}"/></linearGradient>
   <linearGradient id="ground" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#b9a98d"/><stop offset="1" stop-color="#8f7f66"/></linearGradient>
   <linearGradient id="pool" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#8fd3e6"/><stop offset="1" stop-color="#3f97bd"/></linearGradient>
   <filter id="soft"><feGaussianBlur stdDeviation="2"/></filter></defs>
   <rect width="1600" height="900" fill="url(#sky)"/>`;
const hills = `<path d="M0 470 Q200 400 420 450 T860 430 T1300 445 T1600 420 V560 H0z" fill="#9fb68c"/><path d="M0 520 Q300 470 620 505 T1240 495 T1600 505 V600 H0z" fill="#86a374"/>`;
const treeline = (y, n = 18) => Array.from({ length: n }, (_, i) => `<circle cx="${40 + i * 92 + (i % 3) * 14}" cy="${y - (i % 4) * 10}" r="${44 + (i % 3) * 10}" fill="${i % 2 ? "#6b8a5a" : "#5d7d4e"}"/>`).join("");
const photo = (body) => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1600 900" width="1600" height="900">${body}</svg>`;

out("seed/photos/parking-before.svg", photo(`${sky()}${hills}${treeline(470)}
  <rect x="0" y="560" width="1600" height="340" fill="#b5ab99"/>
  <rect x="260" y="250" width="1080" height="360" fill="#d9d0c1"/><polygon points="230,250 800,150 1370,250" fill="#8c6f5a"/>
  ${[0, 1, 2, 3, 4, 5].map((i) => `<rect x="${300 + i * 170}" y="360" width="140" height="250" fill="#9c9488"/><g stroke="#8a8276" stroke-width="4">${[0, 1, 2, 3, 4].map((j) => `<line x1="${300 + i * 170}" y1="${385 + j * 45}" x2="${440 + i * 170}" y2="${385 + j * 45}"/>`).join("")}</g>`).join("")}
  ${Array.from({ length: 40 }, (_, i) => `<circle cx="${(i * 137) % 1600}" cy="${640 + ((i * 53) % 240)}" r="${3 + (i % 4)}" fill="#9c917d"/>`).join("")}`));

out("seed/photos/parking-progress.svg", photo(`${sky()}${hills}${treeline(470)}
  <rect x="0" y="540" width="1600" height="360" fill="url(#ground)"/>
  <rect x="120" y="600" width="1360" height="260" fill="#cdbb95"/>
  ${[0, 1, 2, 3, 4, 5, 6].map((i) => `<line x1="120" y1="${620 + i * 36}" x2="1480" y2="${620 + i * 36}" stroke="#f6f1e4" stroke-width="2" opacity=".8"/>`).join("")}
  ${[0, 1, 2, 3].map((i) => `<rect x="${160 + i * 420}" y="575" width="10" height="70" fill="#7a5b3f"/>`).join("")}
  <ellipse cx="1144" cy="576" rx="140" ry="26" fill="#7fa9c0" opacity=".85"/><ellipse cx="1130" cy="570" rx="70" ry="9" fill="#cfe5f0" opacity=".7"/>
  <rect x="300" y="380" width="360" height="190" fill="#e0b54a"/><rect x="560" y="330" width="120" height="120" fill="#e9c35d"/><circle cx="360" cy="580" r="44" fill="#333"/><circle cx="600" cy="580" r="44" fill="#333"/><path d="M320 380 L190 250 L120 330" stroke="#c7962f" stroke-width="22" fill="none"/>
  ${Array.from({ length: 6 }, (_, i) => `<rect x="${220 + i * 110}" y="${680 + (i % 2) * 40}" width="90" height="46" fill="#e8dcc0" stroke="#d3c39f" stroke-width="4"/>`).join("")}`));

out("seed/photos/wall-before.svg", photo(`${sky("#d6e1e8", "#efece6")}${hills}${treeline(470)}
  <rect x="0" y="520" width="1600" height="380" fill="#9e8f72"/>
  <path d="M0 430 L1600 470 L1600 640 L0 610z" fill="#a39780"/>
  ${Array.from({ length: 30 }, (_, i) => `<rect x="${(i % 10) * 165 + (i % 3) * 8}" y="${470 + Math.floor(i / 10) * 55 + (i % 10) * 4}" width="150" height="48" fill="${i % 2 ? "#b4a990" : "#a99d84"}" transform="rotate(${(i % 5) - 2} ${(i % 10) * 165} 500)"/>`).join("")}
  <path d="M520 620 Q700 700 900 860 L1100 900 L1200 900 Q1000 720 760 600z" fill="#7d6a4f"/>`));

out("seed/photos/wall-progress.svg", photo(`${sky()}${hills}${treeline(470)}
  <rect x="0" y="560" width="1600" height="340" fill="#a8977a"/>
  <rect x="80" y="420" width="1440" height="200" fill="#c9c6bf"/><rect x="80" y="400" width="1440" height="30" fill="#b9b5ad"/>
  ${[0, 1, 2, 3].map((i) => `<path d="M${180 + i * 330} 430 h300 v170 h-300z" fill="#3f7fa8" opacity=".9"/><path d="M${180 + i * 330} 430 l150 60 l150 -60" stroke="#2f6a90" stroke-width="5" fill="none"/>`).join("")}
  ${[0, 1, 2, 3, 4, 5, 6, 7].map((i) => `<rect x="${200 + i * 160}" y="590" width="60" height="30" fill="#8b8b85"/>`).join("")}`));

out("seed/photos/gazebo-before.svg", photo(`${sky()}${hills}${treeline(470)}
  <rect x="0" y="520" width="1600" height="380" fill="#9dba86"/>
  <ellipse cx="800" cy="720" rx="420" ry="110" fill="#b39b78"/>
  ${Array.from({ length: 26 }, (_, i) => `<circle cx="${60 + i * 60}" cy="${600 + (i % 3) * 18}" r="22" fill="#5f8a4f"/><circle cx="${60 + i * 60}" cy="${592 + (i % 3) * 18}" r="9" fill="${i % 2 ? "#d9788c" : "#e7a3b2"}"/>`).join("")}`));

out("seed/photos/gazebo-after.svg", photo(`${sky("#f6d9b8", "#f3e6d6")}${hills}${treeline(470)}
  <rect x="0" y="520" width="1600" height="380" fill="#9dba86"/>
  <ellipse cx="800" cy="730" rx="440" ry="115" fill="#c9b18e"/>
  <polygon points="800,180 1120,360 480,360" fill="#7d5e44"/><rect x="520" y="360" width="560" height="22" fill="#6b4f39"/>
  ${[0, 1, 2, 3].map((i) => `<rect x="${545 + i * 165}" y="380" width="26" height="320" fill="#8a6a4d"/>`).join("")}
  <rect x="520" y="690" width="560" height="30" fill="#b8ad99"/>
  ${[0, 1, 2, 3, 4, 5].map((i) => `<circle cx="${590 + i * 85}" cy="400" r="9" fill="#ffe7a8"/><circle cx="${590 + i * 85}" cy="400" r="26" fill="#ffe7a8" opacity=".25"/>`).join("")}
  ${Array.from({ length: 26 }, (_, i) => `<circle cx="${60 + i * 60}" cy="${620 + (i % 3) * 18}" r="22" fill="#5f8a4f"/><circle cx="${60 + i * 60}" cy="${612 + (i % 3) * 18}" r="9" fill="${i % 2 ? "#d9788c" : "#e7a3b2"}"/>`).join("")}
  <rect x="1150" y="660" width="80" height="60" fill="#55585c" opacity=".7"/>`));

out("seed/photos/suite-before.svg", photo(`<rect width="1600" height="900" fill="#e9e3d8"/>
  <polygon points="0,0 1600,0 1300,180 300,180" fill="#f2eee6"/><polygon points="300,720 1300,720 1600,900 0,900" fill="#b8a48a"/>
  <polygon points="0,0 300,180 300,720 0,900" fill="#ddd5c7"/><polygon points="1600,0 1300,180 1300,720 1600,900" fill="#d6cdbd"/>
  <rect x="300" y="180" width="1000" height="540" fill="#e4ddcf"/>
  <rect x="560" y="280" width="480" height="260" fill="#cfe2ec" stroke="#cfc6b6" stroke-width="16"/><line x1="800" y1="280" x2="800" y2="540" stroke="#cfc6b6" stroke-width="10"/>
  <rect x="380" y="560" width="160" height="160" fill="#c9bba4"/><rect x="1100" y="520" width="120" height="200" fill="#bfae94"/>`));

out("seed/photos/pool-clean.svg", photo(`${sky("#cbe6f5", "#f6f3ec")}${treeline(330, 20)}
  <rect x="0" y="330" width="1600" height="570" fill="#efe9dc"/>
  <rect x="140" y="420" width="1320" height="420" rx="40" fill="url(#pool)" stroke="#f8fcfd" stroke-width="18"/>
  ${Array.from({ length: 9 }, (_, i) => `<path d="M${200 + i * 140} ${520 + (i % 3) * 90} q35 -14 70 0 t70 0" stroke="#d4f0f8" stroke-width="5" fill="none" opacity=".8"/>`).join("")}
  <rect x="160" y="760" width="200" height="60" fill="#e9f6fb" opacity=".8"/>
  <rect x="1300" y="360" width="22" height="80" fill="#b0b8bf"/><rect x="1340" y="360" width="22" height="80" fill="#b0b8bf"/>`));

// ----------------------------------------------------------------- icon
out("icons/icon.svg", `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <rect width="512" height="512" rx="112" fill="#37352f"/>
  <path d="M120 360 L256 140 L392 360 Z" fill="none" stroke="#f7f7f5" stroke-width="22" stroke-linejoin="round"/>
  <circle cx="256" cy="300" r="34" fill="#d97706"/>
  <path d="M120 360 H392" stroke="#f7f7f5" stroke-width="22" stroke-linecap="round"/>
</svg>`);
console.log("seed art generated");
