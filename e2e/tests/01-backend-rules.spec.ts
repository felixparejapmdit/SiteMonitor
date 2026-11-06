/**
 * Phase 1 & 2 — shared models over the wire, the REST API and every
 * automated business rule (weighted progress, 99% lock, quiet day engine,
 * manpower ledger, breakdown deadline shifts, crons, role permissions).
 */
import { test, expect } from "@playwright/test";
import { api, resetDatabase, userFor, workItemByTitle } from "./helpers";

test.describe.configure({ mode: "serial" });
test.beforeAll(() => resetDatabase());

test("health and seeded estate (Phase 2 seed)", async ({ request }) => {
  const health = await (await request.get("/api/health")).json();
  expect(health.ok).toBe(true);

  const map = await (await request.get("/api/estate/map")).json();
  expect(map.base_map.is_active).toBe(true);
  expect(map.base_map_history.length).toBeGreaterThanOrEqual(2);
  const names = (nodes: any[]): string[] => nodes.flatMap((n) => [n.area.name, ...names(n.children)]);
  const all = names(map.areas);
  for (const n of ["Main Residence 1", "Main Residence 2", "Main Residence 3", "Guest Villa 1", "Guest Villa 4", "Grand Garden", "Outdoor Pool", "Power House", "Active Deepwell", "Water Filtration Plant", "Contractor Warehouse", "Contractor Workshop", "Multi-Purpose Hall (Parking Conversion)", "South Wing", "Master Suite"]) {
    expect(all, n).toContain(n);
  }
  const assets = (await (await request.get("/api/assets")).json()).assets;
  const byName = (n: string) => assets.find((a: any) => a.name === n);
  expect(byName("500 kW Generator")).toBeTruthy();
  expect(byName("2-Ton Mini Crane").mobility).toBe("MobileMachinery");
  const array = byName("Solar/Backup Generator Array");
  const batteries = assets.filter((a: any) => a.parent_asset_id === array.id);
  expect(batteries).toHaveLength(4);
  expect(batteries.every((b: any) => b.mobility === "ModularComponent")).toBe(true);
});

test("computed polygon health rolls up the infinite nesting tree", async ({ request }) => {
  const map = await (await request.get("/api/estate/map")).json();
  const find = (nodes: any[], name: string): any => {
    for (const n of nodes) {
      if (n.area.name === name) return n;
      const c = find(n.children, name);
      if (c) return c;
    }
  };
  // Elevator 2 needs service inside North Wing → Main Residence 1 needs attention.
  expect(find(map.areas, "North Wing").status).toBe("AttentionRequired");
  expect(find(map.areas, "Main Residence 1").status).toBe("AttentionRequired");
  expect(find(map.areas, "Old Tool Shed").status).toBe("Archived");
  expect(find(map.areas, "Power House").executive_summary).toContain("Battery array");
});

test("role-based access: the owner cannot do staff work", async ({ request }) => {
  const vip = await userFor(request, "VipClient");
  const wall = await workItemByTitle(request, "Retaining Wall Leveling");
  const detail = await api(request, vip).get(`/work-items/${wall.id}`);
  const res = await api(request, vip).post(`/phases/${detail.phases[3].id}/complete`, {});
  expect(res.status()).toBe(403);
  expect((await res.json()).message).toContain("Switch role");
  const anon = await request.post(`/api/phases/${detail.phases[3].id}/complete`, { data: {} });
  expect(anon.status()).toBe(401);
});

