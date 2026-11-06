/* SiteMonitor service worker.
 *
 * Caches the compiled WASM bundle, CSS, fonts and the active estate base map
 * so the app opens instantly on weak cellular signal and keeps working in
 * dead zones (deepwell pump rooms, basements, far gardens).
 *
 * - App shell & navigations: network first, fall back to the cached shell.
 * - Hashed build assets (/assets/*), fonts, icons, seed media & uploads:
 *   cache first (they never change once published).
 * - The live API and WebSocket are never cached; offline writes are queued
 *   by the app in IndexedDB instead.
 */
const VERSION = "sitemonitor-v1";
const SHELL = ["/", "/manifest.json", "/fonts/inter-variable.woff2", "/icons/icon.svg", "/seed/estate-map-v1.svg"];

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches.open(VERSION).then((cache) => cache.addAll(SHELL)).then(() => self.skipWaiting())
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k !== VERSION).map((k) => caches.delete(k))))
      .then(() => self.clients.claim())
  );
});

function isImmutable(url) {
  return (
    url.pathname.startsWith("/assets/") ||
    url.pathname.startsWith("/wasm/") ||
    url.pathname.startsWith("/fonts/") ||
    url.pathname.startsWith("/icons/") ||
    url.pathname.startsWith("/seed/") ||
    url.pathname.startsWith("/uploads/") ||
    /\.(wasm|js|css|woff2)$/.test(url.pathname)
  );
}

self.addEventListener("fetch", (event) => {
  const req = event.request;
  if (req.method !== "GET") return;
  const url = new URL(req.url);
  if (url.origin !== self.location.origin) return;
  if (url.pathname.startsWith("/api/") || url.pathname === "/ws") return;

  if (req.mode === "navigate") {
    event.respondWith(
      fetch(req)
        .then((res) => {
          const copy = res.clone();
          caches.open(VERSION).then((c) => c.put("/", copy));
          return res;
        })
        .catch(() => caches.match("/"))
    );
    return;
  }

  if (isImmutable(url)) {
    event.respondWith(
      caches.match(req).then(
        (hit) =>
          hit ||
          fetch(req).then((res) => {
            if (res.ok) {
              const copy = res.clone();
              caches.open(VERSION).then((c) => c.put(req, copy));
            }
            return res;
          })
      )
    );
    return;
  }

  // Everything else: stale-while-revalidate.
  event.respondWith(
    caches.match(req).then((hit) => {
      const network = fetch(req)
        .then((res) => {
          if (res.ok) {
            const copy = res.clone();
            caches.open(VERSION).then((c) => c.put(req, copy));
          }
          return res;
        })
        .catch(() => hit);
      return hit || network;
    })
  );
});

/* The app asks us to pre-cache a newly activated base map render. */
self.addEventListener("message", (event) => {
  const data = event.data || {};
  if (data.type === "cache-base-map" && data.url) {
    caches.open(VERSION).then((c) => c.add(data.url)).catch(() => {});
  }
});
