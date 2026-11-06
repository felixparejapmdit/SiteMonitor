import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { APIRequestContext, Page, expect } from "@playwright/test";

const ROOT = join(__dirname, "..", "..");

/** Drops and re-seeds the database so every spec starts from the same estate. */
export function resetDatabase() {
  const exe = join(ROOT, "target", "release", process.platform === "win32" ? "backend.exe" : "backend");
  if (existsSync(exe)) {
    execFileSync(exe, ["reset"], { cwd: ROOT, stdio: "pipe" });
  } else {
    execFileSync("cargo", ["run", "-p", "backend", "--release", "--", "reset"], { cwd: ROOT, stdio: "pipe" });
  }
}

export type Role = "VipClient" | "OfficeAdmin" | "SiteMonitor";

export interface User {
  id: string;
  name: string;
  role: Role;
  assigned_area_ids: string[];
}

export async function users(request: APIRequestContext): Promise<User[]> {
  const res = await request.get("/api/users");
  expect(res.ok()).toBeTruthy();
  return res.json();
}

export async function userFor(request: APIRequestContext, role: Role, firstName?: string): Promise<User> {
  const list = await users(request);
  const u = list.find((x) => x.role === role && (!firstName || x.name.startsWith(firstName)));
  if (!u) throw new Error(`no ${role} user`);
  return u;
}

/** Opens the app already signed in as the given role (dev-mode switcher). */
export async function openAs(page: Page, role: Role, path = "/", firstName?: string) {
  const u = await userFor(page.request, role, firstName);
  await page.addInitScript((id) => window.localStorage.setItem("sitemonitor.user", id), u.id);
  await page.goto(path);
  await expect(page.locator("body.app-ready")).toHaveCount(1, { timeout: 30_000 });
  await expect(page.locator("main")).toBeVisible();
  return u;
}

/** Authenticated API helper acting as `user`. */
export function api(request: APIRequestContext, user: User) {
  const headers = { "x-user-id": user.id };
  return {
    get: async (path: string) => {
      const r = await request.get(`/api${path}`, { headers });
      expect(r.ok(), `${path} → ${r.status()} ${await r.text()}`).toBeTruthy();
      return r.json();
    },
    post: (path: string, data: unknown = {}) => request.post(`/api${path}`, { headers, data }),
    put: (path: string, data: unknown = {}) => request.put(`/api${path}`, { headers, data }),
  };
}

export async function workItemByTitle(request: APIRequestContext, title: string) {
  const items = await (await request.get("/api/work-items")).json();
  const w = items.find((x: { title: string }) => x.title === title);
  if (!w) throw new Error(`work item ${title} not found`);
  return w;
}

/** A small 4:3 PNG (red with a blue band) for the 16:9 cropper tests. */
export function pngBuffer(width = 800, height = 600): Buffer {
  // Minimal uncompressed PNG encoder (zlib "stored" blocks).
  const zlib = require("node:zlib");
  const row = Buffer.alloc(1 + width * 3);
  const raw = Buffer.alloc((1 + width * 3) * height);
  for (let y = 0; y < height; y++) {
    row[0] = 0;
    for (let x = 0; x < width; x++) {
      const band = y > height * 0.4 && y < height * 0.6;
      row[1 + x * 3] = band ? 40 : 200;
      row[2 + x * 3] = band ? 90 : 60;
      row[3 + x * 3] = band ? 200 : 50;
    }
    row.copy(raw, y * row.length);
  }
  const crcTable = Array.from({ length: 256 }, (_, n) => {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    return c >>> 0;
  });
  const crc = (buf: Buffer) => {
    let c = 0xffffffff;
    for (const b of buf) c = crcTable[(c ^ b) & 0xff] ^ (c >>> 8);
    return (c ^ 0xffffffff) >>> 0;
  };
  const chunk = (type: string, data: Buffer) => {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    const td = Buffer.concat([Buffer.from(type), data]);
    const c = Buffer.alloc(4);
    c.writeUInt32BE(crc(td));
    return Buffer.concat([len, td, c]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;
  ihdr[9] = 2;
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", zlib.deflateSync(raw)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

export const toast = (page: Page) => page.getByTestId("toast");