test("weighted progress + the 99% handover lock + owner sign-off", async ({ request }) => {
  const monitor = await userFor(request, "SiteMonitor", "Jun");
  const vip = await userFor(request, "VipClient");
  const wall = await workItemByTitle(request, "Retaining Wall Leveling");
  expect(wall.calculated_progress).toBe(80);
  const detail = await api(request, monitor).get(`/work-items/${wall.id}`);
  const last = detail.phases.find((p: any) => !p.is_completed);
  expect(last.weight_percentage).toBe(20);

  const res = await api(request, monitor).post(`/phases/${last.id}/complete`, {});
  expect(res.ok()).toBeTruthy();
  const update = await res.json();
  expect(update.item.calculated_progress).toBe(99);
  expect(update.item.status).toBe("PendingSignOff");
  expect(update.handover_created).toBe(true);

  const after = await api(request, vip).get(`/work-items/${wall.id}`);
  expect(after.handover).toBeTruthy();
  expect(after.handover.total_delay_days).toBe(1);
  expect(after.handover.delay_audit_summary[0]).toContain("+1 Day: Equipment Failure");
  expect(after.timeline.some((t: any) => t.is_milestone && t.title.includes("Backfill"))).toBe(true);

  const signed = await api(request, vip).post(`/work-items/${wall.id}/sign-off`);
  expect(signed.ok()).toBeTruthy();
  expect((await signed.json()).client_signed_off).toBe(true);
  const final = await workItemByTitle(request, "Retaining Wall Leveling");
  expect(final.calculated_progress).toBe(100);
  expect(final.status).toBe("CompletedAndArchived");
});

test("a final photo note keeps the project at 99% until it is resolved", async ({ request }) => {
  const vip = await userFor(request, "VipClient");
  const admin = await userFor(request, "OfficeAdmin");
  const gazebo = await workItemByTitle(request, "Rose Garden Gazebo Build");
  expect(gazebo.status).toBe("PendingSignOff");
  const detail = await api(request, vip).get(`/work-items/${gazebo.id}`);
  const after = detail.photos.find((p: any) => p.photo.image_url === detail.handover.after_photo_url);
  const pin = await api(request, vip).post(`/photos/${after.photo.id}/callouts`, { x_percent: 40, y_percent: 70, comment: "Move this debris bin out of the corner" });
  expect(pin.ok()).toBeTruthy();
  const callout = await pin.json();

  const blocked = await api(request, vip).post(`/work-items/${gazebo.id}/sign-off`);
  expect(blocked.status()).toBe(400);
  expect((await workItemByTitle(request, "Rose Garden Gazebo Build")).calculated_progress).toBe(99);

  const resolved = await api(request, admin).put(`/callouts/${callout.id}/resolve`, { reply: "Bin moved behind the hedge." });
  expect((await resolved.json()).status).toBe("Resolved");
  expect((await api(request, vip).post(`/work-items/${gazebo.id}/sign-off`)).ok()).toBeTruthy();
});

test("manpower: morning allocation and mid-day transfer write the ledger", async ({ request }) => {
  const admin = await userFor(request, "OfficeAdmin");
  const parking = await workItemByTitle(request, "Parking Conversion to Multi-Purpose Hall");
  const suite = await workItemByTitle(request, "Master Suite Refit");
  const res = await api(request, admin).post("/manpower/reallocate", {
    allocations: [{ work_item_id: suite.id, headcount: 9 }],
    transfers: [{ source_work_item_id: parking.id, destination_work_item_id: suite.id, headcount: 4, reason: "Window glass arrived" }],
    reason: "Morning crew allocation",
  });
  expect(res.ok(), await res.text()).toBeTruthy();
  expect((await workItemByTitle(request, "Master Suite Refit")).active_worker_count).toBe(13);
  expect((await workItemByTitle(request, "Parking Conversion to Multi-Purpose Hall")).active_worker_count).toBe(10);
  const detail = await api(request, admin).get(`/work-items/${suite.id}`);
  const transfer = detail.ledger.find((l: any) => l.source_work_item_id === parking.id);
  expect(transfer.headcount_delta).toBe(4);
  expect(transfer.resulting_headcount).toBe(13);

  const tooMany = await api(request, admin).post("/manpower/reallocate", {
    allocations: [],
    transfers: [{ source_work_item_id: parking.id, destination_work_item_id: suite.id, headcount: 99, reason: "x" }],
  });
  expect(tooMany.status()).toBe(400);
});

