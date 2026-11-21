/* Service 5 print kit for SiteMonitor. GENERATED - do not edit by hand.
 *
 * Every report below is Service 5's own print template, copied verbatim from
 * 01_Rust_WC_DB/js (each section names its source file), so the printed
 * pages match Service 5 exactly: paper size, margins, headers, tables,
 * signatories, fonts and print CSS. The bridge at the end feeds the templates
 * SiteMonitor's records, which are stored in Service 5's own shape.
 * Nothing here stores personnel data in the browser.
 */
(function () {
if (window.S5Print) return;

// Service 5's global data arrays, filled by the bridge before each print.
var workersData = [];
var signatoriesData = [];
var visitationGroupsData = [];
var visitationsData = [];
var historyData = [];
var attendanceLog = [];
var assignmentLog = [];
// Service 5's group-change store, filled from SiteMonitor's group_changes records.
var groupChanges = [];
window.groupChangeStore = window.groupChangeStore || {
  get: function (date) { return Promise.resolve(groupChanges.filter(function (r) { return String(r.timestamp || r.date || '').slice(0, 10) === date; })); },
  getAll: function () { return Promise.resolve([groupChanges]); }
};
Object.defineProperty(window, 'workersData', { get: function () { return workersData; }, set: function (v) { workersData = v; }, configurable: true });
Object.defineProperty(window, 'signatoriesData', { get: function () { return signatoriesData; }, set: function (v) { signatoriesData = v; }, configurable: true });

// Service 5's toast, shown the same way (bottom centre, a few seconds).
function showToast(message) {
  var t = document.createElement('div');
  t.setAttribute('role', 'status');
  t.setAttribute('data-testid', 's5print-toast');
  t.textContent = String(message || '');
  t.style.cssText = 'position:fixed;left:50%;bottom:24px;transform:translateX(-50%);z-index:2147483647;background:#1f2937;color:#fff;padding:10px 16px;border-radius:8px;font:500 14px Inter,system-ui,sans-serif;box-shadow:0 8px 24px rgba(0,0,0,.2);max-width:90vw;';
  document.body.appendChild(t);
  setTimeout(function () { t.remove(); }, 4000);
}
window.showToast = window.showToast || showToast;