test("breakdown with deadline impact shifts the deadline and logs an equipment-failure delay", async ({ request }) => {
  const monitor = await userFor(request, "SiteMonitor", "Jun");
  const assets = (await (await request.get("/api/assets")).json()).assets;
  const backhoe = assets.find((a: any) => a.name === "Backhoe");
  const parking = await workItemByTitle(request, "Parking Conversion to Multi-Purpose Hall");
  const res = await api(request, monitor).post(`/assets/${backhoe.id}/report-issue`, {
    health: "BrokenDown",
    impact: "FullWorkStoppage",
    note: "Hydraulic hose burst.",
    repair_eta: null,
    work_item_id: parking.id,
    days_delayed: 3,
  });
  expect(res.ok(), await res.text()).toBeTruthy();
  const updated = await workItemByTitle(request, "Parking Conversion to Multi-Purpose Hall");
  expect(new Date(updated.current_target_deadline).getTime() - new Date(parking.current_target_deadline).getTime()).toBe(3 * 86_400_000);
  expect(updated.status).toBe("Halted");
  const detail = await api(request, monitor).get(`/work-items/${parking.id}`);
  expect(detail.timeline.some((t: any) => t.delay_category === "EquipmentFailure" && t.days_delayed === 3)).toBe(true);

  // Repairing the machine resumes the stopped work.
  expect((await api(request, monitor).post(`/assets/${backhoe.id}/mark-repaired`)).ok()).toBeTruthy();
  expect((await workItemByTitle(request, "Parking Conversion to Multi-Purpose Hall")).status).toBe("InProgress");
});

test("context-aware movement carries nested batteries with their parent", async ({ request }) => {
  const admin = await userFor(request, "OfficeAdmin");
  const { assets } = await (await request.get("/api/assets")).json();
  const array = assets.find((a: any) => a.name === "Solar/Backup Generator Array");
  const battery = assets.find((a: any) => a.name === "12.8V 200Ah LiFePO4 Battery #3");
  const map = await (await request.get("/api/estate/map")).json();
  const workshop = map.areas.find((n: any) => n.area.name === "Contractor Workshop").area.id;

  // A nested part can't be moved on its own…
  expect((await api(request, admin).post(`/assets/${battery.id}/relocate`, { to_area_id: workshop })).status()).toBe(400);
  // …detach it, then move it to the workshop while the array stays put.
  expect((await api(request, admin).post(`/assets/${battery.id}/link`, { parent_asset_id: null })).ok()).toBeTruthy();
  expect((await api(request, admin).post(`/assets/${battery.id}/relocate`, { to_area_id: workshop })).ok()).toBeTruthy();
  const after = (await (await request.get("/api/assets")).json());
  expect(after.assets.find((a: any) => a.id === battery.id).current_area_id).toBe(workshop);
  expect(after.assets.find((a: any) => a.id === array.id).current_area_id).not.toBe(workshop);
  expect(after.recent_transfers[0].asset_id).toBe(battery.id);
});

test("quiet day engine flags noisy & utility-cut work and pauses those jobs", async ({ request }) => {
  const vip = await userFor(request, "VipClient");
  const agenda = await (await request.get("/api/agenda")).json();
  const compaction = agenda.items.find((i: any) => i.title === "Paving base compaction");
  const start = new Date(new Date(compaction.start_time).getTime() - 3600_000).toISOString();
  const end = new Date(new Date(compaction.end_time).getTime() + 8 * 3600_000).toISOString();
  const res = await api(request, vip).post("/agenda/quiet-day", { title: "Weekend Estate Gala", description: "", start_time: start, end_time: end });
  expect(res.ok(), await res.text()).toBeTruthy();
  const body = await res.json();
  expect(body.confirmation).toContain("All heavy machinery (cranes, backhoes) and utility interruptions are automatically halted");
  const flaggedTitles = body.flagged_items.map((f: any) => f.title);
  expect(flaggedTitles).toContain("Paving base compaction");
  expect(flaggedTitles).toContain("Crane lift — wall capstones");
  expect(body.flagged_items.every((f: any) => f.intrusion_level === "NoiseHeavy" || f.intrusion_level === "UtilityCut")).toBe(true);
  expect((await workItemByTitle(request, "Parking Conversion to Multi-Purpose Hall")).status).toBe("Halted");

  // Staff reschedule after the occasion → flag clears and work resumes.
  const admin = await userFor(request, "OfficeAdmin");
  const later = new Date(new Date(end).getTime() + 24 * 3600_000);
  const moved = await api(request, admin).put(`/agenda/${compaction.id}/reschedule`, {
    start_time: later.toISOString(),
    end_time: new Date(later.getTime() + 4 * 3600_000).toISOString(),
  });
  expect((await moved.json()).is_flagged_conflict).toBe(false);
  expect((await workItemByTitle(request, "Parking Conversion to Multi-Purpose Hall")).status).toBe("InProgress");
});

test("equipment double-booking is detected for the crane", async ({ request }) => {
  const agenda = await (await request.get("/api/agenda")).json();
  expect(agenda.equipment_conflicts.some((c: any) => c.asset_name === "2-Ton Mini Crane" && c.area_ids.length === 2)).toBe(true);
});

test("draft → publish → respond decision pipeline", async ({ request }) => {
  const monitor = await userFor(request, "SiteMonitor", "Ana");
  const admin = await userFor(request, "OfficeAdmin");
  const vip = await userFor(request, "VipClient");
  const suite = await workItemByTitle(request, "Master Suite Refit");
  const draft = await (await api(request, monitor).post("/decisions", {
    work_item_id: suite.id, category: "DesignDecision", urgency: "High", title: "Walnut veneer batch varies in colour",
    issue_description: "Two shades in the delivery.", mitigation_action_taken: "Paused installation.", photo_data_url: null,
  })).json();
  expect(draft.status).toBe("DraftInAdminQueue");
  expect((await api(request, vip).get("/decisions")).some((d: any) => d.id === draft.id)).toBe(false);

  const published = await (await api(request, admin).put(`/decisions/${draft.id}/publish`, {
    title: "Choose the walnut shade", issue_description: "The supplier sent two shades.", mitigation_action_taken: "Installation paused one day.",
    category: "DesignDecision", urgency: "High", option_a_label: "Use the lighter shade - No Delay", option_b_label: "Re-order matched batch - +6 Days Delay",
  })).json();
  expect(published.status).toBe("PublishedPendingClient");
  const answered = await (await api(request, vip).post(`/decisions/${draft.id}/respond`, { response: "ApproveOptionA" })).json();
  expect(answered.status).toBe("ApprovedOptionA");
  expect(answered.resolved_at).toBeTruthy();
});

test("automation jobs: midnight reset, deadline monitor, VIP digest", async ({ request }) => {
  const admin = await userFor(request, "OfficeAdmin");
  const monitor = await userFor(request, "SiteMonitor");
  const vip = await userFor(request, "VipClient");

  const reset = await (await api(request, admin).post("/admin/jobs/midnight-roster-reset/run")).json();
  expect(reset.summary).toContain("Cleared");
  const items = await (await request.get("/api/work-items")).json();
  expect(items.every((w: any) => w.active_worker_count === 0)).toBe(true);

  const deadline = await (await api(request, admin).post("/admin/jobs/deadline-monitor/run")).json();
  expect(deadline.summary).toMatch(/\d+ staff deadline warnings/);
  const staffNotices = await api(request, monitor).get("/notifications");
  expect(staffNotices.some((n: any) => n.kind === "DeadlineWarning" && n.body.includes("Reallocate manpower?"))).toBe(true);

  const digest = await (await api(request, admin).post("/admin/jobs/vip-daily-digest/run")).json();
  expect(digest.summary).toMatch(/digest/i);
  const vipNotices = await api(request, vip).get("/notifications");
  expect(vipNotices.some((n: any) => n.kind === "DailyDigest")).toBe(true);

  // Monitors can't run automations.
  expect((await api(request, monitor).post("/admin/jobs/deadline-monitor/run")).status()).toBe(403);
});
