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

// Inside SiteMonitor's page frames, Ctrl+K opens SiteMonitor's quick search.
if (window.parent !== window) {
  document.addEventListener('keydown', function (e) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') { e.preventDefault(); window.parent.postMessage('s5-quick-search', location.origin); }
  });
}

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

// ===== core.js helpers - verbatim from 01_Rust_WC_DB/js/core.js =====
function esc(str) { return String(str == null ? '' : str).replace(/[&<>"']/g, function(m) { return {'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[m]; }); }

function parseDateSafe(input) {
  if (input === null || input === undefined || input === '') return null;
  if (input instanceof Date) {
    return isNaN(input.getTime()) ? null : input;
  }
  if (typeof input === 'number') {
    const d = new Date(input);
    return isNaN(d.getTime()) ? null : d;
  }
  if (typeof input !== 'string') return null;

  const trimmed = input.trim();
  if (!trimmed) return null;

  // Check pure YYYY-MM-DD (component-based local date instantiation)
  const ymdMatch = trimmed.match(/^(\d{4})-(\d{2})-(\d{2})$/);
  if (ymdMatch) {
    const y = parseInt(ymdMatch[1], 10);
    const m = parseInt(ymdMatch[2], 10);
    const d = parseInt(ymdMatch[3], 10);
    if (m >= 1 && m <= 12 && d >= 1 && d <= 31) {
      const dt = new Date(y, m - 1, d);
      return isNaN(dt.getTime()) ? null : dt;
    }
    return null;
  }

  // Explicit component-based parsing for MM/DD/YYYY or M/D/YYYY
  const mdyMatch = trimmed.match(/^(\d{1,2})\/(\d{1,2})\/(\d{4})$/);
  if (mdyMatch) {
    const m = parseInt(mdyMatch[1], 10);
    const d = parseInt(mdyMatch[2], 10);
    const y = parseInt(mdyMatch[3], 10);
    if (m >= 1 && m <= 12 && d >= 1 && d <= 31) {
      const dt = new Date(y, m - 1, d);
      return isNaN(dt.getTime()) ? null : dt;
    }
    return null;
  }

  // Normalize space separator to 'T' for strict Safari WebKit ISO parsing
  // e.g., "2026-09-21 14:30:00" -> "2026-09-21T14:30:00"
  let normalized = trimmed;
  if (/^\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}/.test(trimmed)) {
    normalized = trimmed.replace(' ', 'T');
  }

  const dt = new Date(normalized);
  if (!isNaN(dt.getTime())) return dt;

  // Fallback for YYYY/MM/DD
  if (/^\d{4}\/\d{2}\/\d{2}/.test(trimmed)) {
    const slashNormalized = trimmed.replace(/\//g, '-').replace(' ', 'T');
    const dt2 = new Date(slashNormalized);
    if (!isNaN(dt2.getTime())) return dt2;
  }

  return null;
}

function formatDateLocal(date) {
  if (window.DateService && date instanceof Date) {
    return window.DateService.getLocalCalendarDate(date);
  }
  const d = parseDateSafe(date);
  if (!d) return '';
  const mm = String(d.getMonth() + 1).padStart(2, '0');
  const dd = String(d.getDate()).padStart(2, '0');
  return `${d.getFullYear()}-${mm}-${dd}`;
}

function todayLocal() {
  return window.DateService ? window.DateService.getLocalCalendarDate() : formatDateLocal(new Date());
}

function getAge(birthdayStr) {
  if (!birthdayStr) return 0;
  const bd = parseDateSafe(birthdayStr);
  if (!bd) return 0;
  const today = new Date();
  let age = today.getFullYear() - bd.getFullYear();
  const mDiff = today.getMonth() - bd.getMonth();
  if (mDiff < 0 || (mDiff === 0 && today.getDate() < bd.getDate())) age--;
  return age;
}

function getFormattedDate(date) {
  const d = parseDateSafe(date) || new Date();
  const mm = String(d.getMonth() + 1).padStart(2, '0');
  const dd = String(d.getDate()).padStart(2, '0');
  const yyyy = d.getFullYear();
  return `${mm}/${dd}/${yyyy}`;
}

function displayDate(dateStr) {
  if (!dateStr) return '\u2014';
  const d = parseDateSafe(dateStr);
  if (!d) return String(dateStr);
  return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
}

const WORKER_DEPARTMENTS = [
  'Job-site Construction',
  'Woodland Shops',
  'Garden',
  'Residential Only',
  'Electrical Rm',
  'Water System',
  'Woodland Office'
];

function inferWorkerDepartment(worker) {
  if (!worker) return 'Job-site Construction';
  if (worker.department && WORKER_DEPARTMENTS.includes(worker.department)) {
    return worker.department;
  }

  const tradeStr = (worker.trade || '').toLowerCase();
  const tradeGroupStr = (worker.tradeGroup || '').toLowerCase();
  const areaStr = (worker.area || '').toLowerCase();
  const projectStr = (worker.project || '').toLowerCase();
  const housingStr = (worker.visitationHousing || '').toLowerCase();

  // 1. Woodland Shops (includes Carpenter Shop as a sub-specialty)
  if (tradeStr.includes('carpent') || tradeGroupStr.includes('carpent') ||
      tradeStr.includes('paint') || tradeGroupStr.includes('paint') ||
      tradeStr.includes('tinsmith') || tradeStr.includes('tin') || tradeGroupStr.includes('tinsmith') ||
      tradeStr.includes('weld') || tradeStr.includes('steel') || tradeGroupStr.includes('weld') || tradeGroupStr.includes('steel') ||
      tradeStr.includes('powder') || tradeStr.includes('coating') || tradeGroupStr.includes('powder') ||
      areaStr.includes('woodland shop') || projectStr.includes('woodland shop') ||
      areaStr.includes('carpenter') || projectStr.includes('carpenter')) {
    return 'Woodland Shops';
  }

  // 3. Garden
  if (areaStr.includes('garden') || projectStr.includes('garden') || tradeStr.includes('garden') || tradeStr.includes('landscap')) {
    return 'Garden';
  }

  // 4. Electrical Rm
  if (areaStr.includes('elect') || projectStr.includes('elect') || tradeStr.includes('electric') || tradeGroupStr.includes('electric')) {
    return 'Electrical Rm';
  }

  // 5. Water System
  if (areaStr.includes('water') || projectStr.includes('water') || tradeStr.includes('plumb') || tradeGroupStr.includes('plumb')) {
    return 'Water System';
  }

  // 6. Woodland Office
  if (areaStr.includes('office') || projectStr.includes('office') || tradeStr.includes('kalihim') || tradeStr.includes('clerk') || tradeStr.includes('admin')) {
    return 'Woodland Office';
  }

  // 7. Residential Only
  if (areaStr.includes('residential') || projectStr.includes('residential') || housingStr.includes('residential only') || tradeStr.includes('domestic') || tradeStr.includes('housekeep')) {
    return 'Residential Only';
  }

  // Default: Job-site Construction
  return 'Job-site Construction';
}

function getWorkerStatusForDate(worker, targetDate) {
  if (!worker) return { status: '', reason: '' };

  const liveStatus = String(worker.dailyStatus || '').trim();
  const liveReason = String(worker.absentReason || '').trim();

  if (!targetDate) {
    return { status: liveStatus, reason: liveReason };
  }

  // Normalize targetDate to YYYY-MM-DD
  let targetIso = '';
  if (targetDate instanceof Date && !isNaN(targetDate)) {
    targetIso = window.DateService ? window.DateService.getLocalCalendarDate(targetDate) : formatDateLocal(targetDate);
  } else if (typeof targetDate === 'string') {
    const s = targetDate.trim();
    if (/^\d{4}-\d{2}-\d{2}$/.test(s)) {
      targetIso = s;
    } else if (/^\d{1,2}\/\d{1,2}\/\d{4}$/.test(s)) {
      const parts = s.split('/');
      targetIso = `${parts[2]}-${parts[0].padStart(2, '0')}-${parts[1].padStart(2, '0')}`;
    } else {
      const parsed = new Date(s);
      if (!isNaN(parsed.getTime())) {
        targetIso = window.DateService ? window.DateService.getLocalCalendarDate(parsed) : formatDateLocal(parsed);
      }
    }
  }

  // Check if targetIso corresponds to active shift (today or tomorrow default)
  const todayIso = window.DateService ? window.DateService.getLocalCalendarDate() : todayLocal();
  const tomorrow = new Date();
  tomorrow.setDate(tomorrow.getDate() + 1);
  const tomorrowIso = window.DateService ? window.DateService.getLocalCalendarDate(tomorrow) : formatDateLocal(tomorrow);

  // If targetIso is today or tomorrow (when preparing next day's slips without prior recorded leave),
  // check if there is an explicit future/scheduled attendanceHistory entry first.
  if (Array.isArray(worker.attendanceHistory) && worker.attendanceHistory.length > 0 && targetIso) {
    const match = worker.attendanceHistory.find(entry => entry && entry.date === targetIso);
    if (match && match.newStatus) {
      return {
        status: String(match.newStatus).trim(),
        reason: String(match.reason || '').trim()
      };
    }
  }

  // Check if worker has a staged tomorrowAssignment matching targetIso
  if (worker.tomorrowAssignment && worker.tomorrowAssignment.date === targetIso && worker.tomorrowAssignment.dailyStatus) {
    return {
      status: String(worker.tomorrowAssignment.dailyStatus).trim(),
      reason: String(worker.tomorrowAssignment.absentReason || '').trim()
    };
  }

  // If target date is today or tomorrow (the active dispatch cycle), default to live status
  if (!targetIso || targetIso === todayIso || targetIso === tomorrowIso) {
    return { status: liveStatus, reason: liveReason };
  }

  // Check historical snapshots if older date
  if (typeof historyData !== 'undefined' && Array.isArray(historyData)) {
    const snap = historyData.find(s => s && s.dateStr === targetIso);
    if (snap && Array.isArray(snap.data)) {
      const targetANum = worker.aNumber != null ? String(worker.aNumber) : null;
      const match = snap.data.find(w =>
        (targetANum && w.aNumber != null && String(w.aNumber) === targetANum) ||
        (w.name === worker.name)
      );
      if (match && match.dailyStatus) {
        return {
          status: String(match.dailyStatus).trim(),
          reason: String(match.absentReason || '').trim()
        };
      }
    }
  }

  // Default fallback to live status
  return { status: liveStatus, reason: liveReason };
}

function checkJobOrderEligibility(worker, targetDate) {
  if (!worker || worker.isArchived) {
    return { eligible: false, reason: 'Worker is archived or invalid' };
  }

  // 1. Personal profile preference check
  // Supports legacy worker.jOrder ('YES'/'NO') and canonical worker.issueJobOrder (boolean).
  // Volunteer workers marked as residential only are issued job orders by default unless explicitly disabled.
  const isJODisabled = worker.jOrder === 'NO' || worker.issueJobOrder === false || (worker.personalInfo && worker.personalInfo.issueJobOrder === false);
  if (isJODisabled) {
    return { eligible: false, reason: "'Issue a job order' is turned off in profile" };
  }

  // 2. Fetch the attendance/status record for the target date
  const statusInfo = getWorkerStatusForDate(worker, targetDate);
  const statusLower = (statusInfo.status || '').trim().toLowerCase();
  const reasonLower = (statusInfo.reason || '').trim().toLowerCase();

  // Excluded status values
  const excludedStatuses = ['pass', 'passes', 'on pass', 'day-off', 'day off', 'dayoff', 'off', 'absent'];

  if (excludedStatuses.includes(statusLower)) {
    return { eligible: false, reason: `Worker is marked ${statusInfo.status || 'inactive'}` };
  }

  // Secondary check: reason contains day-off, on pass, or leave
  if (reasonLower.includes('day-off') || reasonLower.includes('day off') || reasonLower.includes('dayoff') ||
      reasonLower.includes('on pass') || reasonLower.includes('passes') || reasonLower === 'pass') {
    return { eligible: false, reason: `Worker is on leave (${statusInfo.reason})` };
  }

  return { eligible: true, reason: '' };
}

function getWorkerClearanceState(worker, targetDate) {
  return { status: 'CLEARED_FULL', label: 'Fit for Full Duty', color: 'green', isHold: false };
}

function isEligibleForJobOrder(worker, targetDate) {
  return checkJobOrderEligibility(worker, targetDate).eligible;
}

const tradeThemeColors = {
  'carpenter': { bg: '#ebf3f5', text: '#1c5f82' },
  'mason': { bg: '#faebd7', text: '#d9730d' },
  'electrician': { bg: '#fdf5e2', text: '#9a6800' },
  'painter': { bg: '#f4f0f7', text: '#6e4a8a' },
  'gardener': { bg: '#edf3ec', text: '#2d6b49' },
  'landscaper': { bg: '#edf3ec', text: '#2d6b49' },
  'tinsmith': { bg: '#ebf3f5', text: '#2b79a0' },
  'steelman': { bg: '#f4f0f7', text: '#6e4a8a' },
  'driver': { bg: '#f7eaf1', text: '#a03876' },
  'helper': { bg: '#f1f1ef', text: '#59574f' },
  'admin': { bg: '#f1f1ef', text: '#59574f' },
  'office': { bg: '#f7eaf1', text: '#a03876' },
  'default': { bg: '#f1f1ef', text: '#59574f' }
};

function getTradeStyle(trade) {
  if (!trade) return tradeThemeColors.default;
  const t = trade.toLowerCase().replace('*', '').trim();
  for (const key in tradeThemeColors) {
    if (t.includes(key)) return tradeThemeColors[key];
  }
  return tradeThemeColors.default;
}

window.getCombinedMedicalCondition = function (worker) {
    if (!worker) return 'None / Clear';
    const chronic = worker.medical || '';
    if (window._activeHealthEventsCache && window._activeHealthEventsCache[String(worker.aNumber)]) {
        const ev = window._activeHealthEventsCache[String(worker.aNumber)];
        const activeStr = `Active: ${ev.diagnosis} (since ${ev.dayZero})`;
        return chronic ? `${chronic} | ${activeStr}` : activeStr;
    }
    return chronic || 'None / Clear';
}

window.waitForImagesThenCall = function(win, callback) {
  var imgs;
  try { imgs = Array.prototype.slice.call(win.document.images || []); } catch (e) { imgs = []; }
  var pending = imgs.filter(function(img) { return !img.complete; });
  if (!pending.length) { callback(); return; }
  var done = false;
  var finish = function() { if (done) return; done = true; callback(); };
  Promise.all(pending.map(function(img) {
    return new Promise(function(resolve) {
      img.addEventListener('load', resolve, { once: true });
      img.addEventListener('error', resolve, { once: true });
    });
  })).then(finish);
  setTimeout(finish, 2000); // safety net if an image never settles
}

window.safePrint = function(elementIdOrHtml, options) {
  // Prints via a hidden same-page <iframe> rather than window.open() to avoid
  // popup blocker issues and keep print styling scoped cleanly to the document.
  // options: { title: string, orientation: 'portrait'|'landscape' }
  var opts = options || {};

  // Universal print reset: injected into every safePrint document.
  // Eliminates: Blink shrink-to-fit downscale (fluid width), missing thead
  // repetition, row glyph-slicing, toner-stripping, cross-browser margin variance.
  var UNIVERSAL_PRINT_RESET = [
    '<style id="__safe_print_reset">',
    '*, *::before, *::after { box-sizing: border-box !important; }',
    '@page {',
    '  /* size:auto lets browser adapt to the actual selected paper tray (A4 or Letter). */',
    '  /* 8mm/10mm provides standard printer driver clearance preventing clippings & bleeds. */',
    '  size: auto;',
    '  margin: 8mm 10mm;',
    '}',
    'html, body {',
    '  margin: 0 !important;',
    '  padding: 0 !important;',
    '  background: #ffffff !important;',
    '  overflow: visible !important;',
    '  width: 100% !important;',
    '  max-width: 100% !important;',
    '  height: auto !important;',
    '  -webkit-print-color-adjust: exact !important;',
    '  print-color-adjust: exact !important;',
    '  color-adjust: exact !important;',
    '}',
    '/* Fluid container: never exceed printable page boundary, prevents shrink-to-fit */',
    '.print-container, .print-wrap {',
    '  width: 100% !important;',
    '  max-width: 100% !important;',
    '  margin: 0 auto !important;',
    '  box-sizing: border-box !important;',
    '}',
    '/* Table pagination guarantees */',
    'table {',
    '  width: 100% !important;',
    '  max-width: 100% !important;',
    '  min-width: 0 !important;',
    '  border-collapse: collapse !important;',
    '  break-inside: auto !important;',
    '}',
    'th, td {',
    '  word-break: break-word !important;',
    '  overflow-wrap: break-word !important;',
    '}',
    'thead {',
    '  /* Forces table header to repeat at top of every split page */',
    '  display: table-header-group !important;',
    '  break-inside: avoid !important;',
    '}',
    'tfoot {',
    '  display: table-footer-group !important;',
    '  break-inside: avoid !important;',
    '}',
    'tr {',
    '  /* Prevents row text glyphs from being sliced across page seams */',
    '  break-inside: avoid !important;',
    '  page-break-inside: avoid !important;',
    '}',
    '/* Prevent orphaned headings above empty page-bottom space */',
    'h1, h2, h3, h4 {',
    '  break-after: avoid !important;',
    '  page-break-after: avoid !important;',
    '}',
    '/* Card, KPI block, and signature isolation */',
    '.print-card, .kpi, .kpi-card, .section, .summary-box, .signoff-box, .info-box {',
    '  break-inside: avoid !important;',
    '  page-break-inside: avoid !important;',
    '}',
    'img {',
    '  max-width: 100% !important;',
    '  break-inside: avoid !important;',
    '  image-rendering: -webkit-optimize-contrast;',
    '}',
    '</style>'
  ].join('\n');

  function injectReset(html) {
    // Inject universal reset immediately after <head> (or at top if no head)
    if (html.indexOf('</head>') !== -1) {
      return html.replace('</head>', UNIVERSAL_PRINT_RESET + '</head>');
    } else if (html.indexOf('<head>') !== -1) {
      return html.replace('<head>', '<head>' + UNIVERSAL_PRINT_RESET);
    } else {
      return UNIVERSAL_PRINT_RESET + html;
    }
  }

  function stripInlineMaxWidth(html) {
    // Strip inline max-width and min-width px values that cause Blink shrink-to-fit downscale or clipping.
    // Replaces `max-width:850px` style patterns in injected HTML strings with fluid.
    return html.replace(/max-width\s*:\s*\d+px/gi, 'max-width:100%')
               .replace(/min-width\s*:\s*\d+px/gi, 'min-width:0');
  }

  function printViaHiddenFrame(html) {
    var existing = document.getElementById('__safe_print_frame');
    if (existing) existing.remove();

    // Patch the HTML before writing to iframe
    var patchedHtml = stripInlineMaxWidth(injectReset(html));

    var iframe = document.createElement('iframe');
    iframe.id = '__safe_print_frame';
    iframe.style.cssText = 'position:fixed;right:0;bottom:0;width:0;height:0;border:0;visibility:hidden;';
    document.body.appendChild(iframe);

    try {
      var doc = iframe.contentWindow.document;
      doc.open();
      doc.write(patchedHtml);
      doc.close();
    } catch (e) {
      console.warn('Print failed:', e);
      if (typeof showErrorToast === 'function') showErrorToast('Failed to open print preview.');
      iframe.remove();
      return;
    }

    window.waitForImagesThenCall(iframe.contentWindow, function() {
      try {
        iframe.contentWindow.focus();
        iframe.contentWindow.onafterprint = function() { iframe.remove(); };
        iframe.contentWindow.print();
      } catch (e) {
        console.warn('Print failed:', e);
        if (typeof showErrorToast === 'function') showErrorToast('Failed to print.');
        iframe.remove();
      }
    });
  }

  var cleanHtml = typeof elementIdOrHtml === 'string' ? elementIdOrHtml.trim() : '';
  if (cleanHtml && (cleanHtml.startsWith('<') || cleanHtml.indexOf('<!DOCTYPE') !== -1 || cleanHtml.indexOf('<html') !== -1)) {
    printViaHiddenFrame(cleanHtml);
    return;
  }
  if (cleanHtml) {
    var targetEl = document.getElementById(cleanHtml);
    if (targetEl) {
      printViaHiddenFrame(targetEl.outerHTML);
      return;
    }
  }
  setTimeout(function() { try { window.print(); } catch (e) { console.warn('Print blocked:', e); } }, 200);
}

function queryWorkerSnapshots(workerName) {
  if (!workerName) return [];
  const clean = String(workerName).trim().toLowerCase();
  const liveWorker = (typeof workersData !== 'undefined' && Array.isArray(workersData))
    ? workersData.find(w => 
        (w.name && w.name.trim().toLowerCase() === clean) ||
        (w.aNumber != null && String(w.aNumber).trim() === clean) ||
        (w.workerId && String(w.workerId).trim() === clean)
      )
    : null;
  const targetANum = liveWorker ? String(liveWorker.aNumber || '') : (clean.match(/^\d+$/) ? clean : null);

  // PRIMARY: Use embedded attendanceHistory on the worker object
  if (liveWorker && Array.isArray(liveWorker.attendanceHistory) && liveWorker.attendanceHistory.length > 0) {
    const results = liveWorker.attendanceHistory.map(entry => {
      let tsNum = 0;
      if (typeof entry.timestamp === 'number' && !isNaN(entry.timestamp)) {
        tsNum = entry.timestamp;
      } else if (entry.timestamp) {
        tsNum = new Date(entry.timestamp).getTime() || 0;
      } else if (entry.date) {
        tsNum = new Date(entry.date + 'T08:00:00').getTime() || 0;
      }

      // Self-heal inverted month/day on the fly (e.g. 2026-02-10 -> 2026-10-02)
      let resolvedDate = entry.date;
      if (resolvedDate) {
        if (resolvedDate === '2026-02-10' || resolvedDate === '02/10/2026') {
          resolvedDate = '2026-10-02';
          entry.date = '2026-10-02';
          tsNum = new Date(2026, 9, 2, 16, 0, 0).getTime();
          if (entry.recordId && entry.recordId.includes('2026-02-10')) {
            entry.recordId = entry.recordId.replace('2026-02-10', '2026-10-02');
          }
          if (entry.compositeId && entry.compositeId.includes('2026-02-10')) {
            entry.compositeId = entry.compositeId.replace('2026-02-10', '2026-10-02');
          }
        } else {
          const dMatch = String(resolvedDate).match(/^(\d{4})-(\d{2})-(\d{2})$/);
          if (dMatch) {
            const dy = parseInt(dMatch[1], 10);
            const dm = parseInt(dMatch[2], 10);
            const dd = parseInt(dMatch[3], 10);
            if (dy === 2026 && dd === 10 && dm >= 1 && dm <= 9) {
              resolvedDate = `2026-10-${String(dm).padStart(2, '0')}`;
              entry.date = resolvedDate;
            }
          }
        }
      }

      const st = entry.status || entry.newStatus || entry.dailyStatus || 'Present';
      return {
        date: resolvedDate || (entry.timestamp ? String(entry.timestamp).slice(0, 10) : ''),
        timestamp: tsNum,
        dailyStatus: st,
        absentReason: entry.reason || entry.notes || '',
        area: entry.area || liveWorker.area || '',
        project: entry.project || liveWorker.project || '',
        specificTask: entry.specificTask || liveWorker.specificTask || '',
        type: entry.type || 'duty',
        shift: entry.shift || 'Day',
        verifiedBy: entry.verifiedBy || entry.encoder || 'System'
      };
    });
    results.sort((a, b) => b.timestamp - a.timestamp);
    return results;
  }

  // FALLBACK: Scan historyData daily snapshots (pre-migration records)
  if (!historyData || historyData.length === 0) return [];
  const results = [];

  historyData.forEach(snap => {
    if (snap.data && Array.isArray(snap.data)) {
      const match = snap.data.find(w =>
          (targetANum && w.aNumber != null && String(w.aNumber) === targetANum) ||
          (w.name && w.name.trim().toLowerCase() === clean)
      );
      if (match) {
        let snapDate = snap.dateStr || '';
        if (snapDate === '2026-02-10' || snapDate === '02/10/2026') {
          snapDate = '2026-10-02';
        }
        let tsNum = 0;
        if (typeof snap.timestamp === 'number' && !isNaN(snap.timestamp)) {
          tsNum = snap.timestamp;
        } else if (snap.timestamp) {
          tsNum = new Date(snap.timestamp).getTime() || 0;
        } else if (snapDate) {
          tsNum = new Date(snapDate + 'T08:00:00').getTime() || 0;
        }
        if (snap.dateStr === '2026-02-10' || snap.dateStr === '02/10/2026') {
          tsNum = new Date(2026, 9, 2, 16, 0, 0).getTime();
        }
        results.push({
          date: snapDate,
          timestamp: tsNum,
          dailyStatus: match.dailyStatus || 'Present',
          absentReason: match.absentReason || '',
          area: match.area || '',
          project: match.project || '',
          specificTask: match.specificTask || '',
          type: 'duty'
        });
      }
    }
  });
  results.sort((a, b) => b.timestamp - a.timestamp);
  return results;
}

function todayStr() {
  return toDateStr(new Date());
}

window.areaPriorityComparator = function(nameA, nameB) {
  var a = nameA.toUpperCase();
  var b = nameB.toUpperCase();
  var rankA = a.indexOf('CC') !== -1 ? 0 : a.indexOf('STATION 1') !== -1 ? 1 : a.indexOf('MALL') !== -1 ? 2 : 3;
  var rankB = b.indexOf('CC') !== -1 ? 0 : b.indexOf('STATION 1') !== -1 ? 1 : b.indexOf('MALL') !== -1 ? 2 : 3;
  return rankA - rankB || a.localeCompare(b);
}

const DEPARTMENT_META = {
  'Job-site Construction': {
    icon: '🏗️',
    color: '#d97706',
    bg: '#fef3c7',
    isDeployable: true,
    label: 'Job-site Construction',
    badge: 'Deployable',
    description: 'Primary active field workforce deployable across ongoing civil, structural, architectural, and MEP construction sites.'
  },
  'Woodland Shops': {
    icon: '🏭',
    color: '#4f46e5',
    bg: '#e0e7ff',
    isDeployable: false,
    label: 'Woodland Shops',
    badge: 'Shops & Fab',
    description: 'Pre-fabrication and craft finishing shops producing modular components, joinery, and fixtures off-site rather than on active field builds.'
  },
  'Garden': {
    icon: '🌿',
    color: '#16a34a',
    bg: '#dcfce7',
    isDeployable: false,
    label: 'Garden',
    badge: 'Facility',
    description: 'Dedicated campus groundskeeping, landscaping, and agriculture volunteers sustaining facility grounds rather than field construction.'
  },
  'Residential Only': {
    icon: '🏠',
    color: '#9333ea',
    bg: '#f3e8ff',
    isDeployable: false,
    label: 'Residential Only',
    badge: 'Residential',
    description: 'Living quarters accommodations, domestic upkeep, and care personnel stationed at residential facilities; exempt from job-site works.'
  },
  'Electrical Rm': {
    icon: '💡',
    color: '#ca8a04',
    bg: '#fef9c3',
    isDeployable: false,
    label: 'Electrical Rm',
    badge: 'Plant / Utility',
    description: 'Specialized substation, high-voltage switchgear, and plant electrical operators ensuring campus-wide grid stability.'
  },
  'Water System': {
    icon: '💧',
    color: '#0284c7',
    bg: '#e0f2fe',
    isDeployable: false,
    label: 'Water System',
    badge: 'Plant / Utility',
    description: 'Water treatment, pumping station, tank storage, and piping technicians maintaining campus water infrastructure.'
  },
  'Woodland Office': {
    icon: '💼',
    color: '#475569',
    bg: '#f1f5f9',
    isDeployable: false,
    label: 'Woodland Office',
    badge: 'Administration',
    description: 'Administrative coordination, materials planning, logistical support, and data encoding team directing project operations.'
  }
};

function getWorkerDeployability(worker) {
  if (!worker || worker.isArchived) {
    return {
      isDeployable: false,
      tier: 'NON_DEPLOYABLE',
      label: 'Archived / Inactive',
      badgeClass: 'badge-inactive',
      color: '#94a3b8'
    };
  }

  const dept = worker.department || inferWorkerDepartment(worker);
  const subDept = worker.subDepartment || '';
  const isPresent = (worker.dailyStatus === 'Present' || (worker.dailyStatus && worker.dailyStatus.toLowerCase() === 'present'));

  if (!isPresent) {
    return {
      isDeployable: false,
      tier: 'ABSENT_OR_PASS',
      label: worker.dailyStatus || 'Off-Site',
      badgeClass: 'badge-offsite',
      color: '#ef4444'
    };
  }

  if (dept === 'Residential Only') {
    return {
      isDeployable: false,
      tier: 'RESIDENTIAL_ONLY',
      label: 'Residential Only (Residential JO Eligible)',
      badgeClass: 'badge-residential',
      color: '#9333ea'
    };
  }

  if (dept === 'Job-site Construction') {
    return {
      isDeployable: true,
      tier: 'JOB_SITE',
      label: 'Job-Site Deployable',
      badgeClass: 'badge-deployable',
      color: '#16a34a'
    };
  }

  // Also handle legacy workers stored with dept='Carpenter shop'
  if (dept === 'Woodland Shops' || dept === 'Carpenter shop') {
    const shopLabel = subDept || (dept === 'Carpenter shop' ? 'Carpenter Shop' : 'Woodland Shops');
    return {
      isDeployable: false,
      tier: 'SHOPS',
      label: `Shop: ${shopLabel}`,
      badgeClass: 'badge-shop',
      color: '#4f46e5'
    };
  }

  if (dept === 'Electrical Rm' || dept === 'Water System' || dept === 'Garden') {
    return {
      isDeployable: false,
      tier: 'UTILITIES',
      label: `Utility: ${dept}`,
      badgeClass: 'badge-utility',
      color: '#0284c7'
    };
  }

  if (dept === 'Woodland Office') {
    return {
      isDeployable: false,
      tier: 'OFFICE',
      label: 'Office & Admin',
      badgeClass: 'badge-office',
      color: '#64748b'
    };
  }

  return {
    isDeployable: true,
    tier: 'JOB_SITE',
    label: 'Job-Site Deployable',
    badgeClass: 'badge-deployable',
    color: '#16a34a'
  };
}

const areaThemeColors = {
  'station 1': { bg: '#edf3ec', text: '#2d6b49', primary: '#2d6b49' },
  'cc': { bg: '#ebf3f5', text: '#1c5f82', primary: '#1c5f82' },
  'pavilion': { bg: '#f4f0f7', text: '#6e4a8a', primary: '#6e4a8a' },
  'woodland': { bg: '#faebd7', text: '#d9730d', primary: '#d9730d' },
  'mall': { bg: '#f7eaf1', text: '#a03876', primary: '#a03876' },
  'greenville': { bg: '#edf3ec', text: '#2d6b49', primary: '#2d6b49' },
  'perimeter': { bg: '#fdf5e2', text: '#9a6800', primary: '#9a6800' },
  'powder': { bg: '#f4f0f7', text: '#6e4a8a', primary: '#6e4a8a' },
  'default': { bg: '#f1f1ef', text: '#59574f', primary: '#59574f' }
};

function getAreaStyle(area) {
  if (!area) return areaThemeColors.default;
  const a = area.toLowerCase().trim();
  for (const key in areaThemeColors) {
    if (a.includes(key)) return areaThemeColors[key];
  }
  return areaThemeColors.default;
}

function getResidenceStyle(name) {
  if (!name) return { primary: '#64748b', bg: '#f1f5f9', text: '#334155', border: '#cbd5e1' };
  const n = String(name).trim();
  const palette = [
    { primary: '#0284c7', bg: '#e0f2fe', text: '#0369a1', border: '#bae6fd' }, // sky
    { primary: '#7c3aed', bg: '#ede9fe', text: '#6d28d9', border: '#ddd6fe' }, // purple
    { primary: '#059669', bg: '#d1fae5', text: '#047857', border: '#a7f3d0' }, // emerald
    { primary: '#d97706', bg: '#fef3c7', text: '#b45309', border: '#fde68a' }, // amber
    { primary: '#e11d48', bg: '#ffe4e6', text: '#be123c', border: '#fecdd3' }, // rose
    { primary: '#4f46e5', bg: '#e0e7ff', text: '#4338ca', border: '#c7d2fe' }, // indigo
    { primary: '#0d9488', bg: '#ccfbf1', text: '#0f766e', border: '#99f6e4' }, // teal
    { primary: '#ea580c', bg: '#ffedd5', text: '#c2410c', border: '#fed7aa' }, // orange
    { primary: '#6366f1', bg: '#e0e7ff', text: '#4338ca', border: '#c7d2fe' }, // violet
    { primary: '#475569', bg: '#f1f5f9', text: '#334155', border: '#cbd5e1' }  // slate
  ];
  let hash = 0;
  for (let i = 0; i < n.length; i++) {
    hash = (hash * 31 + n.charCodeAt(i)) >>> 0;
  }
  return palette[hash % palette.length];
}

const STORAGE_KEY = 'wc_workers_data';

function debounce(fn, delay = 250) {
  let timer;
  return function(...args) {
    clearTimeout(timer);
    timer = setTimeout(() => fn.apply(this, args), delay);
  };
}

function showConfirm(message, options) {
  return new Promise((resolve) => {
    const modal = document.getElementById('confirm-modal');
    const msgEl = document.getElementById('confirm-modal-message');
    const titleEl = document.getElementById('confirm-modal-title');
    const iconEl = document.getElementById('confirm-modal-icon');
    const okBtn = document.getElementById('confirm-modal-ok');
    const cancelBtn = document.getElementById('confirm-modal-cancel');
    if (!modal || !msgEl) { resolve(window.confirm(message)); return; }
    if (options && options.danger) {
      if (titleEl) titleEl.textContent = options.title || 'Confirm Delete';
      if (iconEl) iconEl.innerHTML = '&#10060;';
      if (iconEl) iconEl.style.color = '#dc2626';
      if (okBtn) { okBtn.className = 'btn btn-danger'; okBtn.textContent = options.confirmText || 'Delete'; }
      if (cancelBtn) cancelBtn.textContent = 'Cancel';
    } else {
      if (titleEl) titleEl.textContent = options && options.title ? options.title : 'Confirm';
      if (iconEl) iconEl.innerHTML = '&#9888;&#65039;';
      if (iconEl) iconEl.style.color = '#f59e0b';
      if (okBtn) { okBtn.className = 'btn btn-primary'; okBtn.textContent = (options && options.confirmText) || 'OK'; }
      if (cancelBtn) cancelBtn.textContent = 'Cancel';
    }
    msgEl.textContent = message;
    modal.classList.add('active');
    modal.setAttribute('data-close-method', '');
    if (typeof window.trapFocus === 'function') {
        setTimeout(function () { window.trapFocus(modal); }, 10);
    }
    _confirmResolve = resolve;
  });
}

window.dbStores = {
  workers: {
    iterate: async (cb) => {
      const data = await window._storage.get(STORAGE_KEY) || [];
      data.forEach(cb);
    }
  },
  duty_areas: {
    getItem: async (id) => {
      const data = await window._storage.get('wc_duty_areas') || [];
      return data.find(item => item.id === id);
    },
    setItem: async (id, obj) => {
      let data = await window._storage.get('wc_duty_areas') || [];
      const idx = data.findIndex(item => item.id === id);
      if (idx > -1) data[idx] = obj;
      else data.push(obj);
      await window._storage.set('wc_duty_areas', data);
    },
    iterate: async (cb) => {
      const data = await window._storage.get('wc_duty_areas') || [];
      data.forEach(cb);
    }
  },
  duty_rosters: {
    getAll: async () => {
      return await window._storage.get('wc_duty_rosters') || [];
    },
    setAll: async (rosters) => {
      await window._storage.set('wc_duty_rosters', rosters);
    },
    iterate: async (cb) => {
      const data = await window._storage.get('wc_duty_rosters') || [];
      data.forEach(cb);
    }
  },
  duty_schedules: {
    getAll: async () => {
      return await window._storage.get('wc_duty_schedules') || [];
    },
    setAll: async (schedules) => {
      await window._storage.set('wc_duty_schedules', schedules);
    },
    getItem: async (id) => {
      const data = await window._storage.get('wc_duty_schedules') || [];
      return data.find(item => item.dateStr === id) || null;
    },
    setItem: async (id, obj) => {
      let data = await window._storage.get('wc_duty_schedules') || [];
      const idx = data.findIndex(item => item.dateStr === id);
      if (idx > -1) data[idx] = obj;
      else data.push(obj);
      await window._storage.set('wc_duty_schedules', data);
    },
    iterate: async (cb) => {
      const data = await window._storage.get('wc_duty_schedules') || [];
      data.forEach(cb);
    }
  },
  health_events: {
    getItem: async (id) => {
      const data = await window._storage.get('wc_health_events') || [];
      return data.find(item => item.id === id);
    },
    setItem: async (id, obj) => {
      let data = await window._storage.get('wc_health_events') || [];
      const idx = data.findIndex(item => item.id === id);
      if (idx > -1) data[idx] = obj;
      else data.push(obj);
      await window._storage.set('wc_health_events', data);
    },
    iterate: async (cb) => {
      const data = await window._storage.get('wc_health_events') || [];
      data.forEach(cb);
    }
  },
  covid_tests: {
    getItem: async (id) => {
      const data = await window._storage.get('wc_covid_tests') || [];
      return data.find(item => item.id === id);
    },
    setItem: async (id, obj) => {
      let data = await window._storage.get('wc_covid_tests') || [];
      const idx = data.findIndex(item => item.id === id);
      if (idx > -1) data[idx] = obj;
      else data.push(obj);
      await window._storage.set('wc_covid_tests', data);
    },
    iterate: async (cb) => {
      const data = await window._storage.get('wc_covid_tests') || [];
      data.forEach(cb);
    }
  },
  covid_overrides: {
    getItem: async (id) => {
      const data = await window._storage.get('wc_covid_overrides') || [];
      return data.find(item => item.id === id);
    },
    setItem: async (id, obj) => {
      let data = await window._storage.get('wc_covid_overrides') || [];
      const idx = data.findIndex(item => item.id === id);
      if (idx > -1) data[idx] = obj;
      else data.push(obj);
      await window._storage.set('wc_covid_overrides', data);
    },
    iterate: async (cb) => {
      const data = await window._storage.get('wc_covid_overrides') || [];
      data.forEach(cb);
    }
  },
  contact_traces: {
    getItem: async (id) => {
      const data = await window._storage.get('wc_contact_traces') || [];
      return data.find(item => item.id === id) || null;
    },
    setItem: async (id, obj) => {
      let data = await window._storage.get('wc_contact_traces') || [];
      const idx = data.findIndex(item => item.id === id);
      if (idx > -1) data[idx] = obj;
      else data.push(obj);
      await window._storage.set('wc_contact_traces', data);
    },
    iterate: async (cb) => {
      const data = await window._storage.get('wc_contact_traces') || [];
      data.forEach(cb);
    }
  },
  vaccines: {
    getItem: async (id) => {
      const data = await window._storage.get('wc_vaccines') || [];
      return data.find(item => item.id === id) || null;
    },
    setItem: async (id, obj) => {
      let data = await window._storage.get('wc_vaccines') || [];
      const idx = data.findIndex(item => item.id === id);
      if (idx > -1) data[idx] = obj;
      else data.push(obj);
      await window._storage.set('wc_vaccines', data);
    },
    removeItem: async (id) => {
      let data = await window._storage.get('wc_vaccines') || [];
      data = data.filter(item => item.id !== id);
      await window._storage.set('wc_vaccines', data);
    },
    iterate: async (cb) => {
      const data = await window._storage.get('wc_vaccines') || [];
      data.forEach(cb);
    },
    getByWorker: async (workerId) => {
      const data = await window._storage.get('wc_vaccines') || [];
      return data.filter(item => String(item.workerId) === String(workerId));
    }
  },
  volunteer_vaccines: {
    getItem: async (id) => {
      const data = await window._storage.get('wc_volunteer_vaccines') || [];
      return data.find(item => item.id === id) || null;
    },
    setItem: async (id, obj) => {
      let data = await window._storage.get('wc_volunteer_vaccines') || [];
      const idx = data.findIndex(item => item.id === id);
      if (idx > -1) data[idx] = obj;
      else data.push(obj);
      await window._storage.set('wc_volunteer_vaccines', data);
    },
    removeItem: async (id) => {
      let data = await window._storage.get('wc_volunteer_vaccines') || [];
      data = data.filter(item => item.id !== id);
      await window._storage.set('wc_volunteer_vaccines', data);
    },
    iterate: async (cb) => {
      const data = await window._storage.get('wc_volunteer_vaccines') || [];
      data.forEach(cb);
    },
    getByVolunteer: async (volunteerId) => {
      const data = await window._storage.get('wc_volunteer_vaccines') || [];
      return data.filter(item => String(item.volunteerId) === String(volunteerId));
    }
  },
  volunteer_medications: {
    getItem: async (id) => {
      const data = await window._storage.get('wc_volunteer_medications') || [];
      return data.find(item => item.id === id) || null;
    },
    setItem: async (id, obj) => {
      let data = await window._storage.get('wc_volunteer_medications') || [];
      const idx = data.findIndex(item => item.id === id);
      if (idx > -1) data[idx] = obj;
      else data.push(obj);
      await window._storage.set('wc_volunteer_medications', data);
    },
    removeItem: async (id) => {
      let data = await window._storage.get('wc_volunteer_medications') || [];
      data = data.filter(item => item.id !== id);
      await window._storage.set('wc_volunteer_medications', data);
    },
    iterate: async (cb) => {
      const data = await window._storage.get('wc_volunteer_medications') || [];
      data.forEach(cb);
    },
    getByVolunteer: async (volunteerId) => {
      const data = await window._storage.get('wc_volunteer_medications') || [];
      return data.filter(item => String(item.volunteerId) === String(volunteerId));
    }
  },
  volunteer_health_logs: {
    getItem: async (id) => {
      const data = await window._storage.get('wc_volunteer_health_logs') || [];
      return data.find(item => item.logId === id) || null;
    },
    setItem: async (id, obj) => {
      let data = await window._storage.get('wc_volunteer_health_logs') || [];
      const idx = data.findIndex(item => item.logId === id);
      if (idx > -1) data[idx] = obj;
      else data.push(obj);
      await window._storage.set('wc_volunteer_health_logs', data);
    },
    removeItem: async (id) => {
      let data = await window._storage.get('wc_volunteer_health_logs') || [];
      data = data.filter(item => item.logId !== id);
      await window._storage.set('wc_volunteer_health_logs', data);
    },
    iterate: async (cb) => {
      const data = await window._storage.get('wc_volunteer_health_logs') || [];
      data.forEach(cb);
    },
    getByVolunteer: async (volunteerId) => {
      const data = await window._storage.get('wc_volunteer_health_logs') || [];
      return data.filter(item => String(item.volunteerId) === String(volunteerId));
    }
  },
  volunteer_isolation_records: {
    getItem: async (id) => {
      const data = await window._storage.get('wc_volunteer_isolation_records') || [];
      return data.find(item => item.isolationId === id) || null;
    },
    setItem: async (id, obj) => {
      let data = await window._storage.get('wc_volunteer_isolation_records') || [];
      const idx = data.findIndex(item => item.isolationId === id);
      if (idx > -1) data[idx] = obj;
      else data.push(obj);
      await window._storage.set('wc_volunteer_isolation_records', data);
    },
    removeItem: async (id) => {
      let data = await window._storage.get('wc_volunteer_isolation_records') || [];
      data = data.filter(item => item.isolationId !== id);
      await window._storage.set('wc_volunteer_isolation_records', data);
    },
    iterate: async (cb) => {
      const data = await window._storage.get('wc_volunteer_isolation_records') || [];
      data.forEach(cb);
    },
    getByVolunteer: async (volunteerId) => {
      const data = await window._storage.get('wc_volunteer_isolation_records') || [];
      return data.filter(item => String(item.volunteerId) === String(volunteerId));
    }
  },
  medical_audit_logs: {
    getItem: async (id) => {
      const data = await window._storage.get('wc_medical_audit_logs') || [];
      return data.find(item => item.id === id) || null;
    },
    setItem: async (id, obj) => {
      let data = await window._storage.get('wc_medical_audit_logs') || [];
      const idx = data.findIndex(item => item.id === id);
      if (idx > -1) data[idx] = obj;
      else data.push(obj);
      await window._storage.set('wc_medical_audit_logs', data);
    },
    removeItem: async (id) => {
      let data = await window._storage.get('wc_medical_audit_logs') || [];
      data = data.filter(item => item.id !== id);
      await window._storage.set('wc_medical_audit_logs', data);
    },
    iterate: async (cb) => {
      const data = await window._storage.get('wc_medical_audit_logs') || [];
      data.forEach(cb);
    }
  },
  ot_entries: {
    getItem: async (id) => {
      const data = await window._storage.get('wc_ot_entries') || [];
      return data.find(item => item.id === id) || null;
    },
    setItem: async (id, obj) => {
      let data = await window._storage.get('wc_ot_entries') || [];
      const idx = data.findIndex(item => item.id === id);
      if (idx > -1) data[idx] = obj;
      else data.push(obj);
      await window._storage.set('wc_ot_entries', data);
    },
    removeItem: async (id) => {
      let data = await window._storage.get('wc_ot_entries') || [];
      data = data.filter(item => item.id !== id);
      await window._storage.set('wc_ot_entries', data);
    },
    getAll: async () => {
      return await window._storage.get('wc_ot_entries') || [];
    },
    setAll: async (items) => {
      await window._storage.set('wc_ot_entries', items);
    },
    iterate: async (cb) => {
      const data = await window._storage.get('wc_ot_entries') || [];
      data.forEach(cb);
    }
  },
};

var ctxEditSvg = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path></svg>';

var ctxGroupSvg = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"></path><circle cx="9" cy="7" r="4"></circle><path d="M23 21v-2a4 4 0 0 0-3-3.87"></path><path d="M16 3.13a4 4 0 0 1 0 7.75"></path></svg>';

var ctxArchiveSvg = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="21 8 21 21 3 21 3 8"></polyline><rect x="1" y="3" width="22" height="5"></rect><line x1="10" y1="12" x2="14" y2="12"></line></svg>';

function toDateStr(date) {
  const d = parseDateSafe(date);
  if (!d) return '';
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

window.guardClick = function guardClick(btn, fnAsync) {
  if (btn.dataset._guardBusy) return () => {};
  return async (...args) => {
    btn.dataset._guardBusy = '1';
    btn.disabled = true;
    btn.style.opacity = '0.6';
    try { await fnAsync(...args); } catch (e) { console.error(e); }
    finally { delete btn.dataset._guardBusy; btn.disabled = false; btn.style.opacity = ''; }
  };
}

window.showUndoToast = function (message, onUndo, duration) {
  var container = document.getElementById('toast-container');
  if (!container) return;
  var toast = document.createElement('div');
  toast.className = 'toast toast-undo';
  var btnId = 'undo-btn-' + Date.now();
  toast.innerHTML = '<span>' + esc(message) + '</span><button id="' + btnId + '" style="margin-left:12px;font-weight:700;color:#60a5fa;background:none;border:none;cursor:pointer;font-family:inherit;white-space:nowrap;">Undo</button>';
  container.appendChild(toast);
  var timer = setTimeout(function () {
    toast.classList.add('hide');
    setTimeout(function () { toast.remove(); }, 300);
  }, (typeof duration === 'number' && duration > 0) ? duration : 8000);
  document.getElementById(btnId).addEventListener('click', function () {
    if (onUndo()) {
      clearTimeout(timer);
      toast.classList.add('hide');
      setTimeout(function () { toast.remove(); }, 300);
    }
  });
}

const SIG_STORAGE_KEY = 'wc_signatories_data';

let defaultSignatories = []; // Service 5's defaults are real people; left out

// verbatim: core.js
// ---- Confirm modal (replaces blocking confirm()) ----

let _confirmResolve = null;

document.addEventListener('click', (e) => {
  const modal = document.getElementById('confirm-modal');
  if (!modal || !modal.classList.contains('active')) return;
  if (e.target === document.getElementById('confirm-modal-ok')) {
    modal.classList.remove('active');
    if (window.releaseFocusTrap) window.releaseFocusTrap();
    if (_confirmResolve) { const r = _confirmResolve; _confirmResolve = null; r(true); }
    return;
  }
  if (e.target === document.getElementById('confirm-modal-cancel') || e.target === modal) {
    modal.classList.remove('active');
    if (window.releaseFocusTrap) window.releaseFocusTrap();
    if (_confirmResolve) { const r = _confirmResolve; _confirmResolve = null; r(false); }
  }
});


function safePrint(html, options) { return window.safePrint(html, options); }
var waitForImagesThenCall = window.waitForImagesThenCall, getCombinedMedicalCondition = window.getCombinedMedicalCondition;

// ===== distribution.js (job orders) - verbatim from 01_Rust_WC_DB/js/distribution.js =====
var S5_distribution = (function () {
var printContainer = null, printDateInput = null, printBatchModeSelect = null, printGlobalSigSelect = null;
var selectedWorkerIndices = new Set();
function generateSigAuthToken(sigName, dateStr, workerId) {
    const raw = `${sigName}|${dateStr}|${workerId}|AGWC-JO-SECURE`;
    let hash = 0;
    for (let i = 0; i < raw.length; i++) {
        const char = raw.charCodeAt(i);
        hash = ((hash << 5) - hash) + char;
        hash |= 0;
    }
    const hex = Math.abs(hash).toString(16).toUpperCase().padStart(8, '0');
    return `JO-${hex.slice(0, 4)}-${hex.slice(4, 8)}`;
}

function buildWorkerFormDiv(worker, signatoryName, currentDate, targetIsoDate) {
    const formDiv = document.createElement('div');
    formDiv.className = 'worker-form';

    // Normalize targetIsoDate if not directly provided (e.g. from currentDate if MM/DD/YYYY)
    let isoDate = targetIsoDate || '';
    if (!isoDate && currentDate) {
        if (currentDate.includes('/')) {
            const parts = currentDate.split('/');
            if (parts.length === 3) {
                isoDate = `${parts[2]}-${parts[0].padStart(2, '0')}-${parts[1].padStart(2, '0')}`;
            }
        } else if (currentDate.includes('-')) {
            isoDate = currentDate;
        }
    }

    const todayIso = (typeof getTodayIso === 'function')
        ? getTodayIso()
        : new Date().toISOString().slice(0, 10);

    // If printing for a specific date that matches tomorrow's staging assignment, use tomorrow's staged task/area
    const isTomorrowMatch = Boolean(
        worker && worker.tomorrowAssignment && (
            (worker.tomorrowAssignment.date && (
                worker.tomorrowAssignment.date === isoDate ||
                worker.tomorrowAssignment.date === targetIsoDate ||
                worker.tomorrowAssignment.date === currentDate
            )) ||
            (!worker.tomorrowAssignment.date && isoDate && isoDate !== todayIso)
        )
    );
    const effectiveArea = (isTomorrowMatch && worker.tomorrowAssignment.area !== undefined && worker.tomorrowAssignment.area !== '') ? worker.tomorrowAssignment.area : (worker.area || '');
    const effectiveProject = (isTomorrowMatch && worker.tomorrowAssignment.project !== undefined && worker.tomorrowAssignment.project !== '') ? worker.tomorrowAssignment.project : (worker.project || '');
    const effectiveTask = (isTomorrowMatch && worker.tomorrowAssignment.specificTask !== undefined && worker.tomorrowAssignment.specificTask !== '') ? worker.tomorrowAssignment.specificTask : (worker.specificTask || '');

    // Dynamic auto-scaling font size for worker's name (1st option: resize to fit)
    const nameStr = (worker.name || '').trim();
    let nameFontSize = '12px';
    if (nameStr.length > 40) {
        nameFontSize = '7.5px';
    } else if (nameStr.length > 30) {
        nameFontSize = '8.5px';
    } else if (nameStr.length > 22) {
        nameFontSize = '9.5px';
    } else if (nameStr.length > 16) {
        nameFontSize = '10.5px';
    }

    // Dynamic auto-scaling font size for Trade (1st option: resize to fit)
    const tradeStr = (worker.trade || '').trim();
    let tradeFontSize = '11px';
    if (tradeStr.length > 32) {
        tradeFontSize = '7.5px';
    } else if (tradeStr.length > 24) {
        tradeFontSize = '8.5px';
    } else if (tradeStr.length > 16) {
        tradeFontSize = '9.5px';
    }

    // Dynamic auto-scaling font size for Area Assignment (1st option: resize to fit)
    let areaProjectText = effectiveArea === effectiveProject
        ? (effectiveArea || '')
        : ((effectiveArea || '') + (effectiveArea && effectiveProject ? ' - ' : '') + (effectiveProject || ''));
    if (!areaProjectText.trim()) {
        const dept = worker.department || (typeof inferWorkerDepartment === 'function' ? inferWorkerDepartment(worker) : '');
        if (dept === 'Residential Only' || (worker.role && worker.role.toLowerCase().includes('residential'))) {
            areaProjectText = worker.visitationHousing ? ('Residential - ' + worker.visitationHousing) : 'Residential Only';
        }
    }
    const areaLen = areaProjectText.trim().length;
    let areaFontSize = '11px';
    if (areaLen > 80) {
        areaFontSize = '7.5px';
    } else if (areaLen > 55) {
        areaFontSize = '8.5px';
    } else if (areaLen > 38) {
        areaFontSize = '9.5px';
    } else if (areaLen > 24) {
        areaFontSize = '10px';
    }

    // Dynamic auto-scaling font size & CSS class for Tasking description (1st option: resize to fit)
    const rawTask = (effectiveTask || '').trim() || 'General Task';
    const taskLen = rawTask.length;
    let taskFontSize = '11px';
    let taskClass = 'task-text-standard';
    if (taskLen > 180) {
        taskFontSize = '7px';
        taskClass = 'task-text-xxs';
    } else if (taskLen > 120) {
        taskFontSize = '7.5px';
        taskClass = 'task-text-xs';
    } else if (taskLen > 75) {
        taskFontSize = '8.5px';
        taskClass = 'task-text-sm';
    } else if (taskLen > 40) {
        taskFontSize = '9.5px';
        taskClass = 'task-text-md';
    }

    // Dynamic auto-scaling font size for Signatory name to prevent clipping (1st option: resize to fit)
    const sigNameStr = (signatoryName || '').trim();
    let sigFontSize = '11px';
    if (sigNameStr.length > 36) {
        sigFontSize = '7.5px';
    } else if (sigNameStr.length > 28) {
        sigFontSize = '8.5px';
    } else if (sigNameStr.length > 20) {
        sigFontSize = '9.5px';
    } else if (sigNameStr.length > 15) {
        sigFontSize = '10px';
    }

    // Resolve digital signature and anti-tamper security token
    const matchedSig = (window.signatoriesData || typeof signatoriesData !== 'undefined' ? signatoriesData : []).find(s => s && s.name && s.name.trim().toLowerCase() === sigNameStr.toLowerCase());
    const hasDigitalSig = !!(matchedSig && matchedSig.signatureUrl);
    const authHash = hasDigitalSig ? generateSigAuthToken(sigNameStr, currentDate, worker.aNumber || worker.name || '') : '';

    formDiv.innerHTML = `
        <table class="form-table" style="width: 100%; border-collapse: collapse; border: 1.5px solid #000; font-family: 'Inter', Arial, sans-serif;">
            <tbody>
                <!-- JOB ORDER HEADER -->
                <tr>
                    <td colspan="4" style="background: #3f3f46; color: white; text-align: center; font-weight: bold; padding: 3px 4px; border-bottom: 1.5px solid #000; font-size: 11.5px; letter-spacing: 0.05em;">JOB ORDER</td>
                </tr>
                <!-- ROW 1: Labels -->
                <tr style="background: #e4e4e7;">
                    <td style="border-right: 1px dotted #000; border-bottom: 1px dotted #000; padding: 1px 4px; width: 25%;">Name:</td>
                    <td style="border-right: 1px dotted #000; border-bottom: 1px dotted #000; padding: 1px 4px; width: 25%;">Trade:</td>
                    <td style="border-right: 1px dotted #000; border-bottom: 1px dotted #000; padding: 1px 4px; width: 15%;">Assigned No.:</td>
                    <td style="border-bottom: 1px dotted #000; padding: 1px 4px; color: #dc2626; font-size: 8.5px; width: 35%; line-height: 1.1; word-break: break-word; overflow-wrap: anywhere; white-space: normal;">Note: Makikisuyo po na isumite po ito sa JOBSITE Office bago po umuwi.</td>
                </tr>
                <!-- ROW 1: Values (Auto-scaling & wrapping without clipping) -->
                <tr>
                    <td class="name-cell" style="border-right: 1px dotted #000; border-bottom: 1px solid #000; padding: 1px 4px; height: auto; min-height: 18px; vertical-align: middle; position: relative;">
                        <input type="text" class="print-field-input print-name-input-compat" value="${esc(worker.name)}" style="position: absolute; inset: 0; width: 100%; height: 100%; opacity: 0.001; pointer-events: none; z-index: 1;" tabindex="-1" aria-hidden="true">
                        <div class="print-field-input print-field-name" contenteditable="true" spellcheck="false"
                             oninput="const inp = this.previousElementSibling; if (inp) inp.value = this.innerText;"
                             style="width: 100%; font-weight: bold; font-size: ${nameFontSize}; color: #000; outline: none; padding: 0; line-height: 1.15; word-break: break-word; overflow-wrap: anywhere; white-space: normal;">
                            ${esc(worker.name)}
                        </div>
                    </td>
                    <td class="trade-cell" style="border-right: 1px dotted #000; border-bottom: 1px solid #000; padding: 1px 4px; height: auto; min-height: 18px; vertical-align: middle; position: relative;">
                        <input type="text" class="print-field-input print-trade-input-compat" value="${esc(worker.trade)}" style="position: absolute; inset: 0; width: 100%; height: 100%; opacity: 0.001; pointer-events: none; z-index: 1;" tabindex="-1" aria-hidden="true">
                        <div class="print-field-input print-field-trade" contenteditable="true" spellcheck="false"
                             oninput="const inp = this.previousElementSibling; if (inp) inp.value = this.innerText;"
                             style="width: 100%; font-weight: bold; font-size: ${tradeFontSize}; color: #000; outline: none; padding: 0; line-height: 1.15; word-break: break-word; overflow-wrap: anywhere; white-space: normal;">
                            ${esc(worker.trade)}
                        </div>
                    </td>
                    <td class="anum-cell" style="border-right: 1px dotted #000; border-bottom: 1px solid #000; padding: 1px 4px; vertical-align: middle;">
                        <input type="text" class="print-field-input" value="${esc(worker.aNumber)}" style="width: 100%; border: none; background: transparent; font-weight: bold; font-size: 11px; color: #000; outline: none; padding: 0; overflow: visible;">
                    </td>
                    <td style="border-bottom: 1px solid #000; padding: 1px 4px;"></td>
                </tr>
                <!-- ROW 2: Labels -->
                <tr style="background: #e4e4e7;">
                    <td colspan="2" style="border-right: 1px dotted #000; border-bottom: 1px dotted #000; padding: 1px 4px;">Area Assignment:</td>
                    <td colspan="2" style="border-bottom: 1px dotted #000; padding: 1px 4px;">Tasking:</td>
                </tr>
                <!-- ROW 2: Values (Multi-line wrapping with dynamic scaling) -->
                <tr class="task-row">
                    <td colspan="2" class="area-cell" style="border-right: 1px dotted #000; border-bottom: 1px solid #000; padding: 2px 4px; min-height: 18px; vertical-align: middle;">
                        <div class="print-field-input print-field-area" contenteditable="true" spellcheck="false"
                             style="width: 100%; min-height: 16px; font-weight: bold; font-size: ${areaFontSize}; color: #000; outline: none; padding: 0; line-height: 1.15; word-break: break-word; overflow-wrap: anywhere; white-space: normal;">
                            ${esc(areaProjectText)}
                        </div>
                    </td>
                    <td colspan="2" class="tasking-cell" style="border-bottom: 1px solid #000; padding: 2px 4px; min-height: 18px; vertical-align: middle;">
                        <div class="print-field-input print-field-task ${taskClass}" contenteditable="true" spellcheck="false"
                             style="width: 100%; min-height: 16px; font-weight: bold; font-size: ${taskFontSize}; color: #000; outline: none; padding: 0; line-height: 1.15; word-break: break-word; overflow-wrap: anywhere; white-space: normal;">
                            ${esc(rawTask)}
                        </div>
                    </td>
                </tr>
                <!-- ROW 3: Labels -->
                <tr style="background: #e4e4e7;">
                    <td style="border-right: 1px dotted #000; border-bottom: 1px dotted #000; padding: 1px 4px;">Date Issued:</td>
                    <td style="border-right: 1px dotted #000; border-bottom: 1px dotted #000; padding: 1px 4px;">Time Released:</td>
                    <td style="border-right: 1px dotted #000; border-bottom: 1px dotted #000; padding: 1px 4px;">Time-In:</td>
                    <td style="border-bottom: 1px dotted #000; padding: 1px 4px;">Time-Out:</td>
                </tr>
                <!-- ROW 3: Values -->
                <tr>
                    <td style="border-right: 1px dotted #000; border-bottom: 1px solid #000; padding: 2px 4px; font-weight: bold; font-size: 10.5px; height: 16px; color: #000;">${currentDate}</td>
                    <td style="border-right: 1px dotted #000; border-bottom: 1px solid #000; padding: 2px 4px;"></td>
                    <td style="border-right: 1px dotted #000; border-bottom: 1px solid #000; padding: 2px 4px;"></td>
                    <td style="border-bottom: 1px solid #000; padding: 2px 4px;"></td>
                </tr>
                <!-- BREAK TIME HEADER -->
                <tr>
                    <td colspan="4" style="background: #a1a1aa; color: #000; text-align: center; font-weight: bold; padding: 3px 4px; border-bottom: 1.5px solid #000; font-size: 11px; letter-spacing: 0.05em;">BREAK TIME</td>
                </tr>
                <!-- BREAK TIME LABELS -->
                <tr style="background: #e4e4e7;">
                    <td style="border-right: 1px dotted #000; border-bottom: 1.5px solid #000; text-align: center; font-weight: bold; padding: 2px 4px; font-size: 8px; white-space: normal;">Morning Break: 9:00 AM - 9:15 AM</td>
                    <td style="border-right: 1px dotted #000; border-bottom: 1.5px solid #000; text-align: center; font-weight: bold; padding: 2px 4px; font-size: 8px; white-space: normal;">Lunch Break: 12:00 PM - 1:00 PM</td>
                    <td style="border-right: 1px dotted #000; border-bottom: 1.5px solid #000; text-align: center; font-weight: bold; padding: 2px 4px; font-size: 8px; white-space: normal;">Afternoon Break: 3:00 PM - 3:15 PM</td>
                    <td style="border-bottom: 1.5px solid #000; text-align: center; font-weight: bold; padding: 2px 4px; font-size: 8px; white-space: normal;">Bondi-Out: 5:00 PM-5:30 PM</td>
                </tr>
                <!-- BREAK TIME NESTED TABLES -->
                <tr>
                    <td style="border-right: 1px dotted #000; border-bottom: 1.5px solid #000; padding: 0; vertical-align: top;">
                        <table style="width: 100%; height: 100%; border-collapse: collapse; font-size: 9.5px;">
                            <tr>
                                <td style="width: 40%; border-right: 1px dotted #000; border-bottom: 1px dotted #000; padding: 2px 3px;">Actual Out:</td>
                                <td style="width: 30%; border-right: 1px dotted #000; border-bottom: 1px dotted #000; padding: 2px 3px; color: #a1a1aa; font-style: italic;">Time:</td>
                                <td style="width: 30%; border-bottom: 1px dotted #000; padding: 2px 3px; color: #a1a1aa; font-style: italic;">Lagda:</td>
                            </tr>
                            <tr>
                                <td style="width: 40%; border-right: 1px dotted #000; padding: 2px 3px;">Actual In:</td>
                                <td style="width: 30%; border-right: 1px dotted #000; padding: 2px 3px; color: #a1a1aa; font-style: italic;">Time:</td>
                                <td style="width: 30%; padding: 2px 3px; color: #a1a1aa; font-style: italic;">Lagda:</td>
                            </tr>
                        </table>
                    </td>
                    <td style="border-right: 1px dotted #000; border-bottom: 1.5px solid #000; padding: 0; vertical-align: top;">
                        <table style="width: 100%; height: 100%; border-collapse: collapse; font-size: 9.5px;">
                            <tr>
                                <td style="width: 40%; border-right: 1px dotted #000; border-bottom: 1px dotted #000; padding: 2px 3px;">Actual Out:</td>
                                <td style="width: 30%; border-right: 1px dotted #000; border-bottom: 1px dotted #000; padding: 2px 3px; color: #a1a1aa; font-style: italic;">Time:</td>
                                <td style="width: 30%; border-bottom: 1px dotted #000; padding: 2px 3px; color: #a1a1aa; font-style: italic;">Lagda:</td>
                            </tr>
                            <tr>
                                <td style="width: 40%; border-right: 1px dotted #000; padding: 2px 3px;">Actual In:</td>
                                <td style="width: 30%; border-right: 1px dotted #000; padding: 2px 3px; color: #a1a1aa; font-style: italic;">Time:</td>
                                <td style="width: 30%; padding: 2px 3px; color: #a1a1aa; font-style: italic;">Lagda:</td>
                            </tr>
                        </table>
                    </td>
                    <td style="border-right: 1px dotted #000; border-bottom: 1.5px solid #000; padding: 0; vertical-align: top;">
                        <table style="width: 100%; height: 100%; border-collapse: collapse; font-size: 9.5px;">
                            <tr>
                                <td style="width: 40%; border-right: 1px dotted #000; border-bottom: 1px dotted #000; padding: 2px 3px;">Actual Out:</td>
                                <td style="width: 30%; border-right: 1px dotted #000; border-bottom: 1px dotted #000; padding: 2px 3px; color: #a1a1aa; font-style: italic;">Time:</td>
                                <td style="width: 30%; border-bottom: 1px dotted #000; padding: 2px 3px; color: #a1a1aa; font-style: italic;">Lagda:</td>
                            </tr>
                            <tr>
                                <td style="width: 40%; border-right: 1px dotted #000; padding: 2px 3px;">Actual In:</td>
                                <td style="width: 30%; border-right: 1px dotted #000; padding: 2px 3px; color: #a1a1aa; font-style: italic;">Time:</td>
                                <td style="width: 30%; padding: 2px 3px; color: #a1a1aa; font-style: italic;">Lagda:</td>
                            </tr>
                        </table>
                    </td>
                    <td style="border-bottom: 1.5px solid #000; padding: 0; vertical-align: top;">
                        <table style="width: 100%; height: 100%; border-collapse: collapse; font-size: 9.5px;">
                            <tr>
                                <td style="width: 40%; border-right: 1px dotted #000; border-bottom: 1px dotted #000; padding: 2px 3px;">Actual Out:</td>
                                <td style="width: 30%; border-right: 1px dotted #000; border-bottom: 1px dotted #000; padding: 2px 3px; color: #a1a1aa; font-style: italic;">Time:</td>
                                <td style="width: 30%; border-bottom: 1px dotted #000; padding: 2px 3px; color: #a1a1aa; font-style: italic;">Lagda:</td>
                            </tr>
                            <tr>
                                <td style="width: 40%; border-right: 1px dotted #000; padding: 2px 3px;">Actual In:</td>
                                <td style="width: 30%; border-right: 1px dotted #000; padding: 2px 3px; color: #a1a1aa; font-style: italic;">Time:</td>
                                <td style="width: 30%; padding: 2px 3px; color: #a1a1aa; font-style: italic;">Lagda:</td>
                            </tr>
                        </table>
                    </td>
                </tr>
            </tbody>
        </table>
        <div class="worker-sig-row" style="display: flex; justify-content: space-between; align-items: flex-end; padding: 4px 24px 2px; font-family: 'Inter', Arial, sans-serif; flex-grow: 1; min-height: 44px; box-sizing: border-box;">
            <!-- Manager Signature (Releasing Officer) -->
            <div class="supervisor-sig-container" style="display: flex; flex-direction: column; align-items: center; position: relative; width: 220px;">
                ${hasDigitalSig ? `
                    <!-- Digital Signature Image & Security Date Stamp -->
                    <div style="height: 34px; display: flex; align-items: flex-end; justify-content: center; width: 100%; position: relative;">
                        <img src="${matchedSig.signatureUrl}" alt="Digital Signature" style="max-height: 38px; max-width: 140px; object-fit: contain; position: absolute; bottom: -4px; z-index: 2; pointer-events: none; filter: contrast(1.15);">
                        <!-- Tamper-evident Security Stamp -->
                        <div class="sig-auth-seal" style="position: absolute; right: 0; bottom: 6px; display: flex; flex-direction: column; align-items: flex-end; line-height: 1.1; pointer-events: none; text-align: right; background: rgba(255,255,255,0.88); padding: 1px 3px; border-radius: 3px;">
                            <span style="font-size: 6.5px; font-weight: 800; color: #1e40af; letter-spacing: 0.2px;">DIGITALLY SIGNED</span>
                            <span style="font-size: 6.5px; font-weight: 700; color: #09090b;">Date: ${esc(currentDate)}</span>
                            <span style="font-size: 5.5px; font-weight: 700; color: #b91c1c; text-transform: uppercase;">VALID ONLY: ${esc(currentDate)}</span>
                            <span style="font-size: 5px; font-family: monospace; color: #64748b;">[${authHash}]</span>
                        </div>
                    </div>
                ` : `
                    <div style="height: 20px;"></div>
                `}
                <div class="sig-name-wrap" style="width: 220px; max-width: 220px; position: relative; text-align: center;">
                    <input type="text" class="sig-name-input" list="signatory-names" value="${esc(signatoryName)}"
                           oninput="this.style.fontSize = this.value.length > 36 ? '7.5px' : (this.value.length > 28 ? '8.5px' : (this.value.length > 20 ? '9.5px' : (this.value.length > 15 ? '10px' : '11px')));"
                           style="width: 220px; max-width: 220px; text-align: center; font-size: ${sigFontSize}; font-weight: bold; border: none; background: transparent; outline: none; margin-bottom: 2px; position: relative; z-index: 3; line-height: 1.15;">
                </div>
                <div style="border-top: 1px solid #000; width: 220px; margin-bottom: 2px;"></div>
                <div style="font-size: 8.5px; font-style: italic; color: #000;">Releasing Officer (Section Chief)</div>
            </div>
            <!-- Worker Signature -->
            <div style="display: flex; flex-direction: column; align-items: center; gap: 0px;">
                <div style="height: 20px;"></div>
                <div style="border-bottom: 1px solid #000; width: 200px; margin-bottom: 2px;"></div>
                <div style="font-size: 8.5px; font-style: italic; color: #000;">Lagda ng Boluntaryo (Volunteer)</div>
            </div>
        </div>
    `;
    return formDiv;
}

function appendSignatoryDivider(sigName, workerCount) {
    const divider = document.createElement('div');
    divider.className = 'no-print';
    divider.style.cssText = 'max-width:8.5in;margin:28px auto 6px;border-radius:8px;overflow:hidden;box-shadow:0 2px 8px rgba(0,0,0,0.12);';
    divider.innerHTML = `
        <div style="display:flex;align-items:center;justify-content:space-between;gap:12px;
            padding:12px 20px;background:linear-gradient(135deg,#1e40af 0%,#3b82f6 100%);color:#fff;">
            <div style="display:flex;align-items:center;gap:10px;">
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                    <path d="M12 20h9"/><path d="M16.5 3.5a2.121 2.121 0 013 3L7 19l-4 1 1-4L16.5 3.5z"/>
                </svg>
                <span style="font-weight:800;font-size:0.95rem;">Signatory Batch: ${esc(sigName)}</span>
            </div>
            <span style="font-size:0.78rem;font-weight:700;background:rgba(255,255,255,0.22);
                padding:3px 10px;border-radius:9999px;">${workerCount} JO slip${workerCount !== 1 ? 's' : ''}</span>
        </div>`;
    printContainer.appendChild(divider);
}

function appendProjectGroupDivider(groupName, workerCount, sigName) {
    const divider = document.createElement('div');
    divider.className = 'no-print';
    divider.style.cssText = 'max-width:8.5in;margin:24px auto 6px;border-radius:6px;overflow:hidden;box-shadow:0 1px 4px rgba(0,0,0,0.06);border:1px solid var(--border-color,#e2e8f0);';
    divider.innerHTML = `
        <div style="display:flex;align-items:center;justify-content:space-between;gap:12px;
            padding:10px 18px;background:#f8fafc;color:#0f172a;border-left:4px solid #3b82f6;">
            <div style="display:flex;align-items:center;gap:10px;">
                <span style="font-size:1.1rem;">👥</span>
                <span style="font-weight:700;font-size:0.92rem;color:#0f172a;">Project Group: ${esc(groupName)}</span>
                ${sigName ? `<span style="font-size:0.75rem;color:#64748b;margin-left:4px;font-weight:500;">(Signatory: <strong>${esc(sigName)}</strong>)</span>` : ''}
            </div>
            <span style="font-size:0.75rem;font-weight:700;background:#e2e8f0;color:#334155;
                padding:2px 10px;border-radius:9999px;">${workerCount} JO slip${workerCount !== 1 ? 's' : ''}</span>
        </div>`;
    printContainer.appendChild(divider);
}

function renderJobOrderBatch(indices, signatoryName, currentDate, formsPerPage, targetIsoDate) {
    let currentPrintPage = null;
    indices.forEach((index, localIdx) => {
        if (localIdx % formsPerPage === 0) {
            currentPrintPage = document.createElement('div');
            currentPrintPage.className = 'print-page';
            printContainer.appendChild(currentPrintPage);
        }
        currentPrintPage.appendChild(buildWorkerFormDiv(workersData[index], signatoryName, currentDate, targetIsoDate));
    });
}

function generateFormsForPrinting() {
    // Datalist for individual signatory overrides on the forms
    const datalistOptions = signatoriesData.map(sig => `<option value="${esc(sig.name)}"></option>`).join('');
    printContainer.innerHTML = `
        <datalist id="signatory-names">
            ${datalistOptions}
        </datalist>
    `;


    // Universal function to change paper size
    window.currentPaperSize = window.currentPaperSize || 'folio';
    window.getFormsPerPage = (size) => size === 'letter' ? 3 : 4;
    window.setGlobalPaperSize = (size) => {
        window.currentPaperSize = size;
        const isLetter = size === 'letter';
        const paperHeight = isLetter ? '11.0in' : '13.0in';
        const rowCount = isLetter ? 3 : 4;

        let pageSpec = document.getElementById('global-page-spec');
        if (!pageSpec) {
            pageSpec = document.createElement('style');
            pageSpec.id = 'global-page-spec';
            document.head.appendChild(pageSpec);
        }

        pageSpec.textContent = `
            @media screen {
                #print-view.active, #visitation-print-view.active {
                   overflow-y: auto !important;
                   height: calc(100dvh - 80px) !important;
                   display: block !important;
                   padding-top: 16px !important;
                }
                .print-page {
                    width: 8.5in !important;
                    height: ${paperHeight} !important;
                    background: white !important;
                    box-shadow: 0 4px 16px rgba(0,0,0,0.15) !important;
                    margin: 24px auto !important;
                    box-sizing: border-box !important;
                    padding: 0.20in 0.30in !important;
                    position: relative !important;
                    display: grid !important;
                    grid-template-rows: repeat(${rowCount}, 1fr) !important;
                    gap: 0 !important;
                    border-radius: 4px !important;
                }
                .visitation-print-page {
                    width: 8.5in !important;
                    min-height: 12.5in !important;
                    background: white !important;
                    box-shadow: 0 4px 16px rgba(0,0,0,0.15) !important;
                    margin: 24px auto !important;
                    box-sizing: border-box !important;
                    padding: 40px !important;
                    position: relative !important;
                    display: block !important;
                    border-radius: 4px !important;
                }
                .worker-form {
                    height: 100% !important;
                    max-height: 100% !important;
                    overflow: visible !important;
                    page-break-inside: avoid !important;
                    break-inside: avoid !important;
                    border-bottom: 1px dashed #444 !important;
                    box-sizing: border-box !important;
                    display: flex !important;
                    flex-direction: column !important;
                    justify-content: space-between !important;
                    padding: 2px 0 4px 0 !important;
                }
                .worker-form:last-child,
                .worker-form:nth-child(${rowCount}n) {
                    border-bottom: none !important;
                }
            }
            @media print {
                @page {
                    size: 8.5in ${paperHeight};
                    margin: 0;
                }
                html, body, #app, .app-container, .app-fixed-height, main, #print-view, #visitation-print-view, #print-container {
                    background: transparent !important;
                    height: auto !important;
                    min-height: 0 !important;
                    max-height: none !important;
                    overflow: visible !important;
                    overflow-x: visible !important;
                    overflow-y: visible !important;
                    position: static !important;
                    margin: 0 !important;
                    padding: 0 !important;
                    border: none !important;
                    box-shadow: none !important;
                }
                .view {
                    display: none !important;
                }
                .view.print-view,
                .view.print-view.active,
                #print-view,
                #visitation-print-view {
                    display: block !important;
                }
                .print-page {
                    width: 8.5in !important;
                    max-width: 8.5in !important;
                    height: ${paperHeight} !important;
                    max-height: ${paperHeight} !important;
                    min-height: ${paperHeight} !important;
                    box-sizing: border-box !important;
                    border: none !important;
                    margin: 0 auto !important;
                    padding: 0.20in 0.30in !important;
                    display: grid !important;
                    grid-template-rows: repeat(${rowCount}, 1fr) !important;
                    gap: 0 !important;
                    overflow: hidden !important;
                    page-break-after: always !important;
                    break-after: page !important;
                    page-break-inside: avoid !important;
                    break-inside: avoid !important;
                    background: #ffffff !important;
                }
                .print-page:last-child {
                    page-break-after: auto !important;
                    break-after: auto !important;
                }
                .visitation-print-page {
                    width: 100% !important;
                    max-width: 100% !important;
                    min-height: auto !important;
                    height: auto !important;
                    margin: 0 auto !important;
                    padding: 0.2in 0.15in !important;
                    box-shadow: none !important;
                    border: none !important;
                    border-radius: 0 !important;
                    box-sizing: border-box !important;
                    break-inside: avoid !important;
                    page-break-inside: avoid !important;
                    break-after: page !important;
                    page-break-after: always !important;
                    background: #ffffff !important;
                }
                .visitation-print-page:last-child {
                    page-break-after: auto !important;
                    break-after: auto !important;
                }
                .worker-form {
                    height: 100% !important;
                    max-height: 100% !important;
                    box-sizing: border-box !important;
                    overflow: visible !important;
                    page-break-inside: avoid !important;
                    break-inside: avoid !important;
                    border: none !important;
                    border-bottom: 1px dashed #444444 !important;
                    display: flex !important;
                    flex-direction: column !important;
                    justify-content: space-between !important;
                    padding: 2px 0 4px 0 !important;
                }
                .worker-form::after {
                    content: "" !important;
                    display: none !important;
                }
                .worker-form:last-child,
                .worker-form:nth-child(${rowCount}n) {
                    border-bottom: none !important;
                }
                .print-field-input, .sig-name-input {
                    border: none !important;
                    background: transparent !important;
                    box-shadow: none !important;
                    outline: none !important;
                }
                .print-field-task, .print-field-area, .print-field-name, .print-field-trade {
                    white-space: normal !important;
                    word-break: break-word !important;
                    overflow-wrap: anywhere !important;
                    height: auto !important;
                    max-height: none !important;
                    overflow: visible !important;
                    line-height: 1.15 !important;
                }
                .tasking-cell, .area-cell, .name-cell, .trade-cell, .anum-cell {
                    height: auto !important;
                    overflow: visible !important;
                    white-space: normal !important;
                }
                .form-table td, .form-table th {
                    text-overflow: unset !important;
                    overflow: visible !important;
                }
                .task-text-standard { font-size: 11px !important; }
                .task-text-md { font-size: 9.5px !important; }
                .task-text-sm { font-size: 8.5px !important; }
                .task-text-xs { font-size: 7.5px !important; line-height: 1.15 !important; }
                .task-text-xxs { font-size: 7px !important; line-height: 1.1 !important; }
                .no-print,
                .health-drawer,
                .health-drawer-backdrop,
                .health-hover-card,
                .batch-health-bar,
                #quick-clearance-drawer,
                #quick-clearance-backdrop,
                #health-hover-card,
                #personnel-batch-health-bar,
                #pb-sync-indicator,
                .modal-overlay,
                .modal-system {
                    display: none !important;
                    visibility: hidden !important;
                    opacity: 0 !important;
                }
            }
        `;
    };

    // Default to folio 8.5x13 (paper picker UI removed)
    window.setGlobalPaperSize(window.currentPaperSize || 'folio');

    let currentDate = getFormattedDate();
    let targetIsoDate = '';
    if (printDateInput && printDateInput.value) {
        const [y, m, d] = printDateInput.value.split('-');
        currentDate = `${m}/${d}/${y}`;
        targetIsoDate = printDateInput.value;
    } else {
        targetIsoDate = new Date().toISOString().slice(0, 10);
    }

    // Ensure we print all selected workers that are issued Job Orders
    const sourceSet = (window.selectedWorkerIndices && window.selectedWorkerIndices.size > 0)
        ? window.selectedWorkerIndices
        : (typeof selectedWorkerIndices !== 'undefined' && selectedWorkerIndices.size > 0)
            ? selectedWorkerIndices
            : null;

    const candidateIndices = sourceSet
        ? Array.from(sourceSet)
        : workersData.map((_, i) => i);

    let skippedCount = 0;
    const skipDetails = [];

    const jobOrderIndices = candidateIndices.filter(i => {
        const w = workersData[i];
        if (!w || w.isArchived) return false;

        const check = (typeof checkJobOrderEligibility === 'function')
            ? checkJobOrderEligibility(w, targetIsoDate)
            : { eligible: w.jOrder !== 'NO', reason: '' };

        if (!check.eligible) {
            skippedCount++;
            skipDetails.push({ name: w.name, reason: check.reason });
            return false;
        }
        return true;
    });

    // Render feedback banner (visible on screen in print preview, hidden in actual print via .no-print)
    const feedbackBanner = document.createElement('div');
    feedbackBanner.className = 'no-print print-summary-feedback';
    feedbackBanner.style.cssText = 'max-width:8.5in;margin:14px auto 10px;padding:10px 16px;border-radius:6px;font-family:var(--font-body,sans-serif);font-size:13px;display:flex;align-items:center;justify-content:space-between;gap:12px;box-shadow:rgba(15,15,15,0.05) 0 0 0 1px, rgba(15,15,15,0.08) 0 2px 4px;box-sizing:border-box;';

    if (skippedCount > 0) {
        feedbackBanner.style.background = '#FFFBEB';
        feedbackBanner.style.border = '1px solid #FDE68A';
        feedbackBanner.style.color = '#92400E';
        feedbackBanner.innerHTML = `
            <div style="display:flex;align-items:center;gap:8px;">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" style="flex-shrink:0;"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="8" x2="12" y2="12"></line><line x1="12" y1="16" x2="12.01" y2="16"></line></svg>
                <span>Generated <strong>${jobOrderIndices.length}</strong> Job Order${jobOrderIndices.length !== 1 ? 's' : ''} <span style="opacity:0.85;">(${skippedCount} skipped due to settings, pass, day-off, or absence)</span></span>
            </div>
            <span style="font-size:11px;font-weight:600;opacity:0.85;white-space:nowrap;">Shift Date: ${esc(currentDate)}</span>
        `;
    } else {
        feedbackBanner.style.background = '#F0FDF4';
        feedbackBanner.style.border = '1px solid #BBF7D0';
        feedbackBanner.style.color = '#166534';
        feedbackBanner.innerHTML = `
            <div style="display:flex;align-items:center;gap:8px;">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" style="flex-shrink:0;"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path><polyline points="22 4 12 14.01 9 11.01"></polyline></svg>
                <span>Generated <strong>${jobOrderIndices.length}</strong> Job Order${jobOrderIndices.length !== 1 ? 's' : ''}. All candidate volunteers verified eligible.</span>
            </div>
            <span style="font-size:11px;font-weight:600;opacity:0.85;white-space:nowrap;">Shift Date: ${esc(currentDate)}</span>
        `;
    }
    printContainer.appendChild(feedbackBanner);

    if (jobOrderIndices.length === 0) {
        const emptyDiv = document.createElement('div');
        emptyDiv.className = 'no-print';
        emptyDiv.style.cssText = 'max-width:8.5in;margin:32px auto;text-align:center;padding:48px 24px;background:#fff;border-radius:6px;border:1px solid rgba(55,53,47,0.12);font-family:var(--font-body,sans-serif);';
        emptyDiv.innerHTML = `
            <div style="font-size:2.5rem;margin-bottom:12px;">📋</div>
            <h3 style="font-size:16px;font-weight:600;color:var(--text-main,#37352f);margin-bottom:6px;">No Eligible Job Orders to Print</h3>
            <p style="font-size:13px;color:var(--text-muted,#787774);max-width:440px;margin:0 auto 16px;">
                ${skippedCount > 0 
                    ? `All ${skippedCount} candidate volunteer(s) were excluded because they are marked Absent, on Pass, on Day-off, or have "Issue Job Order" turned off.`
                    : 'No volunteers were selected or matched your filter.'}
            </p>
            <button class="btn btn-secondary" onclick="document.getElementById('btn-back').click()" style="font-size:13px;padding:6px 14px;">Return to Table</button>
        `;
        printContainer.appendChild(emptyDiv);
        return;
    }

    const currentPaperSize = window.currentPaperSize || 'folio';
    const formsPerPage = window.getFormsPerPage(currentPaperSize);
    const batchMode = printBatchModeSelect?.value || 'all';

    if (batchMode === 'by-signatory') {
        // -------------------------------------------------------
        // BY-SIGNATORY MODE: one batch per signatory, then fallback
        // -------------------------------------------------------

        // Determine which signatories have at least one eligible worker in this print set
        const sigsWithWorkers = signatoriesData.filter(sig =>
            (sig.projectGroups || sig.trades || []).some(g =>
                jobOrderIndices.some(i => (workersData[i].projectGroup || '').trim() === g)
            )
        );

        // Workers whose projectGroup is not mapped to any signatory (fallback batch)
        const defaultSigName = printGlobalSigSelect?.value || signatoriesData[0]?.name || '';
        const unmappedIndices = jobOrderIndices.filter(i => {
            const g = (workersData[i].projectGroup || '').trim();
            if (!g) return true; // workers without project group (e.g. Residential Only) included in fallback batch
            return !signatoriesData.some(s =>
                (s.projectGroups || s.trades || []).includes(g)
            );
        });

        // Render one batch per signatory
        sigsWithWorkers.forEach(sig => {
            const groups = sig.projectGroups || sig.trades || [];
            const sigIndices = jobOrderIndices.filter(i =>
                groups.includes((workersData[i].projectGroup || '').trim())
            );
            // Sort: by project group name, then by worker name within group
            sigIndices.sort((a, b) => {
                const ga = workersData[a].projectGroup || '';
                const gb = workersData[b].projectGroup || '';
                return ga.localeCompare(gb) || workersData[a].name.localeCompare(workersData[b].name);
            });
            appendSignatoryDivider(sig.name, sigIndices.length);
            renderJobOrderBatch(sigIndices, sig.name, currentDate, formsPerPage, targetIsoDate);
        });

        // Render fallback batch for workers not mapped to any signatory
        if (unmappedIndices.length > 0) {
            unmappedIndices.sort((a, b) =>
                (workersData[a].projectGroup || '').localeCompare(workersData[b].projectGroup || '') ||
                workersData[a].name.localeCompare(workersData[b].name)
            );
            appendSignatoryDivider(`Default : ${esc(defaultSigName)}`, unmappedIndices.length);
            renderJobOrderBatch(unmappedIndices, defaultSigName, currentDate, formsPerPage, targetIsoDate);
        }

    } else if (batchMode === 'by-project-group') {
        // -------------------------------------------------------
        // BY-PROJECT-GROUP MODE: one batch per project group
        // -------------------------------------------------------
        const defaultSigName = printGlobalSigSelect?.value || signatoriesData[0]?.name || '';

        // Group candidate indices by their assigned project group
        const groupMap = new Map();
        const unassignedIndices = [];

        jobOrderIndices.forEach(i => {
            const g = (workersData[i].projectGroup || '').trim();
            if (!g) {
                const dept = workersData[i].department || (typeof inferWorkerDepartment === 'function' ? inferWorkerDepartment(workersData[i]) : '');
                if (dept === 'Residential Only') {
                    const resGroup = 'Residential Only';
                    if (!groupMap.has(resGroup)) groupMap.set(resGroup, []);
                    groupMap.get(resGroup).push(i);
                } else {
                    unassignedIndices.push(i);
                }
            } else {
                if (!groupMap.has(g)) groupMap.set(g, []);
                groupMap.get(g).push(i);
            }
        });

        // Sort group names alphabetically
        const sortedGroups = Array.from(groupMap.keys()).sort((a, b) => a.localeCompare(b));

        sortedGroups.forEach(groupName => {
            const indices = groupMap.get(groupName);

            // Sort workers within group: Leaders/Katiwala first, then by name
            indices.sort((a, b) => {
                const roleA = (workersData[a].projectRole || '').toLowerCase();
                const roleB = (workersData[b].projectRole || '').toLowerCase();
                const isLeadA = roleA.includes('katiwala') || roleA.includes('leader') || roleA.includes('lead');
                const isLeadB = roleB.includes('katiwala') || roleB.includes('leader') || roleB.includes('lead');
                if (isLeadA && !isLeadB) return -1;
                if (!isLeadA && isLeadB) return 1;
                return (workersData[a].name || '').localeCompare(workersData[b].name || '');
            });

            // Resolve assigned signatory for this project group
            const matchedSig = signatoriesData.find(s => {
                const pgs = s.projectGroups || s.trades || [];
                return pgs.includes(groupName);
            });
            const sigName = matchedSig ? matchedSig.name : defaultSigName;

            appendProjectGroupDivider(groupName, indices.length, sigName);
            renderJobOrderBatch(indices, sigName, currentDate, formsPerPage, targetIsoDate);
        });

        // Unassigned group at the end if any
        if (unassignedIndices.length > 0) {
            unassignedIndices.sort((a, b) => (workersData[a].name || '').localeCompare(workersData[b].name || ''));
            appendProjectGroupDivider('General / Unassigned Group', unassignedIndices.length, defaultSigName);
            renderJobOrderBatch(unassignedIndices, defaultSigName, currentDate, formsPerPage, targetIsoDate);
        }

    } else {
        // -------------------------------------------------------
        // ALL-WORKERS MODE: original behavior : area → name sort
        // -------------------------------------------------------
        jobOrderIndices.sort((a, b) => {
            const getArea = (w) => {
                const isMatch = Boolean(
                    w && w.tomorrowAssignment && (
                        (w.tomorrowAssignment.date && (
                            w.tomorrowAssignment.date === targetIsoDate ||
                            w.tomorrowAssignment.date === currentDate
                        )) ||
                        (!w.tomorrowAssignment.date && targetIsoDate !== (typeof getTodayIso === 'function' ? getTodayIso() : new Date().toISOString().slice(0, 10)))
                    )
                );
                return (isMatch && w.tomorrowAssignment.area !== undefined && w.tomorrowAssignment.area !== '') ? w.tomorrowAssignment.area : (w.area || '');
            };
            const areaCmp = (getArea(workersData[a]) || '').localeCompare(getArea(workersData[b]) || '');
            if (areaCmp !== 0) return areaCmp;
            return (workersData[a].name || '').localeCompare(workersData[b].name || '');
        });

        let currentPrintPage = null;
        jobOrderIndices.forEach((index, localIdx) => {
            if (localIdx % formsPerPage === 0) {
                currentPrintPage = document.createElement('div');
                currentPrintPage.className = 'print-page';
                printContainer.appendChild(currentPrintPage);
            }
            // Resolve signatory: group-mapped first, then global override
            let finalName = printGlobalSigSelect ? printGlobalSigSelect.value : (signatoriesData[0]?.name || '');
            const workerGroup = (workersData[index].projectGroup || '').trim();
            if (workerGroup) {
                const matchedGroupSig = signatoriesData.find(s => {
                    const groups = s.projectGroups || s.trades || [];
                    return groups.includes(workerGroup);
                });
                if (matchedGroupSig) finalName = matchedGroupSig.name;
            }
            currentPrintPage.appendChild(buildWorkerFormDiv(workersData[index], finalName, currentDate, targetIsoDate));
        });
    }
}
return {
    generateFormsForPrinting: generateFormsForPrinting,
    selectedWorkerIndices: selectedWorkerIndices,
    setInputs: function (c, d, b, s) { printContainer = c; printDateInput = d; printBatchModeSelect = b; printGlobalSigSelect = s; }
};
})();

// ===== personnel.js - verbatim from 01_Rust_WC_DB/js/personnel.js =====
var S5_personnel = (function () {
var selectedPersonnel = new Set();
function _printPopupThenClose(win, delay) {
    setTimeout(() => {
        const doPrint = () => {
            try { win.print(); } catch (e) { console.warn('Print blocked:', e); }
            win.onafterprint = () => { try { win.close(); } catch(_) {} };
            setTimeout(() => { try { win.close(); } catch(_) {} }, 3000);
        };
        if (typeof window.waitForImagesThenCall === 'function') window.waitForImagesThenCall(win, doPrint);
        else doPrint();
    }, delay || 600);
}

function generatePrintableReport(groupByField, reportTitle) {
    const today = new Date();
    const dateStr = today.toLocaleDateString('en-US', { year: 'numeric', month: 'long', day: 'numeric' });

    const groups = {};
    workersData.forEach(w => {
        if (!w || w.isArchived) return;
        const key = (w[groupByField] || '').trim() || (groupByField === 'projectGroup' ? 'Unassigned' : 'Unspecified');
        if (!groups[key]) groups[key] = [];
        groups[key].push(w);
    });

    const sortedKeys = Object.keys(groups).sort((a, b) => {
        if (a.startsWith('Un') && !b.startsWith('Un')) return 1;
        if (!a.startsWith('Un') && b.startsWith('Un')) return -1;
        return a.localeCompare(b);
    });

    const roleOrder = { 'Katiwala': 1, 'II Katiwala': 2, 'Kalihim': 3, 'Member': 4 };

    const groupHtml = sortedKeys.map(groupKey => {
        let members = groups[groupKey];

        if (groupByField === 'projectGroup') {
            members = members.slice().sort((a, b) => {
                const ra = roleOrder[a.projectRole] || 5;
                const rb = roleOrder[b.projectRole] || 5;
                return ra !== rb ? ra - rb : a.name.localeCompare(b.name);
            });
        } else {
            members = members.slice().sort((a, b) => a.name.localeCompare(b.name));
        }

        const rows = members.map((w, i) => {
            const age = getAge(w.birthday);
            const ageStr = age !== null ? `${age} yrs` : '\u2014';
            const eName = esc(w.name);
            const eTrade = esc(w.trade || '');
            const eArea = esc(w.area || '\u2014');
            const eCivil = esc(w.civilStatus || '');
            const eAddress = esc(w.address || '');
            const eRole = esc(w.projectRole || '');
            const roleLabel = (groupByField === 'projectGroup' && w.projectRole) ? `<span style="display:inline-block;background:#dbeafe;color:#1d4ed8;font-size:0.7rem;font-weight:700;padding:1px 7px;border-radius: 2px;margin-left:6px;">${esc(w.projectRole)}</span>` : '';

            return `<tr style="break-inside:avoid;page-break-inside:avoid;">
                <td style="padding:6px 10px;border: 1px solid var(--border-color);font-size:0.8rem;color: var(--text-muted);text-align:center;">${i + 1}</td>
                <td style="padding:6px 10px;border: 1px solid var(--border-color);font-weight:600;font-size:0.85rem;">${eName}${roleLabel}</td>
                <td style="padding:6px 10px;border: 1px solid var(--border-color);font-size:0.82rem;">${eTrade}</td>
                <td style="padding:6px 10px;border: 1px solid var(--border-color);font-size:0.82rem;">${eArea}</td>
                <td style="padding:6px 10px;border: 1px solid var(--border-color);font-size:0.82rem;text-align:center;">${ageStr}</td>
                <td style="padding:6px 10px;border: 1px solid var(--border-color);font-size:0.82rem;text-align:center;">${eCivil}</td>
                <td style="padding:6px 10px;border: 1px solid var(--border-color);font-size:0.82rem;">${eAddress}</td>
            </tr>`;
        }).join('');

        return `
        <div style="margin-bottom:32px; break-inside: auto;">
            <div style="background:#1e40af; color:white; padding:8px 16px; border-radius: 2px 6px 0 0; display:flex; justify-content:space-between; align-items:center; break-after:avoid; page-break-after:avoid;">
                <span style="font-weight:800; font-size:0.95rem; letter-spacing:0.03em;">${esc(groupKey).toUpperCase()}</span>
                <span style="font-size:0.78rem; opacity:0.9;">${members.length} member${members.length !== 1 ? 's' : ''}</span>
            </div>
            <table style="width:100%; border-collapse:collapse; background: var(--card-bg); border: 1px solid var(--border-color);">
                <thead>
                    <tr style="background: var(--bg-color);break-inside:avoid;page-break-inside:avoid;">
                        <th style="padding:7px 10px;text-align:center;font-size:0.72rem;color: var(--text-muted);font-weight:700;border-bottom:2px solid #cbd5e1;width:36px;">#</th>
                        <th style="padding:7px 10px;text-align:left;font-size:0.72rem;color: var(--text-muted);font-weight:700;border-bottom:2px solid #cbd5e1;">NAME</th>
                        <th style="padding:7px 10px;text-align:left;font-size:0.72rem;color: var(--text-muted);font-weight:700;border-bottom:2px solid #cbd5e1;">TRADE</th>
                        <th style="padding:7px 10px;text-align:left;font-size:0.72rem;color: var(--text-muted);font-weight:700;border-bottom:2px solid #cbd5e1;">AREA ASSIGNED</th>
                        <th style="padding:7px 10px;text-align:center;font-size:0.72rem;color: var(--text-muted);font-weight:700;border-bottom:2px solid #cbd5e1;">AGE</th>
                        <th style="padding:7px 10px;text-align:center;font-size:0.72rem;color: var(--text-muted);font-weight:700;border-bottom:2px solid #cbd5e1;">CIVIL</th>
                        <th style="padding:7px 10px;text-align:left;font-size:0.72rem;color: var(--text-muted);font-weight:700;border-bottom:2px solid #cbd5e1;">ADDRESS</th>
                    </tr>
                </thead>
                <tbody>${rows}</tbody>
            </table>
        </div>`;
    }).join('');

    return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>${reportTitle} \u2014 Service Section</title>
    <style>
        /* ponytail: removed Google Fonts @import for offline reliability */
        :root {
            --bg-color: hsl(210, 40%, 98%);
            --card-bg: #ffffff;
            --text-main: hsl(222, 47%, 11%);
            --text-muted: hsl(215, 16%, 47%);
            --border-color: hsl(214, 32%, 91%);
            --row-hover: hsl(210, 40%, 96%);
        }
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body { font-family: 'Inter', Arial, sans-serif; background: var(--bg-color); color: var(--text-main); padding: 32px; }
        .toolbar { display: flex; align-items: center; gap: 12px; margin-bottom: 28px; background: var(--card-bg); border: 1px solid var(--border-color); border-radius: 2px; padding: 12px 18px; flex-wrap: wrap; box-shadow: 0 2px 8px rgba(0,0,0,0.05); }
        .toolbar label { font-size: 0.82rem; font-weight: 700; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em; }
        .size-btn { padding: 7px 18px; border-radius: 2px; border: 1.5px solid #cbd5e1; background: var(--bg-color); font-size: 0.82rem; font-weight: 700; color: #475569; cursor: pointer; transition: all .15s; }
        .size-btn.active { background: #1e40af; color: white; border-color: #1e40af; }
        .print-btn { margin-left: auto; padding: 9px 24px; background: #1e40af; color: white; border: none; border-radius: 2px; font-size: 0.88rem; font-weight: 700; cursor: pointer; display: flex; align-items: center; gap: 8px; }
        .print-btn:hover { background: #1d3a9f; }
        #page-style { }
        @media print {
            * { -webkit-print-color-adjust: exact !important; print-color-adjust: exact !important; }
            .toolbar { display: none !important; }
            body { background: #ffffff !important; color: #0f172a !important; padding: 0 !important; margin: 0 auto !important; box-sizing: border-box !important; orphans: 3; widows: 3; }
            thead { display: table-header-group !important; }
            tfoot { display: table-footer-group !important; }
            tr { break-inside: avoid !important; page-break-inside: avoid !important; }
            table { border-collapse: collapse !important; width: 100% !important; }
            img { image-rendering: -webkit-optimize-contrast; }
        }
    </style>
    <style id="page-style">
        @page { size: auto; margin: 8mm 10mm; }
    </style>
</head>
<body>
    <div class="toolbar">
        <button class="print-btn" onclick="window.print()">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 6 2 18 2 18 9"></polyline><path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"></path><rect x="6" y="14" width="12" height="8"></rect></svg>
            Print This Report
        </button>
    </div>
    <div style="text-align:center;margin-bottom:28px;border-bottom:2px solid #e2e8f0;padding-bottom:20px;">
        <div style="font-size:0.75rem;font-weight:700;letter-spacing:0.1em;color: var(--text-muted);margin-bottom:4px;">Service Section</div>
        <h1 style="font-size:1.4rem;font-weight:800;color: var(--text-main);">${reportTitle}</h1>
        <div style="font-size:0.8rem;color: var(--text-muted);margin-top:4px;">As of ${dateStr} &nbsp;|&nbsp; Total Active Workers: ${workersData.filter(w => w && !w.isArchived).length}</div>
    </div>
    ${groupHtml}
    <div style="margin-top:40px;border: 1px solid var(--border-color);padding-top:16px;font-size:0.72rem;color: var(--text-muted);text-align:center;">
        Generated on ${dateStr} : Service Section Personnel Directory
    </div>
</body>
</html>`;
}

function printPersonnelReport(mode) {
    let title = 'Personnel Directory';
    let field = 'name';

    if (mode === 'trade') {
        title = 'Personnel Directory - By Trade';
        field = 'trade';
    } else if (mode === 'group') {
        title = 'Personnel Directory - By Project Group';
        field = 'projectGroup';
    } else if (mode === 'housing') {
        title = 'Personnel Directory - By Housing Group';
        field = 'visitationHousing';
    }

    const html = generatePrintableReport(field, title);
    const win = window.open('', '_blank');
    if (!win) { showToast('Popup blocked. Please allow popups and try again.'); return; }
    try {
        win.document.open();
        win.document.write(html);
        win.document.close();
    } catch (e) {
        console.error('Failed to open print window:', e);
        showToast('Failed to open print preview.');
        try { win.close(); } catch (_) {}
        return;
    }
    _printPopupThenClose(win);
}

function generateVolunteerFamilyRosterHTML(workers, options = {}) {
    if (!workers || !workers.length) return '';
    const initialLayout = options.layout || 'cards'; // 'cards' or 'table'
    const initialPaper = options.paperSize || 'folio'; // 'folio' or 'letter'
    const today = new Date();
    const dateStr = today.toLocaleDateString('en-US', { year: 'numeric', month: 'long', day: 'numeric' });

    const formattedWorkers = workers.map(w => {
        const age = getAge(w.birthday);
        const rawBday = w.birthday || '';
        const eName = esc(w.name || 'Unnamed Volunteer');
        const ePhoto = esc(w.photoUrl || w.photo || '');
        const eTrade = esc(w.trade || 'General Worker');
        const eANumber = esc(w.aNumber != null ? String(w.aNumber) : '');
        const eCivil = esc(w.civilStatus || '');
        const ePhone = esc(w.phone || '-');
        const eWorkRes = esc(w.workResidence || '-');
        const eAddress = esc(w.address || '-');
        const eEmergency = esc(w.emergencyContact ? (w.emergencyContact + (w.emergencyPhone ? ' (' + w.emergencyPhone + ')' : '')) : '-');
        const eProjectRole = esc(w.projectRole || '');
        const eProjectGroup = esc(w.projectGroup || '');

        const initials = (w.name || '?')
            .trim()
            .split(/\s+/)
            .filter(Boolean)
            .slice(0, 2)
            .map(p => p[0].toUpperCase())
            .join('');

        const spouseName = w.spouseName ? esc(w.spouseName) : '';
        const spouseDuty = w.spouseChurchDuty ? esc(w.spouseChurchDuty) : '';
        const spouseStr = spouseName
            ? `${spouseName}${spouseDuty ? ` (${spouseDuty})` : ''}`
            : (eCivil && eCivil.toLowerCase() !== 'married' ? eCivil : 'None listed');

        const children = (Array.isArray(w.children) ? w.children : []).map(c => {
            const cName = esc(c.name || 'Child');
            let cAge = null;
            if (c.birthday) cAge = getAge(c.birthday);
            if (cAge === null && c.age) cAge = c.age;
            return {
                name: cName,
                age: cAge,
                birthday: c.birthday ? esc(c.birthday) : '',
                churchDuty: c.churchDuty ? esc(c.churchDuty) : '',
                school: c.school ? esc(c.school) : ''
            };
        });

        return {
            name: eName,
            photo: ePhoto,
            initials: initials || '?',
            trade: eTrade,
            aNumber: eANumber,
            age: age,
            birthday: rawBday,
            civil: eCivil,
            phone: ePhone,
            address: eAddress,
            workResidence: eWorkRes,
            emergency: eEmergency,
            projectRole: eProjectRole,
            projectGroup: eProjectGroup,
            spouse: spouseStr,
            children: children
        };
    });

    const serializedWorkers = JSON.stringify(formattedWorkers);

    return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Volunteer &amp; Family Directory Roster \u2014 Service Section</title>
    <style>
        :root {
            --bg-color: #f8fafc;
            --card-bg: #ffffff;
            --text-main: #0f172a;
            --text-muted: #64748b;
            --border-color: #cbd5e1;
            --primary: #1e40af;
            --primary-light: #eff6ff;
            --primary-border: #bfdbfe;
        }
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
            background: var(--bg-color);
            color: var(--text-main);
            padding: 24px;
            -webkit-print-color-adjust: exact !important;
            print-color-adjust: exact !important;
        }

        /* Screen Interactive Toolbar */
        .toolbar {
            display: flex;
            align-items: center;
            gap: 12px;
            margin-bottom: 24px;
            background: #ffffff;
            border: 1px solid #cbd5e1;
            border-radius: 8px;
            padding: 10px 18px;
            flex-wrap: wrap;
            box-shadow: 0 4px 12px rgba(0,0,0,0.06);
            position: sticky;
            top: 12px;
            z-index: 100;
        }
        .tb-grp { display: flex; align-items: center; gap: 6px; }
        .tb-lbl { font-size: 0.72rem; font-weight: 700; color: #64748b; text-transform: uppercase; letter-spacing: 0.05em; margin-right: 2px; }
        .tb-btn {
            padding: 6px 14px;
            border-radius: 4px;
            border: 1.5px solid #cbd5e1;
            background: #f8fafc;
            font-size: 0.78rem;
            font-weight: 700;
            color: #334155;
            cursor: pointer;
            transition: all 0.15s ease;
        }
        .tb-btn:hover { background: #e2e8f0; }
        .tb-btn.active {
            background: #1e40af;
            color: #ffffff;
            border-color: #1e40af;
        }
        .tb-sep { width: 1px; height: 22px; background: #cbd5e1; margin: 0 4px; }
        .print-btn {
            margin-left: auto;
            padding: 8px 22px;
            background: #1e40af;
            color: #ffffff;
            border: none;
            border-radius: 4px;
            font-size: 0.86rem;
            font-weight: 700;
            cursor: pointer;
            display: flex;
            align-items: center;
            gap: 8px;
            box-shadow: 0 2px 6px rgba(30,64,175,0.25);
            transition: background 0.15s;
        }
        .print-btn:hover { background: #1d3a9f; }

        /* Report Header */
        .roster-header {
            margin-bottom: 20px;
            border-bottom: 2.5px solid #1e40af;
            padding-bottom: 12px;
            display: flex;
            justify-content: space-between;
            align-items: flex-end;
            page-break-after: avoid;
            break-after: avoid;
        }
        .rh-title-area h1 {
            font-size: 1.35rem;
            font-weight: 800;
            color: #0f172a;
            letter-spacing: -0.01em;
            text-transform: uppercase;
        }
        .rh-subtitle {
            font-size: 0.78rem;
            color: #475569;
            margin-top: 3px;
        }
        .rh-stats {
            text-align: right;
            font-size: 0.78rem;
            color: #475569;
            font-weight: 600;
        }

        /* Group Heading */
        .group-header {
            background: #f1f5f9;
            border-left: 4px solid #1e40af;
            padding: 6px 12px;
            font-size: 0.82rem;
            font-weight: 800;
            color: #1e40af;
            text-transform: uppercase;
            letter-spacing: 0.04em;
            margin: 18px 0 10px;
            border-radius: 0 4px 4px 0;
            break-after: avoid;
            page-break-after: avoid;
        }

        /* Dossier Cards Layout */
        .cards-page {
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 10px;
            margin-bottom: 16px;
        }
        .cards-page.continuous {
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 10px;
            margin-bottom: 0;
        }
        .roster-card {
            background: #ffffff;
            border: 1px solid #cbd5e1;
            border-radius: 6px;
            padding: 9px 12px;
            break-inside: avoid !important;
            page-break-inside: avoid !important;
            display: flex;
            flex-direction: column;
            gap: 6px;
            position: relative;
        }
        .rc-header {
            display: flex;
            align-items: center;
            gap: 10px;
        }
        .rc-avatar {
            width: 48px;
            height: 48px;
            border-radius: 8px; /* Squircle */
            object-fit: cover;
            border: 1.5px solid #cbd5e1;
            background: #f1f5f9;
            flex-shrink: 0;
        }
        .rc-avatar-fallback {
            width: 48px;
            height: 48px;
            border-radius: 8px; /* Squircle */
            background: linear-gradient(135deg, #1e40af, #3b82f6);
            color: #ffffff;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 1.15rem;
            font-weight: 800;
            flex-shrink: 0;
            border: 1.5px solid #cbd5e1;
        }
        .rc-head-text {
            flex: 1;
            min-width: 0;
        }
        .rc-name-row {
            display: flex;
            align-items: baseline;
            gap: 6px;
            flex-wrap: wrap;
        }
        .rc-name {
            font-size: 0.95rem;
            font-weight: 800;
            color: #0f172a;
            line-height: 1.2;
            word-break: break-word;
        }
        .rc-anumber-badge {
            background: #f1f5f9;
            border: 1px solid #cbd5e1;
            color: #334155;
            font-size: 0.68rem;
            font-weight: 700;
            padding: 1px 5px;
            border-radius: 3px;
        }
        .rc-meta-row {
            display: flex;
            align-items: center;
            gap: 8px;
            margin-top: 3px;
            flex-wrap: wrap;
            font-size: 0.74rem;
            color: #475569;
        }
        .rc-trade-badge {
            background: #eff6ff;
            color: #1e40af;
            border: 1px solid #bfdbfe;
            font-weight: 700;
            font-size: 0.7rem;
            padding: 1px 7px;
            border-radius: 3px;
        }
        .rc-phone {
            font-weight: 700;
            color: #0f172a;
        }
        .rc-group-badge {
            background: #f8fafc;
            border: 1px solid #e2e8f0;
            color: #475569;
            font-size: 0.68rem;
            font-weight: 600;
            padding: 1px 6px;
            border-radius: 3px;
        }
        .rc-divider {
            height: 1px;
            background: #e2e8f0;
            margin: 1px 0;
        }
        .rc-body {
            display: flex;
            flex-direction: column;
            gap: 4px;
            font-size: 0.74rem;
        }
        .rc-row {
            display: flex;
            align-items: baseline;
            gap: 6px;
            line-height: 1.35;
        }
        .rc-lbl {
            font-weight: 700;
            color: #64748b;
            font-size: 0.68rem;
            text-transform: uppercase;
            letter-spacing: 0.03em;
            flex-shrink: 0;
            min-width: 72px;
        }
        .rc-val {
            color: #1e293b;
            word-break: break-word;
            flex: 1;
        }
        .rc-family-sec {
            background: #f8fafc;
            border: 1px solid #e2e8f0;
            border-radius: 4px;
            padding: 5px 8px;
            margin-top: 2px;
            display: flex;
            flex-direction: column;
            gap: 3px;
        }
        .rc-children-val {
            display: inline-block;
        }
        .child-pill {
            display: inline-block;
            background: #ffffff;
            border: 1px solid #cbd5e1;
            padding: 1px 6px;
            border-radius: 3px;
            font-size: 0.72rem;
            margin: 1px 2px;
            line-height: 1.3;
        }
        .child-pill strong {
            color: #0f172a;
            font-weight: 700;
        }
        .child-pill .child-age {
            color: #1e40af;
            font-weight: 700;
        }

        /* Master Table Layout */
        .roster-table {
            width: 100%;
            border-collapse: collapse;
            background: #ffffff;
            border: 1px solid #cbd5e1;
            margin-bottom: 20px;
        }
        .roster-table thead {
            display: table-header-group !important;
        }
        .roster-table th {
            background: #f1f5f9;
            color: #475569;
            font-size: 0.68rem;
            font-weight: 700;
            text-transform: uppercase;
            letter-spacing: 0.05em;
            padding: 8px 10px;
            text-align: left;
            border-bottom: 2px solid #cbd5e1;
            border-right: 1px solid #e2e8f0;
        }
        .roster-table td {
            padding: 7px 10px;
            font-size: 0.75rem;
            color: #1e293b;
            border-bottom: 1px solid #e2e8f0;
            border-right: 1px solid #e2e8f0;
            vertical-align: top;
            line-height: 1.35;
        }
        .rt-row {
            break-inside: avoid !important;
            page-break-inside: avoid !important;
        }
        .rt-row:nth-child(even) {
            background: #fcfcfd;
        }
        .rt-col-num {
            width: 32px;
            text-align: center;
            color: #64748b;
            font-weight: 600;
        }
        .rt-col-photo {
            width: 52px;
            text-align: center;
        }
        .rt-avatar {
            width: 42px;
            height: 42px;
            border-radius: 8px; /* Squircle */
            object-fit: cover;
            border: 1.5px solid #cbd5e1;
            background: #f1f5f9;
            display: block;
            margin: 0 auto;
        }
        .rt-avatar-fallback {
            width: 42px;
            height: 42px;
            border-radius: 8px; /* Squircle */
            background: linear-gradient(135deg, #1e40af, #3b82f6);
            color: #ffffff;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 1rem;
            font-weight: 800;
            margin: 0 auto;
            border: 1.5px solid #cbd5e1;
        }
        .rt-name {
            font-size: 0.88rem;
            font-weight: 800;
            color: #0f172a;
        }
        .rt-anumber {
            font-size: 0.7rem;
            font-weight: 600;
            color: #475569;
            background: #f1f5f9;
            border: 1px solid #cbd5e1;
            padding: 1px 4px;
            border-radius: 2px;
            margin-left: 4px;
        }
        .rt-submeta {
            font-size: 0.72rem;
            color: #475569;
            margin-top: 2px;
            display: flex;
            align-items: center;
            gap: 5px;
            flex-wrap: wrap;
        }
        .rt-trade-badge {
            background: #eff6ff;
            color: #1e40af;
            border: 1px solid #bfdbfe;
            font-weight: 700;
            font-size: 0.68rem;
            padding: 0 5px;
            border-radius: 2px;
        }
        .rt-tag {
            display: inline-block;
            font-size: 0.62rem;
            font-weight: 700;
            padding: 1px 4px;
            border-radius: 2px;
            background: #e2e8f0;
            color: #475569;
            margin-right: 4px;
        }
        .rt-tag-work {
            background: #dbeafe;
            color: #1e40af;
        }
        .rt-tag-children {
            background: #fef3c7;
            color: #92400e;
            border: 1px solid #fde68a;
        }
        .rt-addr-line {
            margin-bottom: 3px;
        }
        .rt-fam-spouse {
            margin-bottom: 4px;
        }
        .rt-fam-lbl {
            font-weight: 700;
            color: #64748b;
            font-size: 0.68rem;
            text-transform: uppercase;
            margin-right: 4px;
        }

        /* Footer */
        .roster-footer {
            margin-top: 24px;
            padding-top: 10px;
            border-top: 1px solid #cbd5e1;
            font-size: 0.7rem;
            color: #64748b;
            text-align: center;
            page-break-before: avoid;
            break-before: avoid;
        }

        /* Print Media Styles */
        @media print {
            .toolbar { display: none !important; }
            body {
                background: #ffffff !important;
                color: #000000 !important;
                padding: 0 !important;
                margin: 0 auto !important;
                width: 100% !important;
            }
            .cards-page {
                break-after: page !important;
                page-break-after: always !important;
                margin-bottom: 0 !important;
                padding-bottom: 2mm !important;
            }
            .cards-page:last-child {
                break-after: auto !important;
                page-break-after: auto !important;
            }
            .cards-page.continuous {
                break-after: auto !important;
                page-break-after: auto !important;
            }
            .roster-card {
                border: 1px solid #94a3b8 !important;
                box-shadow: none !important;
                break-inside: avoid !important;
                page-break-inside: avoid !important;
            }
            .roster-table {
                border: 1px solid #94a3b8 !important;
            }
            .roster-table th {
                background: #f1f5f9 !important;
                color: #0f172a !important;
                border-bottom: 1.5px solid #94a3b8 !important;
            }
            .roster-table td {
                border-bottom: 1px solid #cbd5e1 !important;
            }
            .rt-row {
                break-inside: avoid !important;
                page-break-inside: avoid !important;
            }
            img { image-rendering: -webkit-optimize-contrast !important; }
        }
    </style>
    <style id="page-size-style">
        @page { size: auto; margin: 8mm 10mm; }
    </style>
</head>
<body>
    <!-- Screen Toolbar -->
    <div class="toolbar">
        <div class="tb-grp">
            <span class="tb-lbl">Layout:</span>
            <button class="tb-btn" id="btn-layout-cards" onclick="setLayout('cards')">📇 Dossier Cards</button>
            <button class="tb-btn" id="btn-layout-table" onclick="setLayout('table')">📑 Master Table</button>
        </div>

        <div class="tb-sep"></div>

        <div class="tb-grp" id="cards-per-page-grp">
            <span class="tb-lbl">Cards/Sheet:</span>
            <button class="tb-btn" id="btn-cpp-6" onclick="setCardsPerPage(6)">6 (Letter)</button>
            <button class="tb-btn" id="btn-cpp-8" onclick="setCardsPerPage(8)">8 (Folio)</button>
            <button class="tb-btn" id="btn-cpp-4" onclick="setCardsPerPage(4)">4 (Large)</button>
            <button class="tb-btn" id="btn-cpp-auto" onclick="setCardsPerPage(0)">Continuous</button>
        </div>

        <div class="tb-sep"></div>

        <div class="tb-grp">
            <span class="tb-lbl">Paper:</span>
            <button class="tb-btn" id="btn-paper-folio" onclick="setPaperSize('folio')">Folio</button>
            <button class="tb-btn" id="btn-paper-letter" onclick="setPaperSize('letter')">Letter</button>
        </div>

        <div class="tb-sep"></div>

        <div class="tb-grp">
            <span class="tb-lbl">Grouping:</span>
            <button class="tb-btn" id="btn-grp-none" onclick="setGrouping('none')">A–Z</button>
            <button class="tb-btn" id="btn-grp-trade" onclick="setGrouping('trade')">By Trade</button>
            <button class="tb-btn" id="btn-grp-group" onclick="setGrouping('group')">By Group</button>
        </div>

        <button class="print-btn" onclick="window.print()">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 6 2 18 2 18 9"></polyline><path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"></path><rect x="6" y="14" width="12" height="8"></rect></svg>
            Print Roster
        </button>
    </div>

    <!-- Main Container -->
    <div id="roster-container">
        <!-- Rendered by client script below -->
    </div>

    <script>
        const rawWorkers = ${serializedWorkers};
        let currentLayout = '${initialLayout}';
        let currentCardsPerPage = ${initialPaper === 'letter' ? 6 : 8};
        let currentPaper = '${initialPaper}';
        let currentGrouping = 'none';

        function buildCardHtml(w) {
            const avatar = w.photo
                ? '<img class="rc-avatar" src="' + w.photo + '" alt="' + w.name + '" onerror="this.outerHTML=\\'<div class=\\\\\\'rc-avatar rc-avatar-fallback\\\\\\'>' + w.initials + '</div>\\'">'
                : '<div class="rc-avatar rc-avatar-fallback">' + w.initials + '</div>';

            const anumberBadge = w.aNumber ? '<span class="rc-anumber-badge">#' + w.aNumber + '</span>' : '';
            const groupBadge = w.projectGroup ? '<span class="rc-group-badge">' + (w.projectRole || 'Member') + ' &bull; ' + w.projectGroup + '</span>' : '';
            const ageStr = w.age !== null && w.age !== undefined ? (w.age + ' yrs') : '-';

            let childrenContent = '';
            if (w.children && w.children.length > 0) {
                childrenContent = w.children.map(function(c) {
                    const cAge = c.age ? ' <span class="child-age">(' + c.age + ' yrs)</span>' : '';
                    const cDuty = c.churchDuty ? ' &bull; <em>' + c.churchDuty + '</em>' : '';
                    return '<span class="child-pill"><strong>' + c.name + '</strong>' + cAge + cDuty + '</span>';
                }).join(' ');
            } else {
                childrenContent = '<span style="color:#94a3b8;font-style:italic;">No children listed</span>';
            }

            return '<div class="roster-card">' +
                '<div class="rc-header">' +
                    avatar +
                    '<div class="rc-head-text">' +
                        '<div class="rc-name-row">' +
                            '<span class="rc-name">' + w.name + '</span>' +
                            anumberBadge +
                        '</div>' +
                        '<div class="rc-meta-row">' +
                            '<span class="rc-trade-badge">' + w.trade + '</span>' +
                            '<span><strong>Age:</strong> ' + ageStr + '</span>' +
                            '<span class="rc-phone">📱 ' + w.phone + '</span>' +
                            groupBadge +
                        '</div>' +
                    '</div>' +
                '</div>' +
                '<div class="rc-divider"></div>' +
                '<div class="rc-body">' +
                    '<div class="rc-row"><span class="rc-lbl">Address:</span> <span class="rc-val"><strong>Home:</strong> ' + w.address + ' &nbsp;&bull;&nbsp; <strong>Work:</strong> ' + w.workResidence + '</span></div>' +
                    '<div class="rc-family-sec">' +
                        '<div class="rc-row"><span class="rc-lbl">Spouse:</span> <span class="rc-val"><strong>' + w.spouse + '</strong></span></div>' +
                        '<div class="rc-row"><span class="rc-lbl">Children (' + (w.children ? w.children.length : 0) + '):</span> <div class="rc-val rc-children-val">' + childrenContent + '</div></div>' +
                    '</div>' +
                '</div>' +
            '</div>';
        }

        function buildTableRowHtml(w, idx) {
            const avatar = w.photo
                ? '<img class="rt-avatar" src="' + w.photo + '" alt="' + w.name + '" onerror="this.outerHTML=\\'<div class=\\\\\\'rt-avatar rt-avatar-fallback\\\\\\'>' + w.initials + '</div>\\'">'
                : '<div class="rt-avatar rt-avatar-fallback">' + w.initials + '</div>';

            const anumberBadge = w.aNumber ? '<span class="rt-anumber">#' + w.aNumber + '</span>' : '';
            const groupBadge = w.projectGroup ? ' &bull; <span>' + w.projectGroup + '</span>' : '';
            const ageStr = w.age !== null && w.age !== undefined ? (w.age + ' yrs') : '-';

            let childrenContent = '';
            if (w.children && w.children.length > 0) {
                childrenContent = w.children.map(function(c) {
                    const cAge = c.age ? ' <span class="child-age">(' + c.age + ' yrs)</span>' : '';
                    const cDuty = c.churchDuty ? ' &bull; <em>' + c.churchDuty + '</em>' : '';
                    return '<span class="child-pill"><strong>' + c.name + '</strong>' + cAge + cDuty + '</span>';
                }).join(' ');
            } else {
                childrenContent = '<span style="color:#94a3b8;font-style:italic;">No children listed</span>';
            }

            return '<tr class="rt-row">' +
                '<td class="rt-col-num">' + (idx + 1) + '</td>' +
                '<td class="rt-col-photo">' + avatar + '</td>' +
                '<td class="rt-col-worker">' +
                    '<div class="rt-name">' + w.name + anumberBadge + '</div>' +
                    '<div class="rt-submeta">' +
                        '<span class="rt-trade-badge">' + w.trade + '</span> &bull; ' +
                        '<span>' + ageStr + '</span> &bull; ' +
                        '<span>📞 ' + w.phone + '</span>' +
                        groupBadge +
                    '</div>' +
                '</td>' +
                '<td class="rt-col-address">' +
                    '<div class="rt-addr-line"><span class="rt-tag">HOME</span> ' + w.address + '</div>' +
                    '<div class="rt-addr-line"><span class="rt-tag rt-tag-work">WORK</span> ' + w.workResidence + '</div>' +
                '</td>' +
                '<td class="rt-col-family">' +
                    '<div class="rt-fam-spouse"><span class="rt-fam-lbl">Spouse:</span> <strong>' + w.spouse + '</strong></div>' +
                    '<div class="rt-fam-children"><span class="rt-tag rt-tag-children">CHILDREN (' + (w.children ? w.children.length : 0) + ')</span> ' + childrenContent + '</div>' +
                '</td>' +
            '</tr>';
        }

        function getGroupedWorkers() {
            let sorted = [...rawWorkers];
            if (currentGrouping === 'trade') {
                const groups = {};
                sorted.forEach(function(w) {
                    const key = w.trade || 'General Worker';
                    if (!groups[key]) groups[key] = [];
                    groups[key].push(w);
                });
                return groups;
            } else if (currentGrouping === 'group') {
                const groups = {};
                sorted.forEach(function(w) {
                    const key = w.projectGroup || 'Unassigned';
                    if (!groups[key]) groups[key] = [];
                    groups[key].push(w);
                });
                return groups;
            } else {
                sorted.sort(function(a, b) { return (a.name || '').localeCompare(b.name || ''); });
                return { 'All Volunteers': sorted };
            }
        }

        function renderRoster() {
            const container = document.getElementById('roster-container');
            if (!container) return;

            const groups = getGroupedWorkers();
            const groupKeys = Object.keys(groups).sort();
            let totalCount = rawWorkers.length;

            let html = '<div class="roster-header">' +
                '<div class="rh-title-area">' +
                    '<div style="font-size:0.75rem;font-weight:700;letter-spacing:0.08em;color:#1e40af;margin-bottom:2px;">SERVICE SECTION</div>' +
                    '<h1>Volunteer &amp; Family Directory Roster</h1>' +
                    '<div class="rh-subtitle">Official Personnel Registry &bull; Generated on ${dateStr}</div>' +
                '</div>' +
                '<div class="rh-stats">' +
                    '<div>Total Volunteers: <strong>' + totalCount + '</strong></div>' +
                    '<div>Paper: <strong>' + (currentPaper.toUpperCase()) + '</strong></div>' +
                '</div>' +
            '</div>';

            if (currentLayout === 'cards') {
                groupKeys.forEach(function(gKey) {
                    const list = groups[gKey];
                    if (currentGrouping !== 'none') {
                        html += '<div class="group-header">' + gKey + ' (' + list.length + ' volunteers)</div>';
                    }

                    if (currentCardsPerPage > 0) {
                        for (let i = 0; i < list.length; i += currentCardsPerPage) {
                            const chunk = list.slice(i, i + currentCardsPerPage);
                            html += '<div class="cards-page">' + chunk.map(buildCardHtml).join('') + '</div>';
                        }
                    } else {
                        html += '<div class="cards-page continuous">' + list.map(buildCardHtml).join('') + '</div>';
                    }
                });
            } else {
                groupKeys.forEach(function(gKey) {
                    const list = groups[gKey];
                    if (currentGrouping !== 'none') {
                        html += '<div class="group-header">' + gKey + ' (' + list.length + ' volunteers)</div>';
                    }
                    html += '<table class="roster-table">' +
                        '<thead><tr>' +
                            '<th style="width:32px;text-align:center;">#</th>' +
                            '<th style="width:52px;text-align:center;">Photo</th>' +
                            '<th style="width:28%;">Volunteer Details</th>' +
                            '<th style="width:28%;">Addresses (Home &amp; Work)</th>' +
                            '<th style="width:38%;">Family Members &amp; Children</th>' +
                        '</tr></thead>' +
                        '<tbody>' + list.map(buildTableRowHtml).join('') + '</tbody>' +
                    '</table>';
                });
            }

            html += '<div class="roster-footer">' +
                'Confidential Document &bull; Service Section Personnel Directory &bull; Generated on ${dateStr}' +
            '</div>';

            container.innerHTML = html;
            updateToolbarState();
        }

        function updateToolbarState() {
            document.getElementById('btn-layout-cards').classList.toggle('active', currentLayout === 'cards');
            document.getElementById('btn-layout-table').classList.toggle('active', currentLayout === 'table');
            document.getElementById('cards-per-page-grp').style.display = currentLayout === 'cards' ? 'flex' : 'none';

            document.getElementById('btn-cpp-6').classList.toggle('active', currentCardsPerPage === 6);
            document.getElementById('btn-cpp-8').classList.toggle('active', currentCardsPerPage === 8);
            document.getElementById('btn-cpp-4').classList.toggle('active', currentCardsPerPage === 4);
            document.getElementById('btn-cpp-auto').classList.toggle('active', currentCardsPerPage === 0);

            document.getElementById('btn-paper-folio').classList.toggle('active', currentPaper === 'folio');
            document.getElementById('btn-paper-letter').classList.toggle('active', currentPaper === 'letter');

            document.getElementById('btn-grp-none').classList.toggle('active', currentGrouping === 'none');
            document.getElementById('btn-grp-trade').classList.toggle('active', currentGrouping === 'trade');
            document.getElementById('btn-grp-group').classList.toggle('active', currentGrouping === 'group');

            const styleEl = document.getElementById('page-size-style');
            if (styleEl) {
                if (currentPaper === 'folio') {
                    styleEl.textContent = '@page { size: 8.5in 13in; margin: 8mm 10mm; }';
                } else {
                    styleEl.textContent = '@page { size: 8.5in 11in; margin: 8mm 10mm; }';
                }
            }
        }

        function setLayout(l) {
            currentLayout = l;
            renderRoster();
        }

        function setCardsPerPage(n) {
            currentCardsPerPage = n;
            renderRoster();
        }

        function setPaperSize(s) {
            currentPaper = s;
            if (s === 'letter' && currentCardsPerPage === 8) currentCardsPerPage = 6;
            if (s === 'folio' && currentCardsPerPage === 6) currentCardsPerPage = 8;
            renderRoster();
        }

        function setGrouping(g) {
            currentGrouping = g;
            renderRoster();
        }

        renderRoster();
    <\/script>
</body>
</html>`;
}

function printVolunteerFamilyRoster(workers, options = {}) {
    const list = workers || workersData.filter(w => w && !w.isArchived);
    if (!list || !list.length) {
        showToast('No volunteers found to print.');
        return;
    }
    const html = generateVolunteerFamilyRosterHTML(list, options);
    const win = window.open('', '_blank');
    if (!win) {
        if (typeof window.safePrint === 'function') {
            window.safePrint(html, { title: 'Volunteer & Family Roster' });
            return;
        }
        showToast('Popup blocked. Please allow popups and try again.');
        return;
    }
    try {
        win.document.open();
        win.document.write(html);
        win.document.close();
    } catch (e) {
        console.error('Failed to open print window:', e);
        if (typeof window.safePrint === 'function') {
            window.safePrint(html, { title: 'Volunteer & Family Roster' });
            return;
        }
        showToast('Failed to open print preview.');
        try { win.close(); } catch (_) {}
        return;
    }
    _printPopupThenClose(win);
}

function printSelectedPersonnelCards(indicesSet = selectedPersonnel) {
    if (!indicesSet || indicesSet.size === 0) return;
    const workers = [...indicesSet].map(i => workersData[i]).filter(Boolean);
    if (workers.length === 0) return;

    const today = new Date();
    const dateStr = today.toLocaleDateString('en-US', { year: 'numeric', month: 'long', day: 'numeric' });
    const months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

    const cardHtmlList = workers.map(w => {
        const age = getAge(w.birthday);
        const rawBday = w.birthday || '';
        const eName = esc(w.name);
        const ePhoto = esc(w.photoUrl || w.photo || '');
        const eTrade = esc(w.trade || '');
        const eANumber = esc(w.aNumber != null ? String(w.aNumber) : '');
        const eCivil = esc(w.civilStatus || '');
        const ePhone = esc(w.phone || '');
        const eMedical = esc(window.getCombinedMedicalCondition ? window.getCombinedMedicalCondition(w) : (w.medical || ''));
        const eWorkRes = esc(w.workResidence || '');
        const eAddress = esc(w.address || '');
        const eEmergency = esc(w.emergencyContact || '');
        const eProjectRole = esc(w.projectRole || '');
        const eProjectGroup = esc(w.projectGroup || '');
        const bdayStr = w.birthday
            ? (() => { const d = new Date(rawBday); return isNaN(d) ? esc(rawBday) : `${months[d.getMonth()]} ${d.getDate()}, ${d.getFullYear()}`; })()
            : '\u2014';
        const groupBadge = w.projectGroup
            ? `<span style="background:#dbeafe;color:#1e40af;font-size:0.7rem;font-weight:700;padding:2px 8px;border-radius: 4px;">${eProjectRole || 'Member'} \u2022 ${eProjectGroup}</span>`
            : '';
        const avatar = w.photoUrl || w.photo
            ? `<img class="avatar-img" src="${ePhoto}" style="width:72px;height:72px;border-radius:8px;object-fit:cover;border:2px solid #cbd5e1;flex-shrink:0;">`
            : `<div class="avatar-img" style="width:72px;height:72px;border-radius:8px;background:linear-gradient(135deg,#3b82f6,#1d4ed8);display:flex;align-items:center;justify-content:center;font-size:2rem;font-weight:800;color:white;flex-shrink:0;">${eName.charAt(0)}</div>`;

        const childrenList = Array.isArray(w.children) ? w.children : [];
        const childrenSummary = childrenList.length > 0
            ? childrenList.map(c => {
                const cName = esc(c.name || 'Child');
                const cAge = c.birthday ? (getAge(c.birthday) || c.age || '') : (c.age || '');
                return `<strong>${cName}</strong>${cAge ? ' (' + cAge + ' yrs)' : ''}`;
            }).join(', ')
            : '<span style="color:#94a3b8;font-style:italic;">None listed</span>';

        const spouseSummary = w.spouseName
            ? `<strong>${esc(w.spouseName)}</strong>${w.spouseChurchDuty ? ' (' + esc(w.spouseChurchDuty) + ')' : ''}`
            : (eCivil && eCivil.toLowerCase() !== 'married' ? eCivil : '<span style="color:#94a3b8;font-style:italic;">None listed</span>');

        return `<div class="info-card">
            <div style="display:flex;align-items:center;gap:12px;margin-bottom:10px;">
                ${avatar}
                <div style="flex:1;min-width:0;">
                    <div class="card-name" style="font-weight:800;font-size:1.1rem;color: var(--text-main);line-height:1.25;word-break:break-word;white-space:normal;">${eName}</div>
                    <div style="font-size:0.78rem;color: var(--text-muted);margin-top:2px;">${eTrade || '\u2014'}${w.aNumber ? ' &nbsp;\u00b7&nbsp; #' + eANumber : ''}</div>
                    <div style="margin-top:4px;">${groupBadge}</div>
                </div>
            </div>
            <div style="border: 1px solid var(--border-color);padding:6px;display:grid;grid-template-columns:1fr 1fr;gap:2px 8px;background:var(--bg-color);border-radius:4px;">
                <div class="info-row"><span class="lbl">Birthday / Age</span><span>${bdayStr} &nbsp;(${age !== null ? age + ' yrs' : '\u2014'})</span></div>
                <div class="info-row"><span class="lbl">Civil / Phone</span><span>${eCivil || '\u2014'} &nbsp;&bull;&nbsp; ${ePhone || '\u2014'}</span></div>
                <div class="info-row" style="grid-column:1/-1;"><span class="lbl">Spouse</span><span>${spouseSummary}</span></div>
                <div class="info-row" style="grid-column:1/-1;"><span class="lbl">Children (${childrenList.length})</span><span>${childrenSummary}</span></div>
                <div class="info-row" style="grid-column:1/-1;"><span class="lbl">Medical Condition</span><span style="color:#dc2626;font-weight:700;">${eMedical || 'None / Clear'}</span></div>
                <div class="info-row" style="grid-column:1/-1;"><span class="lbl">Work Residence</span><span>${eWorkRes || 'Not Specified'}</span></div>
                <div class="info-row" style="grid-column:1/-1;"><span class="lbl">Home Address</span><span>${eAddress || 'Not Specified'}</span></div>
                <div class="info-row" style="grid-column:1/-1;"><span class="lbl">Emergency Contact</span><span>${eEmergency || '\u2014'}</span></div>
            </div>
        </div>`;
    });

    const cardJson = JSON.stringify(cardHtmlList);

    const html = `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Personnel Info Cards \u2014 Service Section</title>
    <style>
        :root {
            --bg-color: hsl(210, 40%, 98%);
            --card-bg: #ffffff;
            --text-main: hsl(222, 47%, 11%);
            --text-muted: hsl(215, 16%, 47%);
            --border-color: hsl(214, 32%, 91%);
            --row-hover: hsl(210, 40%, 96%);
        }
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body { font-family: 'Inter', Arial, sans-serif; background: var(--bg-color); color: var(--text-main); padding: 24px; }
        .toolbar { display: flex; align-items: center; gap: 12px; margin-bottom: 24px; background: var(--card-bg); border: 1px solid var(--border-color); border-radius: 4px; padding: 12px 18px; flex-wrap: wrap; box-shadow: 0 2px 8px rgba(0,0,0,0.05); }
        .toolbar label { font-size: 0.82rem; font-weight: 700; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em; }
        .size-btn, .grid-btn { padding: 7px 16px; border-radius: 4px; border: 1.5px solid #cbd5e1; background: var(--bg-color); font-size: 0.8rem; font-weight: 700; color: #475569; cursor: pointer; }
        .size-btn.active, .grid-btn.active { background: #1e40af; color: white; border-color: #1e40af; }
        .sep { width: 1px; height: 24px; background: #e2e8f0; }
        .print-btn { margin-left: auto; padding: 9px 22px; background: #1e40af; color: white; border: none; border-radius: 4px; font-size: 0.88rem; font-weight: 700; cursor: pointer; }
        
        .print-page { display: grid; gap: 16px; margin-bottom: 24px; }
        .print-page.per-2 { grid-template-columns: 1fr; gap: 20px; }
        .print-page.per-4 { grid-template-columns: 1fr 1fr; gap: 12px; }
        .print-page.per-6 { grid-template-columns: 1fr 1fr; gap: 10px; }
        .print-page.per-8 { grid-template-columns: 1fr 1fr; gap: 8px; }
        
        .info-card { background: var(--card-bg); border: 1px solid var(--border-color); border-radius: 4px; padding: 12px; break-inside: avoid; page-break-inside: avoid; }
        .info-row { display: flex; flex-direction: column; font-size: 0.74rem; margin-bottom: 2px; }
        .lbl { font-weight: 700; color: var(--text-muted); font-size: 0.65rem; text-transform: uppercase; letter-spacing: 0.04em; margin-bottom: 1px; }
        #page-style {}
        @media print {
            * { -webkit-print-color-adjust: exact !important; print-color-adjust: exact !important; }
            .toolbar { display: none !important; }
            body { background: #ffffff !important; color: #0f172a !important; padding: 0 !important; margin: 0 auto !important; box-sizing: border-box !important; }
            .print-page {
                break-after: page !important;
                page-break-after: always !important;
                margin-bottom: 0 !important;
                padding-bottom: 2mm !important;
            }
            .print-page:last-child {
                break-after: auto !important;
                page-break-after: auto !important;
            }
            .print-page.per-2 {
                grid-template-columns: 1fr !important;
                gap: 18px !important;
            }
            .print-page.per-4, .print-page.per-6, .print-page.per-8 {
                grid-template-columns: 1fr 1fr !important;
                gap: 8px !important;
            }
            .print-page.per-4 .avatar-img, .print-page.per-6 .avatar-img, .print-page.per-8 .avatar-img {
                width: 52px !important;
                height: 52px !important;
                font-size: 1.4rem !important;
            }
            .print-page.per-4 .card-name, .print-page.per-6 .card-name, .print-page.per-8 .card-name {
                font-size: 0.92rem !important;
            }
            .print-page.per-4 .info-card, .print-page.per-6 .info-card, .print-page.per-8 .info-card {
                padding: 7px 9px !important;
            }
            .info-card { border: 1px dashed #94a3b8 !important; position: relative; break-inside: avoid !important; page-break-inside: avoid !important; }
            .info-card::after { content: ''; position: absolute; inset: -1px; border: 1px solid #cbd5e1; pointer-events: none; }
            img { image-rendering: -webkit-optimize-contrast; }
        }
    </style>
    <style id="page-style">
        @page { size: auto; margin: 8mm 10mm; }
    </style>
</head>
<body>
    <div class="toolbar">
        <label>Per Page:</label>
        <button class="grid-btn active" id="btn-2" onclick="setGrid(2)">2 per page</button>
        <button class="grid-btn" id="btn-4" onclick="setGrid(4)">4 per page</button>
        <button class="grid-btn" id="btn-6" onclick="setGrid(6)">6 per page</button>
        <button class="grid-btn" id="btn-8" onclick="setGrid(8)">8 per page</button>
        <button class="print-btn" onclick="window.print()">🖨 Print Cards</button>
    </div>
    <div id="cards-pages-container">
    </div>
    <div style="margin-top:24px;font-size:0.7rem;color: var(--text-muted);text-align:center;">
        Generated ${dateStr} : Service Section Personnel
    </div>
    <script>
        const cardsList = ${cardJson};
        let currentGrid = 2;

        function renderPages(n) {
            const container = document.getElementById('cards-pages-container');
            if (!container) return;
            let html = '';
            for (let i = 0; i < cardsList.length; i += n) {
                const chunk = cardsList.slice(i, i + n);
                html += '<div class="print-page per-' + n + '">' + chunk.join('') + '</div>';
            }
            container.innerHTML = html;
        }

        function setGrid(n) {
            currentGrid = n;
            renderPages(n);
            document.querySelectorAll('.grid-btn').forEach(b => b.classList.remove('active'));
            const b = document.getElementById('btn-' + n);
            if (b) b.classList.add('active');
        }

        renderPages(2);
    <\/script>
</body>
</html>`;

    const win = window.open('', '_blank');
    if (!win) { showToast('Popup blocked. Please allow popups and try again.'); return; }
    try {
        win.document.open();
        win.document.write(html);
        win.document.close();
    } catch (e) {
        console.error('Failed to open print window:', e);
        showToast('Failed to open print preview.');
        try { win.close(); } catch (_) {}
        return;
    }
    _printPopupThenClose(win);
}

const CATEGORIES = [
  { id:'personal', label:'Personal Info', fields:[
    { id:'name', label:'Name', required:true },
    { id:'aNumber', label:'A#' },
    { id:'photo', label:'Photo' },
    { id:'birthday', label:'Birthday & Age' },
    { id:'civilStatus', label:'Civil Status' },
    { id:'trade', label:'Trade' },
  ]},
  { id:'contact', label:'Contact', fields:[
    { id:'phone', label:'Phone Number' },
    { id:'address', label:'Home Address' },
    { id:'emergencyContact', label:'Emergency Contact Name' },
    { id:'emergencyPhone', label:'Emergency Phone' },
  ]},
  { id:'family', label:'Family', fields:[
    { id:'spouseName', label:'Spouse Name' },
    { id:'spouseBirthday', label:'Spouse Birthday' },
    { id:'spouseChurchDuty', label:'Spouse Church Duty' },
    { id:'weddingDate', label:'Wedding Anniversary' },
    { id:'children', label:'Children' },
  ]},
  { id:'work', label:'Work & Residence', fields:[
    { id:'tradeGroup', label:'Trade Group' },
    { id:'tradeRole', label:'Trade Role' },
    { id:'workResidence', label:'Work Residence' },
    { id:'assignedUnit', label:'Assigned Unit' },
  ]},
  { id:'project', label:'Project', fields:[
    { id:'projectGroup', label:'Project Group' },
    { id:'projectRole', label:'Project Role' },
    { id:'visitationHousing', label:'Visitation Housing' },
  ]},
  { id:'medical', label:'Medical & Health Clearance', fields:[
    { id:'medical', label:'Medical Condition' },
    { id:'bloodType', label:'Blood Type' },
    { id:'allergies', label:'Allergies' },
    { id:'clearanceStatus', label:'Health Clearance' },
  ]},
];

const FIELD_LABELS = {
  name:'Name', aNumber:'A#', photo:'Photo', birthday:'Birthday & Age',
  age:'Age', civilStatus:'Civil Status', trade:'Trade',
  area:'Area',
  phone:'Phone', address:'Home Address', emergencyContact:'Emergency Contact Name',
  emergencyPhone:'Emergency Phone',
  spouseName:'Spouse Name', spouseBirthday:'Spouse Birthday',
  spouseChurchDuty:'Spouse Church Duty', weddingDate:'Wedding Anniversary',
  children:'Children',
  tradeGroup:'Trade Group', tradeRole:'Trade Role', workResidence:'Work Residence',
  assignedUnit:'Assigned Unit',
  projectGroup:'Project Group', projectRole:'Project Role',
  visitationHousing:'Visitation Housing',
  medical:'Medical Condition',
  bloodType:'Blood Type',
  allergies:'Allergies',
  clearanceStatus:'Clearance Status'
};

const FIELD_WEIGHTS = {
  name:24, aNumber:5, photo:4, birthday:9, age:5, civilStatus:6, trade:9,
  area:12,
  phone:10, address:16, emergencyContact:12, emergencyPhone:10,
  spouseName:12, spouseBirthday:8, spouseChurchDuty:10, weddingDate:8, children:14,
  tradeGroup:10, tradeRole:7, workResidence:10, assignedUnit:7,
  projectGroup:10, projectRole:7, visitationHousing:8,
  medical:8, bloodType:6, allergies:10, clearanceStatus:10
};

function _fmtDate(d) {
  return typeof displayDate === 'function' ? displayDate(d) : d;
}

function getFieldValue(w, fid) {
  switch (fid) {
    case 'name': return w.name||'';
    case 'aNumber': return w.aNumber!=null ? String(w.aNumber) : '';
    case 'photo': return w.photoUrl||w.photo||'';
    case 'birthday': {
      if (!w.birthday) return '';
      const a = getAge(w.birthday);
      const ds = _fmtDate(w.birthday);
      return a!==null ? ds+' ('+a+' yrs)' : ds;
    }
    case 'area': return w.area||'';
    case 'age': {
      if (!w.birthday) return '';
      const a = getAge(w.birthday);
      return a !== null ? String(a) : '';
    }
    case 'civilStatus': return w.civilStatus||'';
    case 'trade': return w.trade||'';
    case 'phone': return w.phone||'';
    case 'address': return w.address||'';
    case 'emergencyContact': return w.emergencyContact||'';
    case 'emergencyPhone': return w.emergencyPhone||'';
    case 'spouseName': return w.spouseName||'';
    case 'spouseBirthday': return _fmtDate(w.spouseBirthday);
    case 'spouseChurchDuty': return w.spouseChurchDuty||'';
    case 'weddingDate': return _fmtDate(w.weddingDate);
    case 'children': {
      if (!w.children||!w.children.length) return '';
      return w.children.map(c => {
        const ca = c.birthday ? getAge(c.birthday) : (c.age||'');
        return (c.name||'') + (ca ? ' ('+ca+')' : '');
      }).join(', ');
    }
    case 'tradeGroup': return w.tradeGroup||'';
    case 'tradeRole': return w.tradeRole||'';
    case 'workResidence': return w.workResidence||'';
    case 'assignedUnit': return w.assignedUnit||'';
    case 'projectGroup': return w.projectGroup||'';
    case 'projectRole': return w.projectRole||'';
    case 'visitationHousing': return w.visitationHousing||'';
    case 'medical': return window.getCombinedMedicalCondition ? window.getCombinedMedicalCondition(w) : (w.medical || '');
    case 'bloodType': return w.bloodType || (w.healthClearance && w.healthClearance.bloodType) || '';
    case 'allergies': return Array.isArray(w.allergies) ? w.allergies.join(', ') : (w.healthClearance && Array.isArray(w.healthClearance.allergies) ? w.healthClearance.allergies.join(', ') : '');
    case 'clearanceStatus': {
      if (typeof getWorkerClearanceState === 'function') {
        return getWorkerClearanceState(w).label;
      }
      return (w.healthClearance && w.healthClearance.status) || 'CLEARED_FULL';
    }
    default: return '';
  }
}

function getColumnWidths(fields) {
  const t = fields.reduce((s,f) => s+(FIELD_WEIGHTS[f]||8), 0);
  return fields.map(f => ({ f, p: ((FIELD_WEIGHTS[f]||8)/t*100).toFixed(1) }));
}

function _openPrintWin(html) {
  const w = window.open('','_blank');
  if (!w) { showToast('Popup blocked. Please allow popups.'); return; }
  try {
    w.document.open();
    w.document.write(html);
    w.document.close();
  } catch (e) {
    console.error('Failed to write print document:', e);
    showToast('Failed to open print preview.');
    try { w.close(); } catch (_) {}
  }
}

function _printCSS() {
  return '/* ponytail: system fonts for offline print reliability */'+
    '*{box-sizing:border-box;margin:0;padding:0}'+
    'body{font-family:\'Inter\',-apple-system,BlinkMacSystemFont,\'Segoe UI\',Roboto,Arial,sans-serif;background:#f8fafc;color:#1e293b;padding:24px}'+
    'table{width:100%;border-collapse:collapse}'+
    '.pe-tb{display:flex;align-items:center;gap:12px;margin-bottom:24px;background:#fff;border:1px solid #e2e8f0;border-radius:2px;padding:12px 18px;flex-wrap:wrap;box-shadow:0 2px 8px rgba(0,0,0,0.05)}'+
    '.pe-tb label{font-size:0.82rem;font-weight:700;color:#64748b;text-transform:uppercase;letter-spacing:0.05em}'+
    '.pe-tb button{padding:7px 18px;border-radius:2px;border:1.5px solid #cbd5e1;background:#f8fafc;font-size:0.82rem;font-weight:700;color:#475569;cursor:pointer}'+
    '.pe-tb button.active{background:#1e40af;color:#fff;border-color:#1e40af}'+
    '.pe-tb .print-btn{margin-left:auto;padding:9px 24px;background:#1e40af;color:#fff;border:none;border-radius:2px;font-size:0.88rem;font-weight:700;cursor:pointer}'+
    '.pe-rpt{max-width:8.5in;margin:0 auto;background:#fff;border:1px solid #e2e8f0;border-radius:2px;overflow:visible;box-shadow:0 4px 12px rgba(0,0,0,0.05)}'+
    '.pe-hdr{background:#1e40af;color:#fff;padding:20px 24px;page-break-after:avoid}'+
    '.pe-hdr h1{font-size:1.3rem;font-weight:800;margin:0}'+
    '.pe-hdr .pe-sub{font-size:0.78rem;opacity:.85;margin-top:4px}'+
    '.pe-ft{padding:16px 24px;background:#f1f5f9;font-size:0.7rem;color:#64748b;text-align:center}'+
    '@media print{'+
      '*{-webkit-print-color-adjust:exact;print-color-adjust:exact}'+
      '.pe-tb{display:none!important}'+
      'body{background:#fff!important;color:#0f172a!important;padding: 0 !important; margin: 0 auto !important;box-sizing:border-box!important;width:100%!important}'+
      '.pe-rpt{overflow:visible!important;border:none!important;box-shadow:none!important;max-width:none!important}'+
      '[style*="overflow-x"]{overflow:visible!important}'+
      'thead{display:table-header-group!important}'+
      'tr{page-break-inside:avoid!important;break-inside:avoid!important}'+
      '.pe-cv-page{page-break-inside:avoid!important;break-inside:avoid!important}'+
      'img{image-rendering:-webkit-optimize-contrast}'+
    '}';
}

function generateTablePrintHTML(workers, fields, individual) {
  if (!fields.length) return '';
  const ds = new Date().toLocaleDateString('en-US',{year:'numeric',month:'long',day:'numeric'});
  const wg = getColumnWidths(fields);
  const toolbar = '<div class="pe-tb"><button class="print-btn" onclick="window.print()">🖨 Print Report</button></div>';
  const tdStyle = 'padding:7px 10px;font-size:0.78rem;border-bottom:1px solid #e2e8f0;word-break:break-word;overflow-wrap:break-word;white-space:normal';
  const thStyle = 'padding:8px 10px;font-size:0.68rem;font-weight:700;color:#475569;text-transform:uppercase;letter-spacing:0.04em;text-align:left;border-bottom:2px solid #cbd5e1;background:#f1f5f9';
  const colgroup = '<colgroup>'+wg.map(x=>'<col style="width:'+x.p+'%">').join('')+'</colgroup>';
  const thead = '<thead><tr>'+fields.map(f=>'<th style="'+thStyle+'">'+(FIELD_LABELS[f]||f)+'</th>').join('')+'</tr></thead>';

  function buildRow(w) {
    return '<tr>'+fields.map(f => {
      if (f==='photo') {
        const u = getFieldValue(w,'photo');
        return u ? '<td style="text-align:center;'+tdStyle+'"><img src="'+esc(u)+'" style="width:24px;height:24px;border-radius:50%;object-fit:cover"></td>' : '<td style="text-align:center;'+tdStyle+'">&mdash;</td>';
      }
      return '<td style="'+tdStyle+'">'+(esc(getFieldValue(w,f))||'&mdash;')+'</td>';
    }).join('')+'</tr>';
  }

  let body;
  if (individual) {
    body = workers.map((w, i) => {
      const eName = esc(w.name||'');
      return '<div style="'+(i>0?'page-break-before:always;':'')+'">'+
        '<div class="pe-hdr"><h1>Personnel Details</h1><div class="pe-sub">'+eName+' &nbsp;•&nbsp; '+ds+'</div></div>'+
        '<div style="padding:16px 20px"><table>'+colgroup+thead+'<tbody>'+buildRow(w)+'</tbody></table></div>'+
        '<div class="pe-ft">Worker '+(i+1)+' of '+workers.length+'</div></div>';
    }).join('');
  } else {
    body = '<div class="pe-hdr"><h1>Personnel Details Report</h1><div class="pe-sub">Service Section &nbsp;•&nbsp; '+ds+' &nbsp;•&nbsp; '+workers.length+' volunteer'+(workers.length!==1?'s':'')+'</div></div>'+
      '<div style="padding:16px 20px"><table>'+colgroup+thead+'<tbody>'+workers.map(w=>buildRow(w)).join('')+'</tbody></table></div>'+
      '<div class="pe-ft">Generated by Service 5 DB</div>';
  }
  return '<!DOCTYPE html><html><head><meta charset="UTF-8"><title>Personnel Details Report</title><style>'+
    _printCSS()+
  '</style><style id="pe-ps">@page{size:auto;margin:8mm 10mm}</style></head><body>'+
  toolbar+
  '<div class="pe-rpt">'+body+'</div>'+
  '</body></html>';
}

function generateProfilePrintHTML(workers, fields, individual) {
  if (!fields.length) return '';
  const ds = new Date().toLocaleDateString('en-US',{year:'numeric',month:'long',day:'numeric'});
  const catFields = {};
  CATEGORIES.forEach(c => { catFields[c.id] = c.fields.filter(f => fields.includes(f.id)).map(f => f.id); });
  const visibleCats = CATEGORIES.filter(c => catFields[c.id].length > 0);

  const pages = workers.map((w, idx) => {
    const eName = esc(w.name||'');
    const ePhoto = getFieldValue(w,'photo');
    const eTrade = esc(w.trade||'');
    const eANumber = esc(w.aNumber!=null?String(w.aNumber):'');
    const eAge = w.birthday ? (getAge(w.birthday)!==null ? getAge(w.birthday)+' yrs' : '') : '';
    const eCivil = esc(w.civilStatus||'');
    const ePhone = esc(w.phone||'');

    const photoHtml = (fields.includes('photo') && ePhoto)
      ? '<img src="'+esc(ePhoto)+'" class="pe-cv-photo">'
      : fields.includes('photo')
        ? '<div class="pe-cv-photo-fb">'+eName.charAt(0)+'</div>'
        : '';

    const badges = [];
    if (fields.includes('trade') && eTrade) badges.push(eTrade);
    if (fields.includes('aNumber') && eANumber) badges.push('A# '+eANumber);
    if (fields.includes('birthday') && eAge) badges.push(eAge);
    if (fields.includes('civilStatus') && eCivil) badges.push(eCivil);
    if (fields.includes('phone') && ePhone) badges.push(ePhone);
    const badgeHtml = badges.length
      ? '<div class="pe-cv-badges">'+badges.map(function(b){return '<span class="pe-cv-badge">'+b+'</span>';}).join('')+'</div>'
      : '';

    let secIdx = 0;
    const sectionsHtml = visibleCats.map(function(c){
      const fids = catFields[c.id];
      if (!fids.length) return '';
      secIdx++;
      const rows = fids.map(function(fid){
        if (fid === 'photo') return '';
        const val = getFieldValue(w,fid);
        return '<div class="pe-cv-row"><span class="pe-cv-lbl">'+(FIELD_LABELS[fid]||fid)+'</span><span class="pe-cv-val">'+(esc(val)||'&mdash;')+'</span></div>';
      }).join('');
      return '<div class="pe-cv-sec"><div class="pe-cv-st">'+secIdx+'. '+c.label.toUpperCase()+'</div>'+rows+'</div>';
    }).filter(function(s){return s;}).join('');

    const pageBreak = (individual || workers.length > 1) && idx > 0 ? 'page-break-before:always;' : '';

    return '<div class="pe-cv-page" style="'+pageBreak+'">'+
      '<div class="pe-cv-hdr">'+
        photoHtml+
        '<div style="flex:1;min-width:0"><div class="pe-cv-name">'+eName+'</div>'+badgeHtml+'</div>'+
      '</div>'+
      sectionsHtml+
      '<div class="pe-cv-ft">Service Section &middot; Generated '+ds+'</div>'+
    '</div>';
  }).join('');

  return '<!DOCTYPE html><html><head><meta charset="UTF-8"><title>Personnel Biodata / CV</title><style>'+
    _printCSS()+
    '.pe-cv-page{max-width:8.5in;margin:0 auto;padding:0.4in 0.5in}'+
    '.pe-cv-hdr{display:flex;gap:20px;align-items:center;padding-bottom:18px;border-bottom:2px solid #1e40af;margin-bottom:22px}'+
    '.pe-cv-photo{width:80px;height:80px;border-radius:50%;object-fit:cover;border:3px solid #e2e8f0;box-shadow:0 2px 8px rgba(0,0,0,0.08);flex-shrink:0}'+
    '.pe-cv-photo-fb{width:80px;height:80px;border-radius:50%;background:linear-gradient(135deg,#1e40af,#3b82f6);display:flex;align-items:center;justify-content:center;font-size:2.2rem;font-weight:800;color:#fff;flex-shrink:0}'+
    '.pe-cv-name{font-size:1.5rem;font-weight:800;color:#0f172a;line-height:1.2;margin-bottom:6px}'+
    '.pe-cv-badges{display:flex;flex-wrap:wrap;gap:4px 20px;font-size:0.8rem;color:#475569}'+
    '.pe-cv-badge{display:inline-block}'+
    '.pe-cv-badge::before{content:"\2022";color:#1e40af;margin-right:6px;font-weight:700}'+
    '.pe-cv-sec{margin-bottom:16px;padding-bottom:12px;border-bottom:1px solid #e2e8f0}'+
    '.pe-cv-sec:last-child{border-bottom:none;margin-bottom:0}'+
    '.pe-cv-st{font-size:0.7rem;font-weight:700;color:#1e40af;text-transform:uppercase;letter-spacing:0.08em;margin-bottom:6px;padding-bottom:3px;border-bottom:1px solid #e2e8f0}'+
    '.pe-cv-row{display:grid;grid-template-columns:160px 1fr;gap:2px 12px;padding:3px 0}'+
    '.pe-cv-lbl{font-size:0.73rem;font-weight:600;color:#64748b}'+
    '.pe-cv-val{font-size:0.82rem;color:#1e293b;word-break:break-word;overflow-wrap:break-word}'+
    '.pe-cv-ft{margin-top:20px;padding-top:10px;border-top:1px solid #e2e8f0;font-size:0.65rem;color:#94a3b8;text-align:center}'+
  '</style><style id="pe-ps">@page{size:auto;margin:8mm 10mm}</style></head><body>'+
  '<div class="pe-tb"><button class="print-btn" onclick="window.print()">🖨 Print CV / Biodata</button></div>'+
  pages+
  '</body></html>';
}

  function familyRows(w) {
    const rows = [];
    if (w.spouseName) {
      rows.push({
        name: w.spouseName, relationship: 'Spouse',
        birthday: _fmtDate(w.spouseBirthday),
        age: w.spouseBirthday ? (getAge(w.spouseBirthday) || '') : '',
        school: '', course: '', workplace: w.spouseChurchDuty || '', job: ''
      });
    }
    (w.children || []).forEach(function(c) {
      const ca = c.birthday ? (getAge(c.birthday) || c.age || '') : (c.age || '');
      rows.push({
        name: c.name || '', relationship: 'Child',
        birthday: _fmtDate(c.birthday), age: ca,
        school: c.school || '', course: c.course || '',
        workplace: c.workplace || '', job: c.job || '',
        churchDuty: c.churchDuty || ''
      });
    });
    return rows;
  }

  function familyPrintCSS() {
    return '/* ponytail: system fonts for offline print reliability */'+
      '*{box-sizing:border-box;margin:0;padding:0}'+
      'body{font-family:\'Inter\',-apple-system,BlinkMacSystemFont,\'Segoe UI\',Roboto,Arial,sans-serif;background:#f8fafc;color:#1e293b;padding:32px}'+
      '.fp-hdr{border-bottom:2px solid #1e40af;padding-bottom:16px;margin-bottom:24px}'+
      '.fp-hdr h1{font-size:1.5rem;font-weight:800;color:#0f172a}'+
      '.fp-hdr .fp-sub{font-size:0.78rem;color:#64748b;margin-top:4px}'+
      '.fp-sec{margin-bottom:24px}'+
      '.fp-sec-h{font-size:0.75rem;font-weight:700;color:#1e40af;text-transform:uppercase;letter-spacing:0.08em;border-bottom:1px solid #e2e8f0;padding-bottom:6px;margin-bottom:12px}'+
      '.fp-g{display:grid;grid-template-columns:140px 1fr;gap:6px 12px}'+
      '.fp-ph{display:flex;gap:20px;align-items:center;margin-bottom:24px;padding-bottom:20px;border-bottom:2px solid #1e40af}'+
      '.fp-ph-img{width:72px;height:72px;border-radius:50%;object-fit:cover;border:3px solid #e2e8f0;flex-shrink:0}'+
      '.fp-ph-img-fb{width:72px;height:72px;border-radius:50%;background:linear-gradient(135deg,#1e40af,#3b82f6);display:flex;align-items:center;justify-content:center;font-size:2rem;font-weight:800;color:#fff;flex-shrink:0}'+
      '.fp-ph-info{flex:1;min-width:0}'+
      '.fp-ph-name{font-size:1.4rem;font-weight:800;color:#0f172a;line-height:1.2;margin-bottom:4px}'+
      '.fp-ph-badges{display:flex;flex-wrap:wrap;gap:4px 20px;font-size:0.8rem;color:#475569}'+
      '.fp-ph-badge{display:inline-block}'+
      '.fp-ph-badge::before{content:"\\2022";color:#1e40af;margin-right:6px;font-weight:700}'+
      '.fp-l{font-size:0.73rem;font-weight:600;color:#64748b}'+
      '.fp-v{font-size:0.82rem;color:#1e293b}'+
      'table{width:100%;border-collapse:collapse;font-size:0.78rem}'+
      'th{padding:8px 10px;font-size:0.68rem;font-weight:700;color:#475569;text-transform:uppercase;letter-spacing:0.04em;text-align:left;border-bottom:2px solid #cbd5e1;background:#f1f5f9}'+
      'td{padding:7px 10px;border-bottom:1px solid #e2e8f0}'+
      '.fp-ft{margin-top:24px;padding-top:12px;border-top:1px solid #e2e8f0;font-size:0.65rem;color:#94a3b8;text-align:center}'+
      '@media print{*{-webkit-print-color-adjust:exact;print-color-adjust:exact}body{background:#fff;padding:0}}';
  }

  function familyReportHTML(index) {
    const w = workersData[index];
    if (!w) return '';
    const ds = new Date().toLocaleDateString('en-US',{year:'numeric',month:'long',day:'numeric'});
    const eName = esc(w.name||'');
    const wd = w.weddingDate ? _fmtDate(w.weddingDate) : '';
    const rows = familyRows(w);

    function photoBlock() {
      var photoUrl = w.photoUrl || w.photo;
      if (photoUrl) { return '<img class="fp-ph-img" src="'+esc(photoUrl)+'" alt="">'; }
      var initials = (w.name||' ').match(/\b\w/g);
      var iniStr = initials ? initials.slice(0,2).join('').toUpperCase() : '?';
      return '<div class="fp-ph-img-fb">'+iniStr+'</div>';
    }

    function badgeItem(label, val) {
      return val ? '<span class="fp-ph-badge">'+esc(label)+': '+esc(val)+'</span>' : '';
    }

    var ageStr = w.birthday ? getAge(w.birthday) : '';
    var badgesHtml = [
      badgeItem('Trade', w.trade||w.category),
      badgeItem('Age', ageStr ? ageStr+' yrs' : ''),
      badgeItem('Civil Status', w.civilStatus),
      badgeItem('Phone', w.phone),
      badgeItem('Address', w.address)
    ].filter(Boolean).join('');

    const photoHeader = '<div class="fp-ph">'+
      photoBlock()+
      '<div class="fp-ph-info">'+
        '<div class="fp-ph-name">'+eName+'</div>'+
        (badgesHtml ? '<div class="fp-ph-badges">'+badgesHtml+'</div>' : '')+
      '</div></div>';

    const spouseSection = w.spouseName
      ? '<div class="fp-sec"><div class="fp-sec-h">Spouse</div><div class="fp-g">'+
        '<span class="fp-l">Name</span><span class="fp-v">'+esc(w.spouseName)+'</span>'+
        '<span class="fp-l">Birthday</span><span class="fp-v">'+(w.spouseBirthday ? _fmtDate(w.spouseBirthday)+(w.spouseBirthday ? ' ('+(getAge(w.spouseBirthday)||'')+' yrs)' : '') : '&mdash;')+'</span>'+
        '<span class="fp-l">Church Duty</span><span class="fp-v">'+esc(w.spouseChurchDuty||'&mdash;')+'</span>'+
        (wd ? '<span class="fp-l">Wedding</span><span class="fp-v">'+wd+'</span>' : '')+
      '</div></div>'
      : '';

    const childrenTable = rows.filter(function(r){return r.relationship==='Child';}).length > 0
      ? '<div class="fp-sec"><div class="fp-sec-h">Children & Dependents</div>'+
        '<table><thead><tr>'+
        '<th>Name</th><th>Birthday</th><th>Age</th><th>School</th><th>Course</th><th>Workplace</th><th>Church Duty</th>'+
        '</tr></thead><tbody>'+
        rows.filter(function(r){return r.relationship==='Child';}).map(function(r){
          return '<tr><td>'+esc(r.name)+'</td><td>'+esc(r.birthday)+'</td><td>'+esc(r.age)+'</td><td>'+esc(r.school)+'</td><td>'+esc(r.course)+'</td><td>'+(esc(r.workplace||r.job)||'&mdash;')+'</td><td>'+esc(r.churchDuty||'&mdash;')+'</td></tr>';
        }).join('')+
        '</tbody></table></div>'
      : '<div class="fp-sec"><div class="fp-sec-h">Children & Dependents</div><p style="font-size:0.78rem;color:#94a3b8">No children listed.</p></div>';

    return '<!DOCTYPE html><html><head><meta charset="UTF-8"><title>Family Information &mdash; '+eName+'</title><style>'+familyPrintCSS()+'</style></head><body>'+
      '<div class="fp-hdr"><h1>Family Information</h1><div class="fp-sub">'+eName+' &nbsp;\u2022&nbsp; '+ds+'</div></div>'+
      photoHeader+
      spouseSection+
      childrenTable+
      '<div class="fp-ft">Generated by Service 5 DB</div>'+
      '</body></html>';
  }

function printWorkerAttendanceDossier(targetWorker) {
    let worker = targetWorker;
    if (!worker) {
        const idxInput = document.getElementById('personal-index');
        const idx = idxInput ? parseInt(idxInput.value, 10) : NaN;
        if (!isNaN(idx) && window.workersData && window.workersData[idx]) {
            worker = window.workersData[idx];
        }
    }
    if (!worker) {
        if (typeof showToast === 'function') showToast('No volunteer selected for dossier printout.', 'error');
        return;
    }

    if (typeof window.ensureWorkerCanonical === 'function') {
        worker = window.ensureWorkerCanonical(worker);
    }

    const escapeHtml = typeof window.esc === 'function' 
        ? window.esc 
        : (s) => String(s || '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);

    const fullName = worker.personalInfo?.fullName || worker.name || 'Unnamed Volunteer';
    const aNumber = worker.aNumber || worker.id || '-';
    const tradesList = (worker.trades && worker.trades.length ? worker.trades : [worker.trade]).filter(Boolean);
    const tradesDisplay = tradesList.length ? tradesList.join(', ') : 'General Worker';
    const groupsList = (worker.groups && worker.groups.length ? worker.groups : [worker.projectGroup]).filter(Boolean);
    const groupsDisplay = groupsList.length ? groupsList.join(', ') : 'Unassigned';
    
    
    const printDate = window.DateService ? window.DateService.getLocalCalendarDate() : new Date().toISOString().slice(0, 10);
    const printTimestamp = window.DateService ? window.DateService.getNowTimestamp() : new Date().toLocaleString();

    // Attendance History & Statistics
    const history = Array.isArray(worker.attendanceHistory) ? [...worker.attendanceHistory] : [];
    // Sort descending by date, then timestamp
    history.sort((a, b) => {
        const da = a.date || (a.timestamp ? String(a.timestamp).slice(0, 10) : '');
        const db = b.date || (b.timestamp ? String(b.timestamp).slice(0, 10) : '');
        if (da !== db) return db.localeCompare(da);
        return (new Date(b.timestamp || 0).getTime()) - (new Date(a.timestamp || 0).getTime());
    });

    let presentCount = 0;
    let clinicCount = 0;
    let passCount = 0;
    let dayoffCount = 0;
    let absentCount = 0;
    let otherCount = 0;

    history.forEach(entry => {
        const st = (entry.status || entry.newStatus || '').trim().toLowerCase();
        const tp = (entry.type || '').toLowerCase();
        if (tp === 'clinic_visit' || st.includes('clinic')) clinicCount++;
        else if (tp === 'medical_quarantine' || st.includes('medical')) passCount++;
        else if (st.includes('present') || st === 'p') presentCount++;
        else if (st.includes('pass') || st.includes('excused')) passCount++;
        else if (st.includes('off') || st.includes('day-off')) dayoffCount++;
        else if (st.includes('absent') || st === 'a') absentCount++;
        else otherCount++;
    });

    const dutyLogs = history.filter(e => e.type !== 'clinic_visit');
    const totalLogs = dutyLogs.length;
    const reliabilityPct = totalLogs > 0 ? ((presentCount / totalLogs) * 100).toFixed(1) : '100.0';

    function getStatusBadge(rawStatus, entry = {}) {
        const s = (rawStatus || '').trim();
        const sl = s.toLowerCase();
        const tp = (entry.type || '').toLowerCase();
        let bg = '#f1f5f9', color = '#475569', border = '#cbd5e1', icon = '';
        if (tp === 'clinic_visit' || sl.includes('clinic')) {
            bg = '#fce7f3'; color = '#be185d'; border = '#fbcfe8'; icon = '🏥 ';
        } else if (tp === 'medical_quarantine' || sl.includes('medical')) {
            bg = '#ede9fe'; color = '#6d28d9'; border = '#ddd6fe'; icon = '🩺 ';
        } else if (sl.includes('present') || sl === 'p') {
            bg = '#dcfce7'; color = '#15803d'; border = '#86efac';
        } else if (sl.includes('pass') || sl.includes('excused')) {
            bg = '#e0e7ff'; color = '#3730a3'; border = '#a5b4fc';
        } else if (sl.includes('off') || sl.includes('day-off')) {
            bg = '#f3f4f6'; color = '#4b5563'; border = '#d1d5db';
        } else if (sl.includes('absent') || sl === 'a') {
            bg = '#fee2e2'; color = '#b91c1c'; border = '#fca5a5';
        }
        return `<span style="display:inline-block;padding:2px 8px;border-radius:4px;font-size:0.75rem;font-weight:600;background:${bg};color:${color};border:1px solid ${border};">${icon}${escapeHtml(s || 'Logged')}</span>`;
    }

    function formatTimeCol(entry) {
        if (!entry.timestamp) return '-';
        if (typeof entry.timestamp === 'string' && entry.timestamp.includes('T')) {
            try {
                const d = new Date(entry.timestamp);
                return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: true });
            } catch (e) {
                return entry.timestamp;
            }
        }
        if (typeof entry.timestamp === 'number') {
            try {
                const d = new Date(entry.timestamp);
                return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: true });
            } catch (e) {
                return String(entry.timestamp);
            }
        }
        return String(entry.timestamp);
    }

    const rowsHtml = history.length === 0
        ? `<tr><td colspan="6" style="text-align:center;padding:28px 14px;color:#94a3b8;font-style:italic;">No attendance records on file for this volunteer.</td></tr>`
        : history.map(entry => {
            const dateStr = entry.date || (entry.timestamp ? String(entry.timestamp).slice(0, 10) : '-');
            const shiftStr = entry.shift || 'General / Day';
            const statusStr = entry.status || entry.newStatus || 'Present';
            const timeStr = formatTimeCol(entry);
            const verifiedBy = entry.verifiedBy || entry.encoder || 'System / Auto';
            let notesStr = entry.notes || entry.reason || '-';
            if (entry.type === 'clinic_visit') {
                notesStr = (entry.reason || 'Clinic Consultation') + (entry.notes ? ` (${entry.notes})` : '');
            }
            return `
                <tr style="border-bottom: 1px solid #f1f5f9; ${entry.type === 'clinic_visit' ? 'background:#fff5f7;' : ''}">
                    <td style="padding: 8px 12px; font-weight: 600; font-family: monospace; font-size: 0.82rem; color: #1e293b;">${escapeHtml(dateStr)}</td>
                    <td style="padding: 8px 12px; font-size: 0.8rem; color: #475569;">${escapeHtml(shiftStr)}</td>
                    <td style="padding: 8px 12px;">${getStatusBadge(statusStr, entry)}</td>
                    <td style="padding: 8px 12px; font-size: 0.8rem; color: #475569; font-family: monospace;">${escapeHtml(timeStr)}</td>
                    <td style="padding: 8px 12px; font-size: 0.8rem; color: #475569;">${escapeHtml(verifiedBy)}</td>
                    <td style="padding: 8px 12px; font-size: 0.8rem; color: #64748b;">${escapeHtml(notesStr)}</td>
                </tr>
            `;
        }).join('');

    const html = `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Attendance Dossier : ${escapeHtml(fullName)} (${escapeHtml(aNumber)})</title>
    <style>
        @page {
            size: portrait;
            margin: 14mm 14mm 16mm 14mm;
        }
        * {
            box-sizing: border-box;
            margin: 0;
            padding: 0;
            -webkit-print-color-adjust: exact !important;
            print-color-adjust: exact !important;
        }
        body {
            font-family: 'Inter', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            color: #0f172a;
            background: #ffffff;
            line-height: 1.5;
            font-size: 13px;
            padding: 24px;
        }
        .header {
            display: flex;
            justify-content: space-between;
            align-items: flex-start;
            border-bottom: 2px solid #0f172a;
            padding-bottom: 14px;
            margin-bottom: 18px;
        }
        .org-title {
            font-size: 9px;
            font-weight: 700;
            text-transform: uppercase;
            letter-spacing: 1.2px;
            color: #64748b;
            margin-bottom: 4px;
        }
        .doc-title {
            font-size: 20px;
            font-weight: 800;
            color: #0f172a;
            letter-spacing: -0.3px;
        }
        .doc-meta {
            text-align: right;
            font-size: 11px;
            color: #64748b;
        }
        .profile-card {
            display: grid;
            grid-template-columns: 2fr 1fr;
            gap: 16px;
            background: #f8fafc;
            border: 1px solid #e2e8f0;
            border-radius: 6px;
            padding: 14px 18px;
            margin-bottom: 18px;
        }
        .profile-name {
            font-size: 16px;
            font-weight: 700;
            color: #0f172a;
            margin-bottom: 4px;
        }
        .profile-field {
            font-size: 11.5px;
            color: #475569;
            margin-bottom: 2px;
        }
        .profile-field strong {
            color: #0f172a;
        }
        .kpi-grid {
            display: grid;
            grid-template-columns: repeat(6, 1fr);
            gap: 10px;
            margin-bottom: 20px;
        }
        .kpi-card {
            background: #ffffff;
            border: 1px solid #e2e8f0;
            border-radius: 6px;
            padding: 10px;
            text-align: center;
        }
        .kpi-val {
            font-size: 18px;
            font-weight: 800;
            color: #0f172a;
        }
        .kpi-label {
            font-size: 10px;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            color: #64748b;
            margin-top: 2px;
        }
        .section-title {
            font-size: 13px;
            font-weight: 700;
            color: #0f172a;
            text-transform: uppercase;
            letter-spacing: 0.6px;
            margin-bottom: 8px;
            display: flex;
            align-items: center;
            gap: 6px;
        }
        table.dossier-table {
            width: 100%;
            border-collapse: collapse;
            font-size: 12px;
            margin-bottom: 24px;
            border: 1px solid #e2e8f0;
            border-radius: 4px;
        }
        table.dossier-table th {
            background: #f8fafc;
            border-bottom: 1.5px solid #cbd5e1;
            padding: 8px 12px;
            font-size: 10.5px;
            font-weight: 700;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            color: #475569;
            text-align: left;
        }
        table.dossier-table tr {
            page-break-inside: avoid;
        }
        .signoff {
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 40px;
            margin-top: 32px;
            page-break-inside: avoid;
        }
        .sign-line {
            border-bottom: 1.5px solid #0f172a;
            margin-top: 40px;
            margin-bottom: 4px;
        }
        .sign-label {
            font-size: 11px;
            font-weight: 600;
            color: #475569;
            text-align: center;
        }
        .footer-note {
            margin-top: 24px;
            font-size: 10px;
            color: #94a3b8;
            text-align: center;
            border-top: 1px solid #f1f5f9;
            padding-top: 8px;
        }
        @media print {
            body { padding: 0; }
        }
    </style>
</head>
<body>
    <div class="header">
        <div>
            <div class="org-title">AGWC Volunteer Workforce &bull; Official Dossier</div>
            <h1 class="doc-title">Attendance Dossier</h1>
        </div>
        <div class="doc-meta">
            <div><strong>Generated:</strong> ${escapeHtml(printDate)}</div>
            <div><strong>Timestamp:</strong> ${escapeHtml(printTimestamp)}</div>
            <div><strong>Schema:</strong> Canonical v2 Document</div>
        </div>
    </div>

    <div class="profile-card">
        <div>
            <div class="profile-name">${escapeHtml(fullName)}</div>
            <div class="profile-field"><strong>A-Number:</strong> #${escapeHtml(aNumber)}</div>
            <div class="profile-field"><strong>Trade / Skill:</strong> ${escapeHtml(tradesDisplay)}</div>
            <div class="profile-field"><strong>Project Group:</strong> ${escapeHtml(groupsDisplay)}</div>
        </div>
        <div style="text-align: right;">
            <div class="profile-field"><strong>Medical Clearance:</strong> ${escapeHtml(getWorkerClearanceState(worker).label)}</div>
            <div class="profile-field"><strong>Restrictions:</strong> ${(Array.isArray(worker.restrictions) && worker.restrictions.length) ? escapeHtml(worker.restrictions.join(', ')) : 'None'}</div>
            <div class="profile-field" style="margin-top: 4px; font-weight: 700; color: #0284c7;">Reliability: ${reliabilityPct}%</div>
        </div>
    </div>

    <div class="kpi-grid">
        <div class="kpi-card" style="border-top: 3px solid #0284c7;">
            <div class="kpi-val">${totalLogs}</div>
            <div class="kpi-label">Duty Shifts</div>
        </div>
        <div class="kpi-card" style="border-top: 3px solid #16a34a;">
            <div class="kpi-val" style="color: #16a34a;">${presentCount}</div>
            <div class="kpi-label">Days Present</div>
        </div>
        <div class="kpi-card" style="border-top: 3px solid #be185d;">
            <div class="kpi-val" style="color: #be185d;">${clinicCount}</div>
            <div class="kpi-label">Clinic Visits</div>
        </div>
        <div class="kpi-card" style="border-top: 3px solid #4f46e5;">
            <div class="kpi-val" style="color: #4f46e5;">${passCount}</div>
            <div class="kpi-label">Pass / Excused</div>
        </div>
        <div class="kpi-card" style="border-top: 3px solid #64748b;">
            <div class="kpi-val" style="color: #64748b;">${dayoffCount}</div>
            <div class="kpi-label">Day-offs</div>
        </div>
        <div class="kpi-card" style="border-top: 3px solid #dc2626;">
            <div class="kpi-val" style="color: #dc2626;">${absentCount}</div>
            <div class="kpi-label">Absences</div>
        </div>
    </div>

    <div class="section-title">
        <span>📋</span> Chronological Attendance Ledger (${totalLogs} ${totalLogs === 1 ? 'entry' : 'entries'})
    </div>

    <table class="dossier-table">
        <thead>
            <tr>
                <th style="width: 14%;">Date</th>
                <th style="width: 14%;">Shift</th>
                <th style="width: 14%;">Status</th>
                <th style="width: 14%;">Time In</th>
                <th style="width: 20%;">Verified By</th>
                <th style="width: 24%;">Notes / Reason</th>
            </tr>
        </thead>
        <tbody>
            ${rowsHtml}
        </tbody>
    </table>

    <div class="signoff">
        <div>
            <div class="sign-line"></div>
            <div class="sign-label">Prepared &amp; Exported By (Encoder / Attendance Clerk)</div>
        </div>
        <div>
            <div class="sign-line"></div>
            <div class="sign-label">Verified &amp; Approved By (Site Supervisor / Project Lead)</div>
        </div>
    </div>

    <div class="footer-note">
        This document represents an authoritative attendance dossier generated by the AGWC Worker Management System. All records are bound to the local canonical store and audited per device calendar timeline.
    </div>
</body>
</html>`;

    if (typeof window.safePrint === 'function') {
        window.safePrint(html, {
            title: `Attendance_Dossier_${aNumber}_${fullName.replace(/\s+/g, '_')}`,
            orientation: 'portrait'
        });
    } else {
        const win = window.open('', '_blank');
        if (win) {
            win.document.open();
            win.document.write(html);
            win.document.close();
            win.focus();
            setTimeout(() => { win.print(); win.close(); }, 300);
        }
    }
}

// verbatim: the body of dashboard.js's handler
function printPresentRoster(presentWorkers) {
            const ds = new Date().toLocaleDateString('en-US', { year: 'numeric', month: 'long', day: 'numeric' });
            const areas = {};
            presentWorkers.forEach(w => {
                const area = (w.area || 'UNASSIGNED').trim().toUpperCase();
                if (!areas[area]) areas[area] = [];
                areas[area].push(w);
            });
            const sortedAreas = Object.keys(areas).sort((a, b) => {
                const cmp = window.areaPriorityComparator(a, b);
                return cmp;
            });

            const timeHeaders = ['Time In 1', 'Time Out 1', 'Time In 2', 'Time Out 2', 'Time In 3', 'Time Out 3']
                .map(l => '<th class="time-col" style="padding:7px 5px;font-size:0.62rem;font-weight:700;color:#475569;text-transform:uppercase;letter-spacing:0.03em;text-align:left;border-bottom:2px solid #cbd5e1;background:#f1f5f9;white-space:nowrap;">' + l + '</th>').join('');

            const areaTables = sortedAreas.map(areaName => {
                const list = areas[areaName];
                const rows = list.map((w, i) => {
                    const avatar = (w.photoUrl || w.photo)
                        ? '<img src="' + esc(w.photoUrl || w.photo) + '" style="width:22px;height:22px;border-radius:50%;object-fit:cover;">'
                        : '<span style="font-size:0.6rem;color:#94a3b8;">-</span>';
                    const age = w.birthday ? (getAge(w.birthday) !== null ? getAge(w.birthday) : '') : '';
                    return '<tr>' +
                        '<td style="padding:5px 6px;border-bottom:1px solid #e2e8f0;text-align:center;font-size:0.7rem;color:#64748b;font-weight:600;width:24px;">' + (i + 1) + '</td>' +
                        '<td style="padding:5px 6px;border-bottom:1px solid #e2e8f0;text-align:center;width:30px;">' + avatar + '</td>' +
                        '<td style="padding:5px 6px;border-bottom:1px solid #e2e8f0;font-size:0.78rem;font-weight:600;">' + esc(w.name) + '</td>' +
                        '<td style="padding:5px 6px;border-bottom:1px solid #e2e8f0;font-size:0.72rem;color:#475569;">' + esc(w.aNumber != null ? String(w.aNumber) : '') + '</td>' +
                        '<td style="padding:5px 6px;border-bottom:1px solid #e2e8f0;font-size:0.72rem;">' + esc(w.trade || '-') + '</td>' +
                        '<td style="padding:5px 6px;border-bottom:1px solid #e2e8f0;font-size:0.72rem;text-align:center;">' + age + '</td>' +
                        '<td class="time-col" style="padding:5px 6px;border-bottom:1px solid #e2e8f0;"></td>' +
                        '<td class="time-col" style="padding:5px 6px;border-bottom:1px solid #e2e8f0;"></td>' +
                        '<td class="time-col" style="padding:5px 6px;border-bottom:1px solid #e2e8f0;"></td>' +
                        '<td class="time-col" style="padding:5px 6px;border-bottom:1px solid #e2e8f0;"></td>' +
                        '<td class="time-col" style="padding:5px 6px;border-bottom:1px solid #e2e8f0;"></td>' +
                        '<td class="time-col" style="padding:5px 6px;border-bottom:1px solid #e2e8f0;"></td>' +
                        '</tr>';
                }).join('');

                return '<div style="margin-bottom:24px;page-break-inside:avoid;break-inside:avoid;">' +
                    '<h3 style="font-size:0.95rem;font-weight:800;color:#1e40af;margin:0 0 6px 0;padding-bottom:4px;border-bottom:2px solid #1e40af;">AREA: ' + esc(areaName) + ' <span style="font-weight:400;font-size:0.78rem;color:#64748b;">(' + list.length + ')</span></h3>' +
                    '<table style="width:100%;border-collapse:collapse;font-size:0.82rem;">' +
                    '<thead><tr>' +
                    '<th style="padding:7px 6px;font-size:0.62rem;font-weight:700;color:#475569;text-transform:uppercase;letter-spacing:0.03em;text-align:center;border-bottom:2px solid #cbd5e1;background:#f1f5f9;width:24px;">#</th>' +
                    '<th style="padding:7px 6px;font-size:0.62rem;font-weight:700;color:#475569;text-transform:uppercase;letter-spacing:0.03em;text-align:left;border-bottom:2px solid #cbd5e1;background:#f1f5f9;width:30px;">Photo</th>' +
                    '<th style="padding:7px 6px;font-size:0.62rem;font-weight:700;color:#475569;text-transform:uppercase;letter-spacing:0.03em;text-align:left;border-bottom:2px solid #cbd5e1;background:#f1f5f9;">Fullname</th>' +
                    '<th style="padding:7px 6px;font-size:0.62rem;font-weight:700;color:#475569;text-transform:uppercase;letter-spacing:0.03em;text-align:left;border-bottom:2px solid #cbd5e1;background:#f1f5f9;">A#</th>' +
                    '<th style="padding:7px 6px;font-size:0.62rem;font-weight:700;color:#475569;text-transform:uppercase;letter-spacing:0.03em;text-align:left;border-bottom:2px solid #cbd5e1;background:#f1f5f9;">Trade</th>' +
                    '<th style="padding:7px 6px;font-size:0.62rem;font-weight:700;color:#475569;text-transform:uppercase;letter-spacing:0.03em;text-align:center;border-bottom:2px solid #cbd5e1;background:#f1f5f9;width:30px;">Age</th>' +
                    timeHeaders +
                    '</tr></thead>' +
                    '<tbody>' + rows + '</tbody>' +
                    '</table></div>';
            }).join('');

            const html = '<!DOCTYPE html><html><head><meta charset="UTF-8"><title>Present Volunteers Attendance Sheet</title><style>' +
                _printCSS() +
                'thead th{position:sticky;top:0;z-index:2}' +
                'table{page-break-inside:auto}' +
                '</style><style id="pe-ps">@page{size:auto;margin:8mm 10mm}</style></head><body>' +
                '<div class="pe-tb">' +
                '<label style="display:flex;align-items:center;gap:4px;font-size:0.82rem;cursor:pointer;"><input type="checkbox" id="toggle-times" checked onchange="toggleTimeCols(this.checked)"> Show Time Log Columns</label>' +
                '<button class="print-btn" onclick="window.print()">🖨 Print</button></div>' +
                '<div class="pe-rpt">' +
                '<div class="pe-hdr"><h1>Present Volunteers Attendance Sheet</h1><div class="pe-sub">Service Section &nbsp;•&nbsp; ' + ds + ' &nbsp;•&nbsp; ' + presentWorkers.length + ' volunteer' + (presentWorkers.length !== 1 ? 's' : '') + '</div></div>' +
                '<div style="padding:12px 16px;">' + areaTables + '</div>' +
                '<div class="pe-ft">Generated by Service 5 DB</div>' +
                '</div>' +
                '<script>function toggleTimeCols(v){document.querySelectorAll(".time-col").forEach(function(e){e.style.display=v?"":"none"})}<' + '/script></body></html>';
            safePrint(html);
        }
return {
    printPersonnelReport: printPersonnelReport,
    printVolunteerFamilyRoster: printVolunteerFamilyRoster,
    printSelectedPersonnelCards: printSelectedPersonnelCards,
    generateTablePrintHTML: generateTablePrintHTML,
    generateProfilePrintHTML: generateProfilePrintHTML,
    _openPrintWin: _openPrintWin,
    familyReportHTML: familyReportHTML,
    printWorkerAttendanceDossier: printWorkerAttendanceDossier,
    printPresentRoster: printPresentRoster
};
})();

// ===== table.js (history pages) - verbatim from 01_Rust_WC_DB/js/table.js =====
var S5_table = (function () {

function showWorkerAttendancePopup(workerName) {
    const eName = esc(workerName);
    const worker = workersData.find(w => w.name === workerName);
    const ePhoto = worker && (worker.photoUrl || worker.photo) ? esc(worker.photoUrl || worker.photo) : '';
    const avatarHtml = worker && (worker.photoUrl || worker.photo)
        ? `<img src="${ePhoto}" style="width:36px;height:36px;border-radius:50%;object-fit:cover;border:2px solid #e2e8f0;flex-shrink:0;">`
        : `<div style="width:36px;height:36px;border-radius:50%;background:linear-gradient(135deg,#3b82f6,#1d4ed8);display:flex;align-items:center;justify-content:center;font-size:1rem;font-weight:800;color:white;flex-shrink:0;">${eName.charAt(0)}</div>`;

    let allLogs = (attendanceLog || []).filter(e => e && e.workerName === workerName);
    const allSnapshots = queryWorkerSnapshots(workerName);

    // Merge worker.attendanceHistory into change logs if worker exists
    if (worker && Array.isArray(worker.attendanceHistory)) {
        worker.attendanceHistory.forEach(entry => {
            const dateStr = entry.date || (entry.timestamp ? String(entry.timestamp).slice(0, 10) : '');
            let tsNum = 0;
            if (typeof entry.timestamp === 'number' && !isNaN(entry.timestamp)) tsNum = entry.timestamp;
            else if (entry.timestamp) tsNum = new Date(entry.timestamp).getTime() || 0;
            else if (entry.date) tsNum = new Date(entry.date + 'T08:00:00').getTime() || 0;

            const statusStr = entry.status || entry.newStatus || 'Present';
            const reasonStr = entry.reason || entry.notes || (entry.type === 'clinic_visit' ? 'Clinic Consultation' : '');
            const alreadyInLog = allLogs.some(l => l.date === dateStr && l.newStatus === statusStr);
            if (!alreadyInLog) {
                allLogs.push({
                    date: dateStr,
                    timestamp: tsNum,
                    workerName: worker.name,
                    newStatus: statusStr,
                    oldStatus: entry.oldStatus || '-',
                    reason: reasonStr,
                    verifiedBy: entry.verifiedBy || entry.encoder || 'System'
                });
            }
        });
    }

    // Sort descending by default for rendering
    allLogs.sort((a, b) => {
        const ta = typeof a.timestamp === 'number' ? a.timestamp : new Date(a.timestamp || a.date || 0).getTime();
        const tb = typeof b.timestamp === 'number' ? b.timestamp : new Date(b.timestamp || b.date || 0).getTime();
        return tb - ta;
    });
    allSnapshots.sort((a, b) => {
        const ta = typeof a.timestamp === 'number' ? a.timestamp : new Date(a.timestamp || a.date || 0).getTime();
        const tb = typeof b.timestamp === 'number' ? b.timestamp : new Date(b.timestamp || b.date || 0).getTime();
        return tb - ta;
    });

    const currentStatus = worker ? esc(worker.dailyStatus) : 'Unknown';
    const curStatusClass = worker && worker.dailyStatus === 'Present' ? 'background:#ecfdf5;color:#047857;border-color:#a7f3d0;' : worker && worker.dailyStatus === 'Absent' ? 'background:#fef2f2;color:#b91c1c;border-color:#fca5a5;' : 'background:#fffbeb;color:#b45309;border-color:#fde68a;';

    const html = `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Attendance History &mdash; ${eName}</title>
    <style>
        /* ponytail: removed Google Fonts @import for offline reliability */
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body { font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; padding: 24px; background: #fbfbfa; color: #37352f; line-height: 1.5; }
        .container { max-width: 1000px; margin: 0 auto; }
        .toolbar { display:flex; align-items:center; gap:16px; margin-bottom:20px; background:#fff; border:1px solid rgba(55, 53, 47, 0.09); border-radius:4px; padding:12px 18px; box-shadow: rgba(15, 15, 15, 0.05) 0px 0px 0px 1px, rgba(15, 15, 15, 0.08) 0px 2px 4px; }
        .toolbar-title { font-weight:600; font-size:1.05rem; color: #37352f; }
        .status-badge { display:inline-block; padding:2px 6px; border-radius:3px; font-weight:500; font-size:11px; border:1px solid; ${curStatusClass} }
        .controls { display: flex; gap: 8px; align-items: center; margin-left: auto; }
        .filter-select, .search-input { border: 1px solid rgba(55, 53, 47, 0.16); border-radius: 4px; padding: 6px 10px; font-family: inherit; font-size: 13px; outline: none; background: #fff; color: #37352f; }
        .filter-select { cursor: pointer; }
        .print-btn { padding:6px 14px; background:#2383e2; color:white; border:none; border-radius:4px; font-size:13px; font-weight:500; cursor:pointer; display: flex; align-items: center; gap: 6px; transition: background 0.12s; }
        .print-btn:hover { background:#1b6ec2; }
        .stats-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 12px; margin-bottom: 20px; }
        .stat-card { background: #fff; border: 1px solid rgba(55, 53, 47, 0.09); border-radius: 4px; padding: 12px 14px; }
        .stat-label { font-size: 11px; font-weight: 600; color: rgba(55, 53, 47, 0.65); text-transform: uppercase; letter-spacing: 0.04em; margin-bottom: 4px; }
        .stat-val { font-size: 1.3rem; font-weight: 700; color: #37352f; }
        h1 { font-size:1.35rem; font-weight:700; margin-bottom:4px; color: #37352f; }
        .sub { color:rgba(55, 53, 47, 0.65); font-size:13px; margin-bottom:20px; }
        .card { background: #fff; border: 1px solid rgba(55, 53, 47, 0.09); border-radius: 4px; padding: 18px; margin-bottom: 20px; }
        .section-title { font-size:0.9rem; font-weight:600; color:#37352f; margin-bottom:14px; padding-bottom:6px; border-bottom:1px solid rgba(55, 53, 47, 0.09); display: flex; align-items: center; justify-content: space-between; }
        table { width:100%; border-collapse:collapse; text-align: left; }
        th { background:#f7f6f3; color:rgba(55, 53, 47, 0.65); padding:6px 8px; font-size:11px; font-weight:600; text-transform:uppercase; letter-spacing:0.04em; border-bottom: 1px solid rgba(55, 53, 47, 0.16); }
        td { padding:6px 8px; border-bottom: 1px solid rgba(55, 53, 47, 0.09); font-size:13px; color: #37352f; word-break:break-word; overflow-wrap:break-word; }
        tr:last-child td { border-bottom: none; }
        .no-data { text-align: center; color: rgba(55, 53, 47, 0.4); padding: 20px; font-size: 13px; }
        @media print {
            .toolbar, .stats-grid, .controls, .sub-filter { display:none !important; }
            body { background: white !important; color: #37352f !important; padding: 0 !important; margin: 0 auto !important; box-sizing: border-box !important; width: 100% !important; max-width: 100% !important; }
            .container { max-width: 100% !important; width: 100% !important; }
            .card { border: none; padding: 0; box-shadow: none; margin-bottom: 24px; }
            th { background: #f7f6f3 !important; color: #37352f !important; border-bottom: 1.5px solid rgba(55, 53, 47, 0.2) !important; }
            td { border-bottom: 1px solid rgba(55, 53, 47, 0.12) !important; }
            thead { display: table-header-group !important; }
            tr { break-inside: avoid !important; page-break-inside: avoid !important; }
            @page { size: auto; margin: 8mm 10mm; }
        }
    </style>
</head>
<body>
<div class="container">
    <div class="toolbar">
        ${avatarHtml}
        <div>
            <div class="toolbar-title">${eName}</div>
            <span class="status-badge">Current Status: ${currentStatus}</span>
        </div>
        <div class="controls">
            <input type="text" id="search-input" class="search-input" placeholder="Search logs...">
            <select id="range-select" class="filter-select">
                <option value="30">Last 30 Days</option>
                <option value="90">Last 90 Days</option>
                <option value="180">Last 6 Months</option>
                <option value="365">Last 1 Year</option>
                <option value="all" selected>All Time</option>
            </select>
            <button class="print-btn" onclick="window.print()">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="6 9 6 2 18 2 18 9"></polyline><path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"></path><rect x="6" y="14" width="12" height="8"></rect></svg>
                Print Report
            </button>
        </div>
    </div>

    <h1>Attendance History</h1>
    <div class="sub">Comprehensive activity logs and historical daily statuses for <strong>${eName}</strong>.</div>

    <div class="stats-grid">
        <div class="stat-card">
            <div class="stat-label">Total Logs Found</div>
            <div class="stat-val" id="stat-total-logs">0</div>
        </div>
        <div class="stat-card">
            <div class="stat-label">Absences Recorded</div>
            <div class="stat-val" id="stat-absences" style="color: #ef4444;">0</div>
        </div>
        <div class="stat-card">
            <div class="stat-label">Present Count</div>
            <div class="stat-val" id="stat-presents" style="color: #10b981;">0</div>
        </div>
        <div class="stat-card">
            <div class="stat-label">Out on Passes</div>
            <div class="stat-val" id="stat-passes" style="color: #f59e0b;">0</div>
        </div>
    </div>

    <div class="card">
        <div class="section-title">
            <span>Status Change Log</span>
            <span style="font-size: 0.75rem; font-weight: normal; color: #64748b;" id="log-count-lbl">Showing 0 entries</span>
        </div>
        <div style="overflow-x: auto;">
            <table>
                <thead>
                    <tr>
                        <th>Date</th>
                        <th>Time</th>
                        <th>New Status</th>
                        <th>Previous Status</th>
                        <th>Reason / Remarks</th>
                    </tr>
                </thead>
                <tbody id="logs-tbody"></tbody>
            </table>
        </div>
    </div>

    <div class="card">
        <div class="section-title">
            <span>Daily Snapshot Records</span>
            <span style="font-size: 0.75rem; font-weight: normal; color: #64748b;" id="snap-count-lbl">Showing 0 entries</span>
        </div>
        <div style="overflow-x: auto;">
            <table>
                <thead>
                    <tr>
                        <th>Snapshot Date</th>
                        <th>Daily Status</th>
                        <th>Assigned Area</th>
                        <th>Assigned Project / Task</th>
                    </tr>
                </thead>
                <tbody id="snaps-tbody"></tbody>
            </table>
        </div>
    </div>

    <div style="margin-top:40px;font-size:0.75rem;color:#94a3b8;text-align:center;" class="no-print">
        Generated on ${new Date().toLocaleDateString('en-US',{year:'numeric',month:'long',day:'numeric'})} &bull; Service Section Workforce Command Center
    </div>
</div>

<script>
    const allLogs = ${JSON.stringify(allLogs).replace(/<\//g, '<\\/')};
    const allSnaps = ${JSON.stringify(allSnapshots).replace(/<\//g, '<\\/')};

    const searchInput = document.getElementById('search-input');
    const rangeSelect = document.getElementById('range-select');

    function filterData() {
        const query = searchInput.value.toLowerCase();
        const days = rangeSelect.value;
        const now = Date.now();
        const cutoff = days === 'all' ? 0 : now - (parseInt(days) * 24 * 60 * 60 * 1000);

        // Filter logs
        const filteredLogs = allLogs.filter(log => {
            const ts = typeof log.timestamp === 'number' && !isNaN(log.timestamp) ? log.timestamp : (new Date(log.timestamp || log.date || 0).getTime() || 0);
            if (cutoff && ts < cutoff) return false;
            if (query) {
                const text = ((log.newStatus || '') + ' ' + (log.oldStatus || '') + ' ' + (log.reason || '')).toLowerCase();
                if (!text.includes(query)) return false;
            }
            return true;
        });

        // Filter snaps
        const filteredSnaps = allSnaps.filter(snap => {
            const ts = typeof snap.timestamp === 'number' && !isNaN(snap.timestamp) ? snap.timestamp : (new Date(snap.timestamp || snap.date || 0).getTime() || 0);
            if (cutoff && ts < cutoff) return false;
            if (query) {
                const text = ((snap.dailyStatus || '') + ' ' + (snap.area || '') + ' ' + (snap.project || '') + ' ' + (snap.specificTask || '')).toLowerCase();
                if (!text.includes(query)) return false;
            }
            return true;
        });

        render(filteredLogs, filteredSnaps);
    }

    function render(logs, snaps) {
        // Stats
        const elTotal = document.getElementById('stat-total-logs');
        if (elTotal) elTotal.textContent = snaps.length || logs.length;
        
        let absences = 0, presents = 0, passes = 0;
        
        // Count statuses from snaps (more representative of daily attendance)
        snaps.forEach(s => {
            const st = (s.dailyStatus || '').trim().toLowerCase();
            if (st.includes('absent') || st === 'a') absences++;
            else if (st.includes('present') || st === 'p') presents++;
            else if (st.includes('pass') || st.includes('excused')) passes++;
        });

        const elAbs = document.getElementById('stat-absences');
        if (elAbs) elAbs.textContent = absences;
        const elPres = document.getElementById('stat-presents');
        if (elPres) elPres.textContent = presents;
        const elPass = document.getElementById('stat-passes');
        if (elPass) elPass.textContent = passes;

        // Render Logs Table
        const logsTbody = document.getElementById('logs-tbody');
        const elCount = document.getElementById('log-count-lbl');
        if (elCount) elCount.textContent = 'Showing ' + logs.length + ' entries';
        if (logs.length === 0) {
            logsTbody.innerHTML = '<tr><td colspan="5" class="no-data">No matching change logs found in selected period.</td></tr>';
        } else {
            logsTbody.innerHTML = logs.map(e => {
                let d = new Date(e.timestamp || e.date);
                if (isNaN(d.getTime())) d = new Date();
                const dateStr = e.date || d.toLocaleDateString('en-US');
                const timeStr = d.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' });
                const newStatus = (e.newStatus || e.status || 'Present');
                const st = newStatus.toLowerCase();
                const statusStyle = (st.includes('present') || st === 'p') ? 'color:#059669; font-weight:700;' : (st.includes('absent') || st === 'a') ? 'color:#dc2626; font-weight:700;' : 'color:#d97706; font-weight:700;';
                return '<tr>' +
                    '<td data-label="Date">' + dateStr + '</td>' +
                    '<td data-label="Time">' + timeStr + '</td>' +
                    '<td data-label="Status" style="' + statusStyle + '">' + newStatus + '</td>' +
                    '<td data-label="Previous">' + (e.oldStatus || '\u2014') + '</td>' +
                    '<td data-label="Reason">' + (e.reason || '\u2014') + '</td>' +
                    '</tr>';
            }).join('');
        }

        // Render Snapshots Table
        const snapsTbody = document.getElementById('snaps-tbody');
        document.getElementById('snap-count-lbl').textContent = 'Showing ' + snaps.length + ' entries';
        if (snaps.length === 0) {
            snapsTbody.innerHTML = '<tr><td colspan="4" class="no-data">No snapshots found in selected period.</td></tr>';
        } else {
            snapsTbody.innerHTML = snaps.map(s => {
                const dateStr = s.date || (s.timestamp ? new Date(s.timestamp).toLocaleDateString('en-US') : '-');
                const dailyStatus = (s.dailyStatus || 'Present');
                const st = dailyStatus.toLowerCase();
                const statusStyle = (st.includes('present') || st === 'p') ? 'color:#059669; font-weight:700;' : (st.includes('absent') || st === 'a') ? 'color:#dc2626; font-weight:700;' : 'color:#d97706; font-weight:700;';
                const taskDisplay = s.specificTask ? (' &bull; <span style="color:#64748b;font-size:0.75rem;">' + s.specificTask + '</span>') : '';
                return '<tr>' +
                    '<td data-label="Date">' + dateStr + '</td>' +
                    '<td data-label="Status" style="' + statusStyle + '">' + dailyStatus + '</td>' +
                    '<td data-label="Area">' + (s.area || '\u2014') + '</td>' +
                    '<td data-label="Project">' + (s.project || '\u2014') + taskDisplay + '</td>' +
                    '</tr>';
            }).join('');
        }
    }

    searchInput.addEventListener('input', filterData);
    rangeSelect.addEventListener('change', filterData);

    // Initial load
    filterData();
</script>
</body>
</html>`;

    const win = window.open('', '_blank');
    if (!win) {
        const workerIdx = (window.workersData || []).findIndex(w => w && w.name === workerName);
        if (workerIdx >= 0 && typeof window.openPersonalModal === 'function') {
            window.openPersonalModal(workerIdx, 'attendance');
            return;
        }
        if (typeof window.safePrint === 'function') {
            window.safePrint(html, { title: 'Attendance_History_' + eName });
            return;
        }
        showToast('Popup blocked. Please allow popups and try again.');
        return;
    }
    try {
        win.document.write(html);
        win.document.close();
    } catch (e) {
        console.error('Failed to write print document:', e);
        const workerIdx = (window.workersData || []).findIndex(w => w && w.name === workerName);
        if (workerIdx >= 0 && typeof window.openPersonalModal === 'function') {
            window.openPersonalModal(workerIdx, 'attendance');
        } else {
            showToast('Failed to open print preview. Please try again.');
        }
        try { win.close(); } catch (_) {}
    }
}

function showWorkerAssignmentPopup(workerName) {
    const eName = esc(workerName);
    const worker = workersData.find(w => w.name === workerName);
    const ePhoto = worker && (worker.photoUrl || worker.photo) ? esc(worker.photoUrl || worker.photo) : '';
    const avatarHtml = worker && (worker.photoUrl || worker.photo)
        ? `<img src="${ePhoto}" style="width:36px;height:36px;border-radius:50%;object-fit:cover;border:2px solid #e2e8f0;flex-shrink:0;">`
        : `<div style="width:36px;height:36px;border-radius:50%;background:linear-gradient(135deg,#3b82f6,#1d4ed8);display:flex;align-items:center;justify-content:center;font-size:1rem;font-weight:800;color:white;flex-shrink:0;">${eName.charAt(0)}</div>`;

    const allLogs = assignmentLog.filter(e => e.workerName === workerName);
    const allSnaps = queryWorkerSnapshots(workerName);

    // Sort descending by default
    allLogs.sort((a, b) => b.timestamp - a.timestamp);
    allSnaps.sort((a, b) => b.timestamp - a.timestamp);

    const curArea = worker ? esc(worker.area || '\u2014') : '\u2014';
    const curProj = worker ? esc(worker.project || '\u2014') : '\u2014';
    const curTask = worker ? esc(worker.specificTask || '\u2014') : '\u2014';

    const html = `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Assignment History &mdash; ${eName}</title>
    <style>
        /* ponytail: removed Google Fonts @import for offline reliability */
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body { font-family: 'Inter', Arial, sans-serif; padding: 32px; background: #f8fafc; color: #1e293b; line-height: 1.5; }
        .container { max-width: 1000px; margin: 0 auto; }
        .toolbar { display:flex; align-items:center; gap:16px; margin-bottom:24px; background:#fff; border:1px solid #e2e8f0; border-radius:12px; padding:16px 24px; box-shadow: 0 4px 6px -1px rgba(0,0,0,0.05); }
        .toolbar-title { font-weight:700; font-size:1.1rem; color: #0f172a; }
        .controls { display: flex; gap: 12px; align-items: center; margin-left: auto; }
        .filter-select, .search-input { border: 1px solid #cbd5e1; border-radius: 8px; padding: 8px 14px; font-family: inherit; font-size: 0.85rem; outline: none; background: #fff; color: #1e293b; }
        .filter-select { cursor: pointer; }
        .print-btn { padding:8px 20px; background:#1e40af; color:white; border:none; border-radius:8px; font-size:0.85rem; font-weight:700; cursor:pointer; display: flex; align-items: center; gap: 6px; transition: background 0.15s; }
        .print-btn:hover { background:#1d3a9f; }
        
        .stats-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 16px; margin-bottom: 24px; }
        .stat-card { background: #fff; border: 1px solid #e2e8f0; border-radius: 10px; padding: 16px; box-shadow: 0 1px 3px rgba(0,0,0,0.02); }
        .stat-label { font-size: 0.65rem; font-weight: 700; color: #64748b; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 4px; }
        .stat-val { font-size: 1.4rem; font-weight: 800; color: #0f172a; }

        h1 { font-size:1.5rem; font-weight:800; margin-bottom:4px; color: #0f172a; }
        .sub { color:#64748b; font-size:0.85rem; margin-bottom:24px; }
        
        .current-box { display:grid; grid-template-columns:1fr 1fr 1fr; gap:16px; margin-bottom:24px; }
        .current-field { background:#fff; border:1px solid #e2e8f0; border-radius:10px; padding:16px; box-shadow: 0 1px 3px rgba(0,0,0,0.02); }
        .current-label { font-size:0.65rem; font-weight:700; text-transform:uppercase; color:#64748b; letter-spacing:0.05em; margin-bottom:4px; }
        .current-value { font-weight:800; font-size:1.05rem; color:#0f172a; }
        
        .card { background: #fff; border: 1px solid #e2e8f0; border-radius: 12px; padding: 24px; margin-bottom: 24px; box-shadow: 0 4px 6px -1px rgba(0,0,0,0.02); }
        .section-title { font-size:0.95rem; font-weight:800; color:#0f172a; margin-bottom:16px; padding-bottom:8px; border-bottom:1px solid #f1f5f9; display: flex; align-items: center; justify-content: space-between; }
        
        table { width:100%; border-collapse:collapse; text-align: left; }
        th { background:#f8fafc; color:#475569; padding:10px 12px; font-size:0.7rem; font-weight:700; text-transform:uppercase; letter-spacing:0.05em; border-bottom: 2px solid #e2e8f0; }
        td { padding:12px; border-bottom: 1px solid #f1f5f9; font-size:0.85rem; color: #334155; word-break:break-word; overflow-wrap:break-word; }
        tr:last-child td { border-bottom: none; }
        .no-data { text-align: center; color: #94a3b8; padding: 24px; font-size: 0.85rem; }
        @media print {
            .toolbar, .stats-grid, .controls, .current-box { display:none !important; }
            body { background: white !important; color: #0f172a !important; padding: 0 !important; margin: 0 auto !important; box-sizing: border-box !important; width: 100% !important; max-width: 100% !important; }
            /* Prevent Blink shrink-to-fit - 1000px container exceeds Letter printable width */
            .container { max-width: 100% !important; width: 100% !important; }
            .card { border: none; padding: 0; box-shadow: none; margin-bottom: 32px; }
            th { background: #f1f5f9 !important; color: #0f172a !important; border-bottom: 2px solid #cbd5e1 !important; }
            td { border-bottom: 1px solid #cbd5e1 !important; }
            thead { display: table-header-group !important; }
            tr { break-inside: avoid !important; page-break-inside: avoid !important; }
            @page { size: auto; margin: 8mm 10mm; }
        }
    </style>
</head>
<body>
<div class="container">
    <div class="toolbar">
        ${avatarHtml}
        <div>
            <div class="toolbar-title">${eName}</div>
            <span style="font-size: 0.8rem; color: var(--text-muted);">Workplace Assignments Report</span>
        </div>
        <div class="controls">
            <input type="text" id="search-input" class="search-input" placeholder="Search history...">
            <select id="range-select" class="filter-select">
                <option value="30" selected>Last 30 Days</option>
                <option value="90">Last 90 Days</option>
                <option value="180">Last 6 Months</option>
                <option value="365">Last 1 Year</option>
                <option value="all">All Time</option>
            </select>
            <button class="print-btn" onclick="window.print()">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="6 9 6 2 18 2 18 9"></polyline><path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"></path><rect x="6" y="14" width="12" height="8"></rect></svg>
                Print Report
            </button>
        </div>
    </div>

    <h1>Assignment History</h1>
    <div class="sub">Record of project sites, designated areas, and specific task modifications for <strong>${eName}</strong>.</div>

    <div class="section-title">Current Assignment</div>
    <div class="current-box">
        <div class="current-field"><div class="current-label">Area</div><div class="current-value">${curArea}</div></div>
        <div class="current-field"><div class="current-label">Project</div><div class="current-value">${curProj}</div></div>
        <div class="current-field"><div class="current-label">Specific Task</div><div class="current-value">${curTask}</div></div>
    </div>

    <div class="stats-grid">
        <div class="stat-card">
            <div class="stat-label">Total Logs Found</div>
            <div class="stat-val" id="stat-total-logs">0</div>
        </div>
        <div class="stat-card">
            <div class="stat-label">Total Changes Logged</div>
            <div class="stat-val" id="stat-changes" style="color: #3b82f6;">0</div>
        </div>
        <div class="stat-card">
            <div class="stat-label">Unique Areas Worked</div>
            <div class="stat-val" id="stat-unique-areas" style="color: #10b981;">0</div>
        </div>
        <div class="stat-card">
            <div class="stat-label">Snapshot Days Count</div>
            <div class="stat-val" id="stat-snaps">0</div>
        </div>
    </div>

    <div class="card">
        <div class="section-title">
            <span>Snapshot Records</span>
            <span style="font-size: 0.75rem; font-weight: normal; color: #64748b;" id="snap-count-lbl">Showing 0 entries</span>
        </div>
        <div style="overflow-x: auto;">
            <table>
                <thead>
                    <tr>
                        <th>Date</th>
                        <th>Designated Area</th>
                        <th>Designated Project</th>
                        <th>Specific Task</th>
                    </tr>
                </thead>
                <tbody id="snaps-tbody"></tbody>
            </table>
        </div>
    </div>

    <div class="card">
        <div class="section-title">
            <span>Assignment Change Log</span>
            <span style="font-size: 0.75rem; font-weight: normal; color: #64748b;" id="log-count-lbl">Showing 0 entries</span>
        </div>
        <div style="overflow-x: auto;">
            <table>
                <thead>
                    <tr>
                        <th>Date</th>
                        <th>Field Altered</th>
                        <th>Previous Value</th>
                        <th>Assigned Value</th>
                    </tr>
                </thead>
                <tbody id="logs-tbody"></tbody>
            </table>
        </div>
    </div>

    <div style="margin-top:40px;font-size:0.75rem;color:#94a3b8;text-align:center;" class="no-print">
        Generated on ${new Date().toLocaleDateString('en-US',{year:'numeric',month:'long',day:'numeric'})} &bull; Service Section Workforce Command Center
    </div>
</div>

<script>
    const allLogs = ${JSON.stringify(allLogs).replace(/<\//g, '<\\/')};
    const allSnaps = ${JSON.stringify(allSnaps).replace(/<\//g, '<\\/')};

    const searchInput = document.getElementById('search-input');
    const rangeSelect = document.getElementById('range-select');

    function filterData() {
        const query = searchInput.value.toLowerCase();
        const days = rangeSelect.value;
        const now = Date.now();
        const cutoff = days === 'all' ? 0 : now - (parseInt(days) * 24 * 60 * 60 * 1000);

        // Filter logs
        const filteredLogs = allLogs.filter(log => {
            if (log.timestamp < cutoff) return false;
            if (query) {
                const text = (log.field + ' ' + (log.oldValue || '') + ' ' + (log.newValue || '')).toLowerCase();
                if (!text.includes(query)) return false;
            }
            return true;
        });

        // Filter snaps
        const filteredSnaps = allSnaps.filter(snap => {
            if (snap.timestamp < cutoff) return false;
            if (query) {
                const text = ((snap.area || '') + ' ' + (snap.project || '') + ' ' + (snap.specificTask || '')).toLowerCase();
                if (!text.includes(query)) return false;
            }
            return true;
        });

        render(filteredLogs, filteredSnaps);
    }

    function render(logs, snaps) {
        // Stats
        const elTotal = document.getElementById('stat-total-logs');
        if (elTotal) elTotal.textContent = logs.length + snaps.length;
        const elChanges = document.getElementById('stat-changes');
        if (elChanges) elChanges.textContent = logs.length;
        const elSnaps = document.getElementById('stat-snaps');
        if (elSnaps) elSnaps.textContent = snaps.length;
        
        // Count unique areas
        const uniqueAreas = new Set();
        snaps.forEach(s => { if (s.area) uniqueAreas.add(s.area); });
        const elAreas = document.getElementById('stat-unique-areas');
        if (elAreas) elAreas.textContent = uniqueAreas.size;

        // Render Snaps Table
        const snapsTbody = document.getElementById('snaps-tbody');
        const elSnapCount = document.getElementById('snap-count-lbl');
        if (elSnapCount) elSnapCount.textContent = 'Showing ' + snaps.length + ' entries';
        if (snaps.length === 0) {
            snapsTbody.innerHTML = '<tr><td colspan="4" class="no-data">No snapshot records in selected period.</td></tr>';
        } else {
            snapsTbody.innerHTML = snaps.map(s => {
                return '<tr>' +
                    '<td>' + esc(s.date) + '</td>' +
                    '<td>' + esc(s.area || '\u2014') + '</td>' +
                    '<td>' + esc(s.project || '\u2014') + '</td>' +
                    '<td>' + esc(s.specificTask || '\u2014') + '</td>' +
                    '</tr>';
            }).join('');
        }

        // Render Logs Table
        const logsTbody = document.getElementById('logs-tbody');
        const elCount = document.getElementById('log-count-lbl');
        if (elCount) elCount.textContent = 'Showing ' + logs.length + ' entries';
        if (logs.length === 0) {
            if (logsTbody) logsTbody.innerHTML = '<tr><td colspan="4" class="no-data">No changes logged in selected period.</td></tr>';
        } else if (logsTbody) {
            logsTbody.innerHTML = logs.map(e => {
                const d = new Date(e.timestamp);
                const dateStr = d.toLocaleDateString('en-US');
                return '<tr>' +
                    '<td>' + dateStr + '</td>' +
                    '<td style="font-weight: 600;">' + esc(e.field) + '</td>' +
                    '<td style="color:#64748b;">' + esc(e.oldValue || '\u2014') + '</td>' +
                    '<td style="font-weight: 600;">' + esc(e.newValue || '\u2014') + '</td>' +
                    '</tr>';
            }).join('');
        }
    }

    searchInput.addEventListener('input', filterData);
    rangeSelect.addEventListener('change', filterData);

    // Initial load
    filterData();
</script>
</body>
</html>`;

    const win = window.open('', '_blank');
    if (!win) {
        if (typeof window.safePrint === 'function') {
            window.safePrint(html, { title: 'Assignment_History_' + eName });
            return;
        }
        showToast('Popup blocked. Please allow popups to view full history.');
        return;
    }
    try {
        win.document.write(html);
        win.document.close();
    } catch (e) {
        console.error('Failed to write history popup document:', e);
        showToast('Failed to open history popup.');
        try { win.close(); } catch (_) {}
    }
}
return {
    showWorkerAttendancePopup: showWorkerAttendancePopup,
    showWorkerAssignmentPopup: showWorkerAssignmentPopup
};
})();

// ===== cards.js - verbatim from 01_Rust_WC_DB/js/cards.js =====
var S5_cards = (function () {
var cardsSelectedIndices = new Set();
function printSelectedCardsList() {
    if (!cardsSelectedIndices || cardsSelectedIndices.size === 0) {
        showToast('Select at least one volunteer first.', 'warning');
        return;
    }

    const groupNameInput = document.getElementById('cards-group-name');
    const groupName = groupNameInput ? groupNameInput.value.trim() : '';

    const workers = [...cardsSelectedIndices].map(i => workersData[i]).filter(Boolean);
    if (workers.length === 0) return;

    workers.sort((a, b) => a.name.localeCompare(b.name));

    const today = new Date();
    const dateStr = today.toLocaleDateString('en-US', { year: 'numeric', month: 'long', day: 'numeric' });

    const months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

    const rowsHtml = workers.map(w => {
        const eName = esc(w.name);
        const eTrade = esc(w.trade || '');
        const eANumber = esc(w.aNumber != null ? String(w.aNumber) : '');
        const eArea = esc(w.area || 'Unassigned');
        const eStatus = esc(w.dailyStatus || 'Present');
        const eMedical = esc(window.getCombinedMedicalCondition ? window.getCombinedMedicalCondition(w) : (w.medical || ''));
        const avatar = w.photoUrl || w.photo
            ? `<img src="${esc(w.photoUrl || w.photo)}" style="width:36px;height:36px;border-radius:50%;object-fit:cover;">`
            : `<div style="width:36px;height:36px;border-radius:50%;background:linear-gradient(135deg,#3b82f6,#1d4ed8);display:flex;align-items:center;justify-content:center;font-size:1rem;font-weight:800;color:white;margin:0 auto;">${eName.charAt(0)}</div>`;
        return `<tr>
            <td style="text-align:center;">${avatar}</td>
            <td style="font-weight:700;">${eName}</td>
            <td>${eTrade}</td>
            <td>#${eANumber}</td>
            <td>${eArea}</td>
            <td><span style="color:${eStatus === 'Present' ? '#10b981' : '#ef4444'};font-weight:700;">${eStatus}</span></td>
            <td style="font-size:0.75rem;color:#6b7280;">${eMedical || 'Clear'}</td>
        </tr>`;
    }).join('');

    const title = groupName ? `${esc(groupName)}` : 'Selected Volunteers';
    const subtitle = groupName ? 'Volunteer List' : '';

    const html = `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>${title} - Volunteer List</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: 'Inter', Arial, Helvetica, sans-serif;
            padding: 0.3in;
            color: #0f172a;
            font-size: 10pt;
        }
        .header {
            text-align: center;
            border-bottom: 2px solid #1e3a5f;
            padding-bottom: 16px;
            margin-bottom: 20px;
        }
        .header h1 {
            font-size: 18pt;
            font-weight: 800;
            color: #1e3a5f;
            margin-bottom: 4px;
        }
        .header .subtitle {
            font-size: 10pt;
            color: #64748b;
            font-weight: 600;
        }
        .header .date {
            font-size: 8pt;
            color: #94a3b8;
            margin-top: 4px;
        }
        table {
            width: 100%;
            border-collapse: collapse;
            font-size: 9pt;
        }
        thead th {
            background: #1e3a5f;
            color: white;
            padding: 8px 10px;
            text-align: left;
            font-size: 7.5pt;
            text-transform: uppercase;
            letter-spacing: 0.06em;
            font-weight: 700;
        }
        thead th:first-child { text-align: center; width: 52px; }
        tbody td {
            padding: 7px 10px;
            border-bottom: 1px solid #e2e8f0;
            vertical-align: middle;
        }
        tbody tr:nth-child(even) { background: #f8fafc; }
        tbody tr:hover { background: #f1f5f9; }
        .footer {
            margin-top: 24px;
            text-align: center;
            font-size: 7.5pt;
            color: #94a3b8;
        }
        @media print {
            * { -webkit-print-color-adjust: exact !important; print-color-adjust: exact !important; }
            @page { size: auto; margin: 8mm 10mm; }
            body { background: #ffffff !important; color: #0f172a !important; padding: 0 !important; margin: 0 auto !important; box-sizing: border-box !important; }
            tbody tr:hover { background: inherit; }
            thead { display: table-header-group !important; }
            tr { break-inside: avoid !important; page-break-inside: avoid !important; }
            table { width: 100% !important; border-collapse: collapse !important; }
            img { image-rendering: -webkit-optimize-contrast; }
        }
    </style>
</head>
<body>
    <div class="header">
        <h1>${title}</h1>
        ${subtitle ? `<div class="subtitle">${subtitle}</div>` : ''}
        <div class="date">Generated ${dateStr} - ${workers.length} volunteer${workers.length > 1 ? 's' : ''}</div>
    </div>
    <table>
        <thead>
            <tr>
                <th>Photo</th>
                <th>Name</th>
                <th>Trade</th>
                <th>Badge</th>
                <th>Area</th>
                <th>Status</th>
                <th>Medical</th>
            </tr>
        </thead>
        <tbody>
            ${rowsHtml}
        </tbody>
    </table>
    <div class="footer">Service Section - Printed ${dateStr}</div>
</body>
</html>`;

    // Use safePrint iframe engine instead of popup to avoid popup blocker issues
    if (typeof window.safePrint === 'function') {
        window.safePrint(html);
    } else {
        const win = window.open('', '_blank');
        if (!win) { showToast('Popup blocked. Please allow popups and try again.'); return; }
        try {
            win.document.open();
            win.document.write(html);
            win.document.close();
        } catch (e) {
            console.error('Failed to open print window:', e);
            showToast('Failed to open print preview.');
            try { win.close(); } catch (_) {}
        }
    }
}
return {
    printSelectedCardsList: printSelectedCardsList,
    cardsSelectedIndices: cardsSelectedIndices
};
})();

// ===== records_update_form.js - verbatim from 01_Rust_WC_DB/js/records_update_form.js =====
var S5_records_update_form = (function () {

    function esc(str) {
        if (str == null) return '';
        return String(str)
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;')
            .replace(/"/g, '&quot;')
            .replace(/'/g, '&#039;');
    }

    function getAge(birthday) {
        if (!birthday) return null;
        const b = new Date(birthday);
        if (isNaN(b.getTime())) return null;
        const now = new Date();
        let age = now.getFullYear() - b.getFullYear();
        const m = now.getMonth() - b.getMonth();
        if (m < 0 || (m === 0 && now.getDate() < b.getDate())) {
            age--;
        }
        return age >= 0 ? age : null;
    }

    function getMaintMedsSummary(w) {
        if (!w || !Array.isArray(w.maintenanceMeds) || w.maintenanceMeds.length === 0) return 'None';
        return w.maintenanceMeds.map(m => {
            const name = m.name || m.medication || '';
            const dose = m.dosage ? ` (${m.dosage})` : '';
            return name + dose;
        }).filter(Boolean).join(', ') || 'None';
    }

    function getChildStatusDisplay(c) {
        if (!c || (!c.name && !c.school && !c.job && !c.status)) return '';
        if (c.status) return c.status;
        const parts = [];
        if (c.isStudent === true || c.school) {
            let sch = c.school || 'Estudyante';
            if (c.course) sch += ` (${c.course})`;
            parts.push(`Nag-aaral: ${sch}`);
        }
        if (c.job || c.workplace) {
            let j = c.job || 'Nagtatrabaho';
            if (c.workplace) j += ` sa ${c.workplace}`;
            parts.push(`Trabaho: ${j}`);
        }
        if (parts.length > 0) return parts.join(' | ');
        if (c.isStudent === false && !c.school) return 'Nagtatrabaho / Di nag-aaral';
        return '-';
    }

    function getChildResidingDisplay(c) {
        if (!c || (!c.name && c.livesHere === undefined && !c.residence && !c.address)) return '';
        if (c.livesHere === true || c.livesHere === 'Oo' || c.livesHere === 'Yes' || c.livesHere === 'Opo') return 'Oo (Kasama)';
        if (c.livesHere === false || c.livesHere === 'Hindi' || c.livesHere === 'No') {
            const loc = c.residence || c.address || '';
            return loc ? `Hindi (${loc})` : 'Hindi kasama';
        }
        if (c.residence || c.address) return `Hindi (${c.residence || c.address})`;
        return 'Oo (Kasama)';
    }

    function getChildDutyDisplay(c) {
        if (!c || !c.name) return '';
        if (c.churchDuty && c.churchDuty.trim() && c.churchDuty.toLowerCase() !== 'none' && !c.churchDuty.toLowerCase().startsWith('wala')) {
            return c.churchDuty.trim();
        }
        return 'Wala';
    }

    function generateSingleSheetHTML(w, options = {}) {
        const isBlank = options.mode === 'blank';
        const cycle = options.cycle || 'Semi-Annual Update 2026';
        const paperSize = options.paperSize || 'folio';
        const includePhoto = options.includePhoto !== false;

        const age = isBlank ? null : getAge(w.birthday);
        const ageStr = age !== null ? `${age} yrs` : '-';
        const medsStr = isBlank ? '' : getMaintMedsSummary(w);

        const photoHtml = (includePhoto && !isBlank && (w.photoUrl || w.photo))
            ? `<div class="vru-photo-box"><img src="${esc(w.photoUrl || w.photo)}" alt="${esc(w.name)}"></div>`
            : `<div class="vru-photo-box vru-photo-empty"><span>Attach 2x2 Photo if New</span></div>`;

        // Children cards - 3 dedicated lines per child for maximum reading and handwriting comfort
        const children = (!isBlank && Array.isArray(w.children)) ? w.children : [];
        let childrenCardsHtml = '';
        const minCards = Math.max(3, children.length + 1);

        for (let i = 0; i < minCards; i++) {
            const child = children[i] || null;
            const hasChild = child && child.name && !isBlank;
            const childAge = hasChild ? (child.birthday ? getAge(child.birthday) : (child.age || '')) : '';
            const childDuty = hasChild ? getChildDutyDisplay(child) : '';
            const childStatus = hasChild ? getChildStatusDisplay(child) : '';
            const childResiding = hasChild ? getChildResidingDisplay(child) : '';

            if (hasChild) {
                childrenCardsHtml += `
                    <div class="vru-child-card">
                        <!-- Row 1: Name & Birthday/Age -->
                        <div class="vru-child-row">
                            <span class="vru-child-num">#${i + 1}</span>
                            <div class="vru-child-cell" style="flex:1.5;">
                                <span class="vru-sublbl">Pangalan ng Anak:</span>
                                <span class="vru-curr">${esc(child.name)}</span>
                                <span class="vru-arrow">&rarr;</span>
                                <span class="vru-upd" style="flex:1;">Itama: </span>
                            </div>
                            <div class="vru-child-cell" style="flex:1.1;">
                                <span class="vru-sublbl">Kaarawan / Edad:</span>
                                <span class="vru-curr">${esc(child.birthday || '-')}${childAge ? ` (${childAge} yrs)` : ''}</span>
                                <span class="vru-arrow">&rarr;</span>
                                <span class="vru-upd" style="flex:1;">YYYY-MM-DD: </span>
                            </div>
                        </div>

                        <!-- Row 2: Church Duty (Full width with wide writing line) -->
                        <div class="vru-child-row">
                            <span class="vru-child-spacer"></span>
                            <div class="vru-child-cell" style="flex:1;">
                                <span class="vru-sublbl">Tungkulin sa Iglesia (Anak):</span>
                                <span class="vru-curr" style="max-width:200px;">${esc(childDuty || 'Wala')}</span>
                                <span class="vru-arrow">&rarr;</span>
                                <span class="vru-chk">[ ] Wala &nbsp;&nbsp; [ ] May Tungkulin: </span>
                                <span class="vru-upd" style="flex:2;"></span>
                            </div>
                        </div>

                        <!-- Row 3: School or Job (Full width with wide writing lines) -->
                        <div class="vru-child-row">
                            <span class="vru-child-spacer"></span>
                            <div class="vru-child-cell" style="flex:1;">
                                <span class="vru-sublbl">Paaralan o Trabaho:</span>
                                <span class="vru-curr" style="max-width:240px;" title="${esc(childStatus)}">${esc(childStatus || '-')}</span>
                                <span class="vru-arrow">&rarr;</span>
                                <span class="vru-chk">[ ] Nag-aaral (Paaralan/Antas): </span>
                                <span class="vru-upd" style="flex:1.2;"></span>
                                <span class="vru-chk" style="margin-left:8px;">[ ] Nagtatrabaho (Trabaho/Kumpanya): </span>
                                <span class="vru-upd" style="flex:1.2;"></span>
                            </div>
                        </div>

                        <!-- Row 4: Residence & Location if not cohabiting -->
                        <div class="vru-child-row">
                            <span class="vru-child-spacer"></span>
                            <div class="vru-child-cell" style="flex:1;">
                                <span class="vru-sublbl">Kasama sa Tirahan?</span>
                                <span class="vru-curr" style="max-width:180px;">${esc(childResiding || 'Oo (Kasama)')}</span>
                                <span class="vru-arrow">&rarr;</span>
                                <span class="vru-chk">[ ] Oo, kasama sa tirahan &nbsp;&nbsp;&nbsp; [ ] Hindi kasama (Saan nakatira): </span>
                                <span class="vru-upd" style="flex:2;"></span>
                            </div>
                        </div>
                    </div>
                `;
            } else {
                childrenCardsHtml += `
                    <div class="vru-child-card vru-child-blank">
                        <div class="vru-child-row">
                            <span class="vru-child-num">#${i + 1}</span>
                            <div class="vru-child-cell" style="flex:1.5;">
                                <span class="vru-sublbl">Pangalan:</span>
                                <span class="vru-upd" style="flex:1;">Pangalan ng Anak: </span>
                            </div>
                            <div class="vru-child-cell" style="flex:1.1;">
                                <span class="vru-sublbl">Kaarawan / Edad:</span>
                                <span class="vru-upd" style="flex:1;">YYYY-MM-DD / Edad: </span>
                            </div>
                        </div>
                        <div class="vru-child-row">
                            <span class="vru-child-spacer"></span>
                            <div class="vru-child-cell" style="flex:1;">
                                <span class="vru-sublbl">Tungkulin:</span>
                                <span class="vru-chk">[ ] Wala &nbsp;&nbsp; [ ] May Tungkulin: </span>
                                <span class="vru-upd" style="flex:2;"></span>
                            </div>
                        </div>
                        <div class="vru-child-row">
                            <span class="vru-child-spacer"></span>
                            <div class="vru-child-cell" style="flex:1;">
                                <span class="vru-chk">[ ] Nag-aaral (Paaralan at Antas): </span>
                                <span class="vru-upd" style="flex:1.2;"></span>
                                <span class="vru-chk" style="margin-left:8px;">[ ] Nagtatrabaho (Trabaho at Kumpanya): </span>
                                <span class="vru-upd" style="flex:1.2;"></span>
                            </div>
                        </div>
                        <div class="vru-child-row">
                            <span class="vru-child-spacer"></span>
                            <div class="vru-child-cell" style="flex:1;">
                                <span class="vru-chk">Kasama sa Tirahan? [ ] Oo, kasama &nbsp;&nbsp;&nbsp; [ ] Hindi kasama (Saan nakatira): </span>
                                <span class="vru-upd" style="flex:2;"></span>
                            </div>
                        </div>
                    </div>
                `;
            }
        }

        return `
        <div class="vru-sheet">
            <!-- HEADER -->
            <div class="vru-header">
                <div>
                    <div class="vru-inst-title">SERVICE SECTION &bull; VOLUNTEER WORKFORCE ADMINISTRATION</div>
                    <div class="vru-title">SEMI-ANNUAL VOLUNTEER RECORDS UPDATE FORM</div>
                    <div class="vru-subtitle">FORM SS-VRU-2026 &bull; Cycle: <strong>${esc(cycle)}</strong> &bull; Date Issued: <strong>${new Date().toLocaleDateString('en-US', { year: 'numeric', month: 'short', day: 'numeric' })}</strong></div>
                </div>
                ${photoHtml}
            </div>

            <!-- INSTRUCTIONS BANNER -->
            <div class="vru-notice">
                <strong>PAUNAWA / INSTRUCTIONS:</strong> Pakisuri ang mga kasalukuyang nakatalang impormasyon sa ibaba. Kung may kailangang baguhin, itama, o idagdag (tulad ng bagong tirahan, telepono, tungkulin sa Iglesia ng asawa/anak, paaralan o trabaho ng mga anak, at tirahan), isulat nang malinaw ang wastong datos sa patlang bago lumagda.
            </div>

            <!-- SECTION 1: PERSONAL & CONTACT INFORMATION -->
            <div class="vru-section">
                <div class="vru-sec-hdr">
                    <span>1. Personal &amp; Contact Profile / Personal na Impormasyon</span>
                    <span style="font-size:6.8pt; font-weight:600; color:#475569;">A# ID: <strong>#${isBlank ? '________' : esc(w.aNumber || '-')}</strong></span>
                </div>
                <div class="vru-sec-body">
                    <div class="vru-grid-2col">
                        <div class="vru-field">
                            <span class="vru-lbl">Pangalan (Name):</span>
                            <span class="vru-curr">${isBlank ? '' : esc(w.name || '-')}</span>
                            <span class="vru-arrow">&rarr;</span>
                            <span class="vru-upd">Itama: </span>
                        </div>
                        <div class="vru-field">
                            <span class="vru-lbl">Civil Status:</span>
                            <span class="vru-curr">${isBlank ? '' : esc(w.civilStatus || 'Single')}</span>
                            <span class="vru-arrow">&rarr;</span>
                            <span class="vru-chk">[ ] Walang Asawa &nbsp; [ ] May Asawa &nbsp; [ ] Balo</span>
                        </div>
                    </div>

                    <div class="vru-grid-2col">
                        <div class="vru-field">
                            <span class="vru-lbl">Kapanganakan / Edad:</span>
                            <span class="vru-curr">${isBlank ? '' : esc(w.birthday || '-')} (${ageStr})</span>
                            <span class="vru-arrow">&rarr;</span>
                            <span class="vru-upd">YYYY-MM-DD: </span>
                        </div>
                        <div class="vru-field">
                            <span class="vru-lbl">Mobile Phone #:</span>
                            <span class="vru-curr">${isBlank ? '' : esc(w.phone || '-')}</span>
                            <span class="vru-arrow">&rarr;</span>
                            <span class="vru-upd">Bagong Phone: </span>
                        </div>
                    </div>

                    <div class="vru-grid-2col">
                        <div class="vru-field">
                            <span class="vru-lbl">Salita / Dialect:</span>
                            <span class="vru-curr">${isBlank ? '' : esc(w.language || 'Tagalog')}</span>
                            <span class="vru-arrow">&rarr;</span>
                            <span class="vru-upd">Dialect: </span>
                        </div>
                        <div class="vru-field">
                            <span class="vru-lbl">Work Residence:</span>
                            <span class="vru-curr">${isBlank ? '' : esc(w.workResidence || 'Staff House')}</span>
                            <span class="vru-arrow">&rarr;</span>
                            <span class="vru-chk">[ ] Villa &nbsp; [ ] Staff House &nbsp; [ ] Non-INC</span>
                        </div>
                    </div>

                    <div class="vru-field" style="margin-top:2px;">
                        <span class="vru-lbl" style="min-width:145px;">Permanent Home Address:</span>
                        <span class="vru-curr">${isBlank ? '' : esc(w.address || '-')}</span>
                        <span class="vru-arrow">&rarr;</span>
                        <span class="vru-upd" style="flex:2;">Tirahan sa Lalawigan / Probinsya: </span>
                    </div>

                    <div class="vru-grid-2col" style="margin-top:2px;">
                        <div class="vru-field">
                            <span class="vru-lbl" style="min-width:110px;">Assigned Housing Unit:</span>
                            <span class="vru-curr">${isBlank ? '' : esc(w.visitationHousing || 'Unassigned')}</span>
                            <span class="vru-arrow">&rarr;</span>
                            <span class="vru-upd">Building / Dorm: </span>
                        </div>
                        <div class="vru-field">
                            <span class="vru-lbl">Room / Unit #:</span>
                            <span class="vru-curr">${isBlank ? '' : esc(w.assignedUnit || '-')}</span>
                            <span class="vru-arrow">&rarr;</span>
                            <span class="vru-upd">Kuwarto / Bed #: </span>
                        </div>
                    </div>
                </div>
            </div>

            <!-- SECTION 2: TRADE, SKILL & WORK ASSIGNMENT -->
            <div class="vru-section">
                <div class="vru-sec-hdr">
                    <span>2. Trade, Skills &amp; Work Assignment / Kasanayan at Trabaho</span>
                </div>
                <div class="vru-sec-body">
                    <div class="vru-grid-2col">
                        <div class="vru-field">
                            <span class="vru-lbl">Primary Trade:</span>
                            <span class="vru-curr">${isBlank ? '' : esc(w.trade || 'Helper')}</span>
                            <span class="vru-arrow">&rarr;</span>
                            <span class="vru-upd">Pangunahing Trade: </span>
                        </div>
                        <div class="vru-field">
                            <span class="vru-lbl">Skill Level:</span>
                            <span class="vru-curr">${isBlank ? '' : esc(w.skillLevel || 'Skilled')}</span>
                            <span class="vru-arrow">&rarr;</span>
                            <span class="vru-chk">[ ] Lead &nbsp; [ ] Skilled &nbsp; [ ] Semi &nbsp; [ ] Helper</span>
                        </div>
                    </div>

                    <div class="vru-grid-2col">
                        <div class="vru-field">
                            <span class="vru-lbl">Project Group:</span>
                            <span class="vru-curr">${isBlank ? '' : esc(w.projectGroup || 'Unassigned')} &bull; ${isBlank ? '' : esc(w.projectRole || 'Member')}</span>
                            <span class="vru-arrow">&rarr;</span>
                            <span class="vru-upd">Bagong Grupo / Role: </span>
                        </div>
                        <div class="vru-field">
                            <span class="vru-lbl" style="min-width:115px;">Secondary Trade(s):</span>
                            <span class="vru-upd" style="flex:2;">Iba pang Kakayahan (Plumbing, Welding, Driving, etc.): </span>
                        </div>
                    </div>
                </div>
            </div>

            <!-- SECTION 3: HEALTH, MEDICAL & EMERGENCY PROFILE -->
            <div class="vru-section">
                <div class="vru-sec-hdr">
                    <span>3. Health, Medical &amp; Emergency Profile / Kalusugan at Emergency</span>
                </div>
                <div class="vru-sec-body">
                    <div class="vru-grid-2col">
                        <div class="vru-field">
                            <span class="vru-lbl">Blood Type:</span>
                            <span class="vru-curr">${isBlank ? '' : esc(w.bloodType || 'Unknown')}</span>
                            <span class="vru-arrow">&rarr;</span>
                            <span class="vru-chk">[ ] A+ &nbsp; [ ] B+ &nbsp; [ ] AB+ &nbsp; [ ] O+ &nbsp; [ ] A- &nbsp; [ ] B- &nbsp; [ ] AB- &nbsp; [ ] O-</span>
                        </div>
                        <div class="vru-field">
                            <span class="vru-lbl">COVID-19 Status:</span>
                            <span class="vru-curr">${isBlank ? '' : esc(w.covidStatus || 'Cleared')}</span>
                            <span class="vru-arrow">&rarr;</span>
                            <span class="vru-chk">[ ] Fully Vaccinated &nbsp; [ ] +Booster &nbsp; [ ] Hindi Bakunado</span>
                        </div>
                    </div>

                    <div class="vru-field">
                        <span class="vru-lbl" style="min-width:130px;">Medical Conditions / Allergies:</span>
                        <span class="vru-curr">${isBlank ? '' : esc(w.medical || 'None / Cleared')}</span>
                        <span class="vru-arrow">&rarr;</span>
                        <span class="vru-upd" style="flex:2;">Karamdaman / Altapresyon / Hika / Allergy: </span>
                    </div>

                    <div class="vru-field">
                        <span class="vru-lbl" style="min-width:130px;">Maintenance Medications:</span>
                        <span class="vru-curr">${isBlank ? '' : esc(medsStr)}</span>
                        <span class="vru-arrow">&rarr;</span>
                        <span class="vru-upd" style="flex:2;">Gamot na regular na iniinom at dosage: </span>
                    </div>

                    <div class="vru-grid-2col" style="margin-top:2px;">
                        <div class="vru-field">
                            <span class="vru-lbl" style="min-width:130px;">Emergency Contact Person:</span>
                            <span class="vru-curr">${isBlank ? '' : esc(w.emergencyContact || '-')}</span>
                            <span class="vru-arrow">&rarr;</span>
                            <span class="vru-upd">Pangalan: </span>
                        </div>
                        <div class="vru-field">
                            <span class="vru-lbl">Relasyon / Phone:</span>
                            <span class="vru-curr">${isBlank ? '' : esc(w.emergencyPhone || '-')}</span>
                            <span class="vru-arrow">&rarr;</span>
                            <span class="vru-upd">Relasyon: ________ Phone: </span>
                        </div>
                    </div>
                </div>
            </div>

            <!-- SECTION 4: SPOUSE & FAMILY PROFILE -->
            <div class="vru-section">
                <div class="vru-sec-hdr">
                    <span>4. Spouse &amp; Family Profile / Asawa at mga Anak (Para sa May Asawa)</span>
                </div>
                <div class="vru-sec-body">
                    <div class="vru-grid-2col">
                        <div class="vru-field">
                            <span class="vru-lbl">Pangalan ng Asawa (Spouse):</span>
                            <span class="vru-curr">${isBlank ? '' : esc(w.spouseName || '-')}</span>
                            <span class="vru-arrow">&rarr;</span>
                            <span class="vru-upd">Spouse Name: </span>
                        </div>
                        <div class="vru-field">
                            <span class="vru-lbl">Kapanganakan ng Asawa:</span>
                            <span class="vru-curr">${isBlank ? '' : esc(w.spouseBirthday || '-')}</span>
                            <span class="vru-arrow">&rarr;</span>
                            <span class="vru-upd">YYYY-MM-DD: </span>
                        </div>
                    </div>

                    <div class="vru-grid-2col">
                        <div class="vru-field">
                            <span class="vru-lbl">Petsa ng Kasal (Wedding):</span>
                            <span class="vru-curr">${isBlank ? '' : esc(w.weddingDate || '-')}</span>
                            <span class="vru-arrow">&rarr;</span>
                            <span class="vru-chk">Kasal sa Iglesia? [ ] Oo &nbsp; [ ] Hindi</span>
                        </div>
                        <div class="vru-field">
                            <span class="vru-lbl">Contact # ng Asawa:</span>
                            <span class="vru-curr">${isBlank ? '' : esc(w.spouseContact || '-')}</span>
                            <span class="vru-arrow">&rarr;</span>
                            <span class="vru-upd">Spouse Phone: </span>
                        </div>
                    </div>

                    <div class="vru-field" style="margin-top:2px;">
                        <span class="vru-lbl" style="min-width:145px;">Tungkulin sa Iglesia (Asawa):</span>
                        <span class="vru-curr" style="max-width:180px;">${isBlank ? '' : esc(w.spouseChurchDuty || 'Wala')}</span>
                        <span class="vru-arrow">&rarr;</span>
                        <span class="vru-chk">[ ] Wala &nbsp;&nbsp; [ ] May Tungkulin: </span>
                        <span class="vru-upd" style="flex:2;"></span>
                    </div>

                    <div class="vru-field" style="margin-top:2px;">
                        <span class="vru-lbl" style="min-width:145px;">Nagsasama sa Tirahan?</span>
                        <span class="vru-curr" style="max-width:180px;">${isBlank ? '' : esc(w.spouseNagsasama || 'Oo')}</span>
                        <span class="vru-arrow">&rarr;</span>
                        <span class="vru-chk">[ ] Oo, magkasama &nbsp;&nbsp; [ ] Hindi kasama (Saan nakatira): </span>
                        <span class="vru-upd" style="flex:2;"></span>
                    </div>

                    <!-- CHILDREN CARDS CONTAINER -->
                    <div style="margin-top:3px;">
                        <div style="display:flex; justify-content:space-between; align-items:baseline; margin-bottom:2px;">
                            <span style="font-weight:800; font-size:7pt; color:#1e293b; text-transform:uppercase;">
                                Mga Anak / Dependents (Mula panganay hanggang bunso):
                            </span>
                            <span style="font-size:6.2pt; color:#64748b; font-style:italic;">
                                * Pakitukoy ang Tungkulin sa Iglesia, kung Nag-aaral o Nagtatrabaho, at kung Kasama sa tirahan
                            </span>
                        </div>
                        <div class="vru-children-list">
                            ${childrenCardsHtml}
                        </div>
                    </div>
                </div>
            </div>

            <!-- SECTION 5: VISITATION GROUP -->
            <div class="vru-section">
                <div class="vru-sec-hdr">
                    <span>5. Visitation Group &amp; Church Duty / Dalawahan</span>
                </div>
                <div class="vru-sec-body">
                    <div class="vru-grid-2col">
                        <div class="vru-field">
                            <span class="vru-lbl">Visitation Group:</span>
                            <span class="vru-curr">${isBlank ? '' : esc(w.visitationGroup || 'Unassigned')} &bull; ${isBlank ? '' : esc(w.visitationRole || 'Member')}</span>
                            <span class="vru-arrow">&rarr;</span>
                            <span class="vru-upd">Kasalukuyang Grupo: </span>
                        </div>
                        <div class="vru-field">
                            <span class="vru-lbl">Puna / Karagdagang Tala:</span>
                            <span class="vru-upd" style="flex:2;">__________________________________________________</span>
                        </div>
                    </div>
                </div>
            </div>

            <!-- SECTION 6: DECLARATION, VERIFICATION & SIGNATURES -->
            <div class="vru-sig-grid">
                <div class="vru-sig-box">
                    <div style="font-weight:800; text-transform:uppercase; margin-bottom:2px;">1. Katunayan ng Manggagawa</div>
                    <div style="font-size:6.5pt; color:#475569; font-style:italic; min-height:22px;">
                        "Pinatutunayan ko sa ilalim ng pananagutan na ang lahat ng impormasyong nakasaad dito ay totoo, wasto, at napapanahon."
                    </div>
                    <div class="vru-sig-line"></div>
                    <div style="font-weight:700;">${isBlank ? 'Limbag na Pangalan at Lagda' : esc(w.name || 'Pangalan ng Manggagawa')}</div>
                    <div style="font-size:6.5pt; color:#64748b;">Lagda ng Manggagawa &bull; Petsa: ______________</div>
                </div>

                <div class="vru-sig-box">
                    <div style="font-weight:800; text-transform:uppercase; margin-bottom:2px;">2. Pagpapatotoo ng Katiwala</div>
                    <div style="font-size:6.5pt; color:#475569; font-style:italic; min-height:22px;">
                        "Personal kong sinuri at bineripika ang impormasyon ng manggagawa sa itaas bago i-endorso sa talaan."
                    </div>
                    <div class="vru-sig-line"></div>
                    <div style="font-weight:700;">Lagda sa Ibabaw ng Limbag na Pangalan</div>
                    <div style="font-size:6.5pt; color:#64748b;">Katiwala / Verification Officer &bull; Petsa: ______________</div>
                </div>

                <div class="vru-sig-box" style="background:#f1f5f9; border-color:#94a3b8;">
                    <div style="font-weight:800; text-transform:uppercase; color:#0f172a; margin-bottom:2px;">3. Administrative / Encoding</div>
                    <div style="font-size:6.5pt; color:#475569; text-align:left; padding:2px 4px; min-height:22px;">
                        [ &nbsp; ] Na-encode at updated na sa PWA System<br>
                        [ &nbsp; ] Walang pagbabago (Record Verified)
                    </div>
                    <div class="vru-sig-line" style="border-color:#475569;"></div>
                    <div style="font-weight:700;">Pangalan ng Encoder</div>
                    <div style="font-size:6.5pt; color:#64748b;">System Encoder &bull; Petsa: ______________</div>
                </div>
            </div>

            <!-- FOOTER STAMP -->
            <div style="margin-top:4px; display:flex; justify-content:space-between; font-size:6.5pt; color:#94a3b8; border-top:1px solid #e2e8f0; padding-top:2px;">
                <span>Service 5 DB &bull; PWA Semi-Annual Records Verification</span>
                <span>Page 1 of 1 &bull; Document Classification: Internal Administrative Document</span>
            </div>
        </div>
        `;
    }

    function generateFullPrintableDocumentHTML(workers, options = {}) {
        const paperSize = options.paperSize || 'folio';
        const pageCssSize = paperSize === 'letter' ? '8.5in 11in' : (paperSize === 'a4' ? '210mm 297mm' : '8.5in 13in');

        const sheetsHtml = workers.map(w => generateSingleSheetHTML(w, options)).join('\n');

        return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Semi-Annual Volunteer Records Update Forms</title>
    <style>
        @page {
            size: ${pageCssSize};
            margin: 6mm 8mm;
        }
        *, *::before, *::after {
            box-sizing: border-box !important;
        }
        html, body {
            margin: 0 !important;
            padding: 0 !important;
            background: #ffffff !important;
            color: #0f172a !important;
            font-family: 'Inter', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif !important;
            font-size: 8.2pt !important;
            line-height: 1.3 !important;
            -webkit-print-color-adjust: exact !important;
            print-color-adjust: exact !important;
        }
        .vru-sheet {
            page-break-after: always !important;
            break-after: page !important;
            width: 100% !important;
            max-width: 100% !important;
            margin: 0 auto !important;
            padding: 0 !important;
            display: flex !important;
            flex-direction: column !important;
            background: #ffffff !important;
        }
        .vru-sheet:last-child {
            page-break-after: avoid !important;
            break-after: avoid !important;
        }
        .vru-header {
            display: flex;
            align-items: center;
            justify-content: space-between;
            border-bottom: 2px solid #0f172a;
            padding-bottom: 3px;
            margin-bottom: 4px;
        }
        .vru-inst-title {
            font-size: 7.2pt;
            font-weight: 800;
            color: #475569;
            letter-spacing: 0.08em;
            text-transform: uppercase;
        }
        .vru-title {
            font-size: 12.5pt;
            font-weight: 900;
            color: #0f172a;
            margin: 1px 0;
            letter-spacing: -0.01em;
        }
        .vru-subtitle {
            font-size: 7.8pt;
            color: #334155;
        }
        .vru-photo-box {
            width: 62px;
            height: 62px;
            border: 1.5px solid #64748b;
            border-radius: 4px;
            overflow: hidden;
            display: flex;
            align-items: center;
            justify-content: center;
            background: #f8fafc;
            flex-shrink: 0;
        }
        .vru-photo-box img {
            width: 100%;
            height: 100%;
            object-fit: cover;
        }
        .vru-photo-empty {
            font-size: 6.5pt;
            text-align: center;
            color: #64748b;
            padding: 3px;
            border-style: dashed;
        }
        .vru-notice {
            font-size: 7.5pt;
            background: #f8fafc;
            border-left: 3.5px solid #0284c7;
            padding: 3px 8px;
            margin-bottom: 4px;
            color: #1e293b;
            line-height: 1.3;
        }
        .vru-section {
            border: 1px solid #64748b;
            border-radius: 3px;
            margin-bottom: 4px;
            overflow: hidden;
            background: #ffffff;
        }
        .vru-sec-hdr {
            background: #e2e8f0;
            font-weight: 800;
            font-size: 7.8pt;
            padding: 2px 7px;
            color: #0f172a;
            border-bottom: 1px solid #94a3b8;
            text-transform: uppercase;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }
        .vru-sec-body {
            padding: 4px 7px;
        }
        .vru-grid-2col {
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 2px 14px;
        }
        .vru-field {
            font-size: 7.8pt;
            display: flex;
            align-items: center;
            gap: 4px;
            margin-bottom: 1px;
            min-height: 19px;
        }
        .vru-lbl {
            font-weight: 700;
            color: #334155;
            min-width: 95px;
            flex-shrink: 0;
            font-size: 7.6pt;
        }
        .vru-curr {
            font-weight: 700;
            color: #0f172a;
            background: #f1f5f9;
            padding: 1px 5px;
            border: 1px solid #cbd5e1;
            border-radius: 2px;
            flex: 1;
            min-width: 0;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
            font-size: 7.6pt;
        }
        .vru-arrow {
            color: #64748b;
            font-weight: 800;
            font-size: 7.8pt;
            flex-shrink: 0;
        }
        .vru-upd {
            border-bottom: 1.5px dotted #64748b;
            flex: 1.2;
            min-width: 20px;
            color: #0369a1;
            font-weight: 600;
            font-size: 7.4pt;
            padding-left: 2px;
            height: 16px;
            line-height: 16px;
        }
        .vru-chk {
            font-size: 7.4pt;
            color: #1e293b;
            white-space: nowrap;
        }
        .vru-table {
            width: 100%;
            border-collapse: collapse;
            font-size: 7.4pt;
            margin-top: 2px;
        }
        .vru-table th {
            background: #f1f5f9;
            border: 1px solid #cbd5e1;
            padding: 2px 4px;
            font-weight: 700;
            text-align: left;
            color: #334155;
        }
        .vru-table td {
            border: 1px solid #cbd5e1;
            padding: 2px 4px;
            height: 16px;
        }
        /* Child cards styling */
        .vru-children-list {
            display: flex;
            flex-direction: column;
            gap: 3px;
        }
        .vru-child-card {
            border: 1px solid #94a3b8;
            border-radius: 3px;
            background: #ffffff;
            padding: 3px 6px;
            font-size: 7.5pt;
        }
        .vru-child-blank {
            background: #fafafa;
            border-style: dashed;
            border-color: #94a3b8;
        }
        .vru-child-row {
            display: flex;
            align-items: center;
            gap: 6px;
            min-height: 18px;
            margin-bottom: 1px;
        }
        .vru-child-row:not(:first-child) {
            margin-top: 2px;
            border-top: 1px dotted #e2e8f0;
            padding-top: 2px;
        }
        .vru-child-num {
            background: #0f172a;
            color: #ffffff;
            font-weight: 800;
            font-size: 6.8pt;
            padding: 0 4px;
            border-radius: 2px;
            height: 15px;
            line-height: 15px;
            flex-shrink: 0;
            text-align: center;
            min-width: 18px;
        }
        .vru-child-spacer {
            min-width: 18px;
            flex-shrink: 0;
        }
        .vru-child-cell {
            display: flex;
            align-items: center;
            gap: 4px;
            min-width: 0;
        }
        .vru-sublbl {
            font-weight: 700;
            color: #334155;
            font-size: 7.4pt;
            flex-shrink: 0;
        }
        .vru-sig-grid {
            display: grid;
            grid-template-columns: 1fr 1fr 1fr;
            gap: 8px;
            margin-top: 4px;
            font-size: 7.2pt;
        }
        .vru-sig-box {
            text-align: center;
            border: 1px solid #94a3b8;
            border-radius: 3px;
            padding: 5px 8px;
            background: #fafafa;
        }
        .vru-sig-line {
            border-bottom: 1.5px solid #0f172a;
            margin: 18px 8px 3px;
        }
    </style>
</head>
<body>
    ${sheetsHtml}
</body>
</html>`;
    }

    function printUpdateForms(workersList, options = {}) {
        if (!workersList || !workersList.length) {
            if (typeof showToast === 'function') showToast('No volunteer records selected for printing.', 'error');
            return;
        }
        const html = generateFullPrintableDocumentHTML(workersList, options);
        if (typeof window.safePrint === 'function') {
            window.safePrint(html, {
                title: options.title || 'Semi-Annual Volunteer Records Update Form',
                orientation: 'portrait'
            });
        } else {
            const win = window.open('', '_blank');
            if (win) {
                win.document.open();
                win.document.write(html);
                win.document.close();
                win.focus();
                setTimeout(() => { win.print(); win.close(); }, 300);
            }
        }
    }
return {
    printUpdateForms: printUpdateForms
};
})();

// ===== visitations.js (minister summary sheets) - verbatim from 01_Rust_WC_DB/js/visitations.js =====
var S5_visitations = (function () {

function generateMinisterSummaryHtml(ministerName, visits) {
    const numVisits = visits.length;
    let pagePadding = '40px';
    let headerMarginBottom = '25px';
    let headerPaddingBottom = '20px';
    let tableMarginBottom = '30px';
    let cellPadding = '16px';
    let photoCellPadding = '12px';
    let photoBoxSize = '72px';
    let nameFontSize = '1.25rem';
    let infoFontSize = '0.75rem';
    let infoGridGap = '12px';
    let infoGridMarginBottom = '14px';
    let infoGridPaddingBottom = '12px';
    let noteLineHeight = '32px';
    let noteLineContainerGap = '24px';
    let noteLinesCount = 3;
    let showInstructions = true;
    let sigAreaMarginTop = '25px';
    let sigLineHeight = '48px';
    let statusGridGap = '8px';
    let statusPadding = '12px';
    let statusMarginBottom = '10px';
    let footerMarginTop = '25px';
    let compactOccasions = false;
    let spacingAfterTitle = '3px';
    let titleMainFontSize = '1.6rem';
    let titleSubFontSize = '0.8rem';
    let ministerNameFontSize = '1.05rem';
    let logoSize = '46';
    let schedFontSize = '0.6rem';
    let badgePadding = '4px 10px';
    let badgeFontSize = '0.7rem';
    let noteTitleFontSize = '0.65rem';
    let noteTitleMarginBottom = '8px';

    if (numVisits === 2) {
        pagePadding = '24px';
        headerMarginBottom = '15px';
        headerPaddingBottom = '12px';
        tableMarginBottom = '15px';
        cellPadding = '10px 12px';
        photoCellPadding = '8px';
        photoBoxSize = '56px';
        nameFontSize = '1.05rem';
        infoFontSize = '0.7rem';
        infoGridGap = '8px';
        infoGridMarginBottom = '8px';
        infoGridPaddingBottom = '8px';
        noteLineHeight = '24px';
        noteLineContainerGap = '14px';
        noteLinesCount = 2;
        showInstructions = true;
        sigAreaMarginTop = '15px';
        sigLineHeight = '36px';
        statusGridGap = '6px';
        statusPadding = '8px';
        statusMarginBottom = '6px';
        footerMarginTop = '15px';
        compactOccasions = true;
        titleMainFontSize = '1.45rem';
        titleSubFontSize = '0.78rem';
        ministerNameFontSize = '0.95rem';
        logoSize = '40';
        schedFontSize = '0.55rem';
        badgePadding = '3px 8px';
        badgeFontSize = '0.65rem';
        noteTitleFontSize = '0.62rem';
        noteTitleMarginBottom = '6px';
    } else if (numVisits >= 3) {
        pagePadding = '16px';
        headerMarginBottom = '10px';
        headerPaddingBottom = '8px';
        tableMarginBottom = '10px';
        cellPadding = '6px 8px';
        photoCellPadding = '6px';
        photoBoxSize = '44px';
        nameFontSize = '0.9rem';
        infoFontSize = '0.68rem';
        infoGridGap = '4px 6px';
        infoGridMarginBottom = '4px';
        infoGridPaddingBottom = '4px';
        noteLineHeight = '18px';
        noteLineContainerGap = '10px';
        noteLinesCount = 1;
        showInstructions = false;
        sigAreaMarginTop = '10px';
        sigLineHeight = '24px';
        statusGridGap = '4px';
        statusPadding = '6px';
        statusMarginBottom = '4px';
        footerMarginTop = '10px';
        compactOccasions = true;
        spacingAfterTitle = '1px';
        titleMainFontSize = '1.3rem';
        titleSubFontSize = '0.75rem';
        ministerNameFontSize = '0.9rem';
        logoSize = '36';
        schedFontSize = '0.5rem';
        badgePadding = '2px 6px';
        badgeFontSize = '0.6rem';
        noteTitleFontSize = '0.6rem';
        noteTitleMarginBottom = '4px';
    }

    const dottedLinesHtml = Array(noteLinesCount).fill(`<div class="dotted-note-line" style="height: ${noteLineHeight} !important;"></div>`).join('');

    return `
        <style>
            /* ponytail: removed Google Fonts @import for offline reliability */

            img {
                image-rendering: -webkit-optimize-contrast;
                -webkit-print-color-adjust: exact;
                print-color-adjust: exact;
            }

            @page {
                size: auto;
                margin: 8mm 10mm;
            }

            .visitation-print-page {
                font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Arial, sans-serif !important;
                color: #1e293b !important;
                background: #ffffff !important;
                box-sizing: border-box !important;
                break-inside: avoid !important;
                page-break-inside: avoid !important;
                page-break-after: always !important;
                break-after: page !important;
            }

            .visitation-print-page:last-child {
                page-break-after: auto !important;
                break-after: auto !important;
            }

            .visitation-title-main {
                font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif !important;
                font-weight: 800 !important;
                color: #111827 !important;
                letter-spacing: -0.5px !important;
            }

            .visitation-title-sub {
                font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Arial, sans-serif !important;
                font-weight: 700 !important;
                text-transform: uppercase !important;
                letter-spacing: 1px !important;
                color: #1b4332 !important;
            }

            .visitation-table {
                width: 100% !important;
                border-collapse: collapse !important;
                border: 1px solid #e2e8f0 !important;
                box-shadow: 0 1px 3px rgba(0,0,0,0.02) !important;
            }

            .visitation-table th {
                background: #1b4332 !important;
                color: #ffffff !important;
                font-family: 'Inter', sans-serif !important;
                font-size: 0.75rem !important;
                font-weight: 700 !important;
                text-transform: uppercase !important;
                letter-spacing: 0.5px !important;
                padding: 12px 16px !important;
                border: 1px solid #14532d !important;
            }

            .visitation-table td {
                border: 1px solid #e2e8f0 !important;
                padding: ${cellPadding} !important;
                vertical-align: top !important;
            }

            .visitation-table tbody tr:nth-child(even) {
                background: #f8fafc !important;
            }

            .worker-name-title {
                font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif !important;
                font-weight: 700 !important;
                font-size: ${nameFontSize} !important;
                color: #111827 !important;
            }

            .housing-badge {
                font-family: 'Inter', sans-serif !important;
                font-size: ${badgeFontSize} !important;
                font-weight: 700 !important;
                color: #b45309 !important;
                background: #fffbeb !important;
                padding: ${badgePadding} !important;
                border: 1px solid #fde68a !important;
                border-radius: 2px !important;
                text-transform: uppercase !important;
                letter-spacing: 0.5px !important;
            }

            .profile-info-grid {
                display: grid !important;
                grid-template-columns: 1fr 1fr 1fr !important;
                gap: ${infoGridGap} !important;
                font-size: ${infoFontSize} !important;
                margin-bottom: ${infoGridMarginBottom} !important;
                padding-bottom: ${infoGridPaddingBottom} !important;
                border-bottom: 1px dashed #cbd5e1 !important;
            }

            .profile-info-label {
                font-weight: 700 !important;
                color: #64748b !important;
            }

            .profile-info-val {
                color: #334155 !important;
            }

            .note-line-title {
                font-size: ${noteTitleFontSize} !important;
                font-weight: 800 !important;
                text-transform: uppercase !important;
                color: #475569 !important;
                letter-spacing: 0.5px !important;
                margin-bottom: ${noteTitleMarginBottom} !important;
            }

            .dotted-note-line {
                border-bottom: 1.5px dotted #cbd5e1 !important;
            }

            .status-grid-box {
                display: flex !important;
                align-items: center !important;
                gap: 6px !important;
                font-size: ${numVisits >= 3 ? '0.7rem' : '0.75rem'} !important;
                font-weight: 500 !important;
                color: #334155 !important;
            }

            .status-checkbox-mock {
                width: 14px !important;
                height: 14px !important;
                border: 1.5px solid #94a3b8 !important;
                border-radius: 3px !important;
                background: #fff !important;
            }

            .sig-line {
                height: ${sigLineHeight} !important;
                border-bottom: 1.5px solid #cbd5e1 !important;
            }

            .sig-label {
                font-size: 0.65rem !important;
                font-weight: 700 !important;
                text-align: center !important;
                margin-top: 6px !important;
                text-transform: uppercase !important;
                color: #64748b !important;
                letter-spacing: 0.5px !important;
            }
        </style>

        <div class="visitation-print-page" style="padding: ${pagePadding}; position: relative;">

            <div style="display: flex; justify-content: space-between; align-items: center; border-bottom: 2.5px solid #1b4332; padding-bottom: ${headerPaddingBottom}; margin-bottom: ${headerMarginBottom};">

                <div style="display: flex; align-items: center; gap: 14px;">

                    <svg width="${logoSize}" height="${logoSize}" viewBox="0 0 100 100" fill="none" xmlns="http://www.w3.org/2000/svg" style="color: #1b4332;">
                        <circle cx="50" cy="50" r="46" stroke="currentColor" stroke-width="3" fill="#f0fdf4"/>
                        <path d="M50 18L72 58H28L50 18Z" fill="currentColor" opacity="0.15"/>
                        <path d="M50 14L78 64H22L50 14Z" stroke="currentColor" stroke-width="4" stroke-linejoin="round"/>
                        <path d="M50 32L68 64H32L50 32Z" fill="currentColor"/>
                        <rect x="46" y="64" width="8" height="18" fill="currentColor"/>
                        <path d="M15 82H85" stroke="currentColor" stroke-width="4" stroke-linecap="round"/>
                    </svg>

                    <div>
                        <h1 class="visitation-title-main" style="margin: 0; font-size: ${titleMainFontSize}; line-height: 1.1;">WOODLAND CONSTRUCTION</h1>
                        <p class="visitation-title-sub" style="margin: ${spacingAfterTitle} 0 0; font-size: ${titleSubFontSize};">Minister's Visitation Summary Sheet</p>
                    </div>

                </div>

                <div style="text-align: right;">
                    <div style="font-size: 0.75rem; font-weight: 700; color: #64748b; text-transform: uppercase; letter-spacing: 0.5px;">Visiting Minister</div>
                    <div style="font-size: ${ministerNameFontSize}; font-weight: 800; color: #1b4332; margin-top: 2px;">${esc(ministerName.toUpperCase())}</div>
                    <div style="font-size: 0.7rem; color: #94a3b8; margin-top: 4px; font-weight: 500;">Date Generated: ${new Date().toLocaleDateString('en-US', {year: 'numeric', month: 'short', day: 'numeric'})}</div>
                </div>

            </div>

            <div style="margin-bottom: ${tableMarginBottom};">

                <table class="visitation-table">
                    <thead>
                        <tr>
                            <th style="width: ${photoBoxSize === '72px' ? '80px' : (photoBoxSize === '56px' ? '64px' : '52px')}; text-align: center;">Photo</th>
                            <th>Volunteer Profile & Visitation Record</th>
                        </tr>
                    </thead>
                    <tbody>
                        ${visits.map(v => {
                            const worker = workersData.find(w => w.name === v.workerName) || {};

                            const visitDate = new Date(v.date);
                            const vYear = visitDate.getFullYear();
                            const nearbyOccasions = [];

                            const checkOccasion = (label, dateStr) => {
                                if (!dateStr) return;
                                const milestone = new Date(dateStr);
                                [vYear - 1, vYear, vYear + 1].forEach(yr => {
                                    const anniv = new Date(yr, milestone.getMonth(), milestone.getDate());
                                    const diff = Math.round((anniv - visitDate) / (1000 * 3600 * 24));
                                    if (Math.abs(diff) <= 14) {
                                        nearbyOccasions.push({
                                            label,
                                            date: anniv,
                                            days: Math.round(diff)
                                        });
                                    }
                                });
                            };

                            checkOccasion("Worker's Birthday", worker.birthday);
                            checkOccasion("Spouse's Birthday", worker.spouseBirthday);
                            checkOccasion("Wedding Anniversary", worker.weddingDate);
                            if (worker.children) {
                                worker.children.forEach(c => checkOccasion(`${esc(c.name)}'s Birthday`, c.birthday));
                            }

                            return `
                                <tr>
                                    <td style="padding: ${photoCellPadding}; text-align: center; vertical-align: top; width: ${photoBoxSize === '72px' ? '80px' : (photoBoxSize === '56px' ? '64px' : '52px')}; background: #ffffff;">
                                        <div style="width: ${photoBoxSize}; height: ${photoBoxSize}; border: 1.5px solid #cbd5e1; border-radius: 2px; background: #f8fafc; margin: 0 auto; overflow: hidden; display: flex; align-items: center; justify-content: center; margin-bottom: 8px; box-shadow: inset 0 1px 2px rgba(0,0,0,0.05);">
                                            ${(worker.photoUrl || worker.photo)
                                                ? `<img src="${esc(worker.photoUrl || worker.photo)}" style="width: 100%; height: 100%; object-fit: cover; image-rendering: -webkit-optimize-contrast; -webkit-print-color-adjust: exact; print-color-adjust: exact;">`
                                                : `<span style="font-size: 0.65rem; color: #cbd5e1; font-weight: 700; text-align: center; line-height: 1.2;">NO<br>PHOTO</span>`
                                            }
                                        </div>
                                        <div style="font-size: ${schedFontSize}; font-weight: 800; color: #64748b; border-top: 1px solid #f1f5f9; padding-top: 6px; text-transform: uppercase; letter-spacing: 0.5px;">
                                            SCHED:<br><span style="color: #334155;">${new Date(v.date).toLocaleDateString('en-US', {month: '2-digit', day: '2-digit', year: '2-digit'})}</span>
                                        </div>
                                    </td>
                                    <td style="padding: ${cellPadding}; vertical-align: top; background: #ffffff;">

                                        <div style="display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 8px;">
                                            <div class="worker-name-title">${esc(worker.name || v.workerName)}</div>
                                            <div class="housing-badge">${esc(worker.visitationHousing || 'UNASSIGNED HOUSING')}</div>
                                        </div>

                                        <div class="profile-info-grid">
                                            <div><span class="profile-info-label">Trade:</span> <span class="profile-info-val">${esc(worker.trade || '---')}</span></div>
                                            <div><span class="profile-info-label">Phone:</span> <span class="profile-info-val">${esc(worker.phone || '---')}</span></div>
                                            <div><span class="profile-info-label">Birthday:</span> <span class="profile-info-val">${worker.birthday ? displayDate(worker.birthday) : '---'}</span></div>
                                            <div><span class="profile-info-label">Spouse:</span> <span class="profile-info-val">${esc(worker.spouseName || '---')}${worker.spouseChurchDuty ? ' · Church: ' + esc(worker.spouseChurchDuty) : ''}</span></div>
                                            <div><span class="profile-info-label">Spouse Bday:</span> <span class="profile-info-val">${worker.spouseBirthday ? displayDate(worker.spouseBirthday) : '---'}</span></div>
                                            <div><span class="profile-info-label">Anniversary:</span> <span class="profile-info-val">${worker.weddingDate ? displayDate(worker.weddingDate) : '---'}</span></div>
                                            <div style="grid-column: span 3;"><span class="profile-info-label">Address:</span> <span class="profile-info-val">${esc(worker.address || '---')}</span></div>
                                            <div style="grid-column: span 3;"><span class="profile-info-label">Children:</span> <span class="profile-info-val">${(worker.children && worker.children.length > 0) ? '<div style="margin-top:2px;">' + worker.children.map(c => {
                                                const parts = [esc(c.name)];
                                                if (c.age) parts.push(c.age + 'yo');
                                                if (c.isStudent) parts.push('Student');
                                                if (c.school) parts.push('School: ' + esc(c.school));
                                                if (c.course) parts.push(esc(c.course));
                                                if (c.job) parts.push('Work: ' + esc(c.job));
                                                if (c.workplace) parts.push('@ ' + esc(c.workplace));
                                                if (c.churchDuty) parts.push('Church: ' + esc(c.churchDuty));
                                                if (c.livesHere) parts.push('Lives here');
                                                return '<div style="font-size:0.65rem; padding:1px 0; line-height:1.3;">' + parts.join(' · ') + '</div>';
                                            }).join('') + '</div>' : '---'}</span></div>
                                        </div>

                                        ${nearbyOccasions.length > 0 ? (
                                            compactOccasions
                                            ? `
                                            <div style="margin-bottom: 6px; background: #f0fdf4; border: 1px solid #bbf7d0; border-radius: 4px; padding: 4px 8px; display: flex; align-items: center; gap: 8px;">
                                                <span style="font-size: 0.6rem; font-weight: 800; text-transform: uppercase; color: #15803d; letter-spacing: 0.3px; white-space: nowrap;">Occasions:</span>
                                                <div style="display: flex; flex-wrap: wrap; gap: 8px; font-size: 0.68rem; color: #166534;">
                                                    ${nearbyOccasions.map(occ => `
                                                        <span style="white-space: nowrap;"><strong style="color: #065f46;">${esc(occ.label)}:</strong> ${occ.date.toLocaleDateString('en-US', {month: 'short', day: 'numeric'})} <span style="font-size: 0.6rem; color: #15803d; font-weight: 700;">(${occ.days === 0 ? 'TODAY' : (occ.days > 0 ? `in ${occ.days}d` : `${Math.abs(occ.days)}d ago`)})</span></span>
                                                    `).join(' <span style="color: #bbf7d0;">|</span> ')}
                                                </div>
                                            </div>
                                            `
                                            : `
                                            <div style="margin-bottom: 14px; background: #f0fdf4; border: 1px solid #bbf7d0; border-radius: 2px; padding: 10px;">
                                                <div style="font-size: 0.65rem; font-weight: 800; text-transform: uppercase; color: #15803d; margin-bottom: 4px; letter-spacing: 0.5px;">Upcoming / Recent Occasions:</div>
                                                <div style="display: flex; flex-wrap: wrap; gap: 12px;">
                                                    ${nearbyOccasions.map(occ => `
                                                        <div style="font-size: 0.75rem; color: #166534;">
                                                            <strong style="color: #065f46;">${esc(occ.label)}:</strong> ${occ.date.toLocaleDateString('en-US', {month: 'short', day: 'numeric'})}
                                                            <span style="font-size: 0.65rem; color: #34d399; font-weight: 800;">(${occ.days === 0 ? 'TODAY' : (occ.days > 0 ? `in ${occ.days}d` : `${Math.abs(occ.days)}d ago`)})</span>
                                                        </div>
                                                    `).join('')}
                                                </div>
                                            </div>
                                            `
                                        ) : ''}

                                        <div style="display: grid; grid-template-columns: 1fr 1fr; gap: ${noteLineContainerGap};">
                                            <div class="note-line-container">
                                                <div class="note-line-title" style="font-size: ${noteTitleFontSize} !important; margin-bottom: ${noteTitleMarginBottom} !important;">Family Members Present:</div>
                                                ${dottedLinesHtml}
                                            </div>
                                            <div class="note-line-container">
                                                <div class="note-line-title" style="font-size: ${noteTitleFontSize} !important; margin-bottom: ${noteTitleMarginBottom} !important;">Issues/Concerns Raised:</div>
                                                ${dottedLinesHtml}
                                            </div>
                                        </div>

                                    </td>
                                </tr>
                            `;
                        }).join('')}
                    </tbody>
                </table>

            </div>

            <div style="margin-top: ${sigAreaMarginTop}; break-inside: avoid;">
                <div style="display: flex; flex-direction: column; justify-content: space-between;">
                    <div style="display: flex; gap: 20px;">
                        <div style="flex:1;">
                            <div class="sig-line"></div>
                            <div class="sig-label">Visiting Minister</div>
                        </div>
                        <div style="flex:1;">
                            <div class="sig-line"></div>
                            <div class="sig-label">Volunteer / Head of Household</div>
                        </div>
                    </div>
                    <div style="font-size: ${numVisits >= 3 ? '0.7rem' : '0.75rem'}; color: #64748b; font-weight: 600; margin-top: ${numVisits >= 3 ? '10px' : '15px'}; text-align: right;">
                        Date Visited: ____________________
                    </div>
                </div>
            </div>

            <div style="margin-top: ${footerMarginTop}; border-top: 1px solid #e2e8f0; padding-top: 12px; text-align: center; break-inside: avoid;">
                <div style="font-size: ${numVisits >= 3 ? '0.6rem' : '0.65rem'}; color: #94a3b8; font-style: italic; font-weight: 500; letter-spacing: 0.2px;">
                    Confidential Administrative Document &bull; Service Section Workforce &bull; Return to Workforce Admin post-visit
                </div>
            </div>

        </div>
    `;
}
return {
    generateMinisterSummaryHtml: generateMinisterSummaryHtml
};
})();

// ===== attendance_generator.js - verbatim from 01_Rust_WC_DB/js/attendance_generator.js =====
var S5_attendance_generator = (function () {

    let _selectedWorkerIds = new Set();

    let _sheetTitle = 'Special Meeting / Project Attendance';

    let _sheetDate = new Date().toISOString().split('T')[0];

    let _sheetFacilitator = '';

    let _sheetVenue = '';

    let _sheetType = 'meeting';

    let _includePhotos = true;

    let _groupBy = 'none';

    let _sortBy = 'name';

    let _pageBreakPerGroup = true;

    function esc(str) {
        if (str == null) return '';
        return String(str)
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;')
            .replace(/"/g, '&quot;')
            .replace(/'/g, '&#039;');
    }

    function getWorkerKey(w) {
        if (!w) return '';
        if (w.workerId) return String(w.workerId);
        if (w.aNumber != null && String(w.aNumber).trim() !== '') return String(w.aNumber);
        if (w.id != null && String(w.id).trim() !== '') return String(w.id);
        return 'w_' + encodeURIComponent(w.name || '');
    }

    function getAge(birthday) {
        if (!birthday) return 0;
        const b = new Date(birthday);
        if (isNaN(b.getTime())) return 0;
        const now = new Date();
        let age = now.getFullYear() - b.getFullYear();
        const m = now.getMonth() - b.getMonth();
        if (m < 0 || (m === 0 && now.getDate() < b.getDate())) {
            age--;
        }
        return age > 0 ? age : 0;
    }

    function getPrintableResidence(w) {
        if (!w) return '-';
        const housing = (w.visitationHousing || '').trim();
        const isNonInc = /non[\s_-]?inc/i.test(housing);
        if (isNonInc) {
            if (w.address && w.address.trim()) {
                return w.address.trim();
            }
            return 'Non INC Resident';
        }
        return housing || w.address || '-';
    }

    function getGroupValue(w, groupKey) {
        if (!w) return 'Unassigned';
        switch (groupKey) {
            case 'trade':
                return (w.trade || '').trim() || 'General / Unassigned Trade';
            case 'visitationGroup':
                return (w.visitationGroup || '').trim() || 'No Visitation Group';
            case 'projectGroup':
                return (w.projectGroup || '').trim() || 'No Project Group';
            case 'area':
                return (w.area || '').trim() || 'Unassigned Area';
            case 'visitationHousing':
                return getPrintableResidence(w);
            default:
                return 'General';
        }
    }

    function getGroupLabel(groupKey) {
        switch (groupKey) {
            case 'trade':
                return 'Trade / Skill';
            case 'visitationGroup':
                return 'Visitation Group';
            case 'projectGroup':
                return 'Project Group';
            case 'area':
                return 'Duty Area';
            case 'visitationHousing':
                return 'Housing / Residence';
            default:
                return 'Group';
        }
    }

    function sortWorkers(list, sortKey = 'name') {
        const sorted = [...list];
        switch (sortKey) {
            case 'aNumber':
                return sorted.sort((a, b) => {
                    const aNum = a.aNumber != null && String(a.aNumber).trim() !== '' ? parseInt(a.aNumber, 10) : NaN;
                    const bNum = b.aNumber != null && String(b.aNumber).trim() !== '' ? parseInt(b.aNumber, 10) : NaN;
                    if (isNaN(aNum) && isNaN(bNum)) return (a.name || '').localeCompare(b.name || '');
                    if (isNaN(aNum)) return 1;
                    if (isNaN(bNum)) return -1;
                    if (aNum !== bNum) return aNum - bNum;
                    return (a.name || '').localeCompare(b.name || '');
                });
            case 'trade':
                return sorted.sort((a, b) => {
                    const tA = (a.trade || '').trim();
                    const tB = (b.trade || '').trim();
                    const comp = tA.localeCompare(tB);
                    if (comp !== 0) return comp;
                    return (a.name || '').localeCompare(b.name || '');
                });
            case 'age_asc':
                return sorted.sort((a, b) => {
                    const ageA = a.birthday ? getAge(a.birthday) : 9999;
                    const ageB = b.birthday ? getAge(b.birthday) : 9999;
                    if (ageA !== ageB) return ageA - ageB;
                    return (a.name || '').localeCompare(b.name || '');
                });
            case 'age_desc':
                return sorted.sort((a, b) => {
                    const ageA = a.birthday ? getAge(a.birthday) : -1;
                    const ageB = b.birthday ? getAge(b.birthday) : -1;
                    if (ageA !== ageB) return ageB - ageA;
                    return (a.name || '').localeCompare(b.name || '');
                });
            case 'name':
            default:
                return sorted.sort((a, b) => (a.name || '').localeCompare(b.name || ''));
        }
    }

    function renderRow(w, rowNumber, sheetType, includePhotos) {
        const photo = w.photoUrl || w.photo;
        const aNum = w.aNumber != null ? String(w.aNumber) : '';
        const age = w.birthday ? getAge(w.birthday) : 0;
        const residence = getPrintableResidence(w);

        const avatarHtml = photo
            ? `<img src="${esc(photo)}" style="width: 28px; height: 28px; border-radius: 4px; object-fit: cover; border: 1px solid #cbd5e1; display: block; margin: 0 auto;">`
            : `<div style="width: 28px; height: 28px; border-radius: 4px; background: #f1f5f9; color: #475569; font-weight: 700; font-size: 0.75rem; display: flex; align-items: center; justify-content: center; margin: 0 auto; border: 1px solid #cbd5e1;">${esc((w.name || '?').charAt(0).toUpperCase())}</div>`;

        if (sheetType === 'meeting') {
            return `
                <tr>
                    <td style="text-align: center; color: #64748b; font-weight: 600; width: 34px;">${rowNumber}</td>
                    ${includePhotos ? `<td style="text-align: center; padding: 2px; width: 40px;">${avatarHtml}</td>` : ''}
                    <td style="font-weight: 600; color: #0f172a; font-size: 0.86rem;">${esc(w.name)}</td>
                    <td style="color: #334155; font-size: 0.82rem; width: 220px;">${esc(w.trade || 'Volunteer')}</td>
                    <td style="border-bottom: 1px solid #94a3b8; height: 34px; width: 260px;"></td>
                </tr>
            `;
        } else if (sheetType === 'attendance') {
            return `
                <tr>
                    <td style="text-align: center; color: #64748b; font-weight: 600;">${rowNumber}</td>
                    ${includePhotos ? `<td style="text-align: center; padding: 2px;">${avatarHtml}</td>` : ''}
                    <td style="font-weight: 700; text-align: center; color: #1e293b; font-size: 0.75rem; white-space: nowrap;">${aNum ? '#' + esc(aNum) : '-'}</td>
                    <td style="font-weight: 600; color: #0f172a;">${esc(w.name)}</td>
                    <td style="text-align: center; color: #334155; font-size: 0.75rem; white-space: nowrap;">${age ? age + ' yo' : '-'}</td>
                    <td style="color: #475569;">${esc(w.trade || 'Volunteer')}</td>
                    <td style="color: #475569; font-size: 0.72rem;">${esc(residence)}</td>
                    <td style="border-bottom: 1px solid #cbd5e1; height: 24px;"></td>
                    <td style="border-bottom: 1px solid #cbd5e1; height: 24px;"></td>
                    <td style="border-bottom: 1px solid #cbd5e1; height: 24px;"></td>
                </tr>
            `;
        } else {
            return `
                <tr>
                    <td style="text-align: center; color: #64748b; font-weight: 600;">${rowNumber}</td>
                    ${includePhotos ? `<td style="text-align: center; padding: 2px;">${avatarHtml}</td>` : ''}
                    <td style="font-weight: 700; text-align: center; color: #1e293b; font-size: 0.75rem; white-space: nowrap;">${aNum ? '#' + esc(aNum) : '-'}</td>
                    <td style="font-weight: 600; color: #0f172a;">${esc(w.name)}</td>
                    <td style="text-align: center; color: #334155; font-size: 0.75rem; white-space: nowrap;">${age ? age + ' yo' : '-'}</td>
                    <td style="color: #475569;">${esc(w.trade || 'Volunteer')}</td>
                    <td style="color: #475569; font-size: 0.72rem;">${esc(residence)}</td>
                    <td style="color: #334155; font-size: 0.75rem; white-space: nowrap;">${esc(w.phone || '-')}</td>
                    <td style="color: #64748b; font-size: 0.72rem;">${esc(w.specificTask || w.notes || '')}</td>
                </tr>
            `;
        }
    }

    function renderSignOffGrid() {
        return `
            <div class="sign-off-grid">
                <div class="sign-box">
                    <div>Conducted / Facilitated by:</div>
                    <div class="sign-line"></div>
                    <div style="font-size: 0.72rem; color: #64748b; margin-top: 3px;">Printed Name &amp; Signature</div>
                </div>
                <div class="sign-box">
                    <div>Checked / Recorded by:</div>
                    <div class="sign-line"></div>
                    <div style="font-size: 0.72rem; color: #64748b; margin-top: 3px;">Secretary / Team Lead</div>
                </div>
                <div class="sign-box">
                    <div>Approved by:</div>
                    <div class="sign-line"></div>
                    <div style="font-size: 0.72rem; color: #64748b; margin-top: 3px;">Project Overseer / Minister</div>
                </div>
            </div>
        `;
    }

    function generatePrintSheet() {
        if (_selectedWorkerIds.size === 0) {
            alert('Please select at least one volunteer to generate an attendance sheet.');
            return;
        }

        // Read dynamic config inputs
        const titleEl = document.getElementById('attgen-sheet-title');
        if (titleEl) _sheetTitle = titleEl.value.trim() || 'Attendance Sheet';

        const dateEl = document.getElementById('attgen-sheet-date');
        if (dateEl) _sheetDate = dateEl.value || new Date().toISOString().split('T')[0];

        const facEl = document.getElementById('attgen-sheet-facilitator');
        if (facEl) _sheetFacilitator = facEl.value.trim();

        const venEl = document.getElementById('attgen-sheet-venue');
        if (venEl) _sheetVenue = venEl.value.trim();

        const typeEl = document.getElementById('attgen-sheet-type');
        if (typeEl) _sheetType = typeEl.value || 'attendance';

        const photoEl = document.getElementById('attgen-sheet-photo-toggle');
        if (photoEl) _includePhotos = photoEl.checked;

        const groupByEl = document.getElementById('attgen-group-by');
        if (groupByEl) _groupBy = groupByEl.value || 'none';

        const sortByEl = document.getElementById('attgen-sort-by');
        if (sortByEl) _sortBy = sortByEl.value || 'name';

        const pagebreakEl = document.getElementById('attgen-pagebreak-toggle');
        if (pagebreakEl) _pageBreakPerGroup = pagebreakEl.checked;

        const workers = window.workersData || [];
        const selectedWorkers = workers.filter(w => _selectedWorkerIds.has(getWorkerKey(w)));

        const formattedDate = new Date(_sheetDate + 'T00:00:00').toLocaleDateString('en-US', {
            month: 'long',
            day: 'numeric',
            year: 'numeric'
        });

        let colCount = 4;
        let tableHeaderHtml = '';

        if (_sheetType === 'meeting') {
            colCount = _includePhotos ? 5 : 4;
            tableHeaderHtml = `
                <tr>
                    <th style="width: 34px; text-align: center;">#</th>
                    ${_includePhotos ? '<th style="width: 40px; text-align: center;">Photo</th>' : ''}
                    <th style="text-align: left;">Volunteer Name</th>
                    <th style="width: 220px; text-align: left;">Trade / Skill</th>
                    <th style="width: 260px; text-align: center;">Signature</th>
                </tr>
            `;
        } else if (_sheetType === 'attendance') {
            colCount = _includePhotos ? 10 : 9;
            tableHeaderHtml = `
                <tr>
                    <th style="width: 26px; text-align: center;">#</th>
                    ${_includePhotos ? '<th style="width: 38px; text-align: center;">Photo</th>' : ''}
                    <th style="width: 44px; text-align: center;">A-No.</th>
                    <th style="text-align: left;">Volunteer Name</th>
                    <th style="width: 38px; text-align: center;">Age</th>
                    <th style="text-align: left;">Trade / Skill</th>
                    <th style="text-align: left;">Residence / Housing</th>
                    <th style="width: 80px; text-align: center;">Time In</th>
                    <th style="width: 80px; text-align: center;">Time Out</th>
                    <th style="width: 105px; text-align: center;">Signature</th>
                </tr>
            `;
        } else {
            colCount = _includePhotos ? 9 : 8;
            tableHeaderHtml = `
                <tr>
                    <th style="width: 26px; text-align: center;">#</th>
                    ${_includePhotos ? '<th style="width: 38px; text-align: center;">Photo</th>' : ''}
                    <th style="width: 44px; text-align: center;">A-No.</th>
                    <th style="text-align: left;">Volunteer Name</th>
                    <th style="width: 38px; text-align: center;">Age</th>
                    <th style="text-align: left;">Trade / Skill</th>
                    <th style="text-align: left;">Residence / Housing</th>
                    <th style="width: 95px; text-align: left;">Contact No.</th>
                    <th style="text-align: left;">Task / Role Remarks</th>
                </tr>
            `;
        }

        const hasGrouping = _groupBy && _groupBy !== 'none';
        let bodyContentHtml = '';

        if (!hasGrouping) {
            const sortedWorkers = sortWorkers(selectedWorkers, _sortBy);
            const tableRowsHtml = sortedWorkers.map((w, idx) => renderRow(w, idx + 1, _sheetType, _includePhotos)).join('');

            bodyContentHtml = `
                <div class="print-sheet-wrapper">
                    <div class="header">
                        <h1>Woodland Construction</h1>
                        <h2>${esc(_sheetTitle)}</h2>
                        <div class="header-meta">
                            <div><strong>Date:</strong> ${esc(formattedDate)}${_sheetVenue ? ` • <strong>Venue:</strong> ${esc(_sheetVenue)}` : ''}</div>
                            <div><strong>Attendees:</strong> ${sortedWorkers.length} volunteer(s)</div>
                            ${_sheetFacilitator ? `<div><strong>Presided by:</strong> ${esc(_sheetFacilitator)}</div>` : ''}
                        </div>
                    </div>

                    <table>
                        <thead>
                            ${tableHeaderHtml}
                        </thead>
                        <tbody>
                            ${tableRowsHtml}
                        </tbody>
                    </table>

                    ${renderSignOffGrid()}
                </div>
            `;
        } else if (_pageBreakPerGroup) {
            const groupsMap = {};
            selectedWorkers.forEach(w => {
                const gVal = getGroupValue(w, _groupBy);
                if (!groupsMap[gVal]) groupsMap[gVal] = [];
                groupsMap[gVal].push(w);
            });
            const sortedGroupNames = Object.keys(groupsMap).sort((a, b) => a.localeCompare(b));
            const groupLabel = getGroupLabel(_groupBy);

            const pagesHtml = sortedGroupNames.map(gName => {
                const groupWorkers = sortWorkers(groupsMap[gName], _sortBy);
                const tableRowsHtml = groupWorkers.map((w, idx) => renderRow(w, idx + 1, _sheetType, _includePhotos)).join('');

                return `
                    <div class="print-sheet-wrapper print-sheet-group-page">
                        <div class="header">
                            <h1>Woodland Construction</h1>
                            <h2>${esc(_sheetTitle)}</h2>
                            <div class="group-banner">
                                <span class="group-banner-tag">${esc(groupLabel)}</span>
                                <span class="group-banner-name">${esc(gName)}</span>
                                <span class="group-banner-count">(${groupWorkers.length} volunteer${groupWorkers.length === 1 ? '' : 's'})</span>
                            </div>
                            <div class="header-meta">
                                <div><strong>Date:</strong> ${esc(formattedDate)}${_sheetVenue ? ` • <strong>Venue:</strong> ${esc(_sheetVenue)}` : ''}</div>
                                <div><strong>Group Attendees:</strong> ${groupWorkers.length} of ${selectedWorkers.length} total</div>
                                ${_sheetFacilitator ? `<div><strong>Presided by:</strong> ${esc(_sheetFacilitator)}</div>` : ''}
                            </div>
                        </div>

                        <table>
                            <thead>
                                ${tableHeaderHtml}
                            </thead>
                            <tbody>
                                ${tableRowsHtml}
                            </tbody>
                        </table>

                        ${renderSignOffGrid()}
                    </div>
                `;
            }).join('');

            bodyContentHtml = pagesHtml;
        } else {
            const groupsMap = {};
            selectedWorkers.forEach(w => {
                const gVal = getGroupValue(w, _groupBy);
                if (!groupsMap[gVal]) groupsMap[gVal] = [];
                groupsMap[gVal].push(w);
            });
            const sortedGroupNames = Object.keys(groupsMap).sort((a, b) => a.localeCompare(b));
            const groupLabel = getGroupLabel(_groupBy);

            let continuousCounter = 0;
            const groupedTbodyHtml = sortedGroupNames.map(gName => {
                const groupWorkers = sortWorkers(groupsMap[gName], _sortBy);
                const sectionHeaderRow = `
                    <tr class="group-section-row">
                        <td colspan="${colCount}" style="background: #e2e8f0 !important; color: #0f172a !important; font-weight: 800 !important; font-size: 0.78rem !important; text-transform: uppercase !important; padding: 7px 10px !important; letter-spacing: 0.04em !important; border-top: 2px solid #94a3b8 !important; border-bottom: 1.5px solid #cbd5e1 !important;">
                            📁 ${esc(groupLabel)}: ${esc(gName)} &nbsp;<span style="font-weight: 600; color: #475569; text-transform: none; font-size: 0.72rem;">(${groupWorkers.length} volunteer${groupWorkers.length === 1 ? '' : 's'})</span>
                        </td>
                    </tr>
                `;
                const memberRows = groupWorkers.map(w => {
                    continuousCounter++;
                    return renderRow(w, continuousCounter, _sheetType, _includePhotos);
                }).join('');

                return sectionHeaderRow + memberRows;
            }).join('');

            bodyContentHtml = `
                <div class="print-sheet-wrapper">
                    <div class="header">
                        <h1>Woodland Construction</h1>
                        <h2>${esc(_sheetTitle)}</h2>
                        <div class="header-meta">
                            <div><strong>Date:</strong> ${esc(formattedDate)}${_sheetVenue ? ` • <strong>Venue:</strong> ${esc(_sheetVenue)}` : ''}</div>
                            <div><strong>Attendees:</strong> ${selectedWorkers.length} volunteer(s) grouped by ${esc(groupLabel)}</div>
                            ${_sheetFacilitator ? `<div><strong>Presided by:</strong> ${esc(_sheetFacilitator)}</div>` : ''}
                        </div>
                    </div>

                    <table>
                        <thead>
                            ${tableHeaderHtml}
                        </thead>
                        <tbody>
                            ${groupedTbodyHtml}
                        </tbody>
                    </table>

                    ${renderSignOffGrid()}
                </div>
            `;
        }

        const html = `<!DOCTYPE html>
            <html>
            <head>
                <meta charset="UTF-8">
                <title>${esc(_sheetTitle)} - ${esc(formattedDate)}</title>
                <style>
                    * {
                        box-sizing: border-box !important;
                    }
                    @page {
                        size: auto;
                        margin: 8mm 10mm;
                    }
                    html, body {
                        margin: 0 !important;
                        padding: 0 !important;
                        background: #ffffff !important;
                        color: #0f172a !important;
                        font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
                        -webkit-print-color-adjust: exact !important;
                        print-color-adjust: exact !important;
                    }
                    .print-sheet-wrapper {
                        width: 100% !important;
                        max-width: 100% !important;
                        padding-top: 2mm;
                        box-sizing: border-box !important;
                    }
                    .print-sheet-group-page {
                        page-break-after: always !important;
                        break-after: page !important;
                    }
                    .print-sheet-group-page:last-child {
                        page-break-after: auto !important;
                        break-after: auto !important;
                    }
                    .header {
                        text-align: center;
                        border-bottom: 2px solid #0f172a;
                        padding-bottom: 8px;
                        margin-bottom: 12px;
                    }
                    .header h1 {
                        margin: 0;
                        font-size: 1.25rem;
                        letter-spacing: 0.5px;
                        text-transform: uppercase;
                        color: #0f172a;
                        font-weight: 800;
                    }
                    .header h2 {
                        margin: 3px 0 0;
                        font-size: 1.05rem;
                        color: #1e40af;
                        font-weight: 700;
                    }
                    .group-banner {
                        display: inline-flex;
                        align-items: center;
                        gap: 8px;
                        background: #f1f5f9;
                        border: 1.5px solid #cbd5e1;
                        border-radius: 4px;
                        padding: 3px 10px;
                        margin: 6px auto 4px;
                    }
                    .group-banner-tag {
                        font-size: 0.7rem;
                        font-weight: 700;
                        text-transform: uppercase;
                        color: #475569;
                        letter-spacing: 0.04em;
                    }
                    .group-banner-name {
                        font-size: 0.92rem;
                        font-weight: 800;
                        color: #1e40af;
                    }
                    .group-banner-count {
                        font-size: 0.72rem;
                        font-weight: 600;
                        color: #64748b;
                    }
                    .header-meta {
                        display: flex;
                        justify-content: space-between;
                        align-items: center;
                        margin-top: 6px;
                        font-size: 0.78rem;
                        color: #475569;
                    }
                    table {
                        width: 100% !important;
                        border-collapse: collapse !important;
                        font-size: 0.72rem !important;
                        break-inside: auto !important;
                    }
                    thead {
                        display: table-header-group !important;
                        break-inside: avoid !important;
                    }
                    th {
                        background: #f1f5f9 !important;
                        padding: 5px 6px !important;
                        font-size: 0.65rem !important;
                        font-weight: 700 !important;
                        text-transform: uppercase !important;
                        letter-spacing: 0.03em !important;
                        border-top: 1.5px solid #cbd5e1 !important;
                        border-bottom: 2px solid #cbd5e1 !important;
                        color: #334155 !important;
                        vertical-align: middle !important;
                        white-space: nowrap !important;
                    }
                    td {
                        padding: 4px 6px !important;
                        border-bottom: 1px solid #e2e8f0 !important;
                        vertical-align: middle !important;
                    }
                    tr {
                        break-inside: avoid !important;
                        page-break-inside: avoid !important;
                    }
                    tr:nth-child(even) td {
                        background: #fbfcfe !important;
                    }
                    .sign-off-grid {
                        margin-top: 24px !important;
                        display: grid !important;
                        grid-template-columns: 1fr 1fr 1fr !important;
                        gap: 20px !important;
                        font-size: 0.75rem !important;
                        border-top: 1.5px solid #cbd5e1 !important;
                        padding-top: 12px !important;
                        break-inside: avoid !important;
                        page-break-inside: avoid !important;
                    }
                    .sign-box {
                        line-height: 1.4 !important;
                    }
                    .sign-line {
                        margin-top: 24px !important;
                        border-top: 1px solid #334155 !important;
                        width: 90% !important;
                    }
                    @media print {
                        body {
                            padding: 0 !important;
                            margin: 0 auto !important;
                            width: 100% !important;
                            max-width: 100% !important;
                        }
                    }
                </style>
            </head>
            <body>
                ${bodyContentHtml}
            </body>
            </html>
        `;

        // Use safePrint iframe engine (avoids popup blocker)
        if (typeof window.safePrint === 'function') {
            window.safePrint(html.trim());
        } else {
            const printWin = window.open('', '_blank');
            if (!printWin) { alert('Popup blocked. Please allow popups to open the print sheet.'); return; }
            printWin.document.write(html);
            printWin.document.close();
        }
    }
return {
    generatePrintSheet: generatePrintSheet,
    getWorkerKey: getWorkerKey,
    configure: function (o) { _selectedWorkerIds = new Set(o.keys || []); _sheetTitle = o.title || _sheetTitle; _sheetDate = o.date || _sheetDate; _sheetFacilitator = o.facilitator || ''; _sheetVenue = o.venue || ''; _sheetType = o.type || 'meeting'; _includePhotos = o.photos !== false; _groupBy = o.groupBy || 'none'; _sortBy = o.sortBy || 'name'; _pageBreakPerGroup = o.pageBreak !== false; }
};
})();

// ===== visitation_groups.js - verbatim from 01_Rust_WC_DB/js/visitation_groups.js =====
var S5_visitation_groups = (function () {

    let _vmaCustomMarksCache = {};

    const MAX_MEMBERS_PER_GROUP = 15;

    function formatPrayerTime(t) {
        if (!t) return '';
        const parts = String(t).split(':');
        const h = parseInt(parts[0], 10);
        const m = parts[1] || '00';
        if (isNaN(h)) return t;
        const ampm = h >= 12 ? 'PM' : 'AM';
        return `${(h % 12) || 12}:${m} ${ampm}`;
    }

    function getGroupsSummary() {
        return visitationGroupsData.map(g => {
            const gName = (g.name || '').trim();
            const allAssigned = workersData.filter(w => !w.isArchived && (w.visitationGroup || '').trim().toLowerCase() === gName.toLowerCase());

            const katiwala = allAssigned.find(w => (w.visitationRole || '').trim().toLowerCase() === 'katiwala') || null;
            const iiKatiwala = allAssigned.find(w => (w.visitationRole || '').trim().toLowerCase() === 'ii katiwala') || null;
            const kalihim = allAssigned.find(w => (w.visitationRole || '').trim().toLowerCase() === 'kalihim') || null;
            
            const members = allAssigned.filter(w => {
                const r = (w.visitationRole || '').trim().toLowerCase();
                return r !== 'katiwala' && r !== 'ii katiwala' && r !== 'kalihim';
            }).sort((a, b) => (a.name || '').localeCompare(b.name || ''));

            const officerCount = (katiwala ? 1 : 0) + (iiKatiwala ? 1 : 0) + (kalihim ? 1 : 0);

            return {
                id: g.id,
                name: g.name,
                housing: g.housing || '',
                notes: g.notes || '',
                prayerDay: g.prayerDay || '',
                prayerTime: g.prayerTime || '',
                katiwala,
                iiKatiwala,
                kalihim,
                members,
                membersCount: members.length,
                officerCount,
                maxMembers: MAX_MEMBERS_PER_GROUP,
                isFull: allAssigned.length >= MAX_MEMBERS_PER_GROUP,
                remainingSlots: Math.max(0, MAX_MEMBERS_PER_GROUP - allAssigned.length),
                totalCount: allAssigned.length
            };
        }).sort((a, b) => a.name.localeCompare(b.name, undefined, { numeric: true }));
    }

    function getPrintableResidence(w) {
        if (!w) return '-';
        const rawHousing = (w.visitationHousing || '').trim();
        const address = (w.address || '').trim();
        const isNonInc = /non\s*inc/i.test(rawHousing);
        const housing = isNonInc ? '' : rawHousing;
        if (housing && address) {
            return `${housing} • ${address}`;
        }
        return address || rawHousing || '-';
    }

    function getPrintableResidenceHtml(w) {
        if (!w) return '-';
        const rawHousing = (w.visitationHousing || '').trim();
        const address = (w.address || '').trim();
        const isNonInc = /non\s*inc/i.test(rawHousing);
        const housing = isNonInc ? '' : rawHousing;

        if (housing && address) {
            return `<div style="font-weight: 700; color: #1e3a8a; font-size: 0.76rem; display: flex; align-items: center; gap: 4px;">🏠 ${esc(housing)}</div><div style="color: #475569; font-size: 0.72rem; margin-top: 2px; line-height: 1.25; word-break: break-word;">📍 ${esc(address)}</div>`;
        }
        if (address) {
            return `<div style="color: #475569; font-size: 0.72rem; line-height: 1.25; word-break: break-word;">📍 ${esc(address)}</div>`;
        }
        if (rawHousing) {
            return `<div style="font-weight: 600; color: #1e3a8a; font-size: 0.76rem; display: flex; align-items: center; gap: 4px;">🏠 ${esc(rawHousing)}</div>`;
        }
        return '-';
    }

    function printGroupRoster(groupId) {
        const group = visitationGroupsData.find(g => g.id === groupId);
        if (!group) return;

        const allAssigned = workersData.filter(w => (w.visitationGroup || '').trim().toLowerCase() === group.name.toLowerCase());
        const katiwala = allAssigned.find(w => (w.visitationRole || '').trim().toLowerCase() === 'katiwala');
        const iiKatiwala = allAssigned.find(w => (w.visitationRole || '').trim().toLowerCase() === 'ii katiwala');
        const kalihim = allAssigned.find(w => (w.visitationRole || '').trim().toLowerCase() === 'kalihim');
        const members = allAssigned.filter(w => {
            const r = (w.visitationRole || '').trim().toLowerCase();
            return r !== 'katiwala' && r !== 'ii katiwala' && r !== 'kalihim';
        }).sort((a, b) => a.name.localeCompare(b.name));

        const printWin = window.open('', '_blank');
        if (!printWin) {
            showToast('Popup blocked. Please allow popups to print.', 'error');
            return;
        }

        const dateStr = new Date().toLocaleDateString('en-US', { month: 'long', day: 'numeric', year: 'numeric' });

        const officerCardHtml = (officer, title) => {
            if (!officer) return `
                <div style="border: 1.5px dashed #cbd5e1; border-radius: 6px; padding: 8px 10px; text-align: center; background: #f8fafc;">
                    <div style="font-size: 0.68rem; font-weight: 700; text-transform: uppercase; color: #94a3b8; letter-spacing: 0.5px;">${title}</div>
                    <div style="font-size: 0.82rem; color: #64748b; margin-top: 3px; font-style: italic;">Unassigned</div>
                </div>
            `;
            const photo = officer.photoUrl || officer.photo;
            const aNo = officer.aNumber != null ? String(officer.aNumber) : '';
            const oAge = officer.birthday && typeof getAge === 'function' ? getAge(officer.birthday) : 0;
            const avatarHtml = photo
                ? `<img src="${esc(photo)}" style="width: 52px; height: 52px; border-radius: 6px; object-fit: cover; border: 1.5px solid #94a3b8; flex-shrink: 0; box-shadow: 0 1px 3px rgba(0,0,0,0.08);">`
                : `<div style="width: 52px; height: 52px; border-radius: 6px; background: #e0f2fe; color: #0369a1; font-weight: 800; font-size: 1.2rem; display: flex; align-items: center; justify-content: center; border: 1.5px solid #93c5fd; flex-shrink: 0;">${esc(officer.name.charAt(0).toUpperCase())}</div>`;

            return `
                <div style="border: 1.5px solid #cbd5e1; border-radius: 6px; padding: 6px 8px; background: #fff; display: flex; align-items: center; gap: 8px; min-width: 0; overflow: hidden;">
                    ${avatarHtml}
                    <div style="flex: 1; min-width: 0; overflow: hidden;">
                        <div style="display: flex; align-items: center; justify-content: space-between; gap: 4px; margin-bottom: 2px;">
                            <span style="font-size: 0.65rem; font-weight: 700; text-transform: uppercase; color: #1e40af; background: #dbeafe; display: inline-block; padding: 1px 5px; border-radius: 3px; letter-spacing: 0.4px;">${title}</span>
                            ${aNo ? `<span style="font-size: 0.7rem; font-weight: 800; color: #334155; background: #f1f5f9; border: 1px solid #cbd5e1; padding: 1px 5px; border-radius: 3px;" title="Assigned Number">#${esc(aNo)}</span>` : ''}
                        </div>
                        <div style="font-size: 0.92rem; font-weight: 700; color: #0f172a; word-break: break-word; overflow-wrap: break-word; line-height: 1.25;" title="${esc(officer.name)}">${esc(officer.name)}</div>
                        <div style="font-size: 0.72rem; color: #64748b; margin-top: 1px; word-break: break-word;">
                            ${esc(officer.trade || 'Volunteer')}${oAge ? ` • ${oAge} yo` : ''}
                        </div>
                        <div style="font-size: 0.7rem; margin-top: 2px; word-break: break-word;">
                            ${getPrintableResidenceHtml(officer)}
                        </div>
                        ${officer.phone ? `<div style="font-size: 0.7rem; color: #334155; margin-top: 2px; font-weight: 600; word-break: break-all;">📞 ${esc(officer.phone)}</div>` : ''}
                    </div>
                </div>
            `;
        };

        const html = `
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <title>${esc(group.name)} - Visitation Roster</title>
                <style>
                    @page { size: auto; margin: 8mm 10mm; }
                </style>
                <style>
                    *, *::before, *::after {
                        box-sizing: border-box;
                        margin: 0;
                        padding: 0;
                    }
                    html, body {
                        width: 100%;
                        max-width: 100%;
                        background: #f1f5f9;
                        color: #0f172a;
                        font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
                        font-size: 11px;
                        line-height: 1.35;
                    }
                    body {
                        padding: 24px 16px;
                        min-height: 100vh;
                        display: flex;
                        justify-content: center;
                        -webkit-print-color-adjust: exact;
                        print-color-adjust: exact;
                    }
                    .print-container {
                        width: 100%;
                        max-width: 8.5in;
                        margin: 0 auto;
                        background: #ffffff;
                        box-shadow: 0 4px 20px rgba(0,0,0,0.08);
                        padding: 24px 28px;
                        border-radius: 4px;
                        box-sizing: border-box;
                    }
                    .header {
                        text-align: center;
                        border-bottom: 2px solid #0f172a;
                        padding-bottom: 6px;
                        margin-bottom: 8px;
                        page-break-after: avoid;
                        break-after: avoid;
                    }
                    .header h1 {
                        margin: 0;
                        font-size: 1.25rem;
                        letter-spacing: 0.5px;
                        text-transform: uppercase;
                        color: #0f172a;
                        font-weight: 800;
                    }
                    .header h2 {
                        margin: 2px 0 0;
                        font-size: 0.98rem;
                        color: #1e40af;
                        font-weight: 700;
                    }
                    .header p {
                        margin: 2px 0 0;
                        font-size: 0.74rem;
                        color: #64748b;
                    }
                    .officers-grid {
                        display: grid;
                        grid-template-columns: repeat(3, 1fr);
                        gap: 8px;
                        margin-bottom: 8px;
                        page-break-inside: avoid;
                        break-inside: avoid;
                        page-break-after: avoid;
                        break-after: avoid;
                    }
                    .roster-table {
                        width: 100% !important;
                        max-width: 100% !important;
                        border-collapse: collapse !important;
                        margin-top: 4px;
                        font-size: 0.74rem;
                        table-layout: fixed !important;
                    }
                    .roster-table thead {
                        display: table-header-group;
                    }
                    .roster-table th {
                        background: #f1f5f9;
                        text-align: left;
                        padding: 4px 4px;
                        font-size: 0.66rem;
                        text-transform: uppercase;
                        letter-spacing: 0.04em;
                        border-top: 1.5px solid #cbd5e1;
                        border-bottom: 2px solid #cbd5e1;
                        color: #334155;
                        vertical-align: middle;
                        word-break: break-word;
                        overflow-wrap: break-word;
                    }
                    .roster-table td {
                        padding: 2px 4px;
                        border-bottom: 1px solid #e2e8f0;
                        vertical-align: middle;
                        word-break: break-word;
                        overflow-wrap: break-word;
                        font-size: 0.74rem;
                        line-height: 1.25;
                    }
                    .roster-table tr {
                        page-break-inside: avoid;
                        break-inside: avoid;
                    }
                    .roster-table tr:nth-child(even) td {
                        background: #fbfcfe;
                    }
                    .badge {
                        display: inline-block;
                        padding: 1px 6px;
                        border-radius: 4px;
                        font-size: 0.68rem;
                        font-weight: 600;
                        background: #e2e8f0;
                    }
                    .signatures-block {
                        page-break-inside: avoid;
                        break-inside: avoid;
                    }
                    @media print {
                        body {
                            background: #ffffff !important;
                            padding: 0 !important;
                            margin: 0 auto !important;
                            width: 100% !important;
                            max-width: 100% !important;
                            box-sizing: border-box !important;
                            -webkit-print-color-adjust: exact;
                            print-color-adjust: exact;
                            display: block !important;
                        }
                        .print-container {
                            width: 100% !important;
                            max-width: 100% !important;
                            margin: 0 !important;
                            padding: 2mm 3mm !important;
                            box-shadow: none !important;
                            border: none !important;
                            box-sizing: border-box !important;
                        }
                        .no-print {
                            display: none !important;
                        }
                        img {
                            image-rendering: -webkit-optimize-contrast;
                            -webkit-print-color-adjust: exact;
                            print-color-adjust: exact;
                        }
                        .roster-table thead {
                            display: table-header-group !important;
                        }
                        .roster-table tr {
                            page-break-inside: avoid !important;
                            break-inside: avoid !important;
                        }
                        .signatures-block {
                            page-break-inside: avoid !important;
                            break-inside: avoid !important;
                        }
                    }
                </style>
            </head>
            <body>
                <div class="print-container">

                <div class="header">
                    <h1>Woodland Construction</h1>
                    <h2>Visitation Group Directory: ${esc(group.name)}</h2>
                    <p>Generated on ${dateStr} • Total Volunteers: ${allAssigned.length}</p>
                    ${group.prayerDay ? `<div style="font-size: 0.78rem; color: #6d28d9; font-weight: 700; margin-top: 4px;">🕐 Weekly Prayer Meeting: ${esc(group.prayerDay)}${group.prayerTime ? ` • ${esc(formatPrayerTime(group.prayerTime))}` : ''}</div>` : ''}
                </div>

                <div class="officers-grid">
                    ${officerCardHtml(katiwala, 'Katiwala')}
                    ${officerCardHtml(iiKatiwala, 'II Katiwala')}
                    ${officerCardHtml(kalihim, 'Kalihim')}
                </div>

                <h3 style="font-size: 0.82rem; margin: 0 0 4px; text-transform: uppercase; letter-spacing: 0.5px; color: #334155; font-weight: 700; page-break-after: avoid; break-after: avoid;">
                    Members (${members.length})
                </h3>
                <table class="roster-table">
                    <colgroup>
                        <col style="width: 3.5%;">
                        <col style="width: 6.5%;">
                        <col style="width: 7.5%;">
                        <col style="width: 18.5%;">
                        <col style="width: 5%;">
                        <col style="width: 12.5%;">
                        <col style="width: 24.5%;">
                        <col style="width: 11%;">
                        <col style="width: 11%;">
                    </colgroup>
                    <thead>
                        <tr>
                            <th style="text-align: center;">#</th>
                            <th style="text-align: center;">Photo</th>
                            <th style="text-align: center;">A-No.</th>
                            <th>Volunteer Name</th>
                            <th style="text-align: center;">Age</th>
                            <th>Trade / Skill</th>
                            <th>Housing &amp; Home Address</th>
                            <th>Contact</th>
                            <th>Remarks / Last Visit</th>
                        </tr>
                    </thead>
                    <tbody>
                        ${members.length === 0 ? `<tr><td colspan="9" style="text-align:center; padding: 14px; color:#94a3b8; font-style: italic;">No regular members assigned yet.</td></tr>` : ''}
                        ${members.map((m, idx) => {
                            const mPhoto = m.photoUrl || m.photo;
                            const mANo = m.aNumber != null ? String(m.aNumber) : '';
                            const mAge = m.birthday && typeof getAge === 'function' ? getAge(m.birthday) : 0;
                            const mAvatar = mPhoto
                                ? `<img src="${esc(mPhoto)}" style="width: 38px; height: 38px; border-radius: 4px; object-fit: cover; border: 1px solid #94a3b8; display: block; margin: 0 auto; box-shadow: 0 1px 2px rgba(0,0,0,0.08);">`
                                : `<div style="width: 38px; height: 38px; border-radius: 4px; background: #e2e8f0; color: #475569; font-weight: 700; font-size: 0.9rem; display: flex; align-items: center; justify-content: center; margin: 0 auto; border: 1px solid #cbd5e1;">${esc(m.name.charAt(0).toUpperCase())}</div>`;
                            return `
                                <tr>
                                    <td style="color: #64748b; font-weight: 600; text-align: center;">${idx + 1}</td>
                                    <td style="text-align: center; vertical-align: middle; padding: 2px 2px;">${mAvatar}</td>
                                    <td style="font-weight: 700; text-align: center; color: #1e293b; font-size: 0.78rem;">${mANo ? '#' + esc(mANo) : '-'}</td>
                                    <td style="font-weight: 600; color: #0f172a;">${esc(m.name)}</td>
                                    <td style="text-align: center; color: #334155; font-size: 0.78rem; font-weight: 600;">${mAge ? mAge + ' yo' : '-'}</td>
                                    <td style="color: #475569;">${esc(m.trade || 'Volunteer')}</td>
                                    <td style="color: #334155; vertical-align: middle;">${getPrintableResidenceHtml(m)}</td>
                                    <td style="color: #334155; font-size: 0.78rem;">${esc(m.phone || '-')}</td>
                                    <td style="color: #64748b; font-size: 0.76rem;">${esc(m.notes || '')}</td>
                                </tr>
                            `;
                        }).join('')}
                    </tbody>
                </table>

                <div class="signatures-block" style="margin-top: 12px; display: grid; grid-template-columns: repeat(3, 1fr); gap: 16px; font-size: 0.78rem; border-top: 1.5px solid #cbd5e1; padding-top: 8px; page-break-inside: avoid; break-inside: avoid;">
                    <div>
                        <div style="color: #64748b; margin-bottom: 18px; font-weight: 600; text-transform: uppercase; font-size: 0.66rem; letter-spacing: 0.5px;">Prepared by:</div>
                        <div style="border-top: 1px solid #0f172a; padding-top: 3px; font-weight: 700; color: #0f172a;">${esc(kalihim ? kalihim.name : '________________________')}</div>
                        <div style="color: #64748b; font-size: 0.68rem; margin-top: 1px;">Kalihim ng Dalaw</div>
                    </div>
                    <div>
                        <div style="color: #64748b; margin-bottom: 18px; font-weight: 600; text-transform: uppercase; font-size: 0.66rem; letter-spacing: 0.5px;">Checked by:</div>
                        <div style="border-top: 1px solid #0f172a; padding-top: 3px; font-weight: 700; color: #0f172a;">${esc(katiwala ? katiwala.name : '________________________')}</div>
                        <div style="color: #64748b; font-size: 0.68rem; margin-top: 1px;">Katiwala ng Dalaw</div>
                    </div>
                    <div>
                        <div style="color: #64748b; margin-bottom: 18px; font-weight: 600; text-transform: uppercase; font-size: 0.66rem; letter-spacing: 0.5px;">Approved by:</div>
                        <div style="border-top: 1px solid #0f172a; padding-top: 3px; font-weight: 700; color: #0f172a;">________________________</div>
                        <div style="color: #64748b; font-size: 0.68rem; margin-top: 1px;">Visiting Minister (Ministro ng Ebanghelyo)</div>
                    </div>
                </div>

                </div>

                <script>
                    window.onload = function() {
                        setTimeout(function() {
                            window.print();
                        }, 300);
                    };
                </script>
            </body>
            </html>
        `;

        printWin.document.write(html);
        printWin.document.close();
    }

    function printAllGroupsSummary() {
        const allGroups = getGroupsSummary();
        if (allGroups.length === 0) {
            showToast('No visitation groups to print.', 'info');
            return;
        }

        const printWin = window.open('', '_blank');
        if (!printWin) return;

        const dateStr = new Date().toLocaleDateString('en-US', { month: 'long', day: 'numeric', year: 'numeric' });

        const officerChip = (officer, title) => {
            if (!officer) return `
                <div style="background: #f8fafc; border: 1px dashed #cbd5e1; border-radius: 4px; padding: 5px 7px; font-size: 0.75rem;">
                    <span style="font-size: 0.62rem; font-weight: 700; text-transform: uppercase; color: #94a3b8; display: block;">${title}</span>
                    <span style="color: #94a3b8; font-style: italic;">Unassigned</span>
                </div>
            `;
            const photo = officer.photoUrl || officer.photo;
            const aNo = officer.aNumber != null ? String(officer.aNumber) : '';
            const oAge = officer.birthday && typeof getAge === 'function' ? getAge(officer.birthday) : 0;
            const avatar = photo
                ? `<img src="${esc(photo)}" style="width: 36px; height: 36px; border-radius: 4px; object-fit: cover; border: 1.5px solid #94a3b8; flex-shrink: 0; box-shadow: 0 1px 2px rgba(0,0,0,0.06);">`
                : `<span style="width: 36px; height: 36px; border-radius: 4px; background: #e0f2fe; color: #0369a1; font-weight: 800; font-size: 0.95rem; display: inline-flex; align-items: center; justify-content: center; border: 1px solid #93c5fd; flex-shrink: 0;">${esc(officer.name.charAt(0).toUpperCase())}</span>`;

            return `
                <div style="background: #f8fafc; border: 1px solid #cbd5e1; border-radius: 4px; padding: 5px 7px; display: flex; align-items: center; gap: 7px; min-width: 0; overflow: hidden;">
                    ${avatar}
                    <div style="flex: 1; min-width: 0; overflow: hidden;">
                        <div style="display: flex; align-items: center; justify-content: space-between; gap: 4px;">
                            <span style="font-size: 0.62rem; font-weight: 700; text-transform: uppercase; color: #1e40af; background: #dbeafe; padding: 1px 4px; border-radius: 3px;">${title}</span>
                            ${aNo ? `<strong style="color: #334155; font-size: 0.68rem; background: #e2e8f0; padding: 1px 4px; border-radius: 3px;">#${esc(aNo)}</strong>` : ''}
                        </div>
                        <div style="font-weight: 700; font-size: 0.82rem; color: #0f172a; word-break: break-word; overflow-wrap: break-word; line-height: 1.25; margin-top: 1px;" title="${esc(officer.name)}">${esc(officer.name)}</div>
                        <div style="font-size: 0.68rem; color: #64748b; margin-top: 1px; word-break: break-word;">${esc(officer.trade || 'Volunteer')}${oAge ? ` • ${oAge} yo` : ''}${officer.phone ? ` • 📞 ${esc(officer.phone)}` : ''}</div>
                        <div style="font-size: 0.68rem; margin-top: 1px; word-break: break-word;">${getPrintableResidenceHtml(officer)}</div>
                    </div>
                </div>
            `;
        };

        const html = `
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <title>All Visitation Groups Summary</title>
                <style>
                    @page { size: auto; margin: 8mm 10mm; }
                </style>
                <style>
                    *, *::before, *::after {
                        box-sizing: border-box;
                        margin: 0;
                        padding: 0;
                    }
                    html, body {
                        width: 100%;
                        max-width: 100%;
                        background: #f1f5f9;
                        color: #0f172a;
                        font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
                        font-size: 11px;
                        line-height: 1.35;
                    }
                    body {
                        padding: 24px 16px;
                        min-height: 100vh;
                        display: flex;
                        justify-content: center;
                        -webkit-print-color-adjust: exact;
                        print-color-adjust: exact;
                    }
                    .print-container {
                        width: 100%;
                        max-width: 8.5in;
                        margin: 0 auto;
                        background: #ffffff;
                        box-shadow: 0 4px 20px rgba(0,0,0,0.08);
                        padding: 24px 28px;
                        border-radius: 4px;
                        box-sizing: border-box;
                    }
                    .header {
                        text-align: center;
                        border-bottom: 2px solid #0f172a;
                        padding-bottom: 6px;
                        margin-bottom: 10px;
                        page-break-after: avoid;
                        break-after: avoid;
                    }
                    .header h1 {
                        margin: 0;
                        font-size: 1.25rem;
                        letter-spacing: 0.5px;
                        text-transform: uppercase;
                        font-weight: 800;
                    }
                    .header h2 {
                        margin: 2px 0 0;
                        font-size: 0.98rem;
                        color: #1e40af;
                        font-weight: 700;
                    }
                    .header p {
                        margin: 2px 0 0;
                        font-size: 0.74rem;
                        color: #64748b;
                    }
                    .group-block {
                        display: inline-block;
                        width: 100%;
                        margin-bottom: 14px;
                        border: 1.5px solid #cbd5e1;
                        border-radius: 6px;
                        padding: 8px 10px;
                        background: #fff;
                        box-shadow: 0 1px 2px rgba(0,0,0,0.03);
                        page-break-inside: avoid;
                        break-inside: avoid;
                    }
                    .group-header {
                        display: flex;
                        justify-content: space-between;
                        align-items: center;
                        border-bottom: 1.5px solid #cbd5e1;
                        padding-bottom: 4px;
                        margin-bottom: 6px;
                        page-break-inside: avoid;
                        break-inside: avoid;
                        page-break-after: avoid;
                        break-after: avoid;
                    }
                    .group-header h3 {
                        margin: 0;
                        font-size: 0.95rem;
                        color: #1e3a8a;
                        font-weight: 700;
                    }
                    .officers-bar {
                        display: grid;
                        grid-template-columns: repeat(3, 1fr);
                        gap: 6px;
                        margin-bottom: 6px;
                        page-break-inside: avoid;
                        break-inside: avoid;
                        page-break-after: avoid;
                        break-after: avoid;
                    }
                    .roster-table {
                        width: 100% !important;
                        max-width: 100% !important;
                        border-collapse: collapse !important;
                        font-size: 0.74rem;
                        table-layout: fixed !important;
                    }
                    .roster-table thead {
                        display: table-header-group;
                    }
                    .roster-table th {
                        background: #f1f5f9;
                        text-align: left;
                        padding: 4px 4px;
                        font-size: 0.66rem;
                        text-transform: uppercase;
                        letter-spacing: 0.04em;
                        border-top: 1px solid #cbd5e1;
                        border-bottom: 2px solid #cbd5e1;
                        color: #334155;
                        vertical-align: middle;
                        word-break: break-word;
                        overflow-wrap: break-word;
                    }
                    .roster-table td {
                        padding: 2px 4px;
                        border-bottom: 1px solid #e2e8f0;
                        vertical-align: middle;
                        word-break: break-word;
                        overflow-wrap: break-word;
                        font-size: 0.74rem;
                        line-height: 1.25;
                    }
                    .roster-table tr {
                        page-break-inside: avoid;
                        break-inside: avoid;
                    }
                    .roster-table tr:nth-child(even) td {
                        background: #fbfcfe;
                    }
                    @media print {
                        body {
                            background: #ffffff !important;
                            padding: 0 !important;
                            margin: 0 auto !important;
                            width: 100% !important;
                            max-width: 100% !important;
                            box-sizing: border-box !important;
                            -webkit-print-color-adjust: exact;
                            print-color-adjust: exact;
                            display: block !important;
                        }
                        .print-container {
                            width: 100% !important;
                            max-width: 100% !important;
                            margin: 0 !important;
                            padding: 2mm 3mm !important;
                            box-shadow: none !important;
                            border: none !important;
                            box-sizing: border-box !important;
                        }
                        .no-print {
                            display: none !important;
                        }
                        img {
                            image-rendering: -webkit-optimize-contrast;
                            -webkit-print-color-adjust: exact;
                            print-color-adjust: exact;
                        }
                        .group-block {
                            display: block !important;
                            width: 100% !important;
                            page-break-inside: avoid !important;
                            break-inside: avoid !important;
                            box-sizing: border-box !important;
                            margin-top: 4mm !important;
                            margin-bottom: 12px !important;
                            box-shadow: none !important;
                            padding: 8px 10px !important;
                            border: 1px solid #cbd5e1 !important;
                        }
                        .group-header {
                            page-break-after: avoid !important;
                            break-after: avoid !important;
                        }
                        .officers-bar {
                            page-break-after: avoid !important;
                            break-after: avoid !important;
                        }
                        .roster-table thead {
                            display: table-header-group !important;
                        }
                        .roster-table tr {
                            page-break-inside: avoid !important;
                            break-inside: avoid !important;
                        }
                    }
                </style>
            </head>
            <body>
                <div class="print-container">

                <div class="header">
                    <h1>Woodland Construction</h1>
                    <h2>Master Visitation Groups Directory</h2>
                    <p>Generated on ${dateStr} • Total Groups: ${allGroups.length}</p>
                </div>

                ${allGroups.map(g => `
                    <div class="group-block">
                        <div class="group-header">
                            <h3>${esc(g.name)} ${g.housing ? `<span style="font-size:0.8rem; font-weight:600; color:#475569; background:#f1f5f9; padding:2px 7px; border-radius:3px; border:1px solid #e2e8f0; margin-left:6px;">🏠 ${esc(g.housing)}</span>` : ''}</h3>
                            <span style="font-weight:700; font-size:0.78rem; color: #1e40af; background: #eff6ff; border: 1px solid #bfdbfe; padding: 2px 7px; border-radius: 4px;">${g.totalCount} Total Volunteers</span>
                        </div>
                        ${g.prayerDay ? `<div style="font-size: 0.76rem; color: #6d28d9; font-weight: 700; margin-bottom: 7px;">🕐 Weekly Prayer Meeting: ${esc(g.prayerDay)}${g.prayerTime ? ` • ${esc(formatPrayerTime(g.prayerTime))}` : ''}</div>` : ''}
                        <div class="officers-bar">
                            ${officerChip(g.katiwala, 'Katiwala')}
                            ${officerChip(g.iiKatiwala, 'II Katiwala')}
                            ${officerChip(g.kalihim, 'Kalihim')}
                        </div>
                        <table class="roster-table">
                            <colgroup>
                                <col style="width: 4%;">
                                <col style="width: 6.5%;">
                                <col style="width: 8%;">
                                <col style="width: 21.5%;">
                                <col style="width: 5.5%;">
                                <col style="width: 13.5%;">
                                <col style="width: 27%;">
                                <col style="width: 14%;">
                            </colgroup>
                            <thead>
                                <tr>
                                    <th style="text-align: center;">#</th>
                                    <th style="text-align: center;">Photo</th>
                                    <th style="text-align: center;">A-No.</th>
                                    <th>Member</th>
                                    <th style="text-align: center;">Age</th>
                                    <th>Trade</th>
                                    <th>Housing &amp; Home Address</th>
                                    <th>Phone</th>
                                </tr>
                            </thead>
                            <tbody>
                                ${g.members.length === 0 ? `<tr><td colspan="8" style="text-align:center; padding: 10px; color:#94a3b8; font-style: italic;">No regular members assigned</td></tr>` : ''}
                                ${g.members.map((m, idx) => {
                                    const mPhoto = m.photoUrl || m.photo;
                                    const mANo = m.aNumber != null ? String(m.aNumber) : '';
                                    const mAge = m.birthday && typeof getAge === 'function' ? getAge(m.birthday) : 0;
                                    const mAvatar = mPhoto
                                        ? `<img src="${esc(mPhoto)}" style="width: 38px; height: 38px; border-radius: 4px; object-fit: cover; border: 1px solid #94a3b8; display: block; margin: 0 auto; box-shadow: 0 1px 2px rgba(0,0,0,0.06);">`
                                        : `<div style="width: 38px; height: 38px; border-radius: 4px; background: #e2e8f0; color: #475569; font-weight: 700; font-size: 0.9rem; display: flex; align-items: center; justify-content: center; margin: 0 auto; border: 1px solid #cbd5e1;">${esc(m.name.charAt(0).toUpperCase())}</div>`;
                                    return `
                                        <tr>
                                            <td style="text-align: center; color: #64748b; font-weight: 600;">${idx + 1}</td>
                                            <td style="text-align: center; vertical-align: middle; padding: 3px 2px;">${mAvatar}</td>
                                            <td style="font-weight: 700; text-align: center; color: #1e293b; font-size: 0.78rem;">${mANo ? '#' + esc(mANo) : '-'}</td>
                                            <td><strong style="color: #0f172a;">${esc(m.name)}</strong></td>
                                            <td style="text-align: center; color: #334155; font-size: 0.78rem; font-weight: 600;">${mAge ? mAge + ' yo' : '-'}</td>
                                            <td style="color: #475569;">${esc(m.trade || 'Volunteer')}</td>
                                            <td style="color: #334155; vertical-align: middle;">${getPrintableResidenceHtml(m)}</td>
                                            <td style="color: #334155; font-size: 0.78rem;">${esc(m.phone || '-')}</td>
                                        </tr>
                                    `;
                                }).join('')}
                            </tbody>
                        </table>
                    </div>
                `).join('')}

                </div>

                <script>
                    window.onload = function() {
                        setTimeout(function() {
                            window.print();
                        }, 300);
                    };
                </script>
            </body>
            </html>
        `;

        printWin.document.write(html);
        printWin.document.close();
    }

    const VMA_MONTH_NAMES = [
        'January', 'February', 'March', 'April', 'May', 'June',
        'July', 'August', 'September', 'October', 'November', 'December'
    ];

    const VMA_DOW_INITIALS = ['S', 'M', 'T', 'W', 'T', 'F', 'S'];

    const VMA_MARK_CYCLE = ['', 'P', 'A', 'O', 'H', 'D'];

    function getVmaStorageKey(groupId, year, month) {
        return `wc_visitation_monthly_att_${groupId}_${year}_${month}`;
    }

    async function loadVmaMarks(groupId, year, month) {
        if (!groupId) return {};
        const cacheKey = `${groupId}_${year}_${month}`;
        if (_vmaCustomMarksCache[cacheKey]) {
            return _vmaCustomMarksCache[cacheKey];
        }
        const storageKey = getVmaStorageKey(groupId, year, month);
        let marks = {};
        try {
            if (window._storage && typeof window._storage.get === 'function') {
                const stored = await window._storage.get(storageKey);
                if (stored && typeof stored === 'object') marks = stored;
            }
            if (Object.keys(marks).length === 0 && typeof safeGetItem === 'function') {
                const local = safeGetItem(storageKey);
                if (local) marks = JSON.parse(local);
            }
        } catch (err) {
            console.warn('Failed to load VMA marks for group', groupId, err);
        }
        _vmaCustomMarksCache[cacheKey] = marks || {};
        return _vmaCustomMarksCache[cacheKey];
    }

    async function loadGroupsData() {
        try {
            const stored = await window._storage.get(VISITATION_GROUPS_KEY);
            if (stored && Array.isArray(stored)) {
                visitationGroupsData = stored;
                window.visitationGroupsData = stored;
            } else {
                const local = safeGetItem(VISITATION_GROUPS_KEY);
                if (local) {
                    try {
                        visitationGroupsData = JSON.parse(local);
                        window.visitationGroupsData = visitationGroupsData;
                    } catch (e) {
                        visitationGroupsData = [];
                    }
                }
            }
        } catch (err) {
            console.error('Failed to load visitation groups:', err);
            visitationGroupsData = [];
        }

        // Auto-discover groups from workersData if none in store
        const existingGroupNames = new Set(visitationGroupsData.map(g => (g.name || '').trim().toLowerCase()));
        let discovered = false;
        workersData.forEach(w => {
            if (w.visitationGroup && w.visitationGroup.trim() !== '') {
                const name = w.visitationGroup.trim();
                if (!existingGroupNames.has(name.toLowerCase())) {
                    existingGroupNames.add(name.toLowerCase());
                    visitationGroupsData.push({
                        id: 'vg_' + Date.now() + '_' + Math.random().toString(36).substr(2, 6),
                        name: name,
                        housing: w.visitationHousing || '',
                        notes: '',
                        createdAt: new Date().toISOString()
                    });
                    discovered = true;
                }
            }
        });

        if (discovered) {
            await saveGroupsData();
        }
        return visitationGroupsData;
    }

    function getGroupRosterMembers(group, includeOfficers = true) {
        if (!group) return { officers: { katiwala: null, iiKatiwala: null, kalihim: null }, members: [], allList: [] };
        const gName = (group.name || '').trim().toLowerCase();

        const groupWorkers = workersData.filter(w => (w.visitationGroup || '').trim().toLowerCase() === gName);

        const katiwala = groupWorkers.find(w => w.visitationRole === 'Katiwala') || null;
        const iiKatiwala = groupWorkers.find(w => w.visitationRole === 'II Katiwala') || null;
        const kalihim = groupWorkers.find(w => w.visitationRole === 'Kalihim') || null;

        const officerANumbers = new Set([
            katiwala ? String(katiwala.aNumber) : null,
            iiKatiwala ? String(iiKatiwala.aNumber) : null,
            kalihim ? String(kalihim.aNumber) : null
        ].filter(Boolean));

        const regularMembers = groupWorkers
            .filter(w => !officerANumbers.has(String(w.aNumber)))
            .sort((a, b) => (a.name || '').localeCompare(b.name || ''));

        const officersList = [];
        if (katiwala) officersList.push({ ...katiwala, isOfficer: true, officerRole: 'Katiwala' });
        if (iiKatiwala) officersList.push({ ...iiKatiwala, isOfficer: true, officerRole: 'II Katiwala' });
        if (kalihim) officersList.push({ ...kalihim, isOfficer: true, officerRole: 'Kalihim' });

        const allList = includeOfficers ? [...officersList, ...regularMembers] : regularMembers;

        return {
            officers: { katiwala, iiKatiwala, kalihim },
            members: regularMembers,
            allList
        };
    }

    function generateSingleMonthlySheetHtml(group, year, month, paperSize, includeOfficers, marks, isLastGroup = true) {
        const { officers, allList } = getGroupRosterMembers(group, includeOfficers);
        const daysInMonth = new Date(year, month + 1, 0).getDate();
        const daysArray = Array.from({ length: daysInMonth }, (_, i) => i + 1);
        const monthName = VMA_MONTH_NAMES[month];

        // Dynamic scaling parameters based on row count
        const rowCount = Math.max(allList.length, 15);
        let fontSize = '0.72rem';
        let cellPadding = '2.5px 1px';
        let rowHeight = '18px';
        if (rowCount <= 16) {
            fontSize = '0.75rem';
            cellPadding = '3.5px 2px';
            rowHeight = '21px';
        } else if (rowCount > 24) {
            fontSize = '0.62rem';
            cellPadding = '1.5px 0.5px';
            rowHeight = '15px';
        }

        const prayerStr = group.prayerDay ? `${group.prayerDay}${group.prayerTime ? ' • ' + formatPrayerTime(group.prayerTime) : ''}` : 'Not specified';
        const housingStr = group.housing || 'General Residences';

        let rowsHtml = '';
        if (allList.length === 0) {
            for (let r = 1; r <= 15; r++) {
                rowsHtml += `
                    <tr>
                        <td style="text-align: center; color: #94a3b8;">${r}</td>
                        <td style="text-align: center; color: #94a3b8;">-</td>
                        <td style="text-align: left; padding-left: 6px; color: #94a3b8; font-style: italic;">(Open Volunteer Line)</td>
                        ${daysArray.map(d => {
                            const dow = new Date(year, month, d).getDay();
                            const isWeekend = (dow === 0 || dow === 6);
                            return `<td style="text-align: center; color: #cbd5e1; ${isWeekend ? 'background: #f1f5f9;' : ''}">·</td>`;
                        }).join('')}
                        <td style="text-align: center; color: #cbd5e1;">&nbsp;</td>
                        <td style="text-align: center; color: #cbd5e1;">&nbsp;</td>
                        <td style="text-align: center; color: #cbd5e1;">&nbsp;</td>
                        <td style="text-align: center; color: #cbd5e1;">&nbsp;</td>
                    </tr>
                `;
            }
        } else {
            allList.forEach((person, idx) => {
                const aNoStr = person.aNumber != null ? String(person.aNumber) : '';
                let p = 0, a = 0, o = 0, h = 0, d = 0;

                let roleTag = '';
                if (person.isOfficer) {
                    roleTag = `<span style="font-size: 0.6rem; font-weight: 700; border: 1px solid #64748b; border-radius: 3px; padding: 0 3px; margin-right: 3px; text-transform: uppercase;">${esc(person.officerRole)}</span>`;
                }

                const dayTds = daysArray.map(day => {
                    const dow = new Date(year, month, day).getDay();
                    const isWeekend = (dow === 0 || dow === 6);
                    const mark = marks[`${aNoStr}_${day}`] || '';

                    if (mark === 'P') p++;
                    else if (mark === 'A') a++;
                    else if (mark === 'O') o++;
                    else if (mark === 'H') h++;
                    else if (mark === 'D') d++;

                    let cellStyle = `text-align: center; font-weight: 700; ${isWeekend ? 'background: #f1f5f9;' : ''}`;
                    if (mark === 'P') cellStyle += ' color: #15803d;';
                    else if (mark === 'A') cellStyle += ' color: #b91c1c;';
                    else if (mark === 'O') cellStyle += ' color: #b45309;';
                    else if (mark === 'H') cellStyle += ' color: #1d4ed8;';
                    else if (mark === 'D') cellStyle += ' color: #6d28d9;';
                    else cellStyle += ' color: #cbd5e1; font-weight: 400;';

                    return `<td style="${cellStyle}">${mark || '·'}</td>`;
                }).join('');

                const evaluated = p + a + o + h + d;
                const rateStr = evaluated > 0 ? `${Math.round(((p + d) / evaluated) * 100)}%` : '-';

                rowsHtml += `
                    <tr>
                        <td style="text-align: center; color: #64748b; font-weight: 600;">${idx + 1}</td>
                        <td style="text-align: center; font-weight: 700; color: #0f172a;">${aNoStr ? '#' + esc(aNoStr) : '-'}</td>
                        <td style="text-align: left; padding-left: 6px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
                            ${roleTag}<strong style="color: #0f172a;">${esc(person.name || 'Unnamed')}</strong>
                        </td>
                        ${dayTds}
                        <td style="text-align: center; font-weight: 700; color: #15803d;">${p > 0 ? p : ''}</td>
                        <td style="text-align: center; font-weight: 700; color: #b91c1c;">${a > 0 ? a : ''}</td>
                        <td style="text-align: center; font-weight: 700; color: #b45309;">${o > 0 ? o : ''}</td>
                        <td style="text-align: center; font-weight: 700; color: #0f172a;">${rateStr !== '—' ? rateStr : ''}</td>
                    </tr>
                `;
            });
        }

        const datePrintedStr = new Date().toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });

        return `
            <div class="vma-print-page" style="page-break-after: ${isLastGroup ? 'avoid' : 'always'}; break-after: ${isLastGroup ? 'avoid' : 'page'};">
                <!-- Header Block -->
                <div style="display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 5px; border-bottom: 2px solid #0f172a; padding-bottom: 4px;">
                    <div>
                        <div style="font-size: 0.68rem; font-weight: 800; letter-spacing: 0.8px; color: #475569; text-transform: uppercase;">
                            Woodland Construction • Workforce Management
                        </div>
                        <div style="font-size: 1.05rem; font-weight: 900; color: #0f172a; text-transform: uppercase; margin-top: 1px;">
                            TALAAN NG PRESENSYA / DAILY WORK ATTENDANCE SHEET
                        </div>
                    </div>
                    <div style="text-align: right;">
                        <div style="font-size: 1.05rem; font-weight: 900; color: #0f172a; text-transform: uppercase;">
                            ${esc(monthName.toUpperCase())} ${year}
                        </div>
                        <div style="font-size: 0.65rem; color: #64748b;">
                            Printed: ${esc(datePrintedStr)}
                        </div>
                    </div>
                </div>

                <!-- Group Metadata Bar -->
                <div style="display: flex; justify-content: space-between; align-items: center; background: #f8fafc; border: 1px solid #cbd5e1; border-radius: 4px; padding: 3px 8px; margin-bottom: 5px; font-size: 0.7rem;">
                    <div><strong>Group:</strong> <span style="font-size: 0.82rem; font-weight: 800; color: #0f172a;">${esc(group.name)}</span></div>
                    <div><strong>Work Area / Housing:</strong> <span>${esc(housingStr)}</span></div>
                    <div><strong>Total Members:</strong> <span style="font-weight: 700;">${allList.length}</span></div>
                </div>

                <!-- Table Container -->
                <div style="flex: 1; min-height: 0;">
                    <table class="vma-print-table" style="font-size: ${fontSize};">
                        <thead>
                            <tr>
                                <th style="width: 22px;" rowspan="2">#</th>
                                <th style="width: 46px;" rowspan="2">A-No.</th>
                                <th style="width: 140px; text-align: left; padding-left: 6px;" rowspan="2">Member Full Name</th>
                                ${daysArray.map(d => {
                                    const dow = new Date(year, month, d).getDay();
                                    const isWeekend = (dow === 0 || dow === 6);
                                    return `<th style="width: 2.1%; ${isWeekend ? 'background: #e2e8f0;' : ''}">${d}</th>`;
                                }).join('')}
                                <th style="width: 24px;" rowspan="2" title="Present">P</th>
                                <th style="width: 24px;" rowspan="2" title="Absent">A</th>
                                <th style="width: 24px;" rowspan="2" title="On Pass">O</th>
                                <th style="width: 32px;" rowspan="2" title="Attendance %">%</th>
                            </tr>
                            <tr>
                                ${daysArray.map(d => {
                                    const dow = new Date(year, month, d).getDay();
                                    const isWeekend = (dow === 0 || dow === 6);
                                    const initial = VMA_DOW_INITIALS[dow];
                                    return `<th style="font-size: 0.6rem; font-weight: 800; ${isWeekend ? 'background: #e2e8f0;' : ''}">${initial}</th>`;
                                }).join('')}
                            </tr>
                        </thead>
                        <tbody>
                            ${rowsHtml}
                        </tbody>
                    </table>
                </div>

                <!-- Legend & Signatories Footer -->
                <div style="margin-top: 5px; page-break-inside: avoid; break-inside: avoid;">
                    <!-- Legend Bar -->
                    <div style="font-size: 0.65rem; color: #475569; display: flex; justify-content: space-between; align-items: center; border-top: 1px solid #cbd5e1; padding-top: 3px; margin-bottom: 8px;">
                        <div>
                            <strong>LEGEND:</strong>
                            <span style="margin-left: 8px;"><strong style="color: #15803d;">P</strong> = Present</span>
                            <span style="margin-left: 8px;"><strong style="color: #b91c1c;">A</strong> = Absent</span>
                            <span style="margin-left: 8px;"><strong style="color: #b45309;">O</strong> = On Official Pass</span>
                            <span style="margin-left: 8px;"><strong style="color: #1d4ed8;">H</strong> = Health / Clinic</span>
                            <span style="margin-left: 8px;"><strong style="color: #6d28d9;">D</strong> = Overnight Duty</span>
                        </div>
                        <div style="font-style: italic;">Official Group Daily Attendance Record</div>
                    </div>

                    <!-- 3-Tier Signatory Block -->
                    <table style="width: 100%; table-layout: fixed; border-collapse: collapse; font-size: 0.7rem; text-align: center;">
                        <tr>
                            <td style="padding: 0 16px; vertical-align: top;">
                                <div style="color: #64748b; font-size: 0.64rem; margin-bottom: 22px; text-transform: uppercase; font-weight: 600;">Inihanda ni (Prepared by):</div>
                                <div style="border-bottom: 1px solid #0f172a; font-weight: 700; color: #0f172a; padding-bottom: 2px;">
                                    ${officers.kalihim ? esc(officers.kalihim.name) : '____________________________________'}
                                </div>
                                <div style="font-size: 0.66rem; color: #475569; margin-top: 2px;">Kalihim ng Dalaw (Group Secretary)</div>
                            </td>
                            <td style="padding: 0 16px; vertical-align: top;">
                                <div style="color: #64748b; font-size: 0.64rem; margin-bottom: 22px; text-transform: uppercase; font-weight: 600;">Sinuri ni (Verified by):</div>
                                <div style="border-bottom: 1px solid #0f172a; font-weight: 700; color: #0f172a; padding-bottom: 2px;">
                                    ${officers.katiwala ? esc(officers.katiwala.name) : '____________________________________'}
                                </div>
                                <div style="font-size: 0.66rem; color: #475569; margin-top: 2px;">Katiwala ng Dalaw (Group Leader)</div>
                            </td>
                            <td style="padding: 0 16px; vertical-align: top;">
                                <div style="color: #64748b; font-size: 0.64rem; margin-bottom: 22px; text-transform: uppercase; font-weight: 600;">Pinagtibay ni (Approved by):</div>
                                <div style="border-bottom: 1px solid #0f172a; font-weight: 700; color: #0f172a; padding-bottom: 2px;">
                                    ____________________________________
                                </div>
                                <div style="font-size: 0.66rem; color: #475569; margin-top: 2px;">Ministro ng Ebanghelyo (Visiting Minister)</div>
                            </td>
                        </tr>
                    </table>
                </div>
            </div>
        `;
    }

    function printMonthlyAttendanceSheet(groupId, year, month, paperSize = 'folio', includeOfficers = true) {
        const group = visitationGroupsData.find(g => g.id === groupId);
        if (!group) {
            if (typeof showToast === 'function') showToast('Group not found.', 'warning');
            return;
        }

        const cacheKey = `${groupId}_${year}_${month}`;
        const marks = _vmaCustomMarksCache[cacheKey] || {};
        const sheetHtml = generateSingleMonthlySheetHtml(group, year, month, paperSize, includeOfficers, marks, true);

        const pageCss = `
            @page {
                size: landscape;
                margin: 5mm 6mm;
            }
            * {
                box-sizing: border-box;
                -webkit-print-color-adjust: exact;
                print-color-adjust: exact;
            }
            html, body {
                margin: 0;
                padding: 0;
                width: 100%;
                height: 100%;
                background: #ffffff;
                font-family: 'Inter', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
                color: #0f172a;
            }
            .vma-print-page {
                width: 100%;
                height: 100vh;
                max-height: 98vh;
                display: flex;
                flex-direction: column;
                justify-content: space-between;
                box-sizing: border-box;
                overflow: hidden;
            }
            .vma-print-table {
                width: 100%;
                border-collapse: collapse;
                table-layout: fixed;
            }
            .vma-print-table th,
            .vma-print-table td {
                border: 1px solid #94a3b8;
                padding: 2px 1px;
                vertical-align: middle;
                box-sizing: border-box;
            }
            .vma-print-table th {
                background: #f1f5f9;
                color: #0f172a;
                font-weight: 700;
                text-align: center;
            }
        `;

        const fullHtml = `
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8">
                <title>Monthly Attendance - ${esc(group.name)} - ${VMA_MONTH_NAMES[month]} ${year}</title>
                <style>${pageCss}</style>
            </head>
            <body>
                ${sheetHtml}
            </body>
            </html>
        `;

        if (typeof window.safePrint === 'function') {
            window.safePrint(fullHtml, { title: `Monthly_Attendance_${group.name}_${VMA_MONTH_NAMES[month]}_${year}` });
        } else {
            const printWin = window.open('', '_blank');
            if (printWin) {
                printWin.document.write(fullHtml);
                printWin.document.close();
                printWin.focus();
                setTimeout(() => printWin.print(), 300);
            }
        }
    }

    async function printAllMonthlyAttendanceSheets(year, month, paperSize = 'folio', includeOfficers = true) {
        if (visitationGroupsData.length === 0) {
            await loadGroupsData();
        }

        if (visitationGroupsData.length === 0) {
            if (typeof showToast === 'function') showToast('No visitation groups available to print.', 'warning');
            return;
        }

        const sheets = [];
        for (let i = 0; i < visitationGroupsData.length; i++) {
            const group = visitationGroupsData[i];
            const marks = await loadVmaMarks(group.id, year, month);
            const isLast = (i === visitationGroupsData.length - 1);
            sheets.push(generateSingleMonthlySheetHtml(group, year, month, paperSize, includeOfficers, marks, isLast));
        }

        const pageCss = `
            @page {
                size: landscape;
                margin: 5mm 6mm;
            }
            * {
                box-sizing: border-box;
                -webkit-print-color-adjust: exact;
                print-color-adjust: exact;
            }
            html, body {
                margin: 0;
                padding: 0;
                width: 100%;
                background: #ffffff;
                font-family: 'Inter', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
                color: #0f172a;
            }
            .vma-print-page {
                width: 100%;
                height: 100vh;
                max-height: 98vh;
                display: flex;
                flex-direction: column;
                justify-content: space-between;
                box-sizing: border-box;
                overflow: hidden;
            }
            .vma-print-table {
                width: 100%;
                border-collapse: collapse;
                table-layout: fixed;
            }
            .vma-print-table th,
            .vma-print-table td {
                border: 1px solid #94a3b8;
                padding: 2px 1px;
                vertical-align: middle;
                box-sizing: border-box;
            }
            .vma-print-table th {
                background: #f1f5f9;
                color: #0f172a;
                font-weight: 700;
                text-align: center;
            }
        `;

        const fullHtml = `
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8">
                <title>All Visitation Groups Monthly Attendance - ${VMA_MONTH_NAMES[month]} ${year}</title>
                <style>${pageCss}</style>
            </head>
            <body>
                ${sheets.join('\n')}
            </body>
            </html>
        `;

        if (typeof window.safePrint === 'function') {
            window.safePrint(fullHtml, { title: `All_Visitation_Groups_Monthly_Attendance_${VMA_MONTH_NAMES[month]}_${year}` });
        } else {
            const printWin = window.open('', '_blank');
            if (printWin) {
                printWin.document.write(fullHtml);
                printWin.document.close();
                printWin.focus();
                setTimeout(() => printWin.print(), 300);
            }
        }
    }
return {
    printGroupRoster: printGroupRoster,
    printAllGroupsSummary: printAllGroupsSummary,
    printMonthlyAttendanceSheet: printMonthlyAttendanceSheet,
    printAllMonthlyAttendanceSheets: printAllMonthlyAttendanceSheets
};
})();

// ===== ot_calculator.js - verbatim from 01_Rust_WC_DB/js/ot_calculator.js =====
var S5_ot_calculator = (function () {

function formatSurnameFirst(fullName) {
  if (!fullName) return '';
  const str = fullName.trim();
  if (str.includes(',')) {
    const [last, ...first] = str.split(',');
    return last.trim().toUpperCase() + ', ' + first.join(',').trim();
  }
  const parts = str.split(/\s+/);
  if (parts.length === 1) return str.toUpperCase();
  const last = parts.pop().toUpperCase();
  const first = parts.join(' ');
  return `${last}, ${first}`;
}

function formatScheduleDate(dateStr) {
  if (!dateStr) return '';
  const d = new Date(dateStr + (dateStr.length === 10 ? 'T00:00:00' : ''));
  if (isNaN(d.getTime())) return dateStr;
  return d.toLocaleDateString('en-US', { month: 'long', day: 'numeric', year: 'numeric' });
}

var OtCalcModule = {
  _entries: [],
  _selected: new Set(),
  _encoderPending: [],
  _initialized: false,

  async init() {
    try {
      await this.load();

      if (!this._initialized) {
        this._initialized = true;
        this._initDateRangeDefaults();
        this._populateTradeFilter();
        this._initDateJumpBar();
        this._initVolunteerSearch();
        this._initTabs();
        this._initPrintButtons();
      } else {
        this._populateTradeFilter();
      }

      this._render();
    } catch (e) {
      console.error('OtCalcModule.init() failed:', e);
    }
  },

  async load() {
    const raw = await window.dbStores.ot_entries.getAll() || [];
    this._entries = raw.map(e => {
      const hours = parseFloat(e.hours) || 0;
      const hourlyRate = parseFloat(e.hourlyRate) || 0;
      const computedPay = (e.computedPay !== undefined && e.computedPay !== null)
        ? (parseFloat(e.computedPay) || 0)
        : (hours * hourlyRate);
      return {
        ...e,
        id: e.id || ('ot_' + Date.now() + Math.random().toString(36).slice(2, 6)),
        volunteerId: String(e.volunteerId || ''),
        trade: e.trade || '',
        skillLevel: e.skillLevel || '',
        date: e.date || '',
        hours,
        otArea: (e.otArea || '').trim(),
        otTask: (e.otTask || '').trim(),
        hourlyRate,
        computedPay: Math.round(computedPay * 100) / 100,
        status: e.status || 'Planned',
        createdAt: e.createdAt || new Date().toISOString()
      };
    });
    this._updateSuggestions();
  },

  async save() {
    await window.dbStores.ot_entries.setAll(this._entries);
    this._updateSuggestions();
  },

  _initDateRangeDefaults() {
    const start = document.getElementById('ot-range-start');
    const end = document.getElementById('ot-range-end');
    if (start && !start.value) {
      const d = new Date(); d.setDate(1);
      start.value = typeof toDateStr === 'function' ? toDateStr(d) : d.getFullYear() + '-' + String(d.getMonth() + 1).padStart(2, '0') + '-01';
    }
    if (end && !end.value) {
      end.value = typeof todayStr === 'function' ? todayStr() : new Date().getFullYear() + '-' + String(new Date().getMonth() + 1).padStart(2, '0') + '-' + String(new Date().getDate()).padStart(2, '0');
    }
    const oteStart = document.getElementById('ote-range-start');
    const oteEnd = document.getElementById('ote-range-end');
    const today = typeof todayStr === 'function' ? todayStr() : new Date().getFullYear() + '-' + String(new Date().getMonth() + 1).padStart(2, '0') + '-' + String(new Date().getDate()).padStart(2, '0');
    if (oteStart && !oteStart.value) oteStart.value = today;
    if (oteEnd && !oteEnd.value) oteEnd.value = today;
  },

  _populateTradeFilter() {
    const sel = document.getElementById('ot-trade-filter');
    if (!sel) return;
    const trades = [...new Set((window.workersData || []).map(w => w.trade || '').filter(Boolean))].sort();
    sel.innerHTML = '<option value="">-- All Trades --</option>' + trades.map(t => `<option value="${esc(t)}">${esc(t)}</option>`).join('');
  },

  _initTabs() {
    document.querySelectorAll('.ot-tab-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        const targetId = btn.dataset.tab;
        document.querySelectorAll('.ot-tab-btn').forEach(b => b.classList.toggle('active', b === btn));
        document.querySelectorAll('.ot-tab-content').forEach(c => c.classList.toggle('active', c.id === targetId));
        this._render();
      });
    });
  },



  _findWorker(query) {
    if (!query) return null;
    const q = String(query).trim().toLowerCase();
    if (!q) return null;
    const list = window.workersData || [];
    return list.find(w => String(w.aNumber) === q) ||
           list.find(w => (w.name || '').toLowerCase() === q) ||
           list.find(w => (w.name || '').toLowerCase().startsWith(q)) ||
           list.find(w => (w.name || '').toLowerCase().includes(q)) || null;
  },

  _initPrintButtons() {
    const scheduleBtn = document.getElementById('btn-ot-print-schedule');
    const scheduleAreaBtn = document.getElementById('btn-ot-print-schedule-area');
    const matrixBtn = document.getElementById('btn-ot-print-matrix');
    const dailyBtn = document.getElementById('btn-ot-print-daily');

    scheduleBtn?.addEventListener('click', () => this._printOvertimeSchedule());
    scheduleAreaBtn?.addEventListener('click', () => this._printOvertimeSchedule());
    matrixBtn?.addEventListener('click', () => this._printSummary());
    dailyBtn?.addEventListener('click', () => this._printDailyBreakdown());
  },

  _getEntryDateRange() {
    const isOte = document.getElementById('nav-btn-ot-entries')?.classList.contains('active');
    if (isOte) {
      return this._getOteEntryDateRange();
    }
    const start = document.getElementById('ot-range-start');
    const end = document.getElementById('ot-range-end');
    return { start: start?.value || '', end: end?.value || '' };
  },

  _getOteEntryDateRange() {
    const start = document.getElementById('ote-range-start');
    const end = document.getElementById('ote-range-end');
    return { start: start?.value || '', end: end?.value || '' };
  },

  _getOteFilteredEntries() {
    const range = this._getOteEntryDateRange();
    let entries = this._entries;
    if (range.start || range.end) {
      entries = entries.filter(e => this._entryInRange(e, range));
    }
    return [...entries].sort((a, b) => {
      const areaA = (a.otArea || '').trim();
      const areaB = (b.otArea || '').trim();
      if (!areaA && areaB) return 1;
      if (areaA && !areaB) return -1;
      const areaCmp = areaA.localeCompare(areaB, undefined, { sensitivity: 'base' });
      if (areaCmp !== 0) return areaCmp;

      const nameA = formatSurnameFirst(this._getVolunteerName(a.volunteerId));
      const nameB = formatSurnameFirst(this._getVolunteerName(b.volunteerId));
      return nameA.localeCompare(nameB, undefined, { sensitivity: 'base' });
    });
  },

  _entryInRange(entry, range) {
    if (!range.start && !range.end) return true;
    if (!entry.date) return true;
    if (range.start && entry.date < range.start) return false;
    if (range.end && entry.date > range.end) return false;
    return true;
  },

  _stepDate(deltaDays) {
    const range = this._getEntryDateRange();
    let baseDate = range.start || (typeof todayStr === 'function' ? todayStr() : new Date().toISOString().slice(0, 10));
    const parts = baseDate.split('-').map(Number);
    const d = new Date(parts[0], parts[1] - 1, parts[2] + deltaDays);
    const dateStr = typeof toDateStr === 'function' ? toDateStr(d) : d.getFullYear() + '-' + String(d.getMonth() + 1).padStart(2, '0') + '-' + String(d.getDate()).padStart(2, '0');

    const start = document.getElementById('ot-range-start'); if (start) start.value = dateStr;
    const end = document.getElementById('ot-range-end'); if (end) end.value = dateStr;
    const jump = document.getElementById('ot-jump-date'); if (jump) jump.value = dateStr;

    this._selected.clear();
    this._render();
    if (typeof showToast === 'function') showToast(`Date: ${dateStr}`, 1500);
  },

  _stepDatePage(deltaDays) {
    const start = document.getElementById('ote-range-start');
    const end = document.getElementById('ote-range-end');
    let baseDate = start?.value || (typeof todayStr === 'function' ? todayStr() : new Date().toISOString().slice(0, 10));
    if (start?.value && end?.value && start.value !== end.value) {
      baseDate = deltaDays < 0 ? start.value : end.value;
    }
    const parts = baseDate.split('-').map(Number);
    const d = new Date(parts[0], parts[1] - 1, parts[2] + deltaDays);
    const dateStr = typeof toDateStr === 'function' ? toDateStr(d) : d.getFullYear() + '-' + String(d.getMonth() + 1).padStart(2, '0') + '-' + String(d.getDate()).padStart(2, '0');
    if (start) start.value = dateStr;
    if (end) end.value = dateStr;
    const jd = document.getElementById('ote-jump-date'); if (jd) jd.value = dateStr;
    this._renderPageTable();
  },

  _goToTodayPage() {
    const today = typeof todayStr === 'function' ? todayStr() : new Date().toISOString().slice(0, 10);
    const start = document.getElementById('ote-range-start'); if (start) start.value = today;
    const end = document.getElementById('ote-range-end'); if (end) end.value = today;
    const jd = document.getElementById('ote-jump-date'); if (jd) jd.value = today;
    this._renderPageTable();
  },

  _updateOteDateLabel() {
    const el = document.getElementById('ote-current-date-label');
    if (!el) return;
    const start = document.getElementById('ote-range-start');
    const val = start?.value || '';
    if (val) {
      const d = new Date(val + 'T12:00:00');
      const opts = { weekday: 'long', year: 'numeric', month: 'long', day: 'numeric' };
      el.textContent = d.toLocaleDateString('en-US', opts);
    } else {
      el.textContent = '';
    }
  },

  _getFilteredEntries() {
    const range = this._getEntryDateRange();
    let entries = this._entries;
    if (range.start || range.end) {
      entries = entries.filter(e => this._entryInRange(e, range));
    }
    // Automatically sort workers by Area (otArea) A-Z (unassigned last), then by Name
    return [...entries].sort((a, b) => {
      const areaA = (a.otArea || '').trim();
      const areaB = (b.otArea || '').trim();
      if (!areaA && areaB) return 1;
      if (areaA && !areaB) return -1;
      const areaCmp = areaA.localeCompare(areaB, undefined, { sensitivity: 'base' });
      if (areaCmp !== 0) return areaCmp;

      const nameA = formatSurnameFirst(this._getVolunteerName(a.volunteerId));
      const nameB = formatSurnameFirst(this._getVolunteerName(b.volunteerId));
      return nameA.localeCompare(nameB, undefined, { sensitivity: 'base' });
    });
  },

  _getVolunteer(volunteerId) {
    return (window.workersData || []).find(x => String(x.aNumber) === String(volunteerId));
  },

  _getVolunteerName(volunteerId) {
    const w = this._getVolunteer(volunteerId);
    return w ? w.name : 'Unknown';
  },

  _getVolunteerTrade(volunteerId, fallbackTrade) {
    const w = this._getVolunteer(volunteerId);
    return (w && w.trade) ? w.trade : (fallbackTrade || '');
  },

  _getVolunteerSkillLevel(volunteerId, fallbackSkill) {
    const w = this._getVolunteer(volunteerId);
    return (w && w.skillLevel) ? w.skillLevel : (fallbackSkill || '');
  },

  _initDateJumpBar() {
    const prevBtn = document.getElementById('btn-ot-prev-day');
    const nextBtn = document.getElementById('btn-ot-next-day');
    const todayBtn = document.getElementById('btn-ot-today');
    const jumpDate = document.getElementById('ot-jump-date');

    prevBtn?.addEventListener('click', () => this._stepDate(-1));
    nextBtn?.addEventListener('click', () => this._stepDate(1));
    todayBtn?.addEventListener('click', () => {
      const cur = typeof todayStr === 'function' ? todayStr() : new Date().toISOString().slice(0, 10);
      const start = document.getElementById('ot-range-start'); if (start) start.value = cur;
      const end = document.getElementById('ot-range-end'); if (end) end.value = cur;
      if (jumpDate) jumpDate.value = cur;
      this._selected.clear();
      this._render();
    });

    jumpDate?.addEventListener('change', () => {
      if (!jumpDate.value) return;
      const start = document.getElementById('ot-range-start'); if (start) start.value = jumpDate.value;
      const end = document.getElementById('ot-range-end'); if (end) end.value = jumpDate.value;
      this._selected.clear();
      this._render();
    });

    const weekBtn = document.getElementById('btn-ot-this-week');
    weekBtn?.addEventListener('click', () => {
      const d = new Date(); const day = d.getDay() || 7; d.setDate(d.getDate() - day + 1);
      const end = new Date(d); end.setDate(end.getDate() + 6);
      const sStr = typeof toDateStr === 'function' ? toDateStr(d) : d.toISOString().slice(0, 10);
      const eStr = typeof toDateStr === 'function' ? toDateStr(end) : end.toISOString().slice(0, 10);
      const startEl = document.getElementById('ot-range-start'); if (startEl) startEl.value = sStr;
      const endEl = document.getElementById('ot-range-end'); if (endEl) endEl.value = eStr;
      if (jumpDate) jumpDate.value = sStr;
      this._selected.clear(); this._render();
    });

    const monthBtn = document.getElementById('btn-ot-this-month');
    monthBtn?.addEventListener('click', () => {
      const now = new Date();
      const sStr = now.getFullYear() + '-' + String(now.getMonth() + 1).padStart(2, '0') + '-01';
      const lastDay = new Date(now.getFullYear(), now.getMonth() + 1, 0);
      const eStr = typeof toDateStr === 'function' ? toDateStr(lastDay) : lastDay.toISOString().slice(0, 10);
      const startEl = document.getElementById('ot-range-start'); if (startEl) startEl.value = sStr;
      const endEl = document.getElementById('ot-range-end'); if (endEl) endEl.value = eStr;
      if (jumpDate) jumpDate.value = sStr;
      this._selected.clear(); this._render();
    });

    const clearBtn = document.getElementById('btn-ot-clear-range');
    clearBtn?.addEventListener('click', () => {
      const startEl = document.getElementById('ot-range-start'); if (startEl) startEl.value = '';
      const endEl = document.getElementById('ot-range-end'); if (endEl) endEl.value = '';
      if (jumpDate) jumpDate.value = '';
      this._selected.clear(); this._render();
    });
  },

  _initVolunteerSearch() {
    const input = document.getElementById('ot-volunteer-search');
    const results = document.getElementById('ot-volunteer-search-results');
    const quickHours = document.getElementById('ot-quick-hours');
    const quickTask = document.getElementById('ot-quick-task');
    if (!input) return;

    const doSearch = () => {
      const q = input.value.trim().toLowerCase();
      const tradeSel = document.getElementById('ot-trade-filter');
      const filterTrade = tradeSel ? tradeSel.value : '';
      if (q.length < 1) { results.style.display = 'none'; return; }
      let matches = (window.workersData || []).filter(w =>
        (w.name || '').toLowerCase().includes(q) || String(w.aNumber || '').includes(q)
      );
      if (filterTrade) matches = matches.filter(w => w.trade === filterTrade);
      matches = matches.slice(0, 10);
      if (matches.length === 0) {
        results.innerHTML = '<div style="padding:12px;color:var(--text-muted);font-size:0.82rem;text-align:center;">No matches found</div>';
      } else {
        results.innerHTML = matches.map(w => {
          const photo = w.photoUrl || w.photo;
          const avatar = photo
            ? `<img src="${esc(photo)}" alt="" style="width:28px;height:28px;border-radius:50%;object-fit:cover;">`
            : `<span style="width:28px;height:28px;border-radius:50%;background:var(--primary-light);display:flex;align-items:center;justify-content:center;font-weight:700;color:var(--primary-color);font-size:0.75rem;">${esc((w.name || '?')[0].toUpperCase())}</span>`;
          return `<button type="button" class="ot-search-result-item" data-worker-id="${esc(w.aNumber)}" style="display:flex;align-items:center;gap:10px;width:100%;padding:8px 10px;background:none;border:none;text-align:left;cursor:pointer;border-radius:6px;">${avatar}<div style="flex:1;min-width:0;"><strong style="font-size:0.82rem;display:block;">${esc(w.name)}</strong><span style="font-size:0.7rem;color:var(--text-muted);">#${esc(w.aNumber)} · ${esc(w.trade || '-')}</span></div></button>`;
        }).join('');
        results.querySelectorAll('.ot-search-result-item').forEach(item => {
          item.addEventListener('click', () => {
            const wid = item.dataset.workerId;
            results.style.display = 'none';
            input.value = '';
            this._addEntryForWorkerId(wid);
          });
        });
      }
      results.style.display = 'block';
    };

    input.addEventListener('input', debounce(doSearch, 200));

    // Hotkey: Press Enter inside quick inputs to commit entry fast
    const handleKeyEnter = (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        this._addEntry();
      }
    };
    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        const firstBtn = results.querySelector('.ot-search-result-item');
        if (firstBtn) {
          firstBtn.click();
        } else {
          this._addEntry();
        }
      }
    });
    quickHours?.addEventListener('keydown', handleKeyEnter);
    quickTask?.addEventListener('keydown', handleKeyEnter);

    document.addEventListener('click', e => {
      if (!input.contains(e.target) && !results.contains(e.target)) results.style.display = 'none';
    });
  },

  _updateSuggestions() {
    const tasks = [...new Set(this._entries.map(e => e.otTask).filter(Boolean))].sort();
    const areas = [...new Set(this._entries.map(e => e.otArea).filter(Boolean))].sort();
    const taskDl = document.getElementById('ot-task-options');
    const areaDl = document.getElementById('ot-area-options');
    if (taskDl) taskDl.innerHTML = tasks.map(v => `<option value="${esc(v)}">`).join('');
    if (areaDl) areaDl.innerHTML = areas.map(v => `<option value="${esc(v)}">`).join('');
  },

  // --- ENTRY MANAGEMENT ---

  async _addEntry() {
    const searchInput = document.getElementById('ot-volunteer-search');
    let aNum = '';
    if (searchInput) {
      const q = searchInput.value.trim();
      const match = this._findWorker(q);
      if (match) aNum = match.aNumber;
    }
    if (!aNum) { showToast('Type a volunteer name in the search box first.'); return; }
    this._addEntryForWorkerId(aNum);
  },

  async _addPageEntry() {
    const searchInput = document.getElementById('ote-volunteer-search');
    let aNum = '';
    if (searchInput) {
      const q = searchInput.value.trim();
      const match = this._findWorker(q);
      if (match) aNum = match.aNumber;
    }
    if (!aNum) { showToast('Type a volunteer name in the search box first.'); return; }
    const oteRange = this._getOteEntryDateRange();
    this._addEntryForWorkerId(aNum, oteRange.start || '');
    this._renderPageTable();
  },

  async _addEntryForWorkerId(aNum, optDate) {
    if (!aNum) { showToast('Select a volunteer first.'); return; }
    const w = this._getVolunteer(aNum);
    if (!w) { showToast('Volunteer not found.'); return; }

    const trade = w.trade || '';
    const skillLevel = w.skillLevel || '';
    const hourlyRate = parseFloat(w.hourlyRate) || 0;

    const isOte = document.getElementById('nav-btn-ot-entries')?.classList.contains('active');
    const range = isOte ? this._getOteEntryDateRange() : this._getEntryDateRange();
    const curDateStr = optDate || range.start || (typeof todayStr === 'function' ? todayStr() : new Date().toISOString().slice(0, 10));

    if (typeof getWorkerClearanceState === 'function') {
      const clearance = getWorkerClearanceState(w, curDateStr);
      if (clearance && clearance.isHold) {
        showToast(`Warning: ${w.name} is currently on Medical Hold / Unfit for Duty.`, 'warning');
      } else if (clearance && clearance.status === 'CLEARED_RESTRICTED') {
        const restr = Array.isArray(clearance.restrictions) && clearance.restrictions.length > 0 ? ` (${clearance.restrictions.join(', ')})` : '';
        showToast(`Notice: ${w.name} is on Restricted Duty${restr}.`, 'warning');
      }
    }

    const prefix = isOte ? 'ote-' : 'ot-';
    const quickArea = document.getElementById(prefix + 'quick-area')?.value.trim();
    const quickTask = document.getElementById(prefix + 'quick-task')?.value.trim();
    const quickHoursVal = parseFloat(document.getElementById(prefix + 'quick-hours')?.value);
    const hours = !isNaN(quickHoursVal) && quickHoursVal >= 0 ? quickHoursVal : 2.0;

    const area = quickArea || w.area || '';
    const task = quickTask || w.specificTask || '';
    const computedPay = Math.round(hours * hourlyRate * 100) / 100;

    this._entries.push({
      id: 'ot_' + Date.now() + Math.random().toString(36).slice(2, 6),
      volunteerId: String(aNum),
      trade,
      skillLevel,
      date: curDateStr,
      hours,
      otArea: area,
      otTask: task,
      hourlyRate,
      computedPay,
      status: 'Planned',
      createdAt: new Date().toISOString()
    });

    this._selected.clear();
    await this.save();
    this._render();

    // Refocus volunteer search input for rapid continuous encoding
    const searchInput = document.getElementById(prefix + 'volunteer-search') || document.getElementById('ot-volunteer-search');
    if (searchInput) {
      searchInput.value = '';
      searchInput.focus();
    }

    if (hourlyRate === 0) {
      showToast(`Added ${esc(w.name)} : no hourly rate set.`, 3000);
    } else {
      showToast(`Added ${esc(w.name)} (${hours}h @ ${esc(area || 'Unassigned Area')})`, 2000);
    }
  },

  async _removeEntry(index) {
    this._entries.splice(index, 1);
    await this.save();
    this._render();
    this._renderPageTable();
  },

  async _duplicateNextDay() {
    const range = this._getEntryDateRange();
    const filtered = this._getFilteredEntries();
    if (filtered.length === 0) { showToast('No entries to copy. Add entries first.'); return; }

    const dates = [...new Set(filtered.map(e => e.date).filter(Boolean))].sort();
    const srcDate = dates[dates.length - 1];
    const srcParts = srcDate.split('-').map(Number);
    const next = new Date(srcParts[0], srcParts[1] - 1, srcParts[2] + 1);
    const dateStr = next.getFullYear() + '-' + String(next.getMonth() + 1).padStart(2, '0') + '-' + String(next.getDate()).padStart(2, '0');

    let count = 0;
    filtered.forEach(e => {
      if (e.date !== srcDate) return;
      const w = this._getVolunteer(e.volunteerId);
      const trade = (w && w.trade) || e.trade || '';
      const skillLevel = (w && w.skillLevel) || e.skillLevel || '';
      const rate = parseFloat(w?.hourlyRate) || e.hourlyRate || 0;
      this._entries.push({
        id: 'ot_' + Date.now() + Math.random().toString(36).slice(2, 6) + count,
        volunteerId: e.volunteerId,
        trade, skillLevel,
        date: dateStr,
        hours: 0, otArea: e.otArea || '', otTask: e.otTask || '',
        hourlyRate: rate, computedPay: 0,
        status: 'Planned',
        createdAt: new Date().toISOString()
      });
      count++;
    });

    if (count === 0) { showToast('No entries to copy for the latest date.'); return; }
    this._selected.clear();
    await this.save();
    this._render();
    showToast(`Copied ${count} entries to ${dateStr}. Hours reset to 0.`, 3000);
  },

  async _loadYesterday() {
    const today = typeof todayStr === 'function' ? todayStr() : new Date().toISOString().slice(0, 10);
    const parts = today.split('-').map(Number);
    const yesterday = new Date(parts[0], parts[1] - 1, parts[2] - 1);
    const yStr = yesterday.getFullYear() + '-' + String(yesterday.getMonth() + 1).padStart(2, '0') + '-' + String(yesterday.getDate()).padStart(2, '0');

    const yesterdayEntries = this._entries.filter(e => e.date === yStr);
    if (yesterdayEntries.length === 0) { showToast('No entries from ' + yStr + ' to copy.'); return; }

    let count = 0;
    yesterdayEntries.forEach(e => {
      const w = this._getVolunteer(e.volunteerId);
      const trade = (w && w.trade) || e.trade || '';
      const skillLevel = (w && w.skillLevel) || e.skillLevel || '';
      const rate = parseFloat(w?.hourlyRate) || e.hourlyRate || 0;
      this._entries.push({
        id: 'ot_' + Date.now() + Math.random().toString(36).slice(2, 6) + count,
        volunteerId: e.volunteerId,
        trade, skillLevel,
        date: today,
        hours: 0, otArea: e.otArea || '', otTask: e.otTask || '',
        hourlyRate: rate, computedPay: 0,
        status: 'Planned',
        createdAt: new Date().toISOString()
      });
      count++;
    });

    const start = document.getElementById('ot-range-start');
    const end = document.getElementById('ot-range-end');
    if (start) start.value = today;
    if (end) end.value = today;

    await this.save();
    this._render();
    this._renderPageTable();
    showToast(`Loaded ${count} entries from yesterday (${yStr}) → today. Hours reset to 0.`, 3000);
  },

  async _clearAll() {
    if (!await showConfirm('Remove ALL OT entries from the table? This cannot be undone.')) return;
    this._entries = [];
    await window.dbStores.ot_entries.setAll([]);
    this._render();
    this._renderPageTable();
    showToast('All entries cleared.');
  },

  async _handleEdit(e) {
    const el = e.target;
    const entryId = el.dataset.otId;
    let entry = this._entries.find(x => x.id === entryId);
    if (!entry) {
      const idx = parseInt(el.dataset.otIdx);
      if (!isNaN(idx) && this._entries[idx]) entry = this._entries[idx];
    }
    if (!entry) return;

    const isOte = el.classList.contains('ote-edit-hours') || el.classList.contains('ote-edit-area') || el.classList.contains('ote-edit-task');

    if (el.classList.contains('ot-edit-date') || el.classList.contains('ote-edit-date')) {
      entry.date = el.value;
    } else if (el.classList.contains('ot-edit-hours') || el.classList.contains('ote-edit-hours')) {
      entry.hours = parseFloat(el.value) || 0;
      entry.computedPay = Math.round(entry.hours * (Number(entry.hourlyRate) || 0) * 100) / 100;
      const payCell = el.closest('tr')?.querySelector('.pay-col');
      if (payCell) payCell.textContent = '₱' + entry.computedPay.toFixed(2);
    } else if (el.classList.contains('ote-edit-rate')) {
      entry.hourlyRate = parseFloat(el.value) || 0;
      entry.computedPay = Math.round(entry.hours * entry.hourlyRate * 100) / 100;
      const payCell = el.closest('tr')?.querySelector('.pay-col');
      if (payCell) payCell.textContent = '₱' + entry.computedPay.toFixed(2);
    } else if (el.classList.contains('ot-edit-area') || el.classList.contains('ote-edit-area')) {
      entry.otArea = el.value;
    } else if (el.classList.contains('ot-edit-task') || el.classList.contains('ote-edit-task')) {
      entry.otTask = el.value;
    }

    if (this._saveTimeout) clearTimeout(this._saveTimeout);
    this._saveTimeout = setTimeout(() => { this.save(); }, 300);

    if (e.type === 'change' || e.type === 'blur') {
      if (isOte) { this._renderPageTable(); } else { this._render(); }
    }
  },

  // --- RENDERING ---

  _render() {
    this._renderSummaryTab();
    this._renderByAreaTab();
  },



  async _renderEntriesPage() {
    const tbody = document.getElementById('ote-tbody');
    if (!tbody) return;

    if (!this._initialized) await this.init();
    else {
      this._populateTradeFilter();
    }

    const sel = document.getElementById('ote-trade-filter');
    const otSel = document.getElementById('ot-trade-filter');
    if (sel && otSel) sel.innerHTML = otSel.innerHTML;

    this._renderPageTable();
    this._attachPageListeners();
  },

  _renderPageTable() {
    const tbody = document.getElementById('ote-tbody');
    if (!tbody) return;
    this._updateOteDateLabel();
    let entries = this._getOteFilteredEntries();
    const tradeFilter = document.getElementById('ote-trade-filter')?.value;
    if (tradeFilter) {
      entries = entries.filter(e => {
        const trade = this._getVolunteerTrade(e.volunteerId, e.trade);
        return trade === tradeFilter;
      });
    }
    if (entries.length === 0) {
      tbody.innerHTML = '<tr><td colspan="9" class="empty-state-row">No entries for this date. Add one using the toolbar above.</td></tr>';
      const gt = document.getElementById('ote-grand-total');
      if (gt) gt.textContent = 'Grand Total: ₱0.00';
      return;
    }

    let lastArea = null;
    let rowsHtml = '';

    entries.forEach((e) => {
      const originalIndex = this._entries.indexOf(e);
      const currentArea = (e.otArea || 'Unassigned Area').trim();

      if (currentArea.toUpperCase() !== (lastArea || '').toUpperCase()) {
        lastArea = currentArea;
        rowsHtml += `<tr class="ot-area-group-row"><td colspan="9"><div class="ot-area-group-badge">&#x1F4CD; Area: <strong>${esc(currentArea)}</strong></div></td></tr>`;
      }

      rowsHtml += `<tr>
        <td style="font-weight:600;">${esc(this._getVolunteerName(e.volunteerId))}</td>
        <td>${esc(this._getVolunteerTrade(e.volunteerId, e.trade)) || '-'}</td>
        <td>${esc(this._getVolunteerSkillLevel(e.volunteerId, e.skillLevel)) || '-'}</td>
        <td class="ot-cell-num"><input type="number" step="0.5" min="0" value="${Number(e.hours).toFixed(1)}" data-ot-id="${esc(e.id)}" data-ot-idx="${originalIndex}" class="ote-edit-hours ot-cell-input" style="text-align:right;width:55px;"></td>
        <td><input type="text" value="${esc(e.otArea || '')}" data-ot-id="${esc(e.id)}" data-ot-idx="${originalIndex}" class="ote-edit-area ot-cell-input" placeholder="Where" list="ote-area-options"></td>
        <td><input type="text" value="${esc(e.otTask || '')}" data-ot-id="${esc(e.id)}" data-ot-idx="${originalIndex}" class="ote-edit-task ot-cell-input" placeholder="What" list="ote-task-options"></td>
        <td class="rate-col" style="text-align:right;font-size:0.8rem;white-space:nowrap;">₱${Number(e.hourlyRate).toFixed(2)}</td>
        <td class="pay-col" style="text-align:right;font-weight:700;font-size:0.8rem;">₱${Number(e.computedPay).toFixed(2)}</td>
        <td style="text-align:center;"><button class="btn btn-danger" onclick="window.OtCalcModule._removeEntry(${originalIndex})" title="Remove" aria-label="Remove" style="padding:2px 8px;font-size:0.75rem;">&times;</button></td>
      </tr>`;
    });

    tbody.innerHTML = rowsHtml;
    const total = entries.reduce((sum, e) => sum + (Number(e.computedPay) || 0), 0);
    const gt = document.getElementById('ote-grand-total');
    if (gt) gt.textContent = 'Grand Total: ₱' + total.toFixed(2);

  },

  _attachPageListeners() {
    const showRate = document.getElementById('ote-show-rate');
    if (showRate && !showRate._oteBound) {
      showRate._oteBound = true;
      showRate.addEventListener('change', () => {
        document.querySelectorAll('#ote-table .rate-col, #ote-table .pay-col').forEach(el => {
          el.style.display = showRate.checked ? '' : 'none';
        });
      });
    }

    const searchInput = document.getElementById('ote-volunteer-search');
    const results = document.getElementById('ote-volunteer-search-results');
    if (searchInput && results && !searchInput._oteBound) {
      searchInput._oteBound = true;
      searchInput.addEventListener('input', debounce(() => {
        const q = searchInput.value.trim().toLowerCase();
        if (q.length < 1) { results.style.display = 'none'; return; }
        const tradeFilter = document.getElementById('ote-trade-filter')?.value || '';
        let matches = (window.workersData || []).filter(w =>
          (w.name || '').toLowerCase().includes(q) || String(w.aNumber || '').includes(q)
        );
        if (tradeFilter) matches = matches.filter(w => w.trade === tradeFilter);
        matches = matches.slice(0, 10);
        if (matches.length === 0) {
          results.innerHTML = '<div style="padding:12px;color:var(--text-muted);font-size:0.82rem;text-align:center;">No matches found</div>';
        } else {
          results.innerHTML = matches.map(w => {
            const photo = w.photoUrl || w.photo;
            const avatar = photo
              ? `<img src="${esc(photo)}" alt="" style="width:28px;height:28px;border-radius:50%;object-fit:cover;">`
              : `<span style="width:28px;height:28px;border-radius:50%;background:var(--primary-light);display:flex;align-items:center;justify-content:center;font-weight:700;color:var(--primary-color);font-size:0.75rem;">${esc((w.name || '?')[0].toUpperCase())}</span>`;
            return `<button type="button" class="ote-search-result-item" data-worker-id="${esc(w.aNumber)}" style="display:flex;align-items:center;gap:10px;width:100%;padding:8px 10px;background:none;border:none;text-align:left;cursor:pointer;border-radius:6px;">${avatar}<div style="flex:1;min-width:0;"><strong style="font-size:0.82rem;display:block;">${esc(w.name)}</strong><span style="font-size:0.7rem;color:var(--text-muted);">#${esc(w.aNumber)} · ${esc(w.trade || '-')}</span></div></button>`;
          }).join('');
          results.querySelectorAll('.ote-search-result-item').forEach(item => {
            item.addEventListener('click', () => {
              const wid = item.dataset.workerId;
              results.style.display = 'none';
              searchInput.value = '';
              const range = this._getEntryDateRange();
              const curDate = range.start || (typeof todayStr === 'function' ? todayStr() : new Date().toISOString().slice(0, 10));
              const existing = this._entries.find(e => e.volunteerId === String(wid) && e.date === curDate);
              if (existing) { showToast('Entry already exists for this volunteer on this date.'); return; }
              this._addEntryForWorkerId(wid);
              this._renderPageTable();
});


          });
        }
        results.style.display = '';
      }, 120));

      searchInput.addEventListener('blur', () => {
        setTimeout(() => { results.style.display = 'none'; }, 200);
      });
    }
  },

  _openOtEncoder() {
    const modal = document.getElementById('ot-encoder-modal');
    if (!modal) return;

    this._encoderPending = [];
    const searchInput = document.getElementById('ote-enc-volunteer-search');
    if (searchInput) { searchInput.value = ''; searchInput.focus(); }
    document.getElementById('ote-enc-search-results').style.display = 'none';
    this._renderEncoderPending();

    modal.classList.add('active');
    if (window.trapFocus) window.trapFocus(modal);
  },

  _renderEncoderPending() {
    const tbody = document.getElementById('ote-enc-selected-tbody');
    if (!tbody) return;
    const pending = this._encoderPending;
    if (pending.length === 0) {
      tbody.innerHTML = '<tr><td colspan="6" style="text-align:center;color:var(--text-muted);padding:32px;font-size:0.82rem;">Search and select a volunteer above</td></tr>';
    } else {
      tbody.innerHTML = pending.map((p, i) => `<tr>
        <td style="padding:6px 8px;border-bottom:1px solid #e2e8f0;font-weight:600;white-space:nowrap;">${esc(p.name)}</td>
        <td style="padding:6px 8px;border-bottom:1px solid #e2e8f0;font-size:0.75rem;">${esc(p.trade || '-')}</td>
        <td style="padding:6px 8px;border-bottom:1px solid #e2e8f0;"><input type="date" value="${esc(p.date)}" data-pending-idx="${i}" class="ote-enc-pending-date" style="width:100%;padding:3px 4px;border:1px solid var(--border-color);border-radius:4px;font-size:0.75rem;background:var(--card-bg);color:var(--text-main);box-sizing:border-box;"></td>
        <td style="padding:6px 8px;border-bottom:1px solid #e2e8f0;"><input type="text" value="${esc(p.area || '')}" data-pending-idx="${i}" class="ote-enc-pending-area" placeholder="Area" list="ote-area-options" style="width:100%;padding:3px 4px;border:1px solid var(--border-color);border-radius:4px;font-size:0.75rem;background:var(--card-bg);color:var(--text-main);box-sizing:border-box;"></td>
        <td style="padding:6px 8px;border-bottom:1px solid #e2e8f0;"><input type="text" value="${esc(p.task || '')}" data-pending-idx="${i}" class="ote-enc-pending-task" placeholder="Task" list="ote-task-options" style="width:100%;padding:3px 4px;border:1px solid var(--border-color);border-radius:4px;font-size:0.75rem;background:var(--card-bg);color:var(--text-main);box-sizing:border-box;"></td>
        <td style="padding:6px 8px;border-bottom:1px solid #e2e8f0;text-align:center;"><button class="ote-enc-pending-remove" data-pending-idx="${i}" style="background:none;border:none;color:#ef4444;cursor:pointer;font-size:1.1rem;line-height:1;padding:0 4px;" title="Remove">&times;</button></td>
      </tr>`).join('');
    }
    const count = document.getElementById('ote-enc-count');
    if (count) { count.textContent = pending.length; count.style.display = pending.length > 0 ? 'inline-block' : 'none'; }
    const label = document.getElementById('ote-enc-footer-label');
    if (label) label.textContent = pending.length > 0 ? pending.length + ' entr' + (pending.length === 1 ? 'y' : 'ies') + ' not yet saved' : 'No entries added';
    const btn = document.getElementById('btn-ote-enc-save-all');
    if (btn) btn.disabled = pending.length === 0;
  },

  _updateGrandTotal(total) {
    ['ot-grand-total-value', 'ot-summary-grand-total'].forEach(id => {
      const el = document.getElementById(id);
      if (el) el.textContent = '₱' + total.toFixed(2);
    });
  },

  _renderSummaryTab() {
    const container = document.getElementById('ot-summary-cards-container');
    if (!container) return;
    const range = this._getEntryDateRange();
    const filtered = this._getFilteredEntries();

    const subs = {};
    filtered.forEach(e => {
      const key = e.volunteerId;
      if (!subs[key]) {
        const trade = this._getVolunteerTrade(e.volunteerId, e.trade);
        const skillLevel = this._getVolunteerSkillLevel(e.volunteerId, e.skillLevel);
        subs[key] = { name: this._getVolunteerName(e.volunteerId), trade: trade || '-', skillLevel: skillLevel || '-', hours: 0, pay: 0, entries: [] };
      }
      subs[key].hours += Number(e.hours) || 0;
      subs[key].pay += Number(e.computedPay) || 0;
      subs[key].entries.push(e);
    });

    const grand = Object.values(subs).reduce((sum, s) => sum + s.pay, 0);
    this._updateGrandTotal(grand);

    if (Object.keys(subs).length === 0) {
      container.innerHTML = '<div class="empty-state-row">No entries found for the selected date range.</div>';
      return;
    }

    container.innerHTML = Object.values(subs).map(v => {
      const rows = v.entries.map(e => `
        <tr>
          <td>${esc(e.date || '-')}</td>
          <td>${esc(e.otArea || '-')}</td>
          <td>${esc(e.otTask || '-')}</td>
          <td style="text-align:right;">${Number(e.hours).toFixed(1)}</td>
          <td style="text-align:right;">₱${Number(e.hourlyRate).toFixed(2)}</td>
          <td style="text-align:right;font-weight:600;">₱${Number(e.computedPay).toFixed(2)}</td>
        </tr>
      `).join('');

      return `
        <div class="ot-vol-card">
          <div class="ot-vol-card-header">
            <div class="ot-vol-info">
              <span class="ot-vol-name">${esc(v.name)}</span>
              <span class="ot-vol-badge">${esc(v.trade)}${v.skillLevel && v.skillLevel !== '—' ? ' (' + esc(v.skillLevel) + ')' : ''}</span>
            </div>
            <div class="ot-vol-stats">
              <span class="ot-vol-stat-hours">${v.hours.toFixed(1)} hrs</span>
              <span class="ot-vol-stat-pay">₱${v.pay.toFixed(2)}</span>
            </div>
          </div>
          <table class="ot-vol-table">
            <thead>
              <tr>
                <th style="width:110px;">Date</th>
                <th style="width:25%;">Area (Where)</th>
                <th>Task / Work Done</th>
                <th style="text-align:right;width:70px;">Hours</th>
                <th style="text-align:right;width:90px;">Rate</th>
                <th style="text-align:right;width:90px;">Pay</th>
              </tr>
            </thead>
            <tbody>${rows}</tbody>
          </table>
        </div>
      `;
    }).join('');
  },

  _renderByAreaTab() {
    const container = document.getElementById('ot-area-cards-container');
    if (!container) return;
    const filtered = this._getFilteredEntries();

    if (filtered.length === 0) {
      container.innerHTML = '<div class="empty-state-row">No entries found for the selected date range.</div>';
      const el = document.getElementById('ot-area-grand-total');
      if (el) el.textContent = '₱0.00';
      return;
    }

    const areas = {};
    filtered.forEach(e => {
      const key = e.otArea || '(Unassigned)';
      if (!areas[key]) areas[key] = { hours: 0, pay: 0, entries: [] };
      areas[key].hours += Number(e.hours) || 0;
      areas[key].pay += Number(e.computedPay) || 0;
      areas[key].entries.push(e);
    });

    const grand = Object.values(areas).reduce((sum, s) => sum + s.pay, 0);
    const el = document.getElementById('ot-area-grand-total');
    if (el) el.textContent = '₱' + grand.toFixed(2);

    const sortedAreas = Object.keys(areas).sort();
    container.innerHTML = sortedAreas.map(area => {
      const v = areas[area];
      const rows = v.entries.map(e => `
        <tr>
          <td>${esc(this._getVolunteerName(e.volunteerId))}</td>
          <td>${esc(e.date || '-')}</td>
          <td style="text-align:right;">${Number(e.hours).toFixed(1)}</td>
          <td style="text-align:right;">₱${Number(e.hourlyRate).toFixed(2)}</td>
          <td style="text-align:right;font-weight:600;">₱${Number(e.computedPay).toFixed(2)}</td>
        </tr>
      `).join('');

      return `
        <div class="ot-vol-card">
          <div class="ot-vol-card-header">
            <div class="ot-vol-info">
              <span class="ot-vol-name">${esc(area)}</span>
              <span class="ot-vol-badge">${v.entries.length} entries</span>
            </div>
            <div class="ot-vol-stats">
              <span class="ot-vol-stat-hours">${v.hours.toFixed(1)} hrs</span>
              <span class="ot-vol-stat-pay">₱${v.pay.toFixed(2)}</span>
            </div>
          </div>
          <table class="ot-vol-table">
            <thead>
              <tr>
                <th style="width:30%;">Volunteer</th>
                <th style="width:110px;">Date</th>
                <th style="text-align:right;width:70px;">Hours</th>
                <th style="text-align:right;width:90px;">Rate</th>
                <th style="text-align:right;width:90px;">Pay</th>
              </tr>
            </thead>
            <tbody>${rows}</tbody>
          </table>
        </div>
      `;
    }).join('');
  },

  // --- PRINT REPORTS ---

  async _printOvertimeSchedule() {
    const filtered = this._getFilteredEntries();
    if (filtered.length === 0) {
      showToast('No entries in the selected date range to print.');
      return;
    }

    const dates = [...new Set(filtered.map(e => e.date).filter(Boolean))].sort();
    let dateSubtitle = '';
    if (dates.length === 1) {
      dateSubtitle = formatScheduleDate(dates[0]);
    } else if (dates.length > 1) {
      dateSubtitle = `${formatScheduleDate(dates[0])} – ${formatScheduleDate(dates[dates.length - 1])}`;
    } else {
      const range = this._getEntryDateRange();
      if (range.start && range.end) {
        dateSubtitle = range.start === range.end ? formatScheduleDate(range.start) : `${formatScheduleDate(range.start)} – ${formatScheduleDate(range.end)}`;
      } else {
        dateSubtitle = formatScheduleDate(new Date().toISOString().slice(0, 10));
      }
    }

    // Group by Area
    const areas = {};
    filtered.forEach(e => {
      const area = (e.otArea || 'UNASSIGNED AREA').trim().toUpperCase();
      if (!areas[area]) areas[area] = [];
      areas[area].push(e);
    });
    const sortedAreaNames = Object.keys(areas).sort();

    let grandTotalHours = 0;
    let grandTotalPay = 0;

    const areaBlocks = sortedAreaNames.map(areaName => {
      const entries = areas[areaName];
      entries.sort((a, b) => {
        const nameA = formatSurnameFirst(this._getVolunteerName(a.volunteerId));
        const nameB = formatSurnameFirst(this._getVolunteerName(b.volunteerId));
        return nameA.localeCompare(nameB);
      });

      // Group entries by volunteer within area
      const volMap = {};
      entries.forEach(e => {
        const id = e.volunteerId;
        if (!volMap[id]) volMap[id] = { name: this._getVolunteerName(e.volunteerId), entries: [], totalHours: 0, totalPay: 0 };
        volMap[id].entries.push(e);
        const hrs = Number(e.hours) || 0;
        const pay = Number(e.computedPay) || 0;
        volMap[id].totalHours += hrs;
        volMap[id].totalPay += pay;
      });

      const volOrder = Object.keys(volMap).sort((a, b) => formatSurnameFirst(volMap[a].name).localeCompare(formatSurnameFirst(volMap[b].name)));

      let rowIdx = 0;
      let areaHours = 0;
      let areaPay = 0;
      const volSections = volOrder.map(id => {
        const v = volMap[id];
        const detailRows = v.entries.map(e => {
          rowIdx++;
          const w = this._getVolunteer(e.volunteerId);
          const trade = (w && w.trade) || e.trade || '-';
          const task = e.otTask || '-';
          const hrs = Number(e.hours) || 0;
          const rate = Number(e.hourlyRate) || 0;
          const pay = Number(e.computedPay) || 0;
          areaHours += hrs;
          areaPay += pay;
          return `<tr>
            <td style="text-align:center;width:32px;padding:5px 6px;border:1px solid #E2E8F0;font-weight:600;color:#64748B;font-size:0.8rem;">${rowIdx}</td>
            <td style="padding:5px 6px;border:1px solid #E2E8F0;font-weight:700;font-size:0.82rem;">${esc(formatSurnameFirst(v.name))}</td>
            <td style="padding:5px 6px;border:1px solid #E2E8F0;font-size:0.78rem;color:#475569;">${esc(trade)}</td>
            <td style="padding:5px 6px;border:1px solid #E2E8F0;font-size:0.78rem;">${esc(e.date || '-')}</td>
            <td style="padding:5px 6px;border:1px solid #E2E8F0;font-size:0.78rem;">${esc(task)}</td>
            <td style="padding:5px 6px;border:1px solid #E2E8F0;text-align:right;font-size:0.82rem;">${hrs.toFixed(1)}</td>
            <td style="padding:5px 6px;border:1px solid #E2E8F0;text-align:right;font-size:0.8rem;">₱${rate.toFixed(2)}</td>
            <td style="padding:5px 6px;border:1px solid #E2E8F0;text-align:right;font-size:0.82rem;font-weight:600;">₱${pay.toFixed(2)}</td>
          </tr>`;
        }).join('');

        return detailRows + `<tr style="background:#FFFBEB;">
          <td colspan="5" style="padding:4px 6px;border:1px solid #E2E8F0;font-weight:700;font-size:0.78rem;color:#92400E;text-align:right;">Subtotal : ${esc(formatSurnameFirst(v.name))}</td>
          <td style="padding:4px 6px;border:1px solid #E2E8F0;text-align:right;font-weight:700;font-size:0.82rem;color:#92400E;">${v.totalHours.toFixed(1)}</td>
          <td style="padding:4px 6px;border:1px solid #E2E8F0;"></td>
          <td style="padding:4px 6px;border:1px solid #E2E8F0;text-align:right;font-weight:700;font-size:0.82rem;color:#92400E;">₱${v.totalPay.toFixed(2)}</td>
        </tr>`;
      }).join('');

      grandTotalHours += areaHours;
      grandTotalPay += areaPay;

      return `<div class="ot-area-block">
        <table class="ot-area-table" style="width:100%;border-collapse:collapse;font-size:0.85rem;">
          <thead>
            <tr style="background:#FED7AA;color:#431407;">
              <th style="width:32px;text-align:center;padding:7px 6px;border:1px solid #FDBA74;text-transform:uppercase;font-size:0.75rem;">#</th>
              <th style="text-align:left;padding:7px 6px;border:1px solid #FDBA74;text-transform:uppercase;font-size:0.75rem;letter-spacing:0.03em;">${esc(areaName)}</th>
              <th style="text-align:left;padding:7px 6px;border:1px solid #FDBA74;text-transform:uppercase;font-size:0.75rem;letter-spacing:0.03em;">Trade</th>
              <th style="text-align:left;padding:7px 6px;border:1px solid #FDBA74;text-transform:uppercase;font-size:0.75rem;letter-spacing:0.03em;">Date</th>
              <th style="text-align:left;padding:7px 6px;border:1px solid #FDBA74;text-transform:uppercase;font-size:0.75rem;letter-spacing:0.03em;">Task</th>
              <th style="text-align:right;padding:7px 6px;border:1px solid #FDBA74;text-transform:uppercase;font-size:0.75rem;letter-spacing:0.03em;width:65px;">Hours</th>
              <th style="text-align:right;padding:7px 6px;border:1px solid #FDBA74;text-transform:uppercase;font-size:0.75rem;letter-spacing:0.03em;width:75px;">Rate</th>
              <th style="text-align:right;padding:7px 6px;border:1px solid #FDBA74;text-transform:uppercase;font-size:0.75rem;letter-spacing:0.03em;width:80px;">Pay</th>
            </tr>
          </thead>
          <tbody>
            ${volSections}
            <tr style="background:#FEF2F2;">
              <td colspan="5" style="padding:6px 6px;border:1px solid #E2E8F0;font-weight:800;font-size:0.85rem;color:#991B1B;text-align:right;">AREA TOTAL : ${esc(areaName)}</td>
              <td style="padding:6px 6px;border:1px solid #E2E8F0;text-align:right;font-weight:800;font-size:0.85rem;color:#991B1B;">${areaHours.toFixed(1)}</td>
              <td style="padding:6px 6px;border:1px solid #E2E8F0;"></td>
              <td style="padding:6px 6px;border:1px solid #E2E8F0;text-align:right;font-weight:800;font-size:0.85rem;color:#991B1B;">₱${areaPay.toFixed(2)}</td>
            </tr>
          </tbody>
        </table>
      </div>`;
    }).join('');

    const printHtml = `<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>OVERTIME SCHEDULE</title>
  <style>
    @page { size: auto; margin: 8mm 10mm; }
    body { font-family: 'Inter', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; color: #0f172a; margin: 0; padding: 12px; line-height: 1.4; }
    .ot-schedule-header { background: #5C2C0C; color: #ffffff; text-align: center; padding: 10px 16px; font-size: 1.35rem; font-weight: 800; letter-spacing: 0.05em; text-transform: uppercase; font-family: inherit; border-radius: 4px 4px 0 0; }
    .ot-schedule-subheader { background: #B85C1C; color: #ffffff; text-align: center; padding: 6px 16px; font-size: 0.9rem; font-style: italic; font-weight: 500; border-radius: 0 0 4px 4px; }
    .ot-area-block { margin-top: 18px; break-inside: avoid; page-break-inside: avoid; }
    .ot-area-table tr:nth-child(even) td { background: #F8FAFC; }
  </style>
</head>
<body>
<div class="print-container">
    <div class="ot-schedule-header">OVERTIME SCHEDULE</div>
    <div class="ot-schedule-subheader">By Area : Summary with Compensation &nbsp;|&nbsp; ${esc(dateSubtitle)}</div>
    ${areaBlocks}
    <div style="margin-top:24px;text-align:right;padding:12px 16px;border-top:3px solid #5C2C0C;">
      <span style="font-size:1.1rem;font-weight:800;color:#5C2C0C;">GRAND TOTAL: &nbsp; ${grandTotalHours.toFixed(1)} hrs &nbsp;|&nbsp; ₱${grandTotalPay.toFixed(2)}</span>
    </div>
  </div>
</body>
</html>`;

    safePrint(printHtml);
  },

  async _printSummary() {
    const filtered = this._getFilteredEntries();
    if (filtered.length === 0) { showToast('No entries in the selected date range to print.'); return; }

    const range = this._getEntryDateRange();
    const subs = {};
    filtered.forEach(e => {
      const key = e.volunteerId;
      if (!subs[key]) {
        const trade = this._getVolunteerTrade(e.volunteerId, e.trade);
        const skillLevel = this._getVolunteerSkillLevel(e.volunteerId, e.skillLevel);
        subs[key] = { name: this._getVolunteerName(e.volunteerId), trade: trade || '-', skillLevel: skillLevel || '-', hours: 0, pay: 0, entries: [] };
      }
      subs[key].hours += Number(e.hours) || 0;
      subs[key].pay += Number(e.computedPay) || 0;
      subs[key].entries.push(e);
    });

    const grand = Object.values(subs).reduce((sum, s) => sum + s.pay, 0);
    const periodLabel = (range.start || '...') + ' to ' + (range.end || '...');

    const volSections = Object.values(subs).map(v => {
      const itemRows = v.entries.map(e => `
        <tr>
          <td style="padding:6px 8px;border:1px solid #cbd5e1;white-space:nowrap;">${esc(e.date || '-')}</td>
          <td style="padding:6px 8px;border:1px solid #cbd5e1;word-break:break-word;white-space:normal;">${esc(e.otArea || '-')}</td>
          <td style="padding:6px 8px;border:1px solid #cbd5e1;word-break:break-word;white-space:normal;">${esc(e.otTask || '-')}</td>
          <td style="padding:6px 8px;border:1px solid #cbd5e1;text-align:right;">${Number(e.hours).toFixed(1)}</td>
          <td style="padding:6px 8px;border:1px solid #cbd5e1;text-align:right;">₱${Number(e.hourlyRate).toFixed(2)}</td>
          <td style="padding:6px 8px;border:1px solid #cbd5e1;text-align:right;font-weight:600;">₱${Number(e.computedPay).toFixed(2)}</td>
        </tr>
      `).join('');

      return `
        <div style="break-inside:avoid;page-break-inside:avoid;margin-bottom:20px;border:1px solid #cbd5e1;border-radius:6px;overflow:hidden;">
          <div style="background:#f1f5f9;padding:8px 12px;display:flex;justify-content:space-between;align-items:center;border-bottom:1px solid #cbd5e1;">
            <div>
              <strong style="font-size:0.95rem;color:#0f172a;">${esc(v.name)}</strong>
              <span style="margin-left:8px;font-size:0.75rem;color:#475569;background:#e2e8f0;padding:2px 6px;border-radius:4px;">${esc(v.trade)}${v.skillLevel && v.skillLevel !== '—' ? ' (' + esc(v.skillLevel) + ')' : ''}</span>
            </div>
            <div style="font-size:0.85rem;color:#0f172a;">
              <strong>${v.hours.toFixed(1)} hrs</strong> &mdash; <strong style="color:#0284c7;">₱${v.pay.toFixed(2)}</strong>
            </div>
          </div>
          <table style="width:100%;border-collapse:collapse;font-size:0.8rem;">
            <thead>
              <tr style="background:#f8fafc;color:#475569;text-transform:uppercase;font-size:0.7rem;letter-spacing:0.04em;">
                <th style="padding:6px 8px;border:1px solid #cbd5e1;text-align:left;width:90px;">Date</th>
                <th style="padding:6px 8px;border:1px solid #cbd5e1;text-align:left;width:25%;">Area (Where)</th>
                <th style="padding:6px 8px;border:1px solid #cbd5e1;text-align:left;">Task / Work Done</th>
                <th style="padding:6px 8px;border:1px solid #cbd5e1;text-align:right;width:60px;">Hours</th>
                <th style="padding:6px 8px;border:1px solid #cbd5e1;text-align:right;width:80px;">Rate</th>
                <th style="padding:6px 8px;border:1px solid #cbd5e1;text-align:right;width:80px;">Pay</th>
              </tr>
            </thead>
            <tbody>${itemRows}</tbody>
          </table>
        </div>
      `;
    }).join('');

    const printHtml = `<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>Overtime Compensation Breakdown</title>
  <style>
    @page { size: auto; margin: 8mm 10mm; }
    body { font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; color: #0f172a; margin: 0; padding: 20px; line-height: 1.4; }
    table { word-wrap: break-word; overflow-wrap: break-word; }
  </style>
</head>
<body>
  <div class="print-container">
    <div style="display:flex;justify-content:space-between;align-items:flex-end;margin-bottom:16px;border-bottom:2px solid #0284c7;padding-bottom:10px;">
      <div>
        <h1 style="font-size:1.35rem;margin:0 0 4px 0;color:#0f172a;">Overtime Compensation Breakdown</h1>
        <p style="color:#64748b;margin:0;font-size:0.85rem;">Period: <strong>${esc(periodLabel)}</strong></p>
      </div>
      <div style="text-align:right;background:#f0f9ff;border:1px solid #bae6fd;padding:8px 16px;border-radius:6px;">
        <div style="font-size:0.7rem;text-transform:uppercase;color:#0369a1;font-weight:700;">Grand Total Compensation</div>
        <div style="font-size:1.3rem;font-weight:800;color:#0284c7;">₱${grand.toFixed(2)}</div>
      </div>
    </div>
    ${volSections}
    <div style="margin-top:36px;display:flex;justify-content:space-between;break-inside:avoid;page-break-inside:avoid;">
      <div style="text-align:center;"><div style="border-top:1px solid #334155;width:220px;padding-top:6px;font-size:0.78rem;color:#475569;">Prepared by / Date</div></div>
      <div style="text-align:center;"><div style="border-top:1px solid #334155;width:220px;padding-top:6px;font-size:0.78rem;color:#475569;">Approved by / Employer Signature</div></div>
    </div>
  </div>
</body>
</html>`;

    safePrint(printHtml);
  },

  async _printDailyBreakdown() {
    const filtered = this._getFilteredEntries();
    if (filtered.length === 0) { showToast('No entries in the selected date range to print.'); return; }

    const byDate = {};
    filtered.forEach(e => {
      const d = e.date || '(No Date)';
      if (!byDate[d]) byDate[d] = [];
      byDate[d].push(e);
    });

    const dateTables = Object.keys(byDate).sort().map(date => {
      const entries = byDate[date];
      const dateTotal = entries.reduce((sum, e) => sum + (Number(e.computedPay) || 0), 0);
      const rows = entries.map(e => `
        <tr>
          <td style="padding:6px 8px;border:1px solid #cbd5e1;">${esc(this._getVolunteerName(e.volunteerId))}</td>
          <td style="padding:6px 8px;border:1px solid #cbd5e1;">${esc(e.otArea || '-')}</td>
          <td style="padding:6px 8px;border:1px solid #cbd5e1;">${esc(e.otTask || '-')}</td>
          <td style="padding:6px 8px;border:1px solid #cbd5e1;text-align:right;">${Number(e.hours).toFixed(1)}</td>
          <td style="padding:6px 8px;border:1px solid #cbd5e1;text-align:right;">₱${Number(e.hourlyRate).toFixed(2)}</td>
          <td style="padding:6px 8px;border:1px solid #cbd5e1;text-align:right;font-weight:600;">₱${Number(e.computedPay).toFixed(2)}</td>
        </tr>
      `).join('');

      return `
        <div style="break-inside:avoid;page-break-inside:avoid;margin-bottom:16px;">
          <div style="background:#f1f5f9;padding:8px 12px;border-bottom:1px solid #cbd5e1;font-weight:600;color:#0f172a;">${esc(date)} : Total: <span style="color:#0284c7;">₱${dateTotal.toFixed(2)}</span></div>
          <table style="width:100%;border-collapse:collapse;font-size:0.8rem;">
            <thead>
              <tr style="background:#f8fafc;color:#475569;text-transform:uppercase;font-size:0.7rem;letter-spacing:0.04em;">
                <th style="padding:6px 8px;border:1px solid #cbd5e1;text-align:left;">Volunteer</th>
                <th style="padding:6px 8px;border:1px solid #cbd5e1;text-align:left;">Area</th>
                <th style="padding:6px 8px;border:1px solid #cbd5e1;text-align:left;">Task</th>
                <th style="padding:6px 8px;border:1px solid #cbd5e1;text-align:right;">Hours</th>
                <th style="padding:6px 8px;border:1px solid #cbd5e1;text-align:right;">Rate</th>
                <th style="padding:6px 8px;border:1px solid #cbd5e1;text-align:right;">Net Pay</th>
              </tr>
            </thead>
            <tbody>${rows}</tbody>
          </table>
        </div>
      `;
    }).join('');

    const grandTotal = filtered.reduce((sum, e) => sum + (Number(e.computedPay) || 0), 0);
    const range = this._getEntryDateRange();
    const periodLabel = (range.start || '...') + ' to ' + (range.end || '...');

    const printHtml = `<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>Daily OT Breakdown</title>
  <style>
    @page { size: auto; margin: 8mm 10mm; }
    body { font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; color: #0f172a; margin: 0; padding: 20px; line-height: 1.4; }
  </style>
</head>
<body>
  <div class="print-container">

    <div style="display:flex;justify-content:space-between;align-items:flex-end;margin-bottom:16px;border-bottom:2px solid #0284c7;padding-bottom:10px;">
      <div>
        <h1 style="font-size:1.35rem;margin:0 0 4px 0;color:#0f172a;">Daily Overtime Breakdown</h1>
        <p style="color:#64748b;margin:0;font-size:0.85rem;">Period: <strong>${esc(periodLabel)}</strong></p>
      </div>
      <div style="text-align:right;background:#f0f9ff;border:1px solid #bae6fd;padding:8px 16px;border-radius:6px;">
        <div style="font-size:0.7rem;text-transform:uppercase;color:#0369a1;font-weight:700;">Grand Total</div>
        <div style="font-size:1.3rem;font-weight:800;color:#0284c7;">₱${grandTotal.toFixed(2)}</div>
      </div>
    </div>
    ${dateTables}
    <div style="margin-top:36px;display:flex;justify-content:space-between;break-inside:avoid;page-break-inside:avoid;">
      <div style="text-align:center;"><div style="border-top:1px solid #334155;width:220px;padding-top:6px;font-size:0.78rem;color:#475569;">Prepared by / Date</div></div>
      <div style="text-align:center;"><div style="border-top:1px solid #334155;width:220px;padding-top:6px;font-size:0.78rem;color:#475569;">Approved by / Employer Signature</div></div>
    </div>
  </div>
</body>
</html>`;

    safePrint(printHtml);
  }
};

// verbatim: ot_calculator.js from // ---- EVENT WIRING ----
function wireOtEvents() {
// ---- EVENT WIRING ----

document.addEventListener('change', (e) => {
  if (e.target.id === 'ot-trade-filter' || e.target.id === 'ote-trade-filter') {
    if (window.OtCalcModule) {
      if (e.target.id === 'ote-trade-filter') {
        const otFilter = document.getElementById('ot-trade-filter');
        if (otFilter) otFilter.value = e.target.value;
      }
      const input = document.getElementById('ot-volunteer-search');
      if (input && input.value.trim()) {
        const evt = new Event('input', { bubbles: true });
        input.dispatchEvent(evt);
      }
      if (e.target.id === 'ote-trade-filter') window.OtCalcModule._renderPageTable();
    }
  }
  if (e.target.id === 'ot-range-start' || e.target.id === 'ot-range-end') {
    if (window.OtCalcModule) { window.OtCalcModule._selected.clear(); window.OtCalcModule._render(); }
  }
  if (e.target.id === 'ote-jump-date') {
    if (window.OtCalcModule) {
      const val = e.target.value;
      const start = document.getElementById('ote-range-start'); if (start) start.value = val;
      const end = document.getElementById('ote-range-end'); if (end) end.value = val;
      window.OtCalcModule._renderPageTable();
    }
  }
});

document.addEventListener('input', (e) => {
  if (e.target.classList.contains('ot-edit-date') ||
      e.target.classList.contains('ot-edit-hours') ||
      e.target.classList.contains('ot-edit-rate') ||
      e.target.classList.contains('ot-edit-area') ||
      e.target.classList.contains('ot-edit-task') ||
      e.target.classList.contains('ote-edit-hours') ||
      e.target.classList.contains('ote-edit-area') ||
      e.target.classList.contains('ote-edit-task')) {
    if (window.OtCalcModule) window.OtCalcModule._handleEdit(e);
  }
  // OT Encoder volunteer search
  if (e.target.id === 'ote-enc-volunteer-search' && window.OtCalcModule) {
    const q = e.target.value.trim();
    const results = document.getElementById('ote-enc-search-results');
    if (!results) return;
    if (q.length < 1) { results.style.display = 'none'; return; }
    const ql = q.toLowerCase();
    const ranked = (window.workersData || []).reduce((acc, w) => {
      const name = String(w.name || '').toLowerCase();
      if (name.startsWith(ql)) { acc.prefix.push(w); }
      else if (name.includes(ql)) { acc.substr.push(w); }
      return acc;
    }, { prefix: [], substr: [] });
    const matches = [...ranked.prefix, ...ranked.substr].slice(0, 10);
    if (matches.length === 0) {
      results.innerHTML = '<div style="padding:10px;color:var(--text-muted);font-size:0.78rem;text-align:center;">No matches</div>';
    } else {
      results.innerHTML = matches.map(w =>
        '<button type="button" class="ote-enc-search-result" data-worker-id="' + esc(w.aNumber) + '" style="display:flex;width:100%;padding:8px 10px;background:none;border:none;text-align:left;cursor:pointer;border-radius:4px;gap:8px;align-items:center;">' +
        '<span style="font-weight:600;font-size:0.82rem;">' + esc(w.name) + '</span>' +
        '<span style="font-size:0.7rem;color:var(--text-muted);">#' + esc(w.aNumber) + ' · ' + esc(w.trade || '-') + '</span>' +
        '</button>'
      ).join('');
    }
    results.style.display = '';
  }
  // OT Encoder pending inline edits
  if (e.target.classList.contains('ote-enc-pending-date') || e.target.classList.contains('ote-enc-pending-area') || e.target.classList.contains('ote-enc-pending-task')) {
    if (window.OtCalcModule) {
      const idx = parseInt(e.target.dataset.pendingIdx);
      if (!isNaN(idx) && window.OtCalcModule._encoderPending[idx]) {
        const entry = window.OtCalcModule._encoderPending[idx];
        if (e.target.classList.contains('ote-enc-pending-date')) entry.date = e.target.value;
        else if (e.target.classList.contains('ote-enc-pending-area')) entry.area = e.target.value;
        else if (e.target.classList.contains('ote-enc-pending-task')) entry.task = e.target.value;
      }
    }
  }
});

document.addEventListener('click', async (e) => {
  const tabBtn = e.target.closest('.ot-tab-btn');
  if (tabBtn) {
    const targetId = tabBtn.dataset.tab;
    document.querySelectorAll('.ot-tab-btn').forEach(b => b.classList.toggle('active', b === tabBtn));
    document.querySelectorAll('.ot-tab-content').forEach(c => c.classList.toggle('active', c.id === targetId));
    if (window.OtCalcModule) window.OtCalcModule._render();
    return;
  }
  const btn = e.target.closest('#btn-ot-add');
  if (btn && window.OtCalcModule) { window.guardClick(btn, () => window.OtCalcModule._addEntry())(); return; }
  const copyNextBtn = e.target.closest('#btn-ot-copy-next-day');
  if (copyNextBtn && window.OtCalcModule) { window.OtCalcModule._duplicateNextDay(); return; }
  const saveBtn = e.target.closest('#btn-ot-save');
  if (saveBtn && window.OtCalcModule) { window.guardClick(saveBtn, async () => { await window.OtCalcModule.save(); showToast('OT entries saved.'); })(); return; }
  const clearBtn = e.target.closest('#btn-ot-clear');
  if (clearBtn && window.OtCalcModule) { window.OtCalcModule._clearAll(); return; }
  const schedBtn = e.target.closest('#btn-ot-print-schedule') || e.target.closest('#btn-ot-print-schedule-area');
  if (schedBtn && window.OtCalcModule) { window.OtCalcModule._printOvertimeSchedule(); return; }
  const matrixBtn = e.target.closest('#btn-ot-print-matrix');
  if (matrixBtn && window.OtCalcModule) { window.OtCalcModule._printSummary(); return; }
  const dailyBtn = e.target.closest('#btn-ot-print-daily');
  if (dailyBtn && window.OtCalcModule) { window.OtCalcModule._printDailyBreakdown(); return; }

  if (e.target.closest('#btn-ot-load-yesterday') && window.OtCalcModule) { window.guardClick(e.target.closest('#btn-ot-load-yesterday'), () => window.OtCalcModule._loadYesterday())(); return; }

  // ---- OT Entries Page (ote-*) handlers ----
  if (e.target.closest('#ote-btn-add') && window.OtCalcModule) { window.guardClick(e.target.closest('#ote-btn-add'), () => window.OtCalcModule._addPageEntry())(); return; }
  if (e.target.closest('#ote-btn-save') && window.OtCalcModule) { window.guardClick(e.target.closest('#ote-btn-save'), async () => { await window.OtCalcModule.save(); showToast('OT entries saved.'); })(); return; }
  if (e.target.closest('#ote-btn-clear') && window.OtCalcModule) {
    if (window.OtCalcModule._selected.size > 0) window.OtCalcModule._selected.clear();
    else window.OtCalcModule._entries = [];
    window.OtCalcModule._renderPageTable();
    return;
  }
  if (e.target.closest('#ote-btn-open-encoder') && window.OtCalcModule) { window.OtCalcModule._openOtEncoder(); return; }
  if (e.target.closest('#ote-btn-print-schedule') && window.OtCalcModule) { window.OtCalcModule._printOvertimeSchedule(); return; }
  if (e.target.closest('#ote-btn-print-matrix') && window.OtCalcModule) { window.OtCalcModule._printSummary(); return; }

  // ---- OT Encoder modal close ----
  if (e.target.closest('#btn-close-ot-encoder')) {
    const modal = document.getElementById('ot-encoder-modal');
    if (modal) modal.classList.remove('active');
    return;
  }

  // ---- OT Encoder search result + pending management ----
  const encResult = e.target.closest('.ote-enc-search-result');
  if (encResult && window.OtCalcModule) {
    const wid = encResult.dataset.workerId;
    const w = (window.workersData || []).find(x => String(x.aNumber) === String(wid));
    if (!w) return;
    if (window.OtCalcModule._encoderPending.some(p => p.workerId === String(wid))) {
      showToast(esc(w.name) + ' is already in the list.');
      return;
    }
    const range = window.OtCalcModule._getOteEntryDateRange();
    const date = range.start || (typeof todayStr === 'function' ? todayStr() : new Date().toISOString().slice(0, 10));
    window.OtCalcModule._encoderPending.push({
      workerId: String(wid),
      name: w.name,
      trade: w.trade || '',
      date,
      area: w.area || '',
      task: w.specificTask || ''
    });
    document.getElementById('ote-enc-volunteer-search').value = '';
    document.getElementById('ote-enc-search-results').style.display = 'none';
    window.OtCalcModule._renderEncoderPending();
    return;
  }

  const encSaveBtn = e.target.closest('#btn-ote-enc-save-all');
  if (encSaveBtn && window.OtCalcModule) {
    window.guardClick(encSaveBtn, async () => {
      const pending = window.OtCalcModule._encoderPending;
      if (pending.length === 0) return;
      let count = 0;
      for (const p of pending) {
        const trade = p.trade || '';
        const skillLevel = (window.workersData || []).find(w => String(w.aNumber) === p.workerId)?.skillLevel || '';
        const hourlyRate = parseFloat((window.workersData || []).find(w => String(w.aNumber) === p.workerId)?.hourlyRate) || 0;
        window.OtCalcModule._entries.push({
          id: 'ot_' + Date.now() + Math.random().toString(36).slice(2, 6) + count,
          volunteerId: p.workerId, trade, skillLevel,
          date: p.date, hours: 2, otArea: p.area, otTask: p.task,
          hourlyRate, computedPay: Math.round(2 * hourlyRate * 100) / 100,
          status: 'Planned', createdAt: new Date().toISOString()
        });
        count++;
      }
      await window.OtCalcModule.save();
      window.OtCalcModule._renderPageTable();
      showToast('Added ' + count + ' OT entr' + (count === 1 ? 'y' : 'ies') + '.');
      window.OtCalcModule._encoderPending = [];
      window.OtCalcModule._renderEncoderPending();
      document.getElementById('ote-enc-volunteer-search').focus();
    })();
    return;
  }

  if (e.target.closest('.ote-enc-pending-remove') && window.OtCalcModule) {
    const idx = parseInt(e.target.dataset.pendingIdx);
    if (!isNaN(idx) && idx >= 0 && idx < window.OtCalcModule._encoderPending.length) {
      window.OtCalcModule._encoderPending.splice(idx, 1);
      window.OtCalcModule._renderEncoderPending();
    }
    return;
  }

  // Date nav
  if (e.target.closest('#ote-prev-day') && window.OtCalcModule) { window.OtCalcModule._stepDatePage(-1); return; }
  if (e.target.closest('#ote-today') && window.OtCalcModule) { window.OtCalcModule._goToTodayPage(); return; }
  if (e.target.closest('#ote-next-day') && window.OtCalcModule) { window.OtCalcModule._stepDatePage(1); return; }
});
}
return {
    OtCalcModule: OtCalcModule,
    wireOtEvents: wireOtEvents
};
})();

// ===== project_groups.js - verbatim from 01_Rust_WC_DB/js/project_groups.js =====
var S5_project_groups = (function () {

const roleRank = {
  'Project Overseer': 1,
  'Project Leader': 2,
  'Asst. Project Leader': 3,
  'Member': 4
};

var ProjectGroupsModule = {
  _groups: [],
  _selectedAreaFilter: 'all',
  _searchQuery: '',
  _viewMode: 'grouped',
  _groups: [],
  _allAreas: [],
  _editOriginalName: null,

  init() {
    this._loadGroups();
    this._renderLayout();
  },

  getKnownAreas() {
    const set = new Set(['Station 1', 'CC', 'House 1', 'House 2', 'Tower 1', 'Workshop', 'Dining Hall', 'Compound', 'Main Building', 'Warehouse']);
    if (typeof workersData !== 'undefined' && Array.isArray(workersData)) {
      workersData.forEach(w => {
        if (!w || w.isArchived) return;
        const a = (w.area || '').trim();
        if (a) set.add(a);
      });
    }
    return Array.from(set).sort();
  },

  getCustomAreaMeta() {
    try {
      const meta = localStorage.getItem('wc_project_groups_meta');
      return meta ? JSON.parse(meta) : {};
    } catch (e) {
      return {};
    }
  },

  async setCustomAreaMeta(groupName, area) {
    try {
      const meta = this.getCustomAreaMeta();
      if (area !== null && area !== undefined) {
        meta[groupName] = (typeof area === 'string') ? area.trim() : '';
      } else {
        delete meta[groupName];
      }
      localStorage.setItem('wc_project_groups_meta', JSON.stringify(meta));
      if (window._storage && typeof window._storage.set === 'function') {
        await window._storage.set('wc_project_groups_meta', meta);
      }
    } catch (e) {
      console.warn('Failed to save project group meta:', e);
    }
  },

  extractGroupArea(groupName, workersInGroup = []) {
    if (!groupName) return 'General';
    const cleanName = groupName.trim();

    // 1. Check custom manual override
    const custom = this.getCustomAreaMeta();
    if (custom && custom[cleanName]) return custom[cleanName];

    // 2. Check standard delimiters: " | ", " - ", " / ", ": ", " : "
    const delimMatch = cleanName.match(/^([^|\-\/:—]+?)\s*(?:\||\/|\-|:|—)\s*(.+)$/);
    if (delimMatch && delimMatch[1].trim()) {
      return delimMatch[1].trim();
    }

    // 3. Match known areas prefix (longest match first)
    const known = this.getKnownAreas().sort((a, b) => b.length - a.length);
    for (const a of known) {
      if (cleanName.toLowerCase().startsWith(a.toLowerCase())) {
        return a;
      }
    }

    // 4. Check workers assigned to this group for majority area
    if (workersInGroup && workersInGroup.length > 0) {
      const counts = {};
      workersInGroup.forEach(w => {
        const a = (w.area || '').trim();
        if (a) counts[a] = (counts[a] || 0) + 1;
      });
      const sorted = Object.entries(counts).sort((a, b) => b[1] - a[1]);
      if (sorted.length > 0 && sorted[0][1] > 0) {
        return sorted[0][0];
      }
    }

    return 'General';
  },

  setAreaFilter(area) {
    this._selectedAreaFilter = area;
    this._renderLayout();
  },

  setViewMode(mode) {
    this._viewMode = mode;
    this._renderLayout();
  },

  setSearchQuery(q) {
    this._searchQuery = q || '';
    this._renderLayout();
  },

  _loadGroups() {
    const map = {};
    const meta = this.getCustomAreaMeta();
    if (meta && typeof meta === 'object') {
      Object.keys(meta).forEach(name => {
        if (name && name.trim()) map[name.trim()] = [];
      });
    }

    workersData.forEach(w => {
      if (!w || w.isArchived) return;
      if (!w.projectGroup) return;
      const grp = w.projectGroup.trim();
      if (!map[grp]) map[grp] = [];
      map[grp].push(w);
    });
    this._groups = Object.entries(map).map(([name, workers]) => {
      const overseer = workers.find(w => w.projectRole === 'Project Overseer') || null;
      const leader = workers.find(w => w.projectRole === 'Project Leader') || null;
      const asstLeader = workers.find(w => w.projectRole === 'Asst. Project Leader') || null;
      const members = workers.filter(w =>
        w.projectRole !== 'Project Overseer' &&
        w.projectRole !== 'Project Leader' &&
        w.projectRole !== 'Asst. Project Leader'
      );
      const area = this.extractGroupArea(name, workers);
      return { name, area, overseer, leader, asstLeader, members, total: workers.length };
    }).sort((a, b) => a.name.localeCompare(b.name));

    // Compute distinct areas and stats
    const areaCounts = {};
    this._groups.forEach(g => {
      if (!areaCounts[g.area]) areaCounts[g.area] = { name: g.area, groupCount: 0, volunteerCount: 0 };
      areaCounts[g.area].groupCount++;
      areaCounts[g.area].volunteerCount += g.total;
    });
    this._allAreas = Object.values(areaCounts).sort((a, b) => a.name.localeCompare(b.name));
  },

  _renderLayout() {
    const container = document.getElementById('project-groups-container');
    if (!container) return;

    // 1. Populate stats bar
    const statsBar = document.getElementById('pg-stats-bar');
    if (statsBar) {
      const totalGroups = this._groups.length;
      const totalAreas = this._allAreas.length;
      const totalVols = this._groups.reduce((sum, g) => sum + g.total, 0);
      statsBar.innerHTML = `
        <div style="padding: 12px 20px; width: 100%; display: flex; gap: 32px; align-items: center; justify-content: flex-start; flex-wrap: wrap; background: var(--card-bg); border: 1px solid var(--border-color); border-radius: var(--radius-md); box-shadow: 0 1px 3px rgba(0,0,0,0.03);">
          <div>
            <span style="font-size: 0.68rem; color: var(--text-muted); font-weight: 700; text-transform: uppercase; letter-spacing: 0.5px;">Total Groups</span>
            <div style="font-size: 1.25rem; font-weight: 800; color: var(--primary-color);">${totalGroups}</div>
          </div>
          <div style="width: 1px; height: 30px; background: var(--border-color);"></div>
          <div>
            <span style="font-size: 0.68rem; color: var(--text-muted); font-weight: 700; text-transform: uppercase; letter-spacing: 0.5px;">Active Project Areas</span>
            <div style="font-size: 1.25rem; font-weight: 800; color: #10b981;">${totalAreas}</div>
          </div>
          <div style="width: 1px; height: 30px; background: var(--border-color);"></div>
          <div>
            <span style="font-size: 0.68rem; color: var(--text-muted); font-weight: 700; text-transform: uppercase; letter-spacing: 0.5px;">Total Assigned Workforce</span>
            <div style="font-size: 1.25rem; font-weight: 800; color: var(--text-main);">${totalVols} <span style="font-size: 0.75rem; font-weight: 600; color: var(--text-muted);">volunteers</span></div>
          </div>
        </div>`;
    }

    // 2. Populate known areas datalist
    const areasList = document.getElementById('pg-known-areas-list');
    if (areasList) {
      areasList.innerHTML = this.getKnownAreas().map(a => `<option value="${esc(a)}">`).join('');
    }

    // 3. Render Area Filter Pills
    const pillsContainer = document.getElementById('pg-area-pills');
    if (pillsContainer) {
      const isAllActive = this._selectedAreaFilter === 'all';
      let pillsHtml = `<button type="button" class="btn btn-xs ${isAllActive ? 'btn-primary' : 'btn-secondary'}" onclick="window.ProjectGroupsModule.setAreaFilter('all')" style="font-size: 0.72rem; font-weight: 700; border-radius: 20px; padding: 4px 10px;">All (${this._groups.length})</button>`;
      this._allAreas.forEach(a => {
        const isActive = this._selectedAreaFilter === a.name;
        pillsHtml += `<button type="button" class="btn btn-xs ${isActive ? 'btn-primary' : 'btn-secondary'}" onclick="window.ProjectGroupsModule.setAreaFilter('${esc(a.name)}')" style="font-size: 0.72rem; font-weight: 700; border-radius: 20px; padding: 4px 10px;">📍 ${esc(a.name)} (${a.groupCount})</button>`;
      });
      pillsContainer.innerHTML = pillsHtml;
    }

    // 4. Update View Mode Buttons
    const btnGrouped = document.getElementById('btn-pg-view-grouped');
    const btnGrid = document.getElementById('btn-pg-view-grid');
    if (btnGrouped && btnGrid) {
      btnGrouped.className = `btn btn-xs ${this._viewMode === 'grouped' ? 'btn-primary' : 'btn-secondary'}`;
      btnGrid.className = `btn btn-xs ${this._viewMode === 'grid' ? 'btn-primary' : 'btn-secondary'}`;
      btnGrouped.onclick = () => this.setViewMode('grouped');
      btnGrid.onclick = () => this.setViewMode('grid');
    }

    // 5. Empty State
    if (this._groups.length === 0) {
      container.innerHTML = `
        <div style="display:flex;flex-direction:column;align-items:center;justify-content:center;padding:80px 40px;text-align:center;color:var(--text-muted);">
          <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" style="margin-bottom:16px;opacity:0.4;"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"></path><circle cx="9" cy="7" r="4"></circle><path d="M23 21v-2a4 4 0 0 0-3-3.87"></path><path d="M16 3.13a4 4 0 0 1 0 7.75"></path></svg>
          <p style="font-size:1rem;font-weight:600;margin:0;">No Project Groups Yet</p>
          <p style="font-size:0.85rem;margin:8px 0 20px;">Create a group to organize volunteers by project.</p>
          <button class="btn btn-primary" id="btn-pg-create-empty" style="font-weight:700;">+ Create Project Group</button>
        </div>`;
      const btn = container.querySelector('#btn-pg-create-empty');
      if (btn) btn.addEventListener('click', () => this._openEditModal(null));
      return;
    }

    // 6. Filter groups by Area and Search Query
    let filtered = this._groups;
    if (this._selectedAreaFilter !== 'all') {
      filtered = filtered.filter(g => g.area === this._selectedAreaFilter);
    }
    if (this._searchQuery) {
      const q = this._searchQuery.toLowerCase();
      filtered = filtered.filter(g => {
        return g.name.toLowerCase().includes(q) ||
          g.area.toLowerCase().includes(q) ||
          (g.overseer && g.overseer.name.toLowerCase().includes(q)) ||
          (g.leader && g.leader.name.toLowerCase().includes(q)) ||
          (g.asstLeader && g.asstLeader.name.toLowerCase().includes(q)) ||
          g.members.some(m => m.name.toLowerCase().includes(q) || String(m.aNumber).includes(q));
      });
    }

    let cardsHtml = '';
    if (filtered.length === 0) {
      cardsHtml = `
        <div style="padding: 48px 24px; text-align: center; color: var(--text-muted); background: var(--card-bg); border: 1px dashed var(--border-color); border-radius: 8px;">
          <p style="font-size: 0.95rem; font-weight: 600; margin: 0 0 8px;">No project groups match the selected filter</p>
          <button type="button" class="btn btn-secondary btn-xs" onclick="window.ProjectGroupsModule.setAreaFilter('all'); document.getElementById('pg-search-input').value=''; window.ProjectGroupsModule.setSearchQuery('');">Clear Filters</button>
        </div>`;
    } else if (this._viewMode === 'grouped') {
      // Group by Area Section
      const groupsByArea = {};
      filtered.forEach(g => {
        if (!groupsByArea[g.area]) groupsByArea[g.area] = [];
        groupsByArea[g.area].push(g);
      });
      cardsHtml = Object.entries(groupsByArea).sort((a, b) => a[0].localeCompare(b[0])).map(([area, grps]) => {
        const areaVols = grps.reduce((s, x) => s + x.total, 0);
        return `
        <div class="pg-area-section" style="margin-bottom: 28px;">
          <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px; padding-bottom: 6px; border-bottom: 2px solid var(--border-color);">
            <div style="display: flex; align-items: center; gap: 8px;">
              <span style="font-size: 1.15rem;">📍</span>
              <h3 style="margin: 0; font-size: 1rem; font-weight: 800; color: var(--text-main);">${esc(area)}</h3>
              <span style="font-size: 0.72rem; background: var(--bg-color); border: 1px solid var(--border-color); padding: 2px 8px; border-radius: 12px; font-weight: 700; color: var(--text-muted);">${grps.length} Group${grps.length !== 1 ? 's' : ''} • ${areaVols} Volunteers</span>
            </div>
          </div>
          <div style="display: grid; grid-template-columns: repeat(auto-fill, minmax(380px, 1fr)); gap: 20px; align-content: start;">
            ${grps.map(g => this._buildGroupCard(g)).join('')}
          </div>
        </div>`;
      }).join('');
    } else {
      // Flat Grid View
      cardsHtml = `
        <div style="display: grid; grid-template-columns: repeat(auto-fill, minmax(380px, 1fr)); gap: 20px; align-content: start;">
          ${filtered.map(g => this._buildGroupCard(g)).join('')}
        </div>`;
    }

    container.innerHTML =
      cardsHtml +
      '<div style="margin-top:32px;border-top:1px solid var(--border-color,#e2e8f0);padding-top:16px;">' +
        '<div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:12px;">' +
          '<button id="btn-pg-toggle-changelog" style="display:flex;align-items:center;gap:6px;padding:6px 14px;border:1px solid var(--border-color,#cbd5e1);border-radius:8px;background:var(--card-bg,#fff);cursor:pointer;font-size:0.8rem;font-weight:600;color:var(--text-main);">' +
            '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"></circle><polyline points="12 6 12 12 16 14"></polyline></svg>' +
            ' Change Log' +
          '</button>' +
          '<button id="btn-pg-print-changelog" style="display:flex;align-items:center;gap:6px;padding:6px 14px;border:1px solid var(--border-color,#cbd5e1);border-radius:8px;background:var(--card-bg,#fff);cursor:pointer;font-size:0.8rem;font-weight:600;color:var(--text-muted,#64748b);">' +
            '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="6 9 6 2 18 2 18 9"></polyline><path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"></path><rect x="6" y="14" width="12" height="8"></rect></svg>' +
            ' Print' +
          '</button>' +
        '</div>' +
        '<div id="pg-change-log-body" style="min-height:60px;"></div>' +
      '</div>';
  },

  _buildGroupCard(g) {
    const allSorted = [g.overseer, g.leader, g.asstLeader].filter(Boolean)
      .concat(g.members.slice().sort((a, b) => a.name.localeCompare(b.name)));

    const renderWorker = (w, label) => {
      const photo = w.photoUrl || w.photo;
      const avatar = photo
        ? `<img src="${esc(photo)}" style="width:28px;height:28px;border-radius:50%;object-fit:cover;flex-shrink:0;">`
        : `<span style="width:28px;height:28px;border-radius:50%;background:var(--primary-light);color:var(--primary-color);display:flex;align-items:center;justify-content:center;font-weight:700;font-size:0.7rem;flex-shrink:0;">${esc((w.name||'?')[0])}</span>`;
      const roleBadge = label ? `<span style="font-size:0.6rem;font-weight:700;color:#3b82f6;background:#eff6ff;padding:1px 6px;border-radius:3px;margin-left:6px;white-space:nowrap;">${esc(label)}</span>` : '';
      return `<div style="display:flex;align-items:center;gap:8px;padding:4px 0;">
        ${avatar}
        <div style="flex:1;min-width:0;">
          <span style="font-weight:600;font-size:0.8rem;color:var(--text-main);">${esc(w.name)}</span>
          <span style="font-size:0.65rem;color:var(--text-muted);margin-left:4px;">#${esc(w.aNumber)}</span>
          ${roleBadge}
        </div>
        <span style="font-size:0.65rem;color:var(--text-muted);white-space:nowrap;">${esc(w.trade||'')}</span>
      </div>`;
    };

    let assignedSigName = '';
    if (typeof signatoriesData !== 'undefined' && Array.isArray(signatoriesData)) {
      const match = signatoriesData.find(s => (s.projectGroups || s.trades || []).includes(g.name));
      if (match) assignedSigName = match.name;
    }

    return `<div style="background:var(--card-bg);border:1px solid var(--border-color);border-radius:12px;overflow:hidden;box-shadow:0 1px 3px rgba(0,0,0,0.04);">
      <div style="padding:16px;border-bottom:1px solid var(--border-color);background:#f8fafc;">
        <div style="display:flex;justify-content:space-between;align-items:flex-start;">
          <div>
            <div style="display: flex; align-items: center; gap: 6px; margin-bottom: 4px;">
              <span class="pg-area-badge" data-area="${esc(g.area)}" title="Click to filter by ${esc(g.area)}" style="display: inline-flex; align-items: center; gap: 4px; font-size: 0.68rem; font-weight: 700; background: rgba(30, 64, 175, 0.08); color: var(--primary-color); border: 1px solid rgba(30, 64, 175, 0.18); padding: 2px 7px; border-radius: 4px; cursor: pointer;" onclick="window.ProjectGroupsModule.setAreaFilter('${esc(g.area)}')">
                📍 Area: ${esc(g.area)}
              </span>
            </div>
            <h3 style="margin:0;font-size:0.95rem;font-weight:800;color:var(--text-main);">${esc(g.name)}</h3>
            <div style="display:flex;align-items:center;gap:8px;margin-top:4px;">
              <span style="font-size:0.75rem;color:var(--text-muted);">${g.total} member${g.total !== 1 ? 's' : ''}</span>
              <span style="font-size:0.68rem;font-weight:600;color:${assignedSigName ? '#2563eb' : 'var(--text-muted)'};background:${assignedSigName ? '#eff6ff' : 'var(--bg-color)'};padding:1px 6px;border-radius:4px;border:1px solid ${assignedSigName ? '#bfdbfe' : 'var(--border-color)'};cursor:pointer;" title="Click to manage group signatories" onclick="if(typeof openBulkAssignGroupsModal==='function')openBulkAssignGroupsModal();">
                Signatory: ${esc(assignedSigName || 'Default')}
              </span>
            </div>
          </div>
          <div style="display:flex;gap:4px;">
            <button class="pg-card-btn pg-card-btn-reassign" data-group="${esc(g.name)}" title="Reassign Members from this Group" style="display:inline-flex;align-items:center;justify-content:center;width:30px;height:30px;border:1px solid var(--border-color);border-radius:6px;background:var(--card-bg);cursor:pointer;color:var(--primary-color);"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M16 3h5v5"></path><path d="M4 20L21 3"></path><path d="M21 16v5h-5"></path><path d="M15 15l6 6"></path><path d="M4 4l5 5"></path></svg></button>
            <button class="pg-card-btn pg-card-btn-print" data-group="${esc(g.name)}" title="Print Group" style="display:inline-flex;align-items:center;justify-content:center;width:30px;height:30px;border:1px solid var(--border-color);border-radius:6px;background:var(--card-bg);cursor:pointer;color:var(--text-muted);"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="6 9 6 2 18 2 18 9"></polyline><path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"></path><rect x="6" y="14" width="12" height="8"></rect></svg></button>
            <button class="pg-card-btn pg-card-btn-edit" data-group="${esc(g.name)}" title="Edit Group" style="display:inline-flex;align-items:center;justify-content:center;width:30px;height:30px;border:1px solid var(--border-color);border-radius:6px;background:var(--card-bg);cursor:pointer;color:var(--text-muted);"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path></svg></button>
            <button class="pg-card-btn pg-card-btn-del" data-group="${esc(g.name)}" title="Delete Group" style="display:inline-flex;align-items:center;justify-content:center;width:30px;height:30px;border:1px solid #fecaca;border-radius:6px;background:#fef2f2;cursor:pointer;color:#dc2626;"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg></button>
          </div>
        </div>
      </div>
      <div style="padding:12px 16px;">
        ${allSorted.length > 0 ? allSorted.map(w => {
          const label = w === g.overseer ? 'Overseer' : w === g.leader ? 'Leader' : w === g.asstLeader ? 'Asst. Lead' : null;
          return renderWorker(w, label);
        }).join('') : '<div style="color:var(--text-muted);font-size:0.75rem;font-style:italic;padding:8px 0;text-align:center;">No members assigned yet</div>'}
      </div>
    </div>`;
  },

  async _logGroupChange(worker, fromGroup, toGroup, reason) {
    if (fromGroup === toGroup && fromGroup !== '') return;
    var store = window.groupChangeStore;
    if (!store) return;
    var date = typeof todayStr === 'function' ? todayStr() : new Date().toISOString().slice(0, 10);
    var rec = {
      id: safeUUID(),
      date: date,
      timestamp: new Date().toISOString(),
      workerId: String(worker.aNumber),
      workerName: worker.name,
      trade: worker.trade || '',
      fromGroup: fromGroup || '(none)',
      toGroup: toGroup || '(none)',
      reason: reason || ''
    };
    var existing = await store.get(date) || [];
    existing.push(rec);
    await store.set(date, existing);
  },

  // ------ Edit / Create Modal ------

  _openEditModal(groupName) {
    const modal = document.getElementById('pg-edit-modal');
    if (!modal) return;

    this._editOriginalName = groupName;
    const isNew = !groupName;
    document.getElementById('pg-edit-title').textContent = isNew ? 'Create Project Group' : 'Edit Project Group';
    document.getElementById('btn-save-pg-edit').textContent = isNew ? 'Create Group' : 'Save Changes';
    const reasonInput = document.getElementById('pg-transfer-reason');
    if (reasonInput) reasonInput.value = '';

    const nameInput = document.getElementById('pg-edit-name');
    const areaInput = document.getElementById('pg-edit-area');
    const ovInput = document.getElementById('pg-overseer-input');
    const leaderInput = document.getElementById('pg-leader-input');
    const asstInput = document.getElementById('pg-asst-input');
    const memInput = document.getElementById('pg-members-input');

    if (isNew) {
      nameInput.value = '';
      if (areaInput) { areaInput.value = ''; areaInput.dataset.autoFilled = '0'; }
      ovInput.value = ''; document.getElementById('pg-overseer-chip').innerHTML = '';
      leaderInput.value = ''; document.getElementById('pg-leader-chip').innerHTML = '';
      asstInput.value = ''; document.getElementById('pg-asst-chip').innerHTML = '';
      document.getElementById('pg-member-chips').innerHTML = '';
      this._editSelected = { overseer: null, leader: null, asst: null, members: new Set() };
    } else {
      const g = this._groups.find(x => x.name === groupName);
      if (!g) return;
      nameInput.value = g.name;
      if (areaInput) {
        areaInput.value = (g && g.area) ? g.area : '';
        areaInput.dataset.autoFilled = '0';
      }
      this._editSelected = {
        overseer: g.overseer ? String(g.overseer.aNumber) : null,
        leader: g.leader ? String(g.leader.aNumber) : null,
        asst: g.asstLeader ? String(g.asstLeader.aNumber) : null,
        members: new Set(g.members.map(w => String(w.aNumber)))
      };
      ovInput.value = g.overseer ? g.overseer.name : '';
      leaderInput.value = g.leader ? g.leader.name : '';
      asstInput.value = g.asstLeader ? g.asstLeader.name : '';
      this._renderRoleChip('pg-overseer-chip', this._editSelected.overseer);
      this._renderRoleChip('pg-leader-chip', this._editSelected.leader);
      this._renderRoleChip('pg-asst-chip', this._editSelected.asst);
      this._renderMemberChips();
    }

    if (window.trapFocus) window.trapFocus(modal);
    modal.classList.add('active');
  },

  _renderRoleChip(containerId, workerId) {
    const container = document.getElementById(containerId);
    if (!container) return;
    if (!workerId) { container.innerHTML = ''; return; }
    const w = workersData.find(x => String(x.aNumber) === workerId);
    if (!w) { container.innerHTML = ''; return; }
    const photo = w.photoUrl || w.photo;
    const avatar = photo
      ? `<img src="${esc(photo)}" style="width:22px;height:22px;border-radius:50%;object-fit:cover;">`
      : `<span style="width:22px;height:22px;border-radius:50%;background:var(--primary-light);color:var(--primary-color);display:flex;align-items:center;justify-content:center;font-weight:700;font-size:0.6rem;">${esc((w.name||'?')[0])}</span>`;
    container.innerHTML = `<span style="display:inline-flex;align-items:center;gap:4px;padding:4px 8px;background:#dbeafe;border:1px solid #bfdbfe;border-radius:4px;font-size:0.78rem;margin-top:4px;">
      ${avatar}<span style="font-weight:600;">${esc(w.name)}</span> <span style="color:var(--text-muted);font-size:0.7rem;">#${esc(w.aNumber)}</span>
      <button type="button" class="pg-role-clear" data-target="${containerId.replace('-chip','-input')}" data-field="${containerId.replace('pg-','').replace('-chip','')}" style="background:none;border:none;color:#94a3b8;cursor:pointer;font-size:1rem;line-height:1;padding:0;margin-left:2px;">&times;</button>
    </span>`;
    container.querySelector('.pg-role-clear')?.addEventListener('click', (e) => {
      document.getElementById(e.target.dataset.target).value = '';
      this._editSelected[e.target.dataset.field] = null;
      container.innerHTML = '';
    });
  },

  _renderMemberChips() {
    const container = document.getElementById('pg-member-chips');
    if (!container) return;
    if (!this._editSelected || !this._editSelected.members.size) { container.innerHTML = ''; return; }
    container.innerHTML = [...this._editSelected.members].map(id => {
      const w = workersData.find(x => String(x.aNumber) === id);
      if (!w) return '';
      return `<span style="display:inline-flex;align-items:center;gap:4px;padding:4px 8px;background:#f1f5f9;border:1px solid #e2e8f0;border-radius:4px;font-size:0.78rem;margin:2px;">
        <span style="font-weight:600;">${esc(w.name)}</span><span style="color:var(--text-muted);font-size:0.7rem;">#${esc(w.aNumber)}</span>
        <button type="button" class="pg-member-remove" data-worker-id="${esc(id)}" style="background:none;border:none;color:#94a3b8;cursor:pointer;font-size:1rem;line-height:1;padding:0;">&times;</button>
      </span>`;
    }).join('');
    container.querySelectorAll('.pg-member-remove').forEach(btn => {
      btn.addEventListener('click', () => {
        this._editSelected.members.delete(btn.dataset.workerId);
        this._renderMemberChips();
      });
    });
  },

  _initAutocomplete(inputId, resultsId, onSelect) {
    const input = document.getElementById(inputId);
    const results = document.getElementById(resultsId);
    if (!input || !results) return;

    const doSearch = debounce(() => {
      const q = input.value.trim().toLowerCase();
      if (q.length < 1) { results.style.display = 'none'; return; }
      const matches = (workersData || []).filter(w =>
        !w.isArchived && ((w.name || '').toLowerCase().includes(q) || String(w.aNumber || '').includes(q))
      ).slice(0, 10);
      if (matches.length === 0) {
        results.innerHTML = '<div style="padding:12px;color:var(--text-muted);font-size:0.82rem;text-align:center;">No matches</div>';
      } else {
        results.innerHTML = matches.map(w => {
          const photo = w.photoUrl || w.photo;
          const avatar = photo
            ? `<img src="${esc(photo)}" style="width:28px;height:28px;border-radius:50%;object-fit:cover;">`
            : `<span style="width:28px;height:28px;border-radius:50%;background:var(--primary-light);color:var(--primary-color);display:flex;align-items:center;justify-content:center;font-weight:700;font-size:0.75rem;">${esc((w.name||'?')[0])}</span>`;
          return `<button type="button" class="pg-worker-item" data-worker-id="${esc(w.aNumber)}" style="display:flex;align-items:center;gap:10px;width:100%;padding:8px 10px;background:none;border:none;text-align:left;cursor:pointer;border-radius:6px;">${avatar}<div style="flex:1;min-width:0;"><strong style="font-size:0.82rem;">${esc(w.name)}</strong><span style="font-size:0.7rem;color:var(--text-muted);">#${esc(w.aNumber)} &middot; ${esc(w.trade||'\u2014')}</span></div></button>`;
        }).join('');
        results.querySelectorAll('.pg-worker-item').forEach(item => {
          item.addEventListener('click', () => onSelect(item.dataset.workerId));
        });
      }
      results.style.display = 'block';
    }, 200);

    input.addEventListener('input', doSearch);
    input.addEventListener('focus', () => { if (input.value.trim()) doSearch(); });
  },

  _setupEditModalListeners() {
    // Role single-select autocomplete
    this._initAutocomplete('pg-overseer-input', 'pg-overseer-results', (id) => {
      this._editSelected.overseer = id;
      const w = workersData.find(x => String(x.aNumber) === id);
      if (w) document.getElementById('pg-overseer-input').value = w.name;
      document.getElementById('pg-overseer-results').style.display = 'none';
      this._renderRoleChip('pg-overseer-chip', id);
    });
    this._initAutocomplete('pg-leader-input', 'pg-leader-results', (id) => {
      this._editSelected.leader = id;
      const w = workersData.find(x => String(x.aNumber) === id);
      if (w) document.getElementById('pg-leader-input').value = w.name;
      document.getElementById('pg-leader-results').style.display = 'none';
      this._renderRoleChip('pg-leader-chip', id);
    });
    this._initAutocomplete('pg-asst-input', 'pg-asst-results', (id) => {
      this._editSelected.asst = id;
      const w = workersData.find(x => String(x.aNumber) === id);
      if (w) document.getElementById('pg-asst-input').value = w.name;
      document.getElementById('pg-asst-results').style.display = 'none';
      this._renderRoleChip('pg-asst-chip', id);
    });
    // Multi-select members autocomplete
    this._initAutocomplete('pg-members-input', 'pg-members-results', (id) => {
      if (this._editSelected.members.has(id)) return;
      this._editSelected.members.add(id);
      document.getElementById('pg-members-input').value = '';
      document.getElementById('pg-members-results').style.display = 'none';
      this._renderMemberChips();
    });

    // Auto-detect area from name delimiters in edit modal
    const nameInput = document.getElementById('pg-edit-name');
    const areaInput = document.getElementById('pg-edit-area');
    if (nameInput && areaInput) {
      nameInput.addEventListener('input', () => {
        if (areaInput.dataset.manuallyEdited === '1') return;
        const val = nameInput.value.trim();
        if (!val) return;
        const extracted = this.extractGroupArea(val);
        if (extracted && extracted !== 'General') {
          areaInput.value = extracted;
        }
      });
      areaInput.addEventListener('input', () => {
        areaInput.dataset.manuallyEdited = '1';
      });
    }

    // Save handler
    document.getElementById('btn-save-pg-edit')?.addEventListener('click', () => this._saveEdit());

    // Close handlers (using delegation to avoid dupes)
  },

  async _saveEdit() {
    const name = document.getElementById('pg-edit-name').value.trim();
    if (!name) { showToast('Enter a group name.'); return; }

    const oldName = this._editOriginalName;
    const sel = this._editSelected;
    const nowIso = window.DateService ? window.DateService.getNowTimestamp() : new Date().toISOString();

    // Collect all worker IDs involved
    const allIds = new Set([sel.overseer, sel.leader, sel.asst].filter(Boolean));
    sel.members.forEach(id => allIds.add(id));

    // Snapshot current group assignment before any changes
    const prevGroups = {};
    allIds.forEach(id => {
      const w = workersData.find(x => String(x.aNumber) === id);
      if (w) prevGroups[id] = w.projectGroup || '';
    });

    // Detect removed members from existing group
    const removedMembers = [];
    if (oldName) {
      workersData.forEach(w => {
        if (w.projectGroup === oldName && !allIds.has(String(w.aNumber))) {
          removedMembers.push(w);
          w.projectGroup = '';
          w.projectRole = 'Member';
          if (!w.assignment) w.assignment = {};
          w.assignment.projectGroup = '';
          w.assignment.projectRole = 'Member';
          w.assignment.updatedAt = nowIso;
          w.updatedAt = nowIso;
        }
      });
    }

    // Assign new group to all selected members and bump timestamps
    allIds.forEach(id => {
      const w = workersData.find(x => String(x.aNumber) === id);
      if (!w) return;
      w.projectGroup = name;
      if (!w.assignment) w.assignment = {};
      w.assignment.projectGroup = name;
      w.assignment.updatedAt = nowIso;
      w.updatedAt = nowIso;
    });

    // Assign roles with matching sub-document & timestamps
    const assignRole = (id, role) => {
      if (!id) return;
      const w = workersData.find(x => String(x.aNumber) === id);
      if (w) {
        w.projectRole = role;
        if (!w.assignment) w.assignment = {};
        w.assignment.projectRole = role;
        w.assignment.updatedAt = nowIso;
        w.updatedAt = nowIso;
      }
    };
    assignRole(sel.overseer, 'Project Overseer');
    assignRole(sel.leader, 'Project Leader');
    assignRole(sel.asst, 'Asst. Project Leader');

    // Set role to 'Member' for all other workers in this group
    allIds.forEach(id => {
      if (id !== sel.overseer && id !== sel.leader && id !== sel.asst) {
        const w = workersData.find(x => String(x.aNumber) === id);
        if (w) {
          w.projectRole = 'Member';
          if (!w.assignment) w.assignment = {};
          w.assignment.projectRole = 'Member';
          w.assignment.updatedAt = nowIso;
          w.updatedAt = nowIso;
        }
      }
    });

    // Require reason if any group assignment changed or members removed
    var hasChanges = removedMembers.length > 0;
    allIds.forEach(function (id) {
      if (prevGroups[id] && prevGroups[id] !== name) hasChanges = true;
    });
    if (oldName && oldName !== name) hasChanges = true;
    var transferReason = document.getElementById('pg-transfer-reason')?.value.trim() || '';
    if (hasChanges && !transferReason) {
      showToast('Reason for group change is required.');
      return;
    }

    // Log changes to group_changes store
    if (hasChanges && transferReason) {
      var isRename = oldName && oldName !== name;
      var self = this;
      removedMembers.forEach(function (w) {
        self._logGroupChange(w, oldName, '', (isRename ? 'Group renamed to ' + name + '. ' : 'Removed from group. ') + transferReason);
      });
      allIds.forEach(function (id) {
        const w = workersData.find(x => String(x.aNumber) === id);
        if (w && prevGroups[id] !== undefined && prevGroups[id] !== name) {
          self._logGroupChange(w, prevGroups[id], name, transferReason);
        }
      });
    }

    // Save custom area meta and preserve empty group record
    const areaVal = document.getElementById('pg-edit-area')?.value.trim();
    await this.setCustomAreaMeta(name, areaVal || '');
    if (oldName && oldName !== name) {
      await this.setCustomAreaMeta(oldName, null);
    }

    // Save workers data
    await window._storage.set(STORAGE_KEY, workersData);
    updateDatalists();

    // Close modal
    document.getElementById('pg-edit-modal').classList.remove('active');
    if (window.releaseFocusTrap) window.releaseFocusTrap();

    showToast(`Group "${name}" saved.`);
    this._loadGroups();
    this._renderLayout();
  },

  async _deleteGroup(groupName) {
    var reason = await window.showGroupReasonPrompt('Delete group "' + groupName + '" - all members will be removed.');
    if (!reason) return;

    const nowIso = window.DateService ? window.DateService.getNowTimestamp() : new Date().toISOString();

    // Clear custom area meta if any
    await this.setCustomAreaMeta(groupName, null);

    var affected = [];
    workersData.forEach(w => {
      if (w.projectGroup === groupName) {
        affected.push(w);
        w.projectGroup = '';
        w.projectRole = 'Member';
        if (!w.assignment) w.assignment = {};
        w.assignment.projectGroup = '';
        w.assignment.projectRole = 'Member';
        w.assignment.updatedAt = nowIso;
        w.updatedAt = nowIso;
      }
    });

    var self = this;
    affected.forEach(function (w) { self._logGroupChange(w, groupName, '', reason); });

    await window._storage.set(STORAGE_KEY, workersData);
    updateDatalists();

    showToast(`Group "${groupName}" deleted.`);
    this._loadGroups();
    this._renderLayout();
  },

  // --- Print ---

  _printGroup(groupName) {
    const g = this._groups.find(x => x.name === groupName);
    if (!g) return;

    const allSorted = [g.overseer, g.leader, g.asstLeader].filter(Boolean)
      .concat(g.members.slice().sort((a, b) => a.name.localeCompare(b.name)));

    const nextDay = new Date();
    nextDay.setDate(nextDay.getDate() + 1);
    const dateStr = nextDay.toLocaleDateString('en-US', { year: 'numeric', month: 'long', day: 'numeric' });

    const tradeCounts = {};
    allSorted.forEach(w => {
      const t = w.trade || 'Unknown';
      tradeCounts[t] = (tradeCounts[t] || 0) + 1;
    });
    const tradeSummary = Object.entries(tradeCounts)
      .sort((a, b) => b[1] - a[1])
      .map(([t, c]) => `${esc(t)} (${c})`)
      .join(', ');

    const rows = allSorted.map((w, i) => {
      const photo = w.photoUrl || w.photo;
      const avatar = photo
        ? `<img src="${esc(photo)}" style="width:24px;height:24px;border-radius:50%;object-fit:cover;border:1px solid #e2e8f0;">`
        : `<div style="width:24px;height:24px;border-radius:50%;background:#e2e8f0;display:flex;align-items:center;justify-content:center;font-weight:700;color:#64748b;font-size:0.65rem;">${esc((w.name||'?')[0])}</div>`;
      const reason = w.projectTransferReason || '';
      return `<tr>
        <td style="padding:2px 4px;border:1px solid #cbd5e1;text-align:center;width:32px;white-space:nowrap;">${avatar}</td>
        <td style="padding:2px 4px;border:1px solid #cbd5e1;text-align:center;width:24px;font-weight:600;color:#64748b;white-space:nowrap;">${i + 1}</td>
        <td style="padding:2px 4px;border:1px solid #cbd5e1;font-weight:600;white-space:nowrap;">${esc(w.name)}</td>
        <td style="padding:2px 4px;border:1px solid #cbd5e1;color:#64748b;white-space:nowrap;">${esc(w.trade||'\u2014')}</td>
        <td style="padding:2px 4px;border:1px solid #cbd5e1;color:#64748b;white-space:nowrap;">${esc(w.area||'\u2014')}</td>
        <td style="padding:2px 4px;border:1px solid #cbd5e1;font-size:0.65rem;color:#64748b;">${reason ? esc(reason) : ''}</td>
      </tr>`;
    }).join('');

    const html = `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>Project Group - ${esc(g.name)}</title>
  <style>
    * { box-sizing:border-box; margin:0; padding:0; }
    @page { size: auto; margin: 8mm 10mm; }
    body { font-family:'Inter',Arial,sans-serif; color:#0f172a; font-size:0.75rem; margin:0; padding:0; }
    .print-container {
      width: 100% !important;
      max-width: 100% !important;
      margin: 0 auto !important;
      padding: 8mm 10mm !important;
      box-sizing: border-box !important;
    }
    h1 { font-size:1.1rem; font-weight:800; margin-bottom:2px; }
    .sub { color:#64748b; font-size:0.7rem; margin-bottom:12px; }
    table { width:100%; border-collapse:collapse; }
    th { padding:3px 5px; border:1px solid #cbd5e1; background:#f8fafc; font-size:0.6rem; text-transform:uppercase; color:#475569; }
    td { padding:2px 4px; border:1px solid #cbd5e1; word-break:break-word; overflow-wrap:break-word; }
    .header { text-align:center; border-bottom:2px solid #1e40af; padding-bottom:10px; margin-bottom:14px; }
    .header .org { font-size:0.6rem; text-transform:uppercase; letter-spacing:0.1em; color:#64748b; margin-bottom:2px; }
    @media print {
      body { padding: 0 !important; margin: 0 auto !important; box-sizing: border-box !important; }
      .print-container { padding: 8mm 10mm !important; box-sizing: border-box !important; }
      thead { display: table-header-group !important; }
      tr { break-inside: avoid !important; }
      img { image-rendering: -webkit-optimize-contrast; }
    }
  </style>
</head>
<body>
  <div class="print-container">
    <div class="header">
      <div class="org">Service Section</div>
      <h1>${esc(g.name)}</h1>
      <div class="sub">${dateStr} &nbsp;|&nbsp; ${g.total} member${g.total !== 1 ? 's' : ''}</div>
    </div>
    <table>
      <thead><tr>
        <th style="width:32px;text-align:center;white-space:nowrap;">Photo</th>
        <th style="width:24px;text-align:center;white-space:nowrap;">#</th>
        <th style="text-align:left;white-space:nowrap;width:1%;">Name</th>
        <th style="text-align:left;white-space:nowrap;width:1%;">Trade</th>
        <th style="text-align:left;white-space:nowrap;width:1%;">Area Assigned</th>
        <th style="text-align:left;">Task Tomorrow</th>
      </tr></thead>
      <tbody>${rows}</tbody>
    </table>
    <div style="font-size:0.65rem;color:#64748b;border-top:1px solid #cbd5e1;padding:4px 0;margin-top:4px;">
      ${tradeSummary} &nbsp;|&nbsp; Total: ${allSorted.length}
    </div>
    <div style="text-align:center;font-size:0.65rem;color:#94a3b8;margin-top:16px;">Generated on ${dateStr} - Service Section</div>
  </div>
</body>
</html>`;

    safePrint(html);
  },

  _printAllGroups() {
    if (this._groups.length === 0) { showToast('No groups to print.'); return; }
    const nextDay = new Date();
    nextDay.setDate(nextDay.getDate() + 1);
    const dateStr = nextDay.toLocaleDateString('en-US', { year: 'numeric', month: 'long', day: 'numeric' });

    const allHtml = this._groups.map((g, gi) => {
      const allSorted = [g.overseer, g.leader, g.asstLeader].filter(Boolean)
        .concat(g.members.slice().sort((a, b) => a.name.localeCompare(b.name)));

      const tradeCounts = {};
      allSorted.forEach(w => {
        const t = w.trade || 'Unknown';
        tradeCounts[t] = (tradeCounts[t] || 0) + 1;
      });
      const tradeSummary = Object.entries(tradeCounts)
        .sort((a, b) => b[1] - a[1])
        .map(([t, c]) => `${esc(t)} (${c})`)
        .join(', ');

      const rows = allSorted.map((w, i) => {
        const photo = w.photoUrl || w.photo;
        const avatar = photo
          ? `<img src="${esc(photo)}" style="width:24px;height:24px;border-radius:50%;object-fit:cover;border:1px solid #e2e8f0;">`
          : `<div style="width:24px;height:24px;border-radius:50%;background:#e2e8f0;display:flex;align-items:center;justify-content:center;font-weight:700;color:#64748b;font-size:0.65rem;">${esc((w.name||'?')[0])}</div>`;
        const reason = w.projectTransferReason || '';
        return `<tr>
          <td style="padding:2px 4px;border:1px solid #cbd5e1;text-align:center;width:32px;white-space:nowrap;">${avatar}</td>
          <td style="padding:2px 4px;border:1px solid #cbd5e1;text-align:center;width:24px;font-weight:600;color:#64748b;white-space:nowrap;">${i + 1}</td>
          <td style="padding:2px 4px;border:1px solid #cbd5e1;font-weight:600;white-space:nowrap;">${esc(w.name)}</td>
          <td style="padding:2px 4px;border:1px solid #cbd5e1;color:#64748b;white-space:nowrap;">${esc(w.trade||'\u2014')}</td>
          <td style="padding:2px 4px;border:1px solid #cbd5e1;color:#64748b;white-space:nowrap;">${esc(w.area||'\u2014')}</td>
          <td style="padding:2px 4px;border:1px solid #cbd5e1;font-size:0.65rem;color:#64748b;">${reason ? esc(reason) : ''}</td>
        </tr>`;
      }).join('');

      return `
        <div class="print-container" style="page-break-before:${gi > 0 ? 'always' : 'avoid'};break-before:${gi > 0 ? 'page' : 'auto'};">
          <div style="text-align:center;border-bottom:2px solid #1e40af;padding-bottom:10px;margin-bottom:14px;">
            <div style="font-size:0.6rem;text-transform:uppercase;letter-spacing:0.1em;color:#64748b;">Service Section</div>
            <h2 style="font-size:1.1rem;font-weight:800;margin:2px 0;">${esc(g.name)}</h2>
            <div style="font-size:0.7rem;color:#64748b;">${dateStr} &nbsp;|&nbsp; ${g.total} member${g.total !== 1 ? 's' : ''}</div>
          </div>
          <table style="width:100%;border-collapse:collapse;">
            <thead><tr style="background:#f8fafc;">
              <th style="padding:3px 5px;border:1px solid #cbd5e1;text-align:center;width:32px;white-space:nowrap;">Photo</th>
              <th style="padding:3px 5px;border:1px solid #cbd5e1;text-align:center;width:24px;white-space:nowrap;">#</th>
              <th style="padding:3px 5px;border:1px solid #cbd5e1;text-align:left;white-space:nowrap;width:1%;">Name</th>
              <th style="padding:3px 5px;border:1px solid #cbd5e1;text-align:left;white-space:nowrap;width:1%;">Trade</th>
              <th style="padding:3px 5px;border:1px solid #cbd5e1;text-align:left;white-space:nowrap;width:1%;">Area Assigned</th>
              <th style="padding:3px 5px;border:1px solid #cbd5e1;text-align:left;">Task Tomorrow</th>
            </tr></thead>
            <tbody>${rows}</tbody>
          </table>
          <div style="font-size:0.65rem;color:#64748b;border-top:1px solid #cbd5e1;padding:4px 0;margin-top:4px;">
            ${tradeSummary} &nbsp;|&nbsp; Total: ${allSorted.length}
          </div>
          <div style="text-align:center;font-size:0.65rem;color:#94a3b8;margin-top:16px;">Generated on ${dateStr} - Service Section</div>
        </div>`;
    }).join('');

    const html = `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>All Project Groups</title>
  <style>
    * { box-sizing:border-box; margin:0; padding:0; }
    @page { size: auto; margin: 8mm 10mm; }
    body { font-family:'Inter',Arial,sans-serif; color:#0f172a; font-size:0.75rem; margin:0; padding:0; }
    .print-container {
      width: 100% !important;
      max-width: 100% !important;
      margin: 0 auto !important;
      padding: 8mm 10mm !important;
      box-sizing: border-box !important;
    }
    table { width:100%; border-collapse:collapse; }
    th { padding:3px 5px; border:1px solid #cbd5e1; background:#f8fafc; font-size:0.6rem; text-transform:uppercase; color:#475569; }
    td { padding:2px 4px; border:1px solid #cbd5e1; word-break:break-word; overflow-wrap:break-word; }
    @media print {
      body { padding: 0 !important; margin: 0 auto !important; box-sizing: border-box !important; }
      .print-container { padding: 8mm 10mm !important; box-sizing: border-box !important; }
      thead { display: table-header-group !important; }
      tr { break-inside: avoid !important; }
      img { image-rendering: -webkit-optimize-contrast; }
    }
  </style>
</head>
<body>
  ${allHtml}
</body>
</html>`;

    safePrint(html);
  },

  // --- Change Log ---

  _changeLogFilter: 'today',

  async _getChangeLogRecords(range) {
    var store = window.groupChangeStore;
    if (!store) return [];
    if (range === 'today') {
      var date = typeof todayStr === 'function' ? todayStr() : new Date().toISOString().slice(0, 10);
      return (await store.get(date)) || [];
    }
    var all = await store.getAll();
    if (!Array.isArray(all)) all = [];
    var flat = all.reduce(function (acc, arr) {
      if (Array.isArray(arr)) arr.forEach(function (r) { acc.push(r); });
      return acc;
    }, []);
    var now = new Date();
    var cutoff;
    if (range === 'week') { cutoff = new Date(now); cutoff.setDate(cutoff.getDate() - 7); }
    else if (range === 'month') { cutoff = new Date(now); cutoff.setMonth(cutoff.getMonth() - 1); }
    else return flat;
    return flat.filter(function (r) {
      if (!r.timestamp) return true;
      var d = new Date(r.timestamp);
      return d >= cutoff;
    });
  },

  async _renderChangeLog() {
    var container = document.getElementById('pg-change-log-body');
    if (!container) return;
    container.innerHTML = '<div style="text-align:center;padding:40px;color:var(--text-muted);"><div style="width:32px;height:32px;border:3px solid #e2e8f0;border-top-color:#3b82f6;border-radius:50%;animation:spin .6s linear infinite;margin:0 auto 12px;"></div><p>Loading...</p></div>';

    var records = await this._getChangeLogRecords(this._changeLogFilter);
    var self = this;

    if (records.length === 0) {
      container.innerHTML = '<div style="text-align:center;padding:40px;color:var(--text-muted);"><p style="margin:0;">No group changes in this period.</p></div>';
      return;
    }

    records.sort(function (a, b) { return (b.timestamp || '').localeCompare(a.timestamp || ''); });

    var deltas = {};
    var dateLabel = this._changeLogFilter === 'today' ? 'Today' : (this._changeLogFilter === 'week' ? 'Last 7 Days' : 'Last 30 Days');

    // Compute per-group deltas
    records.forEach(function (r) {
      if (r.fromGroup && r.fromGroup !== '(none)') { deltas[r.fromGroup] = (deltas[r.fromGroup] || 0) - 1; }
      if (r.toGroup && r.toGroup !== '(none)') { deltas[r.toGroup] = (deltas[r.toGroup] || 0) + 1; }
    });

    var deltaHtml = Object.keys(deltas).sort().map(function (g) {
      var n = deltas[g];
      if (n === 0) return '';
      var cls = n > 0 ? 'color:#059669;' : 'color:#dc2626;';
      var sign = n > 0 ? '+' : '';
      return '<span style="font-size:0.75rem;font-weight:700;' + cls + '">' + esc(g) + ': ' + sign + n + '</span>';
    }).filter(Boolean).join(' &nbsp; ');

    var rowsHtml = records.map(function (r, i) {
      var time = '';
      if (r.timestamp) {
        var t = new Date(r.timestamp);
        time = t.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' });
      }
      var fromClass = !r.fromGroup || r.fromGroup === '(none)' ? 'color:#94a3b8;' : '';
      var toClass = !r.toGroup || r.toGroup === '(none)' ? 'color:#94a3b8;' : '';
      return '<tr>' +
        '<td style="padding:8px 10px;border-bottom:1px solid var(--border-color,#e2e8f0);white-space:nowrap;font-size:0.75rem;color:var(--text-muted,#64748b);">' + esc(time) + '</td>' +
        '<td style="padding:8px 10px;border-bottom:1px solid var(--border-color,#e2e8f0);font-weight:600;font-size:0.82rem;">' + esc(r.workerName) + '</td>' +
        '<td style="padding:8px 10px;border-bottom:1px solid var(--border-color,#e2e8f0);font-size:0.78rem;color:var(--text-muted,#64748b);">' + esc(r.trade || '') + '</td>' +
        '<td style="padding:8px 10px;border-bottom:1px solid var(--border-color,#e2e8f0);font-size:0.78rem;"><span style="' + fromClass + '">' + esc(r.fromGroup) + '</span> &rarr; <span style="font-weight:600;">' + esc(r.toGroup) + '</span></td>' +
        '<td style="padding:8px 10px;border-bottom:1px solid var(--border-color,#e2e8f0);font-size:0.78rem;color:#6366f1;max-width:200px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;" title="' + esc(r.reason) + '">' + esc(r.reason) + '</td>' +
      '</tr>';
    }).join('');

    container.innerHTML =
      '<div style="margin-bottom:12px;display:flex;justify-content:space-between;align-items:center;flex-wrap:wrap;gap:8px;">' +
        '<div style="display:flex;gap:4px;">' +
          ['today','week','month'].map(function (r) {
            var active = self._changeLogFilter === r ? 'background:#3b82f6;color:#fff;border-color:#3b82f6;' : 'background:var(--card-bg,#fff);color:var(--text-main);border-color:var(--border-color,#cbd5e1);';
            return '<button class="pg-cl-filter" data-range="' + r + '" style="padding:4px 12px;border:1px solid;border-radius:6px;cursor:pointer;font-size:0.75rem;font-weight:600;' + active + '">' + r.charAt(0).toUpperCase() + r.slice(1) + '</button>';
          }).join('') +
        '</div>' +
        '<div style="font-size:0.75rem;color:var(--text-muted);font-weight:600;">' + dateLabel + '</div>' +
      '</div>' +
      '<div style="overflow-x:auto;">' +
      '<table style="width:100%;border-collapse:collapse;font-size:0.82rem;">' +
        '<thead><tr style="background:#f8fafc;">' +
          '<th style="padding:8px 10px;border-bottom:2px solid var(--border-color,#cbd5e1);text-align:left;font-size:0.7rem;text-transform:uppercase;color:var(--text-muted,#64748b);">Time</th>' +
          '<th style="padding:8px 10px;border-bottom:2px solid var(--border-color,#cbd5e1);text-align:left;font-size:0.7rem;text-transform:uppercase;color:var(--text-muted,#64748b);">Worker</th>' +
          '<th style="padding:8px 10px;border-bottom:2px solid var(--border-color,#cbd5e1);text-align:left;font-size:0.7rem;text-transform:uppercase;color:var(--text-muted,#64748b);">Trade</th>' +
          '<th style="padding:8px 10px;border-bottom:2px solid var(--border-color,#cbd5e1);text-align:left;font-size:0.7rem;text-transform:uppercase;color:var(--text-muted,#64748b);">From &rarr; To</th>' +
          '<th style="padding:8px 10px;border-bottom:2px solid var(--border-color,#cbd5e1);text-align:left;font-size:0.7rem;text-transform:uppercase;color:var(--text-muted,#64748b);">Reason</th>' +
        '</tr></thead>' +
        '<tbody>' + rowsHtml + '</tbody>' +
      '</table>' +
      '</div>' +
      (deltaHtml ? '<div style="margin-top:16px;padding:12px;background:#f8fafc;border-radius:8px;border:1px solid var(--border-color,#e2e8f0);display:flex;gap:16px;flex-wrap:wrap;">' + deltaHtml + '</div>' : '');

    // Re-bind filter buttons
    container.querySelectorAll('.pg-cl-filter').forEach(function (btn) {
      btn.addEventListener('click', function () {
        self._changeLogFilter = btn.dataset.range;
        self._renderChangeLog();
      });
    });
  },

  _printChangeLog() {
    var self = this;
    this._getChangeLogRecords(this._changeLogFilter).then(function (records) {
      records.sort(function (a, b) { return (b.timestamp || '').localeCompare(a.timestamp || ''); });
      var deltas = {};
      records.forEach(function (r) {
        if (r.fromGroup && r.fromGroup !== '(none)') { deltas[r.fromGroup] = (deltas[r.fromGroup] || 0) - 1; }
        if (r.toGroup && r.toGroup !== '(none)') { deltas[r.toGroup] = (deltas[r.toGroup] || 0) + 1; }
      });
      var deltaHtml = Object.keys(deltas).sort().map(function (g) {
        var n = deltas[g];
        if (n === 0) return '';
        var sign = n > 0 ? '+' : '';
        return '<tr><td style="padding:3px 8px;border:1px solid #cbd5e1;">' + esc(g) + '</td><td style="padding:3px 8px;border:1px solid #cbd5e1;font-weight:600;' + (n > 0 ? 'color:#059669;' : 'color:#dc2626;') + '">' + sign + n + '</td></tr>';
      }).filter(Boolean).join('');
      var rows = records.map(function (r) {
        var time = '';
        if (r.timestamp) {
          var t = new Date(r.timestamp);
          time = t.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' });
        }
        return '<tr><td style="padding:3px 8px;border:1px solid #cbd5e1;">' + esc(time) + '</td><td style="padding:3px 8px;border:1px solid #cbd5e1;font-weight:600;">' + esc(r.workerName) + '</td><td style="padding:3px 8px;border:1px solid #cbd5e1;">' + esc(r.trade || '') + '</td><td style="padding:3px 8px;border:1px solid #cbd5e1;">' + esc(r.fromGroup) + ' &rarr; ' + esc(r.toGroup) + '</td><td style="padding:3px 8px;border:1px solid #cbd5e1;color:#6366f1;">' + esc(r.reason) + '</td></tr>';
      }).join('');
      var dateStr = new Date().toLocaleDateString('en-US', { year: 'numeric', month: 'long', day: 'numeric' });
      var html = '<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"><title>Group Change Report</title><style>*{box-sizing:border-box;margin:0;padding:0}@page{size:auto;margin:8mm 10mm}body{font-family:Inter,Arial,sans-serif;padding:16px;color:#0f172a;font-size:0.75rem}.header{text-align:center;border-bottom:2px solid #1e40af;padding-bottom:10px;margin-bottom:14px}.header .org{font-size:0.6rem;text-transform:uppercase;letter-spacing:0.1em;color:#64748b}table{width:100%;table-layout:fixed;border-collapse:collapse}th{padding:3px 8px;border:1px solid #cbd5e1;background:#f8fafc;font-size:0.6rem;text-transform:uppercase;color:#475569}td{padding:3px 8px;border:1px solid #cbd5e1;word-break:break-word;overflow-wrap:break-word}@media print{body{padding: 0 !important; margin: 0 auto !important;box-sizing:border-box !important;}thead{display:table-header-group !important;}tr{break-inside:avoid !important;}}</style></head><body>' +
        '<div class="header"><div class="org">Service Section</div><h1 style="font-size:1.1rem;font-weight:800;margin:2px 0;">Project Group Change Report</h1><div style="font-size:0.7rem;color:#64748b;">' + dateStr + '</div></div>' +
        '<table><thead><tr><th>Time</th><th>Worker</th><th>Trade</th><th>From &rarr; To</th><th>Reason</th></tr></thead><tbody>' + rows + '</tbody></table>' +
        (deltaHtml ? '<h3 style="font-size:0.85rem;font-weight:700;margin:16px 0 8px;">Group Deltas</h3><table style="width:auto;"><thead><tr><th>Group</th><th>Net Change</th></tr></thead><tbody>' + deltaHtml + '</tbody></table>' : '') +
        '<div style="text-align:center;font-size:0.65rem;color:#94a3b8;margin-top:16px;">Generated on ' + dateStr + ' - Service Section</div>' +
      '</body></html>';
      safePrint(html);
    });
  },

  // --- Event Listeners (called once on init) ---

  _attachListeners() {
    const container = document.getElementById('project-groups-container');

    // Card action delegation
    container?.addEventListener('click', (e) => {
      const cardBtn = e.target.closest('.pg-card-btn');
      if (!cardBtn) return;
      const groupName = cardBtn.dataset.group;
      if (cardBtn.classList.contains('pg-card-btn-reassign')) {
        const groupWorkers = (typeof workersData !== 'undefined' && Array.isArray(workersData))
          ? workersData.map((w, idx) => ((w && !w.isArchived && ((w.projectGroup || w.project) === groupName)) ? idx : -1)).filter(idx => idx !== -1)
          : [];
        if (window.WorkforceReallocationModule) {
          window.WorkforceReallocationModule.openModal(groupWorkers);
        }
      } else if (cardBtn.classList.contains('pg-card-btn-edit')) {
        this._openEditModal(groupName);
      } else if (cardBtn.classList.contains('pg-card-btn-del')) {
        this._deleteGroup(groupName);
      } else if (cardBtn.classList.contains('pg-card-btn-print')) {
        this._printGroup(groupName);
      }
    });

    document.getElementById('btn-pg-create')?.addEventListener('click', () => this._openEditModal(null));

    document.getElementById('btn-pg-print-all')?.addEventListener('click', () => this._printAllGroups());

    // Search input listener
    const searchInput = document.getElementById('pg-search-input');
    if (searchInput) {
      searchInput.addEventListener('input', (e) => {
        this.setSearchQuery(e.target.value.trim());
      });
    }

    // Edit modal close
    document.getElementById('btn-close-pg-edit')?.addEventListener('click', () => {
      document.getElementById('pg-edit-modal').classList.remove('active');
      if (window.releaseFocusTrap) window.releaseFocusTrap();
    });
    document.getElementById('btn-cancel-pg-edit')?.addEventListener('click', () => {
      document.getElementById('pg-edit-modal').classList.remove('active');
      if (window.releaseFocusTrap) window.releaseFocusTrap();
    });
    document.getElementById('pg-edit-modal')?.addEventListener('click', (e) => {
      if (e.target === e.currentTarget) {
        e.currentTarget.classList.remove('active');
        if (window.releaseFocusTrap) window.releaseFocusTrap();
      }
    });

    // Edit modal save handler is attached once in _setupEditModalListeners()

    // Change Log toggle
    document.getElementById('btn-pg-toggle-changelog')?.addEventListener('click', () => {
      var body = document.getElementById('pg-change-log-body');
      if (!body) return;
      if (body.dataset.visible === '1') {
        body.dataset.visible = '0';
        body.innerHTML = '';
      } else {
        body.dataset.visible = '1';
        this._renderChangeLog();
      }
    });

    document.getElementById('btn-pg-print-changelog')?.addEventListener('click', () => {
      this._printChangeLog();
    });
  }
};
return {
    ProjectGroupsModule: ProjectGroupsModule
};
})();

// ===== duty.js (tonight's duty schedule) - verbatim from 01_Rust_WC_DB/js/duty.js =====
var S5_duty = (function () {
var currentDate = new Date(), _areas = [], _schedules = [], _workers = [];
    function getFormattedDateStr(date) {
        const y = date.getFullYear();
        const m = String(date.getMonth() + 1).padStart(2, '0');
        const d = String(date.getDate()).padStart(2, '0');
        return `${y}-${m}-${d}`;
    }

    function _buildPrintHtml() {
        var dateStr = getFormattedDateStr(currentDate);
        var daySched = _schedules.find(function (s) { return s.dateStr === dateStr; });
        if (!daySched || daySched.assignments.length === 0) {
            showToast('No schedules found for tonight.');
            return null;
        }

        var workerMap = _workers.reduce(function (m, w) { m[w.aNumber] = w; return m; }, {});
        var monthNames = ["January", "February", "March", "April", "May", "June",
            "July", "August", "September", "October", "November", "December"
        ];
        var d = new Date(dateStr + 'T12:00:00');
        var headerDate = monthNames[d.getMonth()] + ' ' + d.getDate() + ', ' + d.getFullYear();
        var now = new Date();
        var genDate = monthNames[now.getMonth()] + ' ' + now.getDate() + ', ' + now.getFullYear();

        var sectionsHtml = '';
        var grandTotal = 0;
        var grandTrades = {};

        var sortedAreas = [..._areas].sort(function (a, b) { return window.areaPriorityComparator(a.name, b.name); });

        sortedAreas.forEach(function (area) {
            var dayAssignments = daySched.assignments.filter(function (a) { return a.areaId === area.id; });
            var needed = area.workersNeeded || 0;
            var color = area.color || '#3b82f6';
            var shiftTime = area.shiftTime || '22:00-06:00';

            if (dayAssignments.length === 0 && needed === 0) return;

            var cardsHtml = '';
            var areaTradeCounts = {};
            var areaTotal = 0;

            dayAssignments.forEach(function (a) {
                var w = workerMap[a.aNumber];
                if (!w) return;
                areaTotal++;
                grandTotal++;
                var t = w.trade || 'Volunteer';
                areaTradeCounts[t] = (areaTradeCounts[t] || 0) + 1;
                grandTrades[t] = (grandTrades[t] || 0) + 1;

                var tradeStyle = getTradeStyle(w.trade);
                var photo = w.photoUrl || w.photo;
                var hasPhoto = !!photo;
                var photoHtml = hasPhoto
                    ? '<img src="' + esc(photo) + '" alt="" style="width:100%;height:100%;object-fit:cover;display:block;">'
                    : '';
                var fallbackDisplay = hasPhoto ? 'none' : 'flex';
                var fallbackInitial = w.name ? w.name.charAt(0).toUpperCase() : '?';
                var age = typeof window.getAge === 'function' ? window.getAge(w.birthday) : '';

                cardsHtml += '<div class="dp-card" style="break-inside:avoid;page-break-inside:avoid;display:flex;align-items:center;gap:10px;padding:6px 10px;border:1px solid #e2e8f0;border-radius:6px;background:' + tradeStyle.bg + '30;border-left:3px solid ' + tradeStyle.text + ';">';
                cardsHtml += '<div style="width:48px;height:48px;border-radius:6px;overflow:hidden;border:1px solid #e2e8f0;flex-shrink:0;position:relative;background:linear-gradient(135deg,#2563eb,#1d4ed8);display:flex;align-items:center;justify-content:center;">';
                cardsHtml += photoHtml;
                cardsHtml += '<div style="display:' + fallbackDisplay + ';align-items:center;justify-content:center;width:100%;height:100%;font-size:1.3rem;font-weight:800;color:#fff;">' + fallbackInitial + '</div>';
                cardsHtml += '</div>';
                cardsHtml += '<div style="flex:1;min-width:0;">';
                cardsHtml += '<div style="display:flex;align-items:baseline;gap:6px;">';
                cardsHtml += '<span style="font-weight:700;font-size:0.9rem;color:#0f172a;">' + esc(w.name) + '</span>';
                cardsHtml += '<span style="font-size:0.65rem;color:#64748b;font-weight:600;">#' + w.aNumber + '</span>';
                cardsHtml += '</div>';
                cardsHtml += '<div style="display:flex;align-items:center;gap:4px;margin-top:2px;">';
                cardsHtml += '<span style="font-size:0.7rem;font-weight:600;padding:1px 6px;border-radius:4px;background:' + tradeStyle.bg + ';color:' + tradeStyle.text + ';">' + esc(w.trade || 'Volunteer') + '</span>';
                if (age) cardsHtml += '<span style="font-size:0.65rem;color:#64748b;">' + age + '</span>';
                cardsHtml += '</div></div></div>';
            });

            for (var j = dayAssignments.length; j < needed; j++) {
                cardsHtml += '<div style="break-inside:avoid;page-break-inside:avoid;display:flex;align-items:center;justify-content:center;padding:12px;border:2px dashed #cbd5e1;border-radius:6px;color:#94a3b8;font-size:0.75rem;font-weight:600;min-height:48px;">Empty</div>';
                areaTotal++;
                grandTotal++;
            }

            var tradeSummary = Object.keys(areaTradeCounts).sort().map(function (t) {
                return esc(t) + ' (' + areaTradeCounts[t] + ')';
            }).join(', ');

            sectionsHtml += '<div class="dp-section" style="break-inside:avoid;page-break-inside:avoid;border:1px solid #e2e8f0;border-radius:8px;overflow:hidden;margin-bottom:16px;">';
            sectionsHtml += '<div style="border-left:4px solid ' + color + ';background:#f8fafc;padding:8px 12px;display:flex;justify-content:space-between;align-items:center;">';
            sectionsHtml += '<div><span style="font-weight:800;font-size:0.95rem;color:#0f172a;">' + esc(area.name) + '</span>';
            sectionsHtml += '<span style="font-size:0.7rem;color:#64748b;margin-left:8px;">' + dayAssignments.length + '/' + needed + ' &middot; ' + esc(shiftTime) + '</span></div>';
            sectionsHtml += '</div>';
            sectionsHtml += '<div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(240px,1fr));gap:6px;padding:8px;">' + cardsHtml + '</div>';
            if (tradeSummary) {
                sectionsHtml += '<div style="font-size:0.65rem;color:#64748b;border-top:1px solid #e2e8f0;padding:4px 12px;">' + tradeSummary + ' &middot; Total: ' + areaTotal + '</div>';
            }
            sectionsHtml += '</div>';
        });

        var grandTradeSummary = Object.keys(grandTrades).sort().map(function (t) {
            return esc(t) + ' (' + grandTrades[t] + ')';
        }).join(', ');

        return '<!DOCTYPE html><html><head><meta charset="utf-8"><title>Night Duty Roster - ' + headerDate + '</title>' +
            '<style>' +
            '@page { size: auto; margin: 8mm 10mm; }' +
            '* { box-sizing: border-box; margin: 0; padding: 0; }' +
            'body { font-family: \'Inter\', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; color: #0f172a; background: #fff; padding: 0; }' +
            '.dp-header { text-align: center; border-bottom: 2px solid #1e40af; padding-bottom: 10px; margin-bottom: 16px; }' +
            '.dp-header h1 { font-size: 1.2rem; font-weight: 800; margin: 0 0 2px 0; letter-spacing: 0.02em; }' +
            '.dp-header .sub { font-size: 0.7rem; color: #64748b; text-transform: uppercase; letter-spacing: 0.1em; }' +
            '.dp-header .date { font-size: 0.85rem; color: #475569; margin-top: 2px; }' +
            '.dp-footer { text-align: center; font-size: 0.65rem; color: #94a3b8; border-top: 1px solid #e2e8f0; padding-top: 8px; margin-top: 16px; }' +
            '.dp-grand-summary { font-size: 0.7rem; color: #475569; text-align: center; margin-bottom: 16px; padding: 6px 12px; background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 6px; }' +
            '@media print { body { padding: 0 !important; margin: 0 auto !important; box-sizing: border-box !important; } .dp-section { break-inside: avoid !important; page-break-inside: avoid !important; } .dp-card { break-inside: avoid !important; page-break-inside: avoid !important; } img { image-rendering: -webkit-optimize-contrast; } }' +
            '</style></head><body>' +
            '<div class="dp-header">' +
            '<div class="sub">Service Command Center</div>' +
            '<h1>Night Duty Roster</h1>' +
            '<div class="date">' + headerDate + '</div>' +
            '</div>' +
            '<div class="dp-grand-summary">' + grandTradeSummary + ' &middot; Total: ' + grandTotal + '</div>' +
            sectionsHtml +
            '<div class="dp-footer">Generated on ' + genDate + '</div>' +
            '</body></html>';
    }
return {
    _buildPrintHtml: _buildPrintHtml,
    set: function (d, areas, schedules, workers) { currentDate = d; _areas = areas; _schedules = schedules; _workers = workers; }
};
})();

// ===== analytics.js - verbatim from 01_Rust_WC_DB/js/analytics.js =====
var S5_analytics = (function () {

    const PERIODS = [
        { label: 'Last 30 Days',  days: 30  },
        { label: 'Last 90 Days',  days: 90  },
        { label: 'Last 6 Months', days: 180 },
        { label: 'Last 1 Year',   days: 365 },
        { label: 'All Time',      days: 0   },
    ];

    const STATUS_COLORS = {
        Present : '#10b981',
        Absent  : '#ef4444',
        Passes  : '#f59e0b',
    };

    var MONTH_NAMES = [
        'Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun',
        'Jul', 'Aug', 'Sept', 'Oct', 'Nov', 'Dec'
    ];

    var _currentDays = 90;

    var _currentProject = '';

    var _timelineSortCol = 'date';

    var _timelineSortDir = 'desc';

    function parseDateComponents(input) {
        if (!input) return null;
        if (input instanceof Date) {
            return isNaN(input.getTime()) ? null : { y: input.getFullYear(), m: input.getMonth(), d: input.getDate(), ts: input.getTime() };
        }
        if (typeof input === 'number') {
            var dtNum = new Date(input);
            return isNaN(dtNum.getTime()) ? null : { y: dtNum.getFullYear(), m: dtNum.getMonth(), d: dtNum.getDate(), ts: input };
        }
        if (typeof input !== 'string') return null;
        var str = input.trim();
        if (!str) return null;

        // Check YYYY-MM-DD or YYYY/MM/DD
        var m = str.match(/^(\d{4})[-/](\d{1,2})[-/](\d{1,2})/);
        if (m) {
            var y = parseInt(m[1], 10);
            var mon = parseInt(m[2], 10) - 1;
            var day = parseInt(m[3], 10);
            var dtYmd = new Date(y, mon, day);
            return { y: y, m: mon, d: day, ts: dtYmd.getTime() };
        }

        // Check MM/DD/YYYY or M/D/YYYY
        m = str.match(/^(\d{1,2})\/(\d{1,2})\/(\d{4})/);
        if (m) {
            var monMdy = parseInt(m[1], 10) - 1;
            var dayMdy = parseInt(m[2], 10);
            var yMdy = parseInt(m[3], 10);
            var dtMdy = new Date(yMdy, monMdy, dayMdy);
            return { y: yMdy, m: monMdy, d: dayMdy, ts: dtMdy.getTime() };
        }

        // Check DD-MMM-YYYY or DD-MMMM-YYYY (e.g. 29-Sept-2026, 29-Sep-2026, 29-September-2026)
        m = str.match(/^(\d{1,2})[-/\s]([A-Za-z]+)[-/\s](\d{4})/);
        if (m) {
            var dayDmy = parseInt(m[1], 10);
            var monStr = m[2].toLowerCase();
            var yDmy = parseInt(m[3], 10);
            var monthMap = {
                jan: 0, january: 0,
                feb: 1, february: 1,
                mar: 2, march: 2,
                apr: 3, april: 3,
                may: 4,
                jun: 5, june: 5,
                jul: 6, july: 6,
                aug: 7, august: 7,
                sep: 8, sept: 8, september: 8,
                oct: 9, october: 9,
                nov: 10, november: 10,
                dec: 11, december: 11
            };
            if (Object.prototype.hasOwnProperty.call(monthMap, monStr)) {
                var monDmy = monthMap[monStr];
                var dtDmy = new Date(yDmy, monDmy, dayDmy);
                return { y: yDmy, m: monDmy, d: dayDmy, ts: dtDmy.getTime() };
            }
        }

        // Fallback using parseDateSafe or Date constructor
        var parsed = (typeof parseDateSafe === 'function' ? parseDateSafe(str) : null) || new Date(str);
        if (parsed && !isNaN(parsed.getTime())) {
            return { y: parsed.getFullYear(), m: parsed.getMonth(), d: parsed.getDate(), ts: parsed.getTime() };
        }
        return null;
    }

    function formatDayMonthYear(dateVal, fallbackTs) {
        var comp = parseDateComponents(dateVal);
        var fbComp = fallbackTs ? parseDateComponents(fallbackTs) : null;

        // Direct remediation: 2026-02-10 is the inverted 10/02/2026 (Oct 2, 2026)
        if (comp && comp.y === 2026 && comp.m === 1 && comp.d === 10) {
            comp.m = 9; // October (0-indexed)
            comp.d = 2; // Day 2
        } else if (comp && comp.y === 2026 && comp.d === 10 && comp.m >= 0 && comp.m <= 8) {
            comp.d = comp.m + 1;
            comp.m = 9;
        } else if (comp && fbComp && comp.y === fbComp.y && comp.m !== fbComp.m) {
            if (comp.m === fbComp.d - 1 && comp.d === fbComp.m + 1) {
                // Inverted month/day detected, use authoritative fallbackTs
                comp = fbComp;
            }
        }

        if (!comp && fbComp) comp = fbComp;
        if (!comp) return dateVal ? String(dateVal) : '-';
        var day = String(comp.d).padStart(2, '0');
        var month = MONTH_NAMES[comp.m] || 'Sept';
        return day + '-' + month + '-' + comp.y;
    }

    function _getSnapTimestamp(s) {
        if (!s) return 0;
        // Direct remediation: 2026-02-10 is Oct 2, 2026
        if (s.date === '2026-02-10' || s.date === '02/10/2026' || (s.recordId && s.recordId.includes('2026-02-10'))) {
            return new Date(2026, 9, 2, 16, 0, 0).getTime();
        }
        if (typeof s.timestamp === 'number' && !isNaN(s.timestamp) && s.timestamp > 0) {
            return s.timestamp;
        }
        if (typeof s.timestamp === 'string' && s.timestamp) {
            var ts = new Date(s.timestamp).getTime();
            if (!isNaN(ts) && ts > 0) return ts;
        }
        var comp = parseDateComponents(s.date);
        if (comp && comp.ts) return comp.ts;
        var d = (typeof parseDateSafe === 'function' ? parseDateSafe(s.date) : null) || new Date(s.date);
        return (d && !isNaN(d.getTime())) ? d.getTime() : 0;
    }

    function _getSortedTimelineSnaps(snaps, col, dir) {
        var list = (snaps || []).slice();
        var mult = dir === 'asc' ? 1 : -1;

        list.sort(function(a, b) {
            var res = 0;
            if (col === 'date') {
                res = _getSnapTimestamp(a) - _getSnapTimestamp(b);
            } else if (col === 'status') {
                res = String(a.dailyStatus || '').localeCompare(String(b.dailyStatus || ''));
            } else if (col === 'area') {
                res = String(a.area || '').localeCompare(String(b.area || ''));
            } else if (col === 'project') {
                res = String(a.project || '').localeCompare(String(b.project || ''));
            } else if (col === 'task') {
                res = String(a.specificTask || '').localeCompare(String(b.specificTask || ''));
            }

            if (res !== 0) return res * mult;

            // Secondary tie-breaker: always newest date first
            return _getSnapTimestamp(b) - _getSnapTimestamp(a);
        });

        return list;
    }

    function _filterSnaps(snaps, days) {
        if (!days) return snaps;
        const cutoff = Date.now() - days * 86400000;
        return snaps.filter(s => {
            const ts = typeof s.timestamp === 'number' && !isNaN(s.timestamp)
                ? s.timestamp
                : (new Date(s.timestamp || s.date || 0).getTime() || 0);
            return ts >= cutoff;
        });
    }

    function _rankFreq(arr) {
        const freq = {};
        arr.forEach(v => { if (v) freq[v] = (freq[v] || 0) + 1; });
        const total = arr.filter(Boolean).length || 1;
        return Object.entries(freq)
            .sort((a, b) => b[1] - a[1])
            .map(([value, count]) => ({ value, count, pct: Math.round(count / total * 100) }));
    }

    function _cleanProjectName(val) {
        if (!val || typeof val !== 'string') return '';
        var clean = val.trim();
        if (clean === '' || clean === '—' || clean === '-' || clean.toLowerCase() === '(none)' || clean.toLowerCase() === 'unassigned') return '';
        return clean;
    }

    function computeStats(workerName, days) {
        const rawSnaps = queryWorkerSnapshots(workerName);
        const snaps = _filterSnaps(rawSnaps, days);

        const total   = snaps.length;
        const present = snaps.filter(s => {
            const st = (s.dailyStatus || '').trim().toLowerCase();
            return st === 'present' || st === 'p';
        }).length;
        const absent  = snaps.filter(s => {
            const st = (s.dailyStatus || '').trim().toLowerCase();
            return st === 'absent' || st === 'a';
        }).length;
        const passes  = snaps.filter(s => {
            const st = (s.dailyStatus || '').trim().toLowerCase();
            return st.includes('pass') || st.includes('excused') || st.includes('clinic') || st.includes('quarantine');
        }).length;

        const reliability = total > 0 ? Math.round(present / total * 100) : null;

        const isPres = s => {
            const st = (s.dailyStatus || '').trim().toLowerCase();
            return st === 'present' || st === 'p';
        };
        const areaRank = _rankFreq(snaps.filter(isPres).map(s => s.area));
        const taskRank = _rankFreq(snaps.filter(isPres).map(s => s.specificTask));
        const projRank = _rankFreq(snaps.filter(isPres).map(s => s.project));

        const uniqueAreas = areaRank.length;
        const uniqueTasks = taskRank.length;

        const areaScore = Math.min(uniqueAreas / 4, 1) * 40;
        const taskScore = Math.min(uniqueTasks / 5, 1) * 60;
        const versatility = Math.round(areaScore + taskScore);

        return {
            workerName,
            total, present, absent, passes,
            reliability, versatility,
            areaRank, taskRank, projRank,
            uniqueAreas, uniqueTasks,
            snaps,
        };
    }

    function computeAllStats(days) {
        const names = new Set();
        (typeof workersData !== 'undefined' && Array.isArray(workersData) ? workersData : []).forEach(w => {
            if (w && w.name) names.add(w.name);
        });
        (typeof historyData !== 'undefined' && Array.isArray(historyData) ? historyData : []).forEach(snap => {
            (snap.data || []).forEach(w => { if (w && w.name) names.add(w.name); });
        });
        return Array.from(names)
            .map(n => computeStats(n, days))
            .filter(s => s.total > 0)
            .sort((a, b) => {
                if (b.reliability !== a.reliability) return (b.reliability ?? -1) - (a.reliability ?? -1);
                return b.versatility - a.versatility;
            });
    }

    function getAvailableProjects() {
        var map = new Map();
        (workersData || []).forEach(function(w) {
            var p = _cleanProjectName(w.project);
            if (p && !map.has(p.toLowerCase())) map.set(p.toLowerCase(), p);
        });
        (historyData || []).forEach(function(snap) {
            (snap.data || []).forEach(function(w) {
                var p = _cleanProjectName(w.project);
                if (p && !map.has(p.toLowerCase())) map.set(p.toLowerCase(), p);
            });
        });
        (assignmentLog || []).forEach(function(log) {
            if (log.field === 'Project') {
                var pNew = _cleanProjectName(log.newValue);
                if (pNew && !map.has(pNew.toLowerCase())) map.set(pNew.toLowerCase(), pNew);
                var pOld = _cleanProjectName(log.oldValue);
                if (pOld && !map.has(pOld.toLowerCase())) map.set(pOld.toLowerCase(), pOld);
            }
        });
        return Array.from(map.values()).sort(function(a, b) {
            return a.localeCompare(b, undefined, { sensitivity: 'base' });
        });
    }

    function computeProjectStats(projectName, days) {
        if (!projectName) return null;
        var targetKey = projectName.trim().toLowerCase();
        var cutoff = days ? (Date.now() - days * 86400000) : 0;

        var workerMap = new Map();
        var timelineMap = new Map();
        var taskFreq = {};
        var tradeFreq = {};

        function getOrCreateWorker(name, aNum, trade) {
            var key = (aNum ? String(aNum).trim() : '') || (name ? String(name).trim().toLowerCase() : 'unknown');
            if (!workerMap.has(key)) {
                workerMap.set(key, {
                    key: key,
                    name: name || (aNum ? 'Volunteer ' + aNum : 'Unknown Worker'),
                    aNumber: aNum || '',
                    trade: trade || '',
                    daysPresent: 0,
                    daysAbsent: 0,
                    daysPasses: 0,
                    totalDays: 0,
                    firstSeenDate: '',
                    firstSeenTs: Infinity,
                    lastSeenDate: '',
                    lastSeenTs: -Infinity,
                    tasks: new Set(),
                    areas: new Set(),
                    isCurrentlyAssigned: false,
                    currentTask: '',
                    currentArea: '',
                    records: []
                });
            }
            var entry = workerMap.get(key);
            if (!entry.trade && trade) entry.trade = trade;
            if (!entry.aNumber && aNum) entry.aNumber = aNum;
            return entry;
        }

        // 1. Process History Snapshots
        var snaps = (historyData || []).filter(function(s) {
            return !cutoff || (s.timestamp && s.timestamp >= cutoff);
        });

        snaps.forEach(function(snap) {
            var snapDate = formatDayMonthYear(snap.dateStr, snap.timestamp);
            var snapTs = snap.timestamp || 0;
            var matchingWorkersInSnap = [];

            (snap.data || []).forEach(function(w) {
                var p = _cleanProjectName(w.project);
                if (!p || p.toLowerCase() !== targetKey) return;

                var rec = {
                    date: snapDate,
                    timestamp: snapTs,
                    name: w.name,
                    aNumber: w.aNumber,
                    trade: w.trade || '',
                    area: w.area || '',
                    specificTask: w.specificTask || '',
                    dailyStatus: w.dailyStatus || 'Present',
                    absentReason: w.absentReason || ''
                };
                matchingWorkersInSnap.push(rec);

                var workerInfo = getOrCreateWorker(w.name, w.aNumber, w.trade);
                workerInfo.totalDays++;
                if (rec.dailyStatus === 'Present') workerInfo.daysPresent++;
                else if (rec.dailyStatus === 'Absent') workerInfo.daysAbsent++;
                else if (rec.dailyStatus === 'Passes') workerInfo.daysPasses++;

                if (rec.specificTask) {
                    workerInfo.tasks.add(rec.specificTask);
                    taskFreq[rec.specificTask] = (taskFreq[rec.specificTask] || 0) + 1;
                }
                if (rec.area) workerInfo.areas.add(rec.area);
                if (w.trade) tradeFreq[w.trade] = (tradeFreq[w.trade] || 0) + 1;

                if (snapTs < workerInfo.firstSeenTs) {
                    workerInfo.firstSeenTs = snapTs;
                    workerInfo.firstSeenDate = snapDate;
                }
                if (snapTs > workerInfo.lastSeenTs) {
                    workerInfo.lastSeenTs = snapTs;
                    workerInfo.lastSeenDate = snapDate;
                }
                workerInfo.records.push(rec);
            });

            if (matchingWorkersInSnap.length > 0) {
                timelineMap.set(snapDate, {
                    date: snapDate,
                    timestamp: snapTs,
                    workers: matchingWorkersInSnap,
                    headcount: matchingWorkersInSnap.filter(function(x) { return x.dailyStatus === 'Present'; }).length,
                    total: matchingWorkersInSnap.length
                });
            }
        });

        // 2. Correlate Live Roster (workersData)
        var liveWorkers = [];
        (workersData || []).forEach(function(w) {
            var p = _cleanProjectName(w.project);
            if (p && p.toLowerCase() === targetKey) {
                var workerInfo = getOrCreateWorker(w.name, w.aNumber, w.trade);
                workerInfo.isCurrentlyAssigned = true;
                workerInfo.currentTask = w.specificTask || '';
                workerInfo.currentArea = w.area || '';
                liveWorkers.push({
                    name: w.name,
                    aNumber: w.aNumber,
                    trade: w.trade || '',
                    area: w.area || '',
                    specificTask: w.specificTask || '',
                    taskStartedAt: w.taskStartedAt || '',
                    dailyStatus: w.dailyStatus || 'Present'
                });
            }
        });

        // 3. Correlate Assignment Log (transfers in & out)
        var transfers = [];
        (assignmentLog || []).forEach(function(log) {
            if (log.field !== 'Project') return;
            if (cutoff && log.timestamp && log.timestamp < cutoff) return;
            var pOld = _cleanProjectName(log.oldValue);
            var pNew = _cleanProjectName(log.newValue);
            var isInto = pNew && pNew.toLowerCase() === targetKey;
            var isOut = pOld && pOld.toLowerCase() === targetKey;

            if (isInto || isOut) {
                transfers.push({
                    id: log.id || '',
                    workerName: log.workerName,
                    aNumber: log.aNumber || '',
                    date: formatDayMonthYear(log.date, log.timestamp),
                    timestamp: log.timestamp || 0,
                    direction: isInto ? 'in' : 'out',
                    fromProject: pOld || '(none)',
                    toProject: pNew || '(none)',
                });
            }
        });
        transfers.sort(function(a, b) { return b.timestamp - a.timestamp; });

        // Worker list sorted by active presence, then name
        var workersList = Array.from(workerMap.values()).map(function(w) {
            return {
                key: w.key,
                name: w.name,
                aNumber: w.aNumber,
                trade: w.trade || 'General Worker',
                daysPresent: w.daysPresent,
                daysAbsent: w.daysAbsent,
                daysPasses: w.daysPasses,
                totalDays: w.totalDays,
                firstSeenDate: w.firstSeenDate || '-',
                lastSeenDate: w.lastSeenDate || (w.isCurrentlyAssigned ? 'Active Today' : '-'),
                tasks: Array.from(w.tasks),
                areas: Array.from(w.areas),
                isCurrentlyAssigned: w.isCurrentlyAssigned,
                currentTask: w.currentTask,
                currentArea: w.currentArea,
                records: w.records
            };
        }).sort(function(a, b) {
            if (b.isCurrentlyAssigned !== a.isCurrentlyAssigned) return b.isCurrentlyAssigned ? 1 : -1;
            if (b.daysPresent !== a.daysPresent) return b.daysPresent - a.daysPresent;
            return a.name.localeCompare(b.name);
        });

        // Timeline list sorted descending by date
        var timelineList = Array.from(timelineMap.values()).sort(function(a, b) {
            return b.timestamp - a.timestamp;
        });

        var sortedTrades = Object.entries(tradeFreq).sort(function(a, b) { return b[1] - a[1]; });
        var sortedTasks = Object.entries(taskFreq).sort(function(a, b) { return b[1] - a[1]; });

        var totalManDays = workersList.reduce(function(sum, w) { return sum + w.daysPresent; }, 0);
        var firstActive = timelineList.length > 0 ? formatDayMonthYear(timelineList[timelineList.length - 1].date) : '-';
        var lastActive = liveWorkers.length > 0 ? 'Currently Active' : (timelineList.length > 0 ? formatDayMonthYear(timelineList[0].date) : '-');

        return {
            projectName: projectName,
            workers: workersList,
            timeline: timelineList,
            transfers: transfers,
            liveWorkers: liveWorkers,
            totalUniqueWorkers: workersList.length,
            totalCurrentlyAssigned: liveWorkers.length,
            totalManDays: totalManDays,
            totalActiveDates: timelineList.length,
            firstActiveDate: firstActive,
            lastActiveDate: lastActive,
            topTrade: sortedTrades[0] ? sortedTrades[0][0] : '-',
            topTask: sortedTasks[0] ? sortedTasks[0][0] : '-',
        };
    }

    function printWorkerDossier(workerName, days) {
        var stats  = computeStats(workerName, days || _currentDays);
        var worker = (workersData || []).find(function(w) { return w.name === workerName; });
        var trade  = worker ? (worker.trade || '-') : '-';
        var eName  = esc(workerName);
        var period = PERIODS.find(function(p) { return p.days === (days || _currentDays); });
        var periodLabel = period ? period.label : 'Selected Period';
        var today  = new Date().toLocaleDateString('en-PH', { year: 'numeric', month: 'long', day: 'numeric' });
        var relColor = stats.reliability !== null
            ? (stats.reliability >= 90 ? '#10b981' : stats.reliability >= 70 ? '#f59e0b' : '#ef4444')
            : '#94a3b8';

        var areaRows = stats.areaRank.slice(0, 10).map(function(r, i) {
            return '<tr><td>' + (i+1) + '</td><td>' + (esc(r.value) || '(not set)') + '</td><td>' + r.count + '</td><td>' + r.pct + '%</td></tr>';
        }).join('') || '<tr><td colspan="4" style="text-align:center;color:#94a3b8;font-style:italic;">No data</td></tr>';

        var taskRows = stats.taskRank.slice(0, 10).map(function(r, i) {
            return '<tr><td>' + (i+1) + '</td><td>' + (esc(r.value) || '(not set)') + '</td><td>' + r.count + '</td><td>' + r.pct + '%</td></tr>';
        }).join('') || '<tr><td colspan="4" style="text-align:center;color:#94a3b8;font-style:italic;">No data</td></tr>';

        var sortedSnaps = _getSortedTimelineSnaps(stats.snaps, _timelineSortCol, _timelineSortDir);
        var timelineRows = sortedSnaps.map(function(s) {
            return '<tr>'
                + '<td>' + esc(formatDayMonthYear(s.date, s.timestamp)) + '</td>'
                + '<td style="font-weight:700;color:' + (STATUS_COLORS[s.dailyStatus] || '#94a3b8') + '">' + (esc(s.dailyStatus) || '-') + '</td>'
                + '<td>' + (esc(s.area) || '-') + '</td>'
                + '<td>' + (esc(s.project) || '-') + '</td>'
                + '<td>' + (esc(s.specificTask) || '-') + '</td>'
                + '</tr>';
        }).join('') || '<tr><td colspan="5" style="text-align:center;color:#94a3b8;font-style:italic;">No records</td></tr>';

        var html = '<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"><title>Worker Dossier \u2014 ' + eName + '</title>'
            + '<style>'
            + '* { box-sizing:border-box; margin:0; padding:0; }'
            + 'body { font-family: \'Inter\', -apple-system, BlinkMacSystemFont, \'Segoe UI\', Roboto, Helvetica, Arial, sans-serif; padding:32px; background:#fff; color:#1e293b; font-size:13px; line-height:1.5; }'
            + '.page-hdr { display:flex; align-items:flex-start; justify-content:space-between; margin-bottom:24px; padding-bottom:16px; border-bottom:2px solid #e2e8f0; }'
            + '.org-name { font-size:10px; font-weight:700; text-transform:uppercase; letter-spacing:.08em; color:#64748b; }'
            + 'h1 { font-size:20px; font-weight:800; color:#0f172a; margin-bottom:2px; }'
            + '.sub { font-size:11px; color:#64748b; }'
            + '.kpi-grid { display:grid; grid-template-columns:repeat(6,1fr); gap:10px; margin-bottom:20px; }'
            + '.kpi { border:1px solid #e2e8f0; border-radius:8px; padding:12px; background:#fff; }'
            + '.kpi-label { font-size:9px; font-weight:700; text-transform:uppercase; letter-spacing:.05em; color:#64748b; margin-bottom:4px; }'
            + '.kpi-val { font-size:20px; font-weight:800; }'
            + '.section { background:#fff; border:1px solid #e2e8f0; border-radius:8px; padding:16px; margin-bottom:16px; }'
            + '.section-title { font-size:11px; font-weight:800; text-transform:uppercase; letter-spacing:.05em; color:#475569; margin-bottom:12px; padding-bottom:6px; border-bottom:1px solid #f1f5f9; }'
            + 'table { width:100%; border-collapse:collapse; font-size:12px; }'
            + 'th { padding:8px 10px; text-align:left; font-size:9px; font-weight:700; text-transform:uppercase; letter-spacing:.05em; color:#64748b; background:#f8fafc; border-bottom:2px solid #e2e8f0; }'
            + 'td { padding:8px 10px; border-bottom:1px solid #f1f5f9; color:#334155; }'
            + 'tr:last-child td { border-bottom:none; }'
            + '.two-col { display:grid; grid-template-columns:1fr 1fr; gap:16px; }'
            + '.footer { margin-top:24px; font-size:10px; color:#94a3b8; text-align:center; border-top:1px solid #e2e8f0; padding-top:12px; }'
            + '@media print { body { background:white; } @page { size: auto; margin: 8mm 10mm; } }'
            + '</style></head><body>'
            + '<div class="page-hdr">'
            + '<div><div class="org-name">Service Section Workforce Command Center</div>'
            + '<h1>' + eName + '</h1>'
            + '<div class="sub">' + esc(trade) + ' &nbsp;&bull;&nbsp; Worker Dependability Dossier &nbsp;&bull;&nbsp; ' + periodLabel + '</div></div>'
            + '<div style="text-align:right;"><div class="org-name">Generated</div>'
            + '<div style="font-size:12px;font-weight:600;color:#0f172a;">' + today + '</div>'
            + '<div style="margin-top:6px;"><span style="display:inline-block;padding:3px 10px;border-radius:99px;font-size:10px;font-weight:700;background:' + relColor + '22;color:' + relColor + ';border:1px solid ' + relColor + '44;">'
            + 'Reliability: ' + (stats.reliability !== null ? stats.reliability + '%' : 'No data') + '</span></div></div></div>'
            + '<div class="kpi-grid">'
            + '<div class="kpi"><div class="kpi-label">Snapshot Days</div><div class="kpi-val">' + stats.total + '</div></div>'
            + '<div class="kpi"><div class="kpi-label">Present</div><div class="kpi-val" style="color:#10b981;">' + stats.present + '</div></div>'
            + '<div class="kpi"><div class="kpi-label">Absent</div><div class="kpi-val" style="color:#ef4444;">' + stats.absent + '</div></div>'
            + '<div class="kpi"><div class="kpi-label">Passes</div><div class="kpi-val" style="color:#f59e0b;">' + stats.passes + '</div></div>'
            + '<div class="kpi"><div class="kpi-label">Reliability</div><div class="kpi-val" style="color:' + relColor + ';">' + (stats.reliability !== null ? stats.reliability + '%' : '-') + '</div></div>'
            + '<div class="kpi"><div class="kpi-label">Versatility</div><div class="kpi-val" style="color:#3b82f6;">' + stats.versatility + '/100</div></div>'
            + '</div>'
            + '<div class="two-col">'
            + '<div class="section"><div class="section-title">Top Assigned Areas</div>'
            + '<table><thead><tr><th>#</th><th>Area</th><th>Days</th><th>%</th></tr></thead><tbody>' + areaRows + '</tbody></table></div>'
            + '<div class="section"><div class="section-title">Top Specific Tasks</div>'
            + '<table><thead><tr><th>#</th><th>Task</th><th>Days</th><th>%</th></tr></thead><tbody>' + taskRows + '</tbody></table></div>'
            + '</div>'
            + '<div class="section"><div class="section-title">Attendance Timeline (' + stats.snaps.length + ' records)</div>'
            + '<table><thead><tr><th>Date</th><th>Status</th><th>Area</th><th>Project</th><th>Specific Task</th></tr></thead>'
            + '<tbody>' + timelineRows + '</tbody></table></div>'
            + '<div class="footer">Service Section Workforce Command Center &nbsp;&bull;&nbsp; Analytics &amp; Dependability Report &nbsp;&bull;&nbsp; ' + today + '</div>'
            + '</body></html>';

        if (typeof window.safePrint === 'function') {
            window.safePrint(html);
        } else {
            var win = window.open('', '_blank');
            if (!win) { if (typeof showToast === 'function') showToast('Popup blocked. Please allow popups and try again.'); return; }
            win.document.write(html);
            win.document.close();
        }
    }

    function printDependabilityRoster(days) {
        var allStats   = computeAllStats(days || _currentDays);
        var period     = PERIODS.find(function(p) { return p.days === (days || _currentDays); });
        var periodLabel = period ? period.label : 'Selected Period';
        var today      = new Date().toLocaleDateString('en-PH', { year: 'numeric', month: 'long', day: 'numeric' });

        var rows = allStats.map(function(s, i) {
            var w = (workersData || []).find(function(x) { return x.name === s.workerName; });
            var wTrade = w ? (w.trade || '-') : '-';
            var primaryArea = s.areaRank[0] ? s.areaRank[0].value : '-';
            var primaryTask = s.taskRank[0] ? s.taskRank[0].value : '-';
            var relColor = s.reliability !== null
                ? (s.reliability >= 90 ? '#10b981' : stats.reliability >= 70 ? '#f59e0b' : '#ef4444')
                : '#94a3b8';
            return '<tr>'
                + '<td style="font-weight:800;color:' + (i < 3 ? '#d97706' : '#94a3b8') + ';">' + (i+1) + '</td>'
                + '<td style="font-weight:700;color:#0f172a;">' + esc(s.workerName) + '</td>'
                + '<td>' + esc(wTrade) + '</td>'
                + '<td style="font-weight:700;color:' + relColor + ';">' + (s.reliability !== null ? s.reliability + '%' : '-') + '</td>'
                + '<td style="color:#3b82f6;font-weight:700;">' + s.versatility + '/100</td>'
                + '<td>' + esc(primaryArea) + '</td>'
                + '<td>' + esc(primaryTask) + '</td>'
                + '<td style="color:#64748b;">' + s.total + '</td>'
                + '</tr>';
        }).join('') || '<tr><td colspan="8" style="text-align:center;color:#94a3b8;padding:24px;font-style:italic;">No snapshot data available.</td></tr>';

        var html = '<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"><title>Dependability Roster \u2014 ' + periodLabel + '</title>'
            + '<style>'
            + '* { box-sizing:border-box; margin:0; padding:0; }'
            + 'body { font-family: \'Inter\', -apple-system, BlinkMacSystemFont, \'Segoe UI\', Roboto, Helvetica, Arial, sans-serif; padding:24px; background:#fff; color:#1e293b; font-size:12px; line-height:1.4; }'
            + '.page-hdr { margin-bottom:20px; padding-bottom:12px; border-bottom:2px solid #e2e8f0; }'
            + '.org-name { font-size:9px; font-weight:700; text-transform:uppercase; letter-spacing:.08em; color:#64748b; margin-bottom:4px; }'
            + 'h1 { font-size:18px; font-weight:800; color:#0f172a; }'
            + '.sub { font-size:10px; color:#64748b; margin-top:2px; }'
            + 'table { width:100%; border-collapse:collapse; }'
            + 'th { padding:8px 10px; text-align:left; font-size:8px; font-weight:700; text-transform:uppercase; letter-spacing:.05em; color:#64748b; background:#f8fafc; border-bottom:2px solid #e2e8f0; }'
            + 'td { padding:8px 10px; border-bottom:1px solid #f1f5f9; }'
            + 'tr:last-child td { border-bottom:none; }'
            + '.footer { margin-top:16px; font-size:9px; color:#94a3b8; text-align:center; border-top:1px solid #e2e8f0; padding-top:10px; }'
            + '@media print { @page { size: auto; margin: 8mm 10mm; } }'
            + '</style></head><body>'
            + '<div class="page-hdr"><div class="org-name">Service Section Workforce Command Center</div>'
            + '<h1>Worker Dependability Roster</h1>'
            + '<div class="sub">' + periodLabel + ' &nbsp;&bull;&nbsp; ' + allStats.length + ' worker(s) with recorded history &nbsp;&bull;&nbsp; Generated ' + today + '</div></div>'
            + '<table><thead><tr>'
            + ['#','Name','Trade','Reliability','Versatility','Primary Area','Primary Task','Snapshot Days'].map(function(h) {
                return '<th>' + h + '</th>';
              }).join('')
            + '</tr></thead><tbody>' + rows + '</tbody></table>'
            + '<div class="footer">Service Section Workforce Command Center &nbsp;&bull;&nbsp; Analytics &amp; Dependability &nbsp;&bull;&nbsp; Sorted by Reliability Index (Highest First)</div>'
            + '</body></html>';

        if (typeof window.safePrint === 'function') {
            window.safePrint(html);
        } else {
            var win = window.open('', '_blank');
            if (!win) { if (typeof showToast === 'function') showToast('Popup blocked. Please allow popups and try again.'); return; }
            win.document.write(html);
            win.document.close();
        }
    }

    function printProjectDossier(projectName, days) {
        var stats = computeProjectStats(projectName || _currentProject, days || _currentDays);
        if (!stats) return;

        var period = PERIODS.find(function(p) { return p.days === (days || _currentDays); });
        var periodLabel = period ? period.label : 'Selected Period';
        var today = new Date().toLocaleDateString('en-PH', { year: 'numeric', month: 'long', day: 'numeric' });
        var eProj = esc(stats.projectName);

        // Manifest rows
        var manifestRows = stats.workers.map(function(w, i) {
            return '<tr>'
                + '<td>' + (i + 1) + '</td>'
                + '<td style="font-weight:700;color:#0f172a;">' + esc(w.name) + '</td>'
                + '<td>' + esc(w.aNumber || '-') + '</td>'
                + '<td>' + esc(w.trade) + '</td>'
                + '<td style="font-weight:700;">' + w.daysPresent + '</td>'
                + '<td>' + esc(w.firstSeenDate) + '</td>'
                + '<td>' + esc(w.lastSeenDate) + '</td>'
                + '<td style="font-size:11px;">' + (w.tasks.join(', ') || '-') + '</td>'
                + '<td>' + (w.isCurrentlyAssigned ? '<span style="color:#10b981;font-weight:700;">Active</span>' : 'Former') + '</td>'
                + '</tr>';
        }).join('') || '<tr><td colspan="9" style="text-align:center;color:#94a3b8;padding:16px;">No recorded personnel.</td></tr>';

        // Daily Activity Timeline summary (first 30 dates if many)
        var timelineRows = stats.timeline.slice(0, 30).map(function(tl) {
            var sampleWorkers = tl.workers.map(function(w) { return esc(w.name) + ' (' + esc(w.specificTask || w.trade || 'Worker') + ')'; }).join('; ');
            return '<tr>'
                + '<td style="font-weight:700;">' + esc(tl.date) + '</td>'
                + '<td style="font-weight:700;color:#10b981;">' + tl.headcount + '</td>'
                + '<td style="font-size:11px;color:#334155;">' + sampleWorkers + '</td>'
                + '</tr>';
        }).join('') || '<tr><td colspan="3" style="text-align:center;color:#94a3b8;padding:16px;">No snapshot timeline data.</td></tr>';

        // Reassignment Log rows
        var transferRows = stats.transfers.map(function(t) {
            var dir = t.direction === 'in' ? 'Assigned In' : 'Transferred Out';
            var dtObj2 = t.timestamp ? parseDateSafe(t.timestamp) : null;
            var timeFormatted = dtObj2 ? dtObj2.toLocaleTimeString() : '';
            return '<tr>'
                + '<td>' + esc(t.date) + ' ' + esc(timeFormatted) + '</td>'
                + '<td style="font-weight:700;">' + esc(t.workerName) + '</td>'
                + '<td>' + dir + '</td>'
                + '<td>' + esc(t.fromProject) + '</td>'
                + '<td>' + esc(t.toProject) + '</td>'
                + '</tr>';
        }).join('') || '<tr><td colspan="5" style="text-align:center;color:#94a3b8;padding:12px;">No recorded reassignment events.</td></tr>';

        var html = '<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"><title>Project Audit Dossier \u2014 ' + eProj + '</title>'
            + '<style>'
            + '* { box-sizing:border-box; margin:0; padding:0; }'
            + 'body { font-family: \'Inter\', -apple-system, BlinkMacSystemFont, \'Segoe UI\', Roboto, Helvetica, Arial, sans-serif; padding:28px; background:#fff; color:#1e293b; font-size:12px; line-height:1.45; }'
            + '.page-hdr { display:flex; align-items:flex-start; justify-content:space-between; margin-bottom:20px; padding-bottom:14px; border-bottom:2px solid #0f172a; }'
            + '.org-name { font-size:9px; font-weight:800; text-transform:uppercase; letter-spacing:.08em; color:#475569; margin-bottom:2px; }'
            + 'h1 { font-size:20px; font-weight:900; color:#0f172a; margin-bottom:2px; }'
            + '.doc-badge { display:inline-block; font-size:9px; font-weight:800; text-transform:uppercase; letter-spacing:.05em; background:#0f172a; color:#fff; padding:3px 8px; border-radius:4px; margin-top:4px; }'
            + '.kpi-grid { display:grid; grid-template-columns:repeat(5,1fr); gap:10px; margin-bottom:20px; }'
            + '.kpi { border:1px solid #cbd5e1; border-radius:6px; padding:10px 12px; background:#f8fafc; }'
            + '.kpi-label { font-size:8px; font-weight:800; text-transform:uppercase; letter-spacing:.05em; color:#64748b; margin-bottom:3px; }'
            + '.kpi-val { font-size:16px; font-weight:800; color:#0f172a; }'
            + '.section { margin-bottom:20px; }'
            + '.section-title { font-size:10px; font-weight:800; text-transform:uppercase; letter-spacing:.06em; color:#0f172a; margin-bottom:8px; padding-bottom:4px; border-bottom:1.5px solid #cbd5e1; display:flex; justify-content:space-between; }'
            + 'table { width:100%; border-collapse:collapse; font-size:11px; margin-bottom:6px; }'
            + 'th { padding:6px 8px; text-align:left; font-size:8px; font-weight:800; text-transform:uppercase; letter-spacing:.05em; color:#475569; background:#f1f5f9; border-bottom:1.5px solid #cbd5e1; }'
            + 'td { padding:6px 8px; border-bottom:1px solid #e2e8f0; vertical-align:top; }'
            + 'tr:last-child td { border-bottom:none; }'
            + '.signoff-box { border:1px solid #94a3b8; border-radius:6px; padding:14px; margin-top:24px; background:#fcfcfc; page-break-inside:avoid; }'
            + '.signoff-grid { display:grid; grid-template-columns:1fr 1fr 1fr; gap:20px; margin-top:20px; }'
            + '.sign-line { border-bottom:1px solid #0f172a; height:32px; margin-bottom:4px; }'
            + '.sign-label { font-size:9px; font-weight:700; color:#475569; text-transform:uppercase; }'
            + '.footer { margin-top:20px; font-size:9px; color:#94a3b8; text-align:center; border-top:1px solid #cbd5e1; padding-top:10px; }'
            + '@media print { @page { size: auto; margin: 8mm 10mm; } body { padding:0; } }'
            + '</style></head><body>'
            + '<div class="page-hdr">'
            + '<div>'
            + '<div class="org-name">Service Section Workforce Command Center &bull; Internal Affairs &amp; Operations Audit</div>'
            + '<h1>' + eProj + '</h1>'
            + '<div style="font-size:11px;color:#475569;margin-top:2px;">Official Project Personnel Roster &amp; Assignment Investigation Dossier</div>'
            + '<span class="doc-badge">Official Audit Record</span>'
            + '</div>'
            + '<div style="text-align:right;">'
            + '<div style="font-size:9px;font-weight:700;color:#64748b;text-transform:uppercase;">Audit Period</div>'
            + '<div style="font-size:11px;font-weight:700;color:#0f172a;">' + periodLabel + '</div>'
            + '<div style="font-size:9px;color:#64748b;margin-top:4px;">Date Generated: <strong>' + today + '</strong></div>'
            + '</div></div>'
            + '<div class="kpi-grid">'
            + '<div class="kpi"><div class="kpi-label">Distinct Personnel</div><div class="kpi-val">' + stats.totalUniqueWorkers + '</div></div>'
            + '<div class="kpi"><div class="kpi-label">Active on Site Now</div><div class="kpi-val" style="color:#10b981;">' + stats.totalCurrentlyAssigned + '</div></div>'
            + '<div class="kpi"><div class="kpi-label">Recorded Man-Days</div><div class="kpi-val">' + stats.totalManDays + '</div></div>'
            + '<div class="kpi"><div class="kpi-label">Active Timeline Span</div><div class="kpi-val" style="font-size:12px;margin-top:3px;">' + esc(stats.firstActiveDate) + ' &ndash; ' + esc(stats.lastActiveDate) + '</div></div>'
            + '<div class="kpi"><div class="kpi-label">Primary Trade</div><div class="kpi-val" style="font-size:13px;margin-top:2px;">' + esc(stats.topTrade) + '</div></div>'
            + '</div>'
            + '<div class="section"><div class="section-title"><span>1. Comprehensive Personnel Manifest (' + stats.workers.length + ' Recorded Workers)</span><span>Chronological by Days Assigned</span></div>'
            + '<table><thead><tr><th>#</th><th>Personnel Name</th><th>A#</th><th>Trade</th><th>Days Active</th><th>First Seen</th><th>Last Seen</th><th>Specific Tasks Logged</th><th>Status</th></tr></thead>'
            + '<tbody>' + manifestRows + '</tbody></table></div>'
            + '<div class="section"><div class="section-title"><span>2. Chronological Daily Assignments &amp; Shifts (' + stats.timeline.length + ' Dates Recorded)</span><span>Showing Recent Snapshots</span></div>'
            + '<table><thead><tr><th>Snapshot Date</th><th>Headcount</th><th>Personnel Present &amp; Assigned Specific Tasks</th></tr></thead>'
            + '<tbody>' + timelineRows + '</tbody></table></div>'
            + '<div class="section"><div class="section-title"><span>3. Movement &amp; Reassignment Audit Log (' + stats.transfers.length + ' Transfer Events)</span><span>Source Audit Trail</span></div>'
            + '<table><thead><tr><th>Date &amp; Time</th><th>Personnel Name</th><th>Movement</th><th>Previous Project</th><th>Assigned Project</th></tr></thead>'
            + '<tbody>' + transferRows + '</tbody></table></div>'
            + '<div class="signoff-box">'
            + '<div style="font-size:10px;font-weight:800;text-transform:uppercase;letter-spacing:.05em;color:#0f172a;">Official Audit Verification &amp; Investigation Sign-Off</div>'
            + '<div style="font-size:10px;color:#475569;margin-top:2px;">This document certifies that the historical assignment and workforce records above reflect verified database archives as of the date indicated.</div>'
            + '<div class="signoff-grid">'
            + '<div><div class="sign-line"></div><div class="sign-label">Auditing Officer / Investigator</div></div>'
            + '<div><div class="sign-line"></div><div class="sign-label">Project Coordinator / Supervisor</div></div>'
            + '<div><div class="sign-line"></div><div class="sign-label">Date &amp; Official Seal</div></div>'
            + '</div></div>'
            + '<div class="footer">Service Section Workforce Command Center &nbsp;&bull;&nbsp; Project Investigation Audit System &nbsp;&bull;&nbsp; Confidential Record &nbsp;&bull;&nbsp; Page 1 of 1</div>'
            + '</body></html>';

        if (typeof window.safePrint === 'function') {
            window.safePrint(html);
        } else {
            var win = window.open('', '_blank');
            if (!win) { if (typeof showToast === 'function') showToast('Popup blocked. Please allow popups and try again.'); return; }
            win.document.write(html);
            win.document.close();
        }
    }
return {
    printWorkerDossier: printWorkerDossier,
    printDependabilityRoster: printDependabilityRoster,
    printProjectDossier: printProjectDossier
};
})();

// ===== dashboard.js (departments & shops) - verbatim from 01_Rust_WC_DB/js/dashboard.js =====
var S5_dashboard = (function () {

function renderDeployabilityBanner(activeWorkers) {
    const banner = document.getElementById('dist-deployability-banner') || document.getElementById('dash-deployability-banner');
    if (!banner) return;

    let jobSite = 0, shops = 0, utilities = 0, office = 0, residential = 0, absentOrHold = 0;
    const total = activeWorkers.length;

    for (const w of activeWorkers) {
        const dep = typeof getWorkerDeployability === 'function' ? getWorkerDeployability(w) : null;
        if (!dep) continue;
        if (dep.tier === 'JOB_SITE') jobSite++;
        else if (dep.tier === 'SHOPS') shops++;
        else if (dep.tier === 'UTILITIES') utilities++;
        else if (dep.tier === 'OFFICE') office++;
        else if (dep.tier === 'RESIDENTIAL_ONLY') residential++;
        else absentOrHold++;
    }

    const pct = total > 0 ? Math.round((jobSite / total) * 100) : 0;
    const pJobSite = total > 0 ? ((jobSite / total) * 100).toFixed(1) : 0;
    const pShops = total > 0 ? ((shops / total) * 100).toFixed(1) : 0;
    const pUtils = total > 0 ? ((utilities / total) * 100).toFixed(1) : 0;
    const pOffice = total > 0 ? ((office / total) * 100).toFixed(1) : 0;
    const pRes = total > 0 ? ((residential / total) * 100).toFixed(1) : 0;
    const pNon = total > 0 ? ((absentOrHold / total) * 100).toFixed(1) : 0;

    banner.innerHTML = `
        <div class="table-card" style="padding: 16px 18px; border-left: 4px solid #10b981; background: var(--card-bg); box-shadow: var(--shadow-sm);">
            <div style="display:flex; justify-content:space-between; align-items:center; flex-wrap:wrap; gap:10px; margin-bottom:12px;">
                <div style="display:flex; align-items:center; gap:8px;">
                    <span style="font-size:1.15rem;">🎯</span>
                    <div>
                        <h3 style="margin:0; font-size:0.92rem; font-weight:700; color:var(--text-main);">Job-Site Deployability &amp; Capacity</h3>
                        <p style="margin:2px 0 0; font-size:0.72rem; color:var(--text-muted);">Real-time deployable capacity vs shop-bound, utility, and restricted workforce</p>
                    </div>
                </div>
                <div style="display:flex; align-items:center; gap:8px;">
                    <span class="badge" style="background:#dcfce7; color:#15803d; font-weight:700; font-size:0.75rem; padding:4px 10px; border-radius:9999px; border:1px solid #bbf7d0;">
                        ${jobSite} of ${total} (${pct}%) Field Deployable
                    </span>
                </div>
            </div>

            <!-- Segmented Capacity Bar -->
            <div style="width:100%; height:14px; background:#e2e8f0; border-radius:9999px; overflow:hidden; display:flex; margin-bottom:12px; box-shadow:inset 0 1px 2px rgba(0,0,0,0.08);">
                ${jobSite > 0 ? `<div title="Job-Site Deployable: ${jobSite} (${pJobSite}%)" style="width:${pJobSite}%; background:#10b981; transition:width 0.3s;"></div>` : ''}
                ${shops > 0 ? `<div title="Shops & Fabrication: ${shops} (${pShops}%)" style="width:${pShops}%; background:#06b6d4; transition:width 0.3s;"></div>` : ''}
                ${utilities > 0 ? `<div title="Facilities & Utilities: ${utilities} (${pUtils}%)" style="width:${pUtils}%; background:#6366f1; transition:width 0.3s;"></div>` : ''}
                ${office > 0 ? `<div title="Office & Admin: ${office} (${pOffice}%)" style="width:${pOffice}%; background:#3b82f6; transition:width 0.3s;"></div>` : ''}
                ${residential > 0 ? `<div title="Residential Only: ${residential} (${pRes}%)" style="width:${pRes}%; background:#f97316; transition:width 0.3s;"></div>` : ''}
                ${absentOrHold > 0 ? `<div title="Absent & Medical Hold: ${absentOrHold} (${pNon}%)" style="width:${pNon}%; background:#94a3b8; transition:width 0.3s;"></div>` : ''}
            </div>

            <!-- Metric Breakdown Chips -->
            <div style="display:grid; grid-template-columns:repeat(auto-fit, minmax(135px, 1fr)); gap:8px;">
                <div style="background:var(--bg-color); border:1px solid var(--border-color); border-radius:6px; padding:8px 10px; cursor:pointer;" onclick="openDepartmentRosterModal('Job-site Construction')">
                    <div style="display:flex; align-items:center; gap:5px; font-size:0.7rem; font-weight:700; color:#10b981;">
                        <span style="width:7px; height:7px; border-radius:50%; background:#10b981;"></span> Field Deployable
                    </div>
                    <div style="font-size:1.1rem; font-weight:800; color:var(--text-main); margin-top:2px;">${jobSite} <span style="font-size:0.7rem; font-weight:500; color:var(--text-muted);">(${pJobSite}%)</span></div>
                </div>

                <div style="background:var(--bg-color); border:1px solid var(--border-color); border-radius:6px; padding:8px 10px; cursor:pointer;" onclick="openDepartmentRosterModal('Woodland Shops')">
                    <div style="display:flex; align-items:center; gap:5px; font-size:0.7rem; font-weight:700; color:#06b6d4;">
                        <span style="width:7px; height:7px; border-radius:50%; background:#06b6d4;"></span> Shops &amp; Fab
                    </div>
                    <div style="font-size:1.1rem; font-weight:800; color:var(--text-main); margin-top:2px;">${shops} <span style="font-size:0.7rem; font-weight:500; color:var(--text-muted);">(${pShops}%)</span></div>
                </div>

                <div style="background:var(--bg-color); border:1px solid var(--border-color); border-radius:6px; padding:8px 10px; cursor:pointer;" onclick="openDepartmentRosterModal('Electrical Rm')">
                    <div style="display:flex; align-items:center; gap:5px; font-size:0.7rem; font-weight:700; color:#6366f1;">
                        <span style="width:7px; height:7px; border-radius:50%; background:#6366f1;"></span> Utilities &amp; Plant
                    </div>
                    <div style="font-size:1.1rem; font-weight:800; color:var(--text-main); margin-top:2px;">${utilities} <span style="font-size:0.7rem; font-weight:500; color:var(--text-muted);">(${pUtils}%)</span></div>
                </div>

                <div style="background:var(--bg-color); border:1px solid var(--border-color); border-radius:6px; padding:8px 10px; cursor:pointer;" onclick="openDepartmentRosterModal('Woodland Office')">
                    <div style="display:flex; align-items:center; gap:5px; font-size:0.7rem; font-weight:700; color:#3b82f6;">
                        <span style="width:7px; height:7px; border-radius:50%; background:#3b82f6;"></span> Woodland Office
                    </div>
                    <div style="font-size:1.1rem; font-weight:800; color:var(--text-main); margin-top:2px;">${office} <span style="font-size:0.7rem; font-weight:500; color:var(--text-muted);">(${pOffice}%)</span></div>
                </div>

                <div style="background:var(--bg-color); border:1px solid var(--border-color); border-radius:6px; padding:8px 10px; cursor:pointer;" onclick="openDepartmentRosterModal('Residential Only')">
                    <div style="display:flex; align-items:center; gap:5px; font-size:0.7rem; font-weight:700; color:#f97316;">
                        <span style="width:7px; height:7px; border-radius:50%; background:#f97316;"></span> Residential Only
                    </div>
                    <div style="font-size:1.1rem; font-weight:800; color:var(--text-main); margin-top:2px;">${residential} <span style="font-size:0.7rem; font-weight:500; color:var(--text-muted);">(${pRes}%)</span></div>
                </div>

                <div style="background:var(--bg-color); border:1px solid var(--border-color); border-radius:6px; padding:8px 10px; cursor:pointer;" onclick="openAreaRosterModal('Absent and On Passes')">
                    <div style="display:flex; align-items:center; gap:5px; font-size:0.7rem; font-weight:700; color:#94a3b8;">
                        <span style="width:7px; height:7px; border-radius:50%; background:#94a3b8;"></span> Absent / Hold
                    </div>
                    <div style="font-size:1.1rem; font-weight:800; color:var(--text-main); margin-top:2px;">${absentOrHold} <span style="font-size:0.7rem; font-weight:500; color:var(--text-muted);">(${pNon}%)</span></div>
                </div>
            </div>
        </div>
    `;
}

function renderDepartmentMatrix(activeWorkers) {
    // Render into every registered container (distribution view slot, or dashboard widget if present)
    const panels = [
        document.getElementById('dist-dept-panel'),
        document.getElementById('dash-department-panel')
    ].filter(Boolean);
    if (panels.length === 0) return;

    const depts = (typeof WORKER_DEPARTMENTS !== 'undefined') ? WORKER_DEPARTMENTS : [
        'Garden', 'Job-site Construction', 'Woodland Shops',
        'Residential Only', 'Electrical Rm', 'Water System', 'Woodland Office'
    ];

    const stats = {};
    depts.forEach(d => {
        stats[d] = { total: 0, present: 0, absent: 0, passes: 0, subShops: {} };
    });

    if (stats['Woodland Shops']) {
        const subShops = (typeof WOODLAND_SUB_SHOPS !== 'undefined') ? WOODLAND_SUB_SHOPS : [
            'Carpenter Shop', 'Painter Shop', 'Tinsmith Shop', 'Steelman/Welder Shop', 'Powder Coating Shop'
        ];
        subShops.forEach(s => { stats['Woodland Shops'].subShops[s] = { total: 0, present: 0 }; });
    }

    activeWorkers.forEach(w => {
        const d = w.department || (typeof inferWorkerDepartment === 'function' ? inferWorkerDepartment(w) : 'Job-site Construction');
        if (!stats[d]) stats[d] = { total: 0, present: 0, absent: 0, passes: 0, subShops: {} };
        const st = (w.dailyStatus || 'Present').toLowerCase().trim();
        if (st === 'present') stats[d].present++;
        else if (st === 'absent') stats[d].absent++;
        else if (st.startsWith('pass')) stats[d].passes++;

        if (d === 'Woodland Shops' && w.subDepartment) {
            if (!stats[d].subShops[w.subDepartment]) stats[d].subShops[w.subDepartment] = { total: 0, present: 0 };
            stats[d].subShops[w.subDepartment].total++;
            if (w.dailyStatus === 'Present') stats[d].subShops[w.subDepartment].present++;
        }
    });

    const meta = (typeof DEPARTMENT_META !== 'undefined') ? DEPARTMENT_META : {};

    let cardsHtml = depts.map(dept => {
        const m = meta[dept] || { icon: '📁', color: '#64748b', bg: '#f1f5f9', badge: 'DEPT' };
        const s = stats[dept] || { total: 0, present: 0, absent: 0, passes: 0, subShops: {} };
        const isWoodlandShops = dept === 'Woodland Shops';

        let subShopsHtml = '';
        if (isWoodlandShops && Object.keys(s.subShops).length > 0) {
            subShopsHtml = `
                <div style="margin-top:8px; padding-top:8px; border-top:1px dashed var(--border-color); display:flex; flex-wrap:wrap; gap:4px;">
                    ${Object.entries(s.subShops).map(([shopName, shopStat]) => {
                        const shopDesc = (typeof getWoodlandSubShopDescription === 'function') ? getWoodlandSubShopDescription(shopName) : '';
                        const tooltip = shopDesc ? `${shopName}: ${shopStat.present}/${shopStat.total} present : ${shopDesc}` : `${shopName}: ${shopStat.present}/${shopStat.total} present`;
                        return `<span class="notion-tag" onclick="event.stopPropagation(); openDepartmentRosterModal('Woodland Shops', '${esc(shopName)}');" title="${esc(tooltip)}" style="font-size:0.67rem; padding:2px 6px; border-radius:4px; background:#ecfeff; color:#0e7490; border:1px solid #cffafe; cursor:pointer;">
                            ${esc(shopName)}: <strong>${shopStat.present}</strong>/${shopStat.total}
                        </span>`;
                    }).join('')}
                </div>
            `;
        }

        const deptDescHtml = m.description
            ? `<div class="dept-card-desc" style="margin-top:8px; font-size:0.71rem; line-height:1.35; color:var(--text-muted);">${esc(m.description)}</div>`
            : '';

        return `
            <div class="hover-card dept-overview-card" onclick="openDepartmentRosterModal('${esc(dept)}')" style="background:var(--card-bg); border:1px solid var(--border-color); border-radius:8px; padding:12px 14px; cursor:pointer; display:flex; flex-direction:column; justify-content:space-between; transition:transform 0.15s, box-shadow 0.15s; position:relative; overflow:hidden;">
                <div style="position:absolute; top:0; left:0; width:4px; height:100%; background:${m.color};"></div>
                <div>
                    <div style="display:flex; justify-content:space-between; align-items:flex-start; margin-bottom:6px; gap:8px;">
                        <div style="display:flex; align-items:center; gap:6px; min-width:0;">
                            <span style="font-size:1.15rem; flex-shrink:0;">${m.icon}</span>
                            <span style="font-weight:700; font-size:0.85rem; color:var(--text-main); white-space:nowrap; overflow:hidden; text-overflow:ellipsis;">${esc(dept)}</span>
                        </div>
                        <span class="badge" style="background:${m.bg}; color:${m.color}; font-weight:700; font-size:0.65rem; padding:2px 7px; border-radius:9999px; flex-shrink:0;">
                            ${m.badge || (m.isDeployable ? 'Deployable' : 'Facility')}
                        </span>
                    </div>

                    <div style="display:flex; align-items:baseline; justify-content:space-between; margin-top:4px;">
                        <div>
                            <span style="font-size:1.35rem; font-weight:800; color:var(--text-main);">${s.present}</span>
                            <span style="font-size:0.75rem; color:var(--text-muted); font-weight:500;"> / ${s.total} present</span>
                        </div>
                        ${s.absent + s.passes > 0 ? `<span style="font-size:0.7rem; color:#ef4444; font-weight:600;">${s.absent + s.passes} off-site</span>` : ''}
                    </div>
                    ${deptDescHtml}
                </div>
                ${subShopsHtml}
            </div>
        `;
    }).join('');

    const innerHtml = `
        <div style="margin-bottom:12px; display:flex; justify-content:space-between; align-items:center;">
            <div style="display:flex; align-items:center; gap:6px;">
                <span style="font-size:1rem;">🏢</span>
                <h3 style="font-size:0.88rem; font-weight:700; color:var(--text-main); margin:0;">Departments &amp; Shops Overview</h3>
            </div>
            <span style="font-size:0.72rem; color:var(--text-muted);">Click any card to inspect deployable roster</span>
        </div>
        <div style="display:grid; grid-template-columns:repeat(auto-fit, minmax(260px, 1fr)); gap:10px;">
            ${cardsHtml}
        </div>
    `;
    panels.forEach(p => { p.innerHTML = innerHtml; });
}

function buildDonutChart(counts, total, styleGetter, size = 150, tipId = 'donut-tip', maxSlices = 8) {
    if (total === 0) return `<div style="width:${size}px;height:${size}px;display:flex;align-items:center;justify-content:center;color:var(--text-muted);font-size:0.75rem;">No data</div>`;

    let sorted = Object.entries(counts).sort((a, b) => b[1] - a[1]);
    if (sorted.length > maxSlices) {
        const top = sorted.slice(0, maxSlices - 1);
        const rest = sorted.slice(maxSlices - 1);
        const othersSum = rest.reduce((s, [, v]) => s + v, 0);
        sorted = [...top, [`Others (${rest.length})`, othersSum]];
    }

    const cx = size / 2, cy = size / 2;
    const R  = size * 0.37;   // outer radius
    const r  = size * 0.22;   // inner radius (hole)
    const gap = 0.018;        // gap in radians between segments

    let paths = '';
    let cumulAngle = -Math.PI / 2; // start at top

    sorted.forEach(([label, count]) => {
        const frac = count / total;
        if (frac <= 0) return;
        const angle = frac * 2 * Math.PI - gap;
        const startA = cumulAngle + gap / 2;
        const endA   = startA + angle;
        cumulAngle  += frac * 2 * Math.PI;

        const cos0 = Math.cos(startA), sin0 = Math.sin(startA);
        const cos1 = Math.cos(endA),   sin1 = Math.sin(endA);
        const large = angle > Math.PI ? 1 : 0;

        const isOther = label.startsWith('Others');
        const style   = isOther ? { primary: '#94a3b8' } : styleGetter(label);
        const color   = style.primary || style.text || '#94a3b8';

        const pct = (frac * 100).toFixed(1);
        const d = [
            `M ${cx + R * cos0} ${cy + R * sin0}`,
            `A ${R} ${R} 0 ${large} 1 ${cx + R * cos1} ${cy + R * sin1}`,
            `L ${cx + r * cos1} ${cy + r * sin1}`,
            `A ${r} ${r} 0 ${large} 0 ${cx + r * cos0} ${cy + r * sin0}`,
            'Z'
        ].join(' ');

        paths += `<path d="${d}" fill="${color}" opacity="0.88"
            data-label="${esc(label)}" data-count="${count}" data-pct="${pct}"
            style="cursor:pointer;transition:opacity 0.15s;"
            onmouseenter="this.style.opacity='1';var t=document.getElementById('${tipId}');if(t)t.textContent='${esc(label)}: ${count} (${pct}%)';"
            onmouseleave="this.style.opacity='0.88';var t=document.getElementById('${tipId}');if(t)t.textContent='';" />`;
    });

    const center_label = total.toLocaleString();
    return `
        <div style="position:relative;width:${size}px;height:${size}px;flex-shrink:0;">
            <svg width="${size}" height="${size}" viewBox="0 0 ${size} ${size}">
                <circle cx="${cx}" cy="${cy}" r="${R}" fill="none" stroke="var(--border-color)" stroke-width="${(R-r)}" opacity="0.15"/>
                ${paths}
                <text x="${cx}" y="${cy - 5}" text-anchor="middle" font-size="${size*0.13}" font-weight="800" fill="var(--text-main)">${center_label}</text>
                <text x="${cx}" y="${cy + size*0.1}" text-anchor="middle" font-size="${size*0.07}" font-weight="600" fill="var(--text-muted)">present</text>
            </svg>
            <div id="${tipId}" style="position:absolute;bottom:-20px;left:50%;transform:translateX(-50%);font-size:0.65rem;font-weight:600;color:var(--text-muted);white-space:nowrap;pointer-events:none;min-height:14px;"></div>
        </div>`;
}

function buildDonutLegend(counts, total, styleGetter, maxSlices = 8) {
    let sorted = Object.entries(counts).sort((a, b) => b[1] - a[1]);
    if (sorted.length > maxSlices) {
        const top = sorted.slice(0, maxSlices - 1);
        const rest = sorted.slice(maxSlices - 1);
        sorted = [...top, [`Others (${rest.length})`, rest.reduce((s,[,v])=>s+v,0)]];
    }
    return sorted.map(([label, count]) => {
        const isOther = label.startsWith('Others');
        const style   = isOther ? { primary: '#94a3b8' } : styleGetter(label);
        const color   = style.primary || style.text || '#94a3b8';
        const pct     = total > 0 ? ((count / total) * 100).toFixed(1) : '0.0';
        return `
            <div style="display:flex;align-items:center;gap:6px;padding:2px 0;">
                <span style="width:8px;height:8px;border-radius:2px;background:${color};flex-shrink:0;opacity:0.88;"></span>
                <span style="font-size:0.68rem;color:var(--text-main);flex:1;min-width:0;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;" title="${esc(label)}">${esc(label)}</span>
                <span style="font-size:0.66rem;font-weight:700;color:var(--text-muted);flex-shrink:0;">${count} <span style="font-weight:400;opacity:0.7;">(${pct}%)</span></span>
            </div>`;
    }).join('');
}

function renderDeptAnalytics(activeWorkers) {
    const container = document.getElementById('dist-analytics-enhanced');
    if (!container) return;

    // ── gather stats ──────────────────────────────────────────────
    let totalAll = 0, presentCount = 0, absentCount = 0, passesCount = 0;
    const areaCounts = {}, tradeCounts = {}, deptCounts = {}, statusCounts = {};

    activeWorkers.forEach(w => {
        if (!w || w.isArchived) return;
        totalAll++;
        const status = w.dailyStatus || 'Unknown';
        statusCounts[status] = (statusCounts[status] || 0) + 1;

        if (status === 'Present') {
            presentCount++;
            const area  = (w.area  || 'Unassigned').trim();
            const trade = (w.trade || 'Helper').trim();
            const dept  = w.department || (typeof inferWorkerDepartment === 'function' ? inferWorkerDepartment(w) : 'Job-site Construction');
            areaCounts[area]  = (areaCounts[area]  || 0) + 1;
            tradeCounts[trade]= (tradeCounts[trade] || 0) + 1;
            deptCounts[dept]  = (deptCounts[dept]  || 0) + 1;
        } else if (status === 'Absent')  absentCount++;
        else if  (status === 'Passes')  passesCount++;
    });

    const offSite = absentCount + passesCount;

    // ── status donut (special colours) ───────────────────────────
    function statusStyleGetter(s) {
        if (s === 'Present') return { primary: 'var(--color-notion-green, #2d6b49)' };
        if (s === 'Absent')  return { primary: 'var(--color-notion-red, #eb5757)' };
        if (s === 'Passes')  return { primary: 'var(--color-notion-orange, #d9730d)' };
        return { primary: 'var(--color-notion-gray, #59574f)' };
    }
    function deptStyleGetter(d) {
        if (typeof DEPARTMENT_META !== 'undefined' && DEPARTMENT_META[d]) return { primary: DEPARTMENT_META[d].color };
        return { primary: '#64748b' };
    }

    // ── stat summary row ──────────────────────────────────────────
    const attendance = totalAll > 0 ? ((presentCount / totalAll) * 100).toFixed(0) : 0;

    const statCards = `
        <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(130px,1fr));gap:10px;margin-bottom:20px;">
            ${[
                { label: 'Total Volunteers', value: totalAll,        icon: '👥', color: 'var(--text-main)', bg: 'var(--color-surface-secondary, #f1f1ef)' },
                { label: 'Present Today',    value: presentCount,    icon: '✅', color: 'var(--color-notion-green, #2d6b49)', bg: 'var(--color-notion-green-bg, #edf3ec)' },
                { label: 'Off-site',         value: offSite,         icon: '🚫', color: 'var(--color-notion-red, #eb5757)', bg: 'var(--color-notion-red-bg, #ffeeee)' },
                { label: 'Attendance Rate',  value: attendance+'%',  icon: '📊', color: 'var(--color-notion-blue, #1c5f82)', bg: 'var(--color-notion-blue-bg, #ebf3f5)' },
            ].map(s => `
                <div style="background:${s.bg};border:1px solid var(--border-color);border-radius:8px;padding:10px 14px;display:flex;align-items:center;gap:10px;">
                    <span style="font-size:1.3rem;">${s.icon}</span>
                    <div>
                        <div style="font-size:1.1rem;font-weight:800;color:${s.color};line-height:1;">${s.value}</div>
                        <div style="font-size:0.65rem;font-weight:600;color:${s.color};opacity:0.85;margin-top:2px;">${s.label}</div>
                    </div>
                </div>`).join('')}
        </div>`;

    // ── four charts (Operations / Department Analytics) ──────────
    const charts = [
        { title: '📋 Attendance Status', counts: statusCounts, total: totalAll,     getter: statusStyleGetter, tipId: 'tip-status' },
        { title: '🏢 By Department',     counts: deptCounts,   total: presentCount, getter: deptStyleGetter,   tipId: 'tip-dept'   },
        { title: '📍 By Area',           counts: areaCounts,   total: presentCount, getter: getAreaStyle,      tipId: 'tip-area'   },
        { title: '🔧 By Trade',          counts: tradeCounts,  total: presentCount, getter: getTradeStyle,     tipId: 'tip-trade'  },
    ].map(({ title, counts, total, getter, tipId }) => {
        const donutSize = 150;
        const donutHtml  = buildDonutChart(counts, total, getter, donutSize, tipId);
        const legendHtml = buildDonutLegend(counts, total, getter);
        return `
            <div style="background:var(--card-bg);border:1px solid var(--border-color);border-radius:8px;padding:14px 16px;display:flex;flex-direction:column;gap:12px;min-width:0;">
                <div style="font-size:0.75rem;font-weight:700;color:var(--text-main);">${title}</div>
                <div style="display:flex;align-items:flex-start;gap:14px;min-width:0;">
                    ${donutHtml}
                    <div style="flex:1;min-width:0;display:flex;flex-direction:column;gap:1px;max-height:130px;overflow-y:auto;scrollbar-width:none;">
                        ${legendHtml}
                    </div>
                </div>
            </div>`;
    }).join('');

    container.innerHTML = `
        <style>
            #dist-analytics-enhanced svg path { transition: opacity 0.15s, transform 0.15s; }
        </style>
        <div style="border-bottom:1px solid var(--border-color);padding-bottom:16px;margin-bottom:16px;">
            ${statCards}
            <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(280px,1fr));gap:12px;">
                ${charts}
            </div>
        </div>`;

    // Also trigger Housing & Residence overview update if panel exists
    if (document.getElementById('dist-housing-panel')) {
        renderHousingOverview(activeWorkers);
    }
}

function renderHousingOverview(activeWorkers) {
    const container = document.getElementById('dist-housing-panel');
    if (!container) return;

    const workers = (activeWorkers || (typeof workersData !== 'undefined' ? workersData : [])).filter(w => w && !w.isArchived);

    let totalVolunteers = 0, presentCount = 0;
    let workResAssigned = 0, housingAssigned = 0;
    const workResCounts = {}, housingCounts = {};
    const workResMap = {};
    const housingMap = {};

    workers.forEach(w => {
        totalVolunteers++;
        const isPresent = w.dailyStatus === 'Present';
        if (isPresent) presentCount++;

        const rawRes = (w.workResidence || '').trim();
        const rawHous = (w.visitationHousing || (typeof detectResidence === 'function' ? detectResidence(w) : '') || '').trim();

        if (rawRes) workResAssigned++;
        if (rawHous) housingAssigned++;

        const wRes = rawRes || 'Unassigned Residence';
        const wHous = rawHous || 'Unassigned Housing';

        if (isPresent) {
            workResCounts[wRes] = (workResCounts[wRes] || 0) + 1;
            housingCounts[wHous] = (housingCounts[wHous] || 0) + 1;
        }

        if (!workResMap[wRes]) {
            workResMap[wRes] = { total: 0, present: 0, absent: 0, passes: 0, departments: {} };
        }
        const st = (w.dailyStatus || 'Present').toLowerCase().trim();
        if (st === 'present') workResMap[wRes].present++;
        else if (st === 'absent') workResMap[wRes].absent++;
        else if (st.startsWith('pass')) workResMap[wRes].passes++;

        const dept = w.department || (typeof inferWorkerDepartment === 'function' ? inferWorkerDepartment(w) : 'Job-site Construction');
        workResMap[wRes].departments[dept] = (workResMap[wRes].departments[dept] || 0) + 1;

        if (!housingMap[wHous]) {
            housingMap[wHous] = { total: 0, present: 0, absent: 0, passes: 0, departments: {} };
        }
        housingMap[wHous].total++;
        if (st === 'present') housingMap[wHous].present++;
        else if (st === 'absent') housingMap[wHous].absent++;
        else if (st.startsWith('pass')) housingMap[wHous].passes++;

        housingMap[wHous].departments[dept] = (housingMap[wHous].departments[dept] || 0) + 1;
    });

    const workResRate = totalVolunteers > 0 ? Math.round((workResAssigned / totalVolunteers) * 100) : 0;
    const housingRate = totalVolunteers > 0 ? Math.round((housingAssigned / totalVolunteers) * 100) : 0;
    const unassignedRes = totalVolunteers - workResAssigned;
    const unassignedHous = totalVolunteers - housingAssigned;

    const resStyleGetter = typeof getResidenceStyle === 'function' ? getResidenceStyle : () => ({ primary: '#0284c7' });

    // ── Stat Cards for Housing Tab ──
    const statCards = `
        <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:10px;margin-bottom:16px;">
            ${[
                { label: 'Total Volunteers',   value: totalVolunteers, icon: '👥', color: 'var(--text-main)', bg: 'var(--color-surface-secondary, #f1f1ef)' },
                { label: 'In Work Residence',  value: `${workResAssigned} (${workResRate}%)`, icon: '🛏️', color: '#0369a1', bg: '#e0f2fe' },
                { label: 'In Housing Cluster', value: `${housingAssigned} (${housingRate}%)`, icon: '🏠', color: '#7c3aed', bg: '#ede9fe' },
                { label: 'Unassigned Resid.',  value: unassignedRes, icon: '⚠️', color: unassignedRes > 0 ? '#d97706' : 'var(--text-muted)', bg: unassignedRes > 0 ? '#fef3c7' : 'var(--color-surface-secondary, #f1f1ef)' },
                { label: 'Unassigned Housing', value: unassignedHous, icon: '⚠️', color: unassignedHous > 0 ? '#d97706' : 'var(--text-muted)', bg: unassignedHous > 0 ? '#fef3c7' : 'var(--color-surface-secondary, #f1f1ef)' },
            ].map(s => `
                <div style="background:${s.bg};border:1px solid var(--border-color);border-radius:8px;padding:10px 14px;display:flex;align-items:center;gap:10px;">
                    <span style="font-size:1.3rem;">${s.icon}</span>
                    <div>
                        <div style="font-size:1.1rem;font-weight:800;color:${s.color};line-height:1;">${s.value}</div>
                        <div style="font-size:0.65rem;font-weight:600;color:${s.color};opacity:0.85;margin-top:2px;">${s.label}</div>
                    </div>
                </div>`).join('')}
        </div>`;

    // ── Donut Charts for Housing Tab ──
    const housingCharts = [
        { title: '🛏️ By Work Residence',  counts: workResCounts, total: presentCount, getter: resStyleGetter, tipId: 'tip-dist-housing-work-res' },
        { title: '🏠 By Housing Cluster', counts: housingCounts, total: presentCount, getter: resStyleGetter, tipId: 'tip-dist-housing-cluster'  },
    ].map(({ title, counts, total, getter, tipId }) => {
        const donutSize = 150;
        const donutHtml  = buildDonutChart(counts, total, getter, donutSize, tipId);
        const legendHtml = buildDonutLegend(counts, total, getter);
        return `
            <div style="background:var(--card-bg);border:1px solid var(--border-color);border-radius:8px;padding:14px 16px;display:flex;flex-direction:column;gap:12px;min-width:0;">
                <div style="font-size:0.75rem;font-weight:700;color:var(--text-main);">${title}</div>
                <div style="display:flex;align-items:flex-start;gap:14px;min-width:0;">
                    ${donutHtml}
                    <div style="flex:1;min-width:0;display:flex;flex-direction:column;gap:1px;max-height:130px;overflow-y:auto;scrollbar-width:none;">
                        ${legendHtml}
                    </div>
                </div>
            </div>`;
    }).join('');

    const analyticsHeaderHtml = `
        <div style="margin-bottom:20px;padding-bottom:18px;border-bottom:1px solid var(--border-color);">
            ${statCards}
            <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(280px,1fr));gap:12px;">
                ${housingCharts}
            </div>
        </div>`;

    function renderLocationCard(name, data, type) {
        const isWorkRes = type === 'workResidence';
        const icon = isWorkRes ? '🛏️' : '🏠';
        const badgeLabel = isWorkRes ? 'Work Residence' : 'Housing Cluster';
        const style = typeof getResidenceStyle === 'function' ? getResidenceStyle(name) : { primary: '#4f46e5', bg: '#e0e7ff', text: '#4338ca' };
        const offSiteCount = data.absent + data.passes;

        const topDepts = Object.entries(data.departments).sort((a, b) => b[1] - a[1]).slice(0, 3);
        const tagsHtml = topDepts.map(([dept, count]) => {
            return `<span style="font-size:0.65rem; padding:1px 5px; border-radius:3px; background:var(--bg-color); color:var(--text-muted); border:1px solid var(--border-color);">${esc(dept)}: <strong>${count}</strong></span>`;
        }).join(' ');

        return `
            <div class="hover-card housing-overview-card" data-type="${type}" data-name="${esc(name).toLowerCase()}" onclick="openResidenceRosterModal('${type}', '${esc(name)}')" style="background:var(--card-bg); border:1px solid var(--border-color); border-radius:8px; padding:12px 14px; cursor:pointer; display:flex; flex-direction:column; justify-content:space-between; transition:transform 0.15s, box-shadow 0.15s; position:relative; overflow:hidden;">
                <div style="position:absolute; top:0; left:0; width:4px; height:100%; background:${style.primary};"></div>
                <div>
                    <div style="display:flex; justify-content:space-between; align-items:flex-start; margin-bottom:6px; gap:8px;">
                        <div style="display:flex; align-items:center; gap:6px; min-width:0;">
                            <span style="font-size:1.15rem; flex-shrink:0;">${icon}</span>
                            <span style="font-weight:700; font-size:0.85rem; color:var(--text-main); white-space:nowrap; overflow:hidden; text-overflow:ellipsis;" title="${esc(name)}">${esc(name)}</span>
                        </div>
                        <span class="badge" style="background:${style.bg}; color:${style.text}; font-weight:700; font-size:0.65rem; padding:2px 7px; border-radius:9999px; flex-shrink:0;">
                            ${badgeLabel}
                        </span>
                    </div>

                    <div style="display:flex; align-items:baseline; justify-content:space-between; margin-top:4px;">
                        <div>
                            <span style="font-size:1.35rem; font-weight:800; color:var(--text-main); font-variant-numeric:tabular-nums; line-height:1;">${data.present}</span>
                            <span style="font-size:0.75rem; color:var(--text-muted); font-weight:500;"> / ${data.total} present</span>
                        </div>
                        ${offSiteCount > 0 ? `<span style="font-size:0.7rem; color:#ef4444; font-weight:600;">${offSiteCount} off-site</span>` : ''}
                    </div>
                </div>

                ${tagsHtml ? `<div style="margin-top:8px; padding-top:8px; border-top:1px dashed var(--border-color); display:flex; flex-wrap:wrap; gap:4px;">${tagsHtml}</div>` : ''}
            </div>
        `;
    }

    const sortedWorkRes = Object.entries(workResMap).sort((a, b) => b[1].total - a[1].total);
    const sortedHousing = Object.entries(housingMap).sort((a, b) => b[1].total - a[1].total);

    const workResCardsHtml = sortedWorkRes.map(([name, data]) => renderLocationCard(name, data, 'workResidence')).join('');
    const housingCardsHtml = sortedHousing.map(([name, data]) => renderLocationCard(name, data, 'housing')).join('');

    const totalLocations = sortedWorkRes.length + sortedHousing.length;

    container.innerHTML = `
        <style>
            #dist-housing-panel svg path { transition: opacity 0.15s, transform 0.15s; }
        </style>
        ${analyticsHeaderHtml}
        <div>
            <div style="margin-bottom:14px; display:flex; justify-content:space-between; align-items:center; flex-wrap:wrap; gap:10px;">
                <div style="display:flex; align-items:center; gap:8px;">
                    <span style="font-size:1.15rem;">🏡</span>
                    <div>
                        <h3 style="font-size:0.88rem; font-weight:700; color:var(--text-main); margin:0;">Work Residence &amp; Housing Overview</h3>
                        <p style="font-size:0.7rem; color:var(--text-muted); margin:2px 0 0 0;">Click any card to inspect resident volunteer roster</p>
                    </div>
                </div>

                <div style="display:flex; align-items:center; flex-wrap:wrap; gap:8px;">
                    <div style="display:flex; background:var(--bg-color); border:1px solid var(--border-color); border-radius:6px; padding:2px;" id="housing-overview-filter-bar">
                        <button type="button" class="btn-housing-filter active" data-filter="all" style="padding:4px 10px; font-size:0.72rem; font-weight:700; border:none; border-radius:4px; background:var(--color-primary); color:#ffffff; cursor:pointer; transition:all 0.15s;">
                            All (${totalLocations})
                        </button>
                        <button type="button" class="btn-housing-filter" data-filter="workResidence" style="padding:4px 10px; font-size:0.72rem; font-weight:700; border:none; border-radius:4px; background:transparent; color:var(--text-muted); cursor:pointer; transition:all 0.15s;">
                            🛏️ Work Residences (${sortedWorkRes.length})
                        </button>
                        <button type="button" class="btn-housing-filter" data-filter="housing" style="padding:4px 10px; font-size:0.72rem; font-weight:700; border:none; border-radius:4px; background:transparent; color:var(--text-muted); cursor:pointer; transition:all 0.15s;">
                            🏠 Housing Clusters (${sortedHousing.length})
                        </button>
                    </div>

                    <div style="position:relative; min-width:180px; max-width:240px;">
                        <input type="text" id="housing-overview-search" placeholder="Search residence / housing..." style="width:100%; padding:4px 10px 4px 26px; font-size:0.75rem; border:1px solid var(--border-color); border-radius:var(--radius-sm, 4px); background:var(--bg-color); color:var(--text-main);">
                        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="position:absolute; left:8px; top:50%; transform:translateY(-50%); color:var(--text-muted); pointer-events:none;"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/></svg>
                    </div>
                </div>
            </div>

            <div id="housing-overview-grid" style="display:grid; grid-template-columns:repeat(auto-fit, minmax(260px, 1fr)); gap:10px;">
                ${workResCardsHtml}
                ${housingCardsHtml}
            </div>

            <div id="housing-overview-no-match" style="display:none; text-align:center; padding:28px 16px; color:var(--text-muted); font-size:0.8rem; font-style:italic;">
                No residences or housing clusters match the search criteria.
            </div>
        </div>
    `;

    // Attach filter & search listeners
    const gridEl = document.getElementById('housing-overview-grid');
    const searchEl = document.getElementById('housing-overview-search');
    const noMatchEl = document.getElementById('housing-overview-no-match');
    const filterBtns = container.querySelectorAll('.btn-housing-filter');

    let currentFilter = 'all';

    function applyHousingFilter() {
        const query = searchEl ? searchEl.value.trim().toLowerCase() : '';
        const cards = gridEl ? Array.from(gridEl.querySelectorAll('.housing-overview-card')) : [];
        let visibleCount = 0;

        cards.forEach(card => {
            const matchesType = currentFilter === 'all' || card.dataset.type === currentFilter;
            const matchesQuery = !query || card.dataset.name.includes(query);

            if (matchesType && matchesQuery) {
                card.style.display = 'flex';
                visibleCount++;
            } else {
                card.style.display = 'none';
            }
        });

        if (noMatchEl) noMatchEl.style.display = visibleCount === 0 ? 'block' : 'none';
    }

    filterBtns.forEach(btn => {
        btn.addEventListener('click', () => {
            filterBtns.forEach(b => {
                b.classList.remove('active');
                b.style.background = 'transparent';
                b.style.color = 'var(--text-muted)';
            });
            btn.classList.add('active');
            btn.style.background = 'var(--color-primary)';
            btn.style.color = '#ffffff';
            currentFilter = btn.dataset.filter;
            applyHousingFilter();
        });
    });

    if (searchEl) {
        searchEl.addEventListener('input', applyHousingFilter);
    }
}

function openDepartmentRosterModal(deptName, subDeptName) {
    const modal = document.getElementById('area-roster-modal');
    const title = document.getElementById('area-roster-title');
    const body = document.getElementById('area-roster-body');
    if (!modal || !body) return;

    const isWoodlandShops = deptName === 'Woodland Shops';

    // Filter to strictly deployable (Present) workers; exclude absent and on-pass personnel
    let workers = workersData.filter(w => {
        if (!w || w.isArchived) return false;
        const wDept = w.department || (typeof inferWorkerDepartment === 'function' ? inferWorkerDepartment(w) : 'Job-site Construction');
        if (wDept !== deptName) return false;
        if (w.dailyStatus === 'Absent' || w.dailyStatus === 'Passes') return false;
        return true;
    });

    const meta = (typeof DEPARTMENT_META !== 'undefined' && DEPARTMENT_META[deptName]) ? DEPARTMENT_META[deptName] : { icon: '🏢' };
    const headerLabel = subDeptName ? `${meta.icon} ${deptName} : ${subDeptName}` : `${meta.icon} ${deptName}`;
    title.textContent = `${headerLabel} (${workers.length} deployable)`;

    if (workers.length === 0) {
        body.innerHTML = `<div style="text-align:center; padding:36px 16px; color:var(--text-muted); font-style:italic; font-size:0.85rem;">No deployable volunteers currently present in this department today.</div>`;
        modal.classList.add('active');
        if (window.trapFocus) window.trapFocus(modal);
        return;
    }

    const subShopIcons = {
        'Carpenter Shop': '🪚',
        'Painter Shop': '🎨',
        'Tinsmith Shop': '🔨',
        'Steelman/Welder Shop': '⚡',
        'Powder Coating Shop': '🛡️',
        'Electrical Shop': '💡'
    };

    // For Woodland Shops, group volunteers by Shop Specialty; for other departments, group by area
    const groupKeyFn = isWoodlandShops
        ? (w) => w.subDepartment || (typeof inferWorkerSubDepartment === 'function' ? inferWorkerSubDepartment(w) : '') || 'General Shop'
        : (w) => w.area || 'Unassigned';

    const groupCounts = {};
    workers.forEach(w => {
        const k = groupKeyFn(w);
        groupCounts[k] = (groupCounts[k] || 0) + 1;
    });

    const groups = {};
    workers.forEach(w => {
        const k = groupKeyFn(w);
        if (!groups[k]) groups[k] = [];
        groups[k].push(w);
    });

    const sortedGroupNames = Object.keys(groups).sort((a, b) => {
        if (isWoodlandShops) {
            const priority = ['Carpenter Shop', 'Painter Shop', 'Tinsmith Shop', 'Steelman/Welder Shop', 'Powder Coating Shop', 'Electrical Shop'];
            const idxA = priority.indexOf(a);
            const idxB = priority.indexOf(b);
            if (idxA !== -1 && idxB !== -1) return idxA - idxB;
            if (idxA !== -1) return -1;
            if (idxB !== -1) return 1;
            return a.localeCompare(b);
        }
        if (a === 'Unassigned') return 1;
        if (b === 'Unassigned') return -1;
        if (typeof window.areaPriorityComparator === 'function') {
            return window.areaPriorityComparator(a, b);
        }
        return a.localeCompare(b);
    });

    sortedGroupNames.forEach(grp => {
        groups[grp].sort((a, b) => (a.dailyStatus === 'Present' ? 0 : 1) - (b.dailyStatus === 'Present' ? 0 : 1) || (a.trade || '').localeCompare(b.trade || '') || (a.name || '').localeCompare(b.name || ''));
    });

    const tradeCounts = {};
    workers.forEach(w => {
        const tr = w.trade || 'Unassigned';
        tradeCounts[tr] = (tradeCounts[tr] || 0) + 1;
    });
    const sortedTrades = Object.keys(tradeCounts).sort();

    // Determine default active filter chip (subDeptName if specified, otherwise 'all')
    const initialFilter = (isWoodlandShops && subDeptName && groups[subDeptName]) ? subDeptName : 'all';

    let html = `
        <div style="display: flex; flex-direction: column; gap: 12px; margin-bottom: 14px;">
            ${meta.description ? `
                <div class="roster-dept-banner" style="font-size: 0.74rem; line-height: 1.4; color: var(--text-muted); background: var(--bg-color); padding: 8px 12px; border-radius: 6px; border-left: 3px solid ${meta.color || 'var(--color-primary)'}; display: flex; align-items: flex-start; gap: 8px;">
                    <span style="font-size: 0.95rem; flex-shrink: 0; margin-top: 1px;">ℹ️</span>
                    <div>
                        <strong style="color: var(--text-main);">${esc(deptName)}:</strong> ${esc(meta.description)}
                    </div>
                </div>
            ` : ''}
            <div style="display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 10px; padding-bottom: 10px; border-bottom: 1px solid var(--border-color);">
                <div style="display: flex; align-items: center; flex-wrap: wrap; gap: 6px;" id="roster-trade-filter-bar">
                    <button class="roster-filter-chip ${initialFilter === 'all' ? 'active' : ''}" data-filter="all" style="padding: 3px 10px; font-size: 0.72rem; font-weight: 700; border-radius: var(--radius-full, 9999px); border: 1px solid var(--border-color); background: ${initialFilter === 'all' ? 'var(--color-primary)' : 'var(--card-bg)'}; color: ${initialFilter === 'all' ? '#ffffff' : 'var(--text-main)'}; cursor: pointer; transition: all 0.15s;">
                        All (${workers.length})
                    </button>
    `;

    if (isWoodlandShops) {
        sortedGroupNames.forEach(shopName => {
            const isSelected = initialFilter === shopName;
            const icon = subShopIcons[shopName] || '🏭';
            html += `
                <button class="roster-filter-chip ${isSelected ? 'active' : ''}" data-filter="${esc(shopName)}" style="padding: 3px 10px; font-size: 0.72rem; font-weight: 700; border-radius: var(--radius-full, 9999px); border: 1px solid var(--border-color); background: ${isSelected ? 'var(--color-primary)' : 'var(--card-bg)'}; color: ${isSelected ? '#ffffff' : 'var(--text-main)'}; cursor: pointer; transition: all 0.15s; display: inline-flex; align-items: center; gap: 5px;">
                    <span>${icon}</span>
                    <span>${esc(shopName)}</span>
                    <span style="font-size: 0.65rem; padding: 0 5px; border-radius: 9999px; background: ${isSelected ? 'rgba(255,255,255,0.2)' : 'var(--primary-light, #e0e7ff)'}; color: ${isSelected ? '#ffffff' : 'var(--primary-color, #4f46e5)'};">${groupCounts[shopName]}</span>
                </button>
            `;
        });
    } else {
        sortedTrades.forEach(trade => {
            const style = getTradeStyle(trade);
            const isSelected = initialFilter === trade;
            html += `
                <button class="roster-filter-chip ${isSelected ? 'active' : ''}" data-filter="${esc(trade)}" style="padding: 3px 10px; font-size: 0.72rem; font-weight: 700; border-radius: var(--radius-full, 9999px); border: 1px solid var(--border-color); background: ${isSelected ? 'var(--color-primary)' : 'var(--card-bg)'}; color: ${isSelected ? '#ffffff' : 'var(--text-main)'}; cursor: pointer; transition: all 0.15s; display: inline-flex; align-items: center; gap: 5px;">
                    <span style="width: 7px; height: 7px; border-radius: 50%; background: ${style.text || 'var(--color-primary)'};"></span>
                    <span>${esc(trade)}</span>
                    <span style="font-size: 0.65rem; padding: 0 5px; border-radius: 9999px; background: ${style.bg || 'var(--primary-light)'}; color: ${style.text || 'var(--primary-color)'};">${tradeCounts[trade]}</span>
                </button>
            `;
        });
    }

    html += `
                </div>
                <div style="position: relative; min-width: 180px; max-width: 240px; flex: 1;">
                    <input type="text" id="roster-quick-search" placeholder="Search name, trade, task..." autofocus style="width: 100%; padding: 4px 10px 4px 26px; font-size: 0.75rem; border: 1px solid var(--border-color); border-radius: var(--radius-sm, 4px); background: var(--bg-color); color: var(--text-main);">
                    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="position: absolute; left: 8px; top: 50%; transform: translateY(-50%); color: var(--text-muted); pointer-events: none;"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/></svg>
                </div>
            </div>

            <div id="area-roster-sections" style="display: flex; flex-direction: column; gap: 16px;">
    `;

    sortedGroupNames.forEach(grpName => {
        const grpWorkers = groups[grpName];
        const grpStyle = typeof getAreaStyle === 'function' ? getAreaStyle(grpName) : { primary: 'var(--color-primary)' };
        const iconPrefix = isWoodlandShops ? (subShopIcons[grpName] || '🏭') : '';
        const shopDesc = isWoodlandShops
            ? ((typeof getWoodlandSubShopDescription === 'function') ? getWoodlandSubShopDescription(grpName) : '')
            : '';

        html += `
            <div class="roster-area-section" data-group="${esc(grpName)}" style="display: flex; flex-direction: column; gap: 8px;">
                <div style="display: flex; flex-direction: column; gap: 2px; padding-bottom: 4px; border-bottom: 1px solid var(--border-color);">
                    <div style="display: flex; align-items: center; gap: 8px;">
                        ${iconPrefix ? `<span style="font-size: 0.95rem;">${iconPrefix}</span>` : `<span style="width: 8px; height: 8px; border-radius: 50%; background: ${grpStyle.primary || 'var(--color-primary)'};"></span>`}
                        <h4 style="font-size: 0.82rem; font-weight: 700; color: var(--text-main); margin: 0;">${esc(grpName)}</h4>
                        <span style="font-size: 0.68rem; color: var(--text-muted); font-weight: 500;">(${grpWorkers.length})</span>
                    </div>
                    ${shopDesc ? `<div class="roster-shop-desc" style="font-size: 0.68rem; color: var(--text-muted); padding-left: ${iconPrefix ? '24px' : '16px'}; line-height: 1.35;">${esc(shopDesc)}</div>` : ''}
                </div>
                <div class="worker-roster-grid-4col" style="display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 8px;">
        `;

        grpWorkers.forEach(w => {
            const style = getTradeStyle(w.trade);
            const avatarHtml = (w.photoUrl || w.photo)
                ? `<img src="${esc(w.photoUrl || w.photo)}" alt="${esc(w.name)}" style="width: 36px; height: 36px; border-radius: 50%; object-fit: cover; border: 1px solid var(--border-color); flex-shrink: 0;">`
                : `<div style="width: 36px; height: 36px; border-radius: 50%; background: ${style.bg || 'var(--primary-light)'}; color: ${style.text || 'var(--primary-color)'}; display: flex; align-items: center; justify-content: center; font-size: 0.8rem; font-weight: 700; flex-shrink: 0; border: 1px solid ${style.text || 'transparent'}; opacity: 0.95;">${(w.name || '?').charAt(0).toUpperCase()}</div>`;

            const statusBadge = w.dailyStatus === 'Present'
                ? `<span style="font-size:0.63rem; font-weight:700; color:var(--color-success); background:var(--color-success-bg); border:1px solid var(--color-success-border); padding:1px 5px; border-radius:3px;">Present</span>`
                : `<span style="font-size:0.63rem; font-weight:700; color:var(--color-danger); background:var(--color-danger-bg); border:1px solid var(--color-danger-border); padding:1px 5px; border-radius:3px;">${esc(w.dailyStatus)}</span>`;

            const workerShop = w.subDepartment || (typeof inferWorkerSubDepartment === 'function' ? inferWorkerSubDepartment(w) : '');
            const workerAge = (typeof getWorkerAge === 'function') ? getWorkerAge(w) : null;
            const ageBadge = (workerAge != null && workerAge > 0)
                ? `<span class="worker-roster-age" style="font-size: 0.65rem; font-weight: 700; padding: 1px 5px; border-radius: 3px; background: var(--bg-color); color: var(--text-muted); border: 1px solid var(--border-color);" title="Age: ${workerAge} years old">${workerAge} yrs</span>`
                : '';

            const taskText = w.area ? (w.area + (w.specificTask ? ' · ' + w.specificTask : '')) : (w.specificTask || 'No Area Assigned');

            html += `
                <div class="hover-card worker-roster-card" data-trade="${esc(w.trade || '')}" data-shop="${esc(workerShop)}" data-area="${esc(w.area || '')}" data-name="${esc(w.name || '').toLowerCase()}" data-task="${esc(w.specificTask || '').toLowerCase()}" data-skill="${esc(w.skillLevel || '').toLowerCase()}" style="display: flex; align-items: flex-start; gap: 9px; background: var(--card-bg); border: 1px solid var(--border-color); padding: 8px 10px; border-radius: var(--radius-sm, 4px); box-shadow: var(--shadow-sm); min-width: 0; transition: transform 0.15s, box-shadow 0.15s, border-color 0.15s;">
                    ${avatarHtml}
                    <div style="flex: 1; min-width: 0; line-height: 1.25;">
                        <div style="display: flex; align-items: baseline; justify-content: space-between; gap: 6px;">
                            <span style="font-weight: 700; font-size: 0.8rem; color: var(--text-main); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;" title="${esc(w.name)}">${esc(w.name)}</span>
                            <div style="display: flex; align-items: center; gap: 4px; flex-shrink: 0;">
                                ${ageBadge}
                                ${w.aNumber != null ? `<span style="font-size: 0.65rem; font-weight: 700; color: var(--text-muted);">#${esc(w.aNumber)}</span>` : ''}
                            </div>
                        </div>
                        <div style="display: flex; align-items: center; gap: 4px; margin-top: 3px; flex-wrap: wrap;">
                            ${statusBadge}
                            <span style="font-size: 0.67rem; font-weight: 700; padding: 1px 6px; border-radius: 3px; background: ${style.bg}; color: ${style.text}; text-transform: uppercase; letter-spacing: 0.03em;">${esc(w.trade || 'General')}</span>
                            ${workerShop ? `<span style="font-size: 0.65rem; font-weight: 700; padding: 1px 5px; border-radius: 3px; background: #ecfeff; color: #0e7490; border: 1px solid #cffafe;">${esc(workerShop)}</span>` : ''}
                        </div>
                        <div class="worker-roster-task" style="font-size: 0.68rem; color: var(--text-muted); margin-top: 4px; line-height: 1.35; word-break: break-word; white-space: normal;" title="${esc(taskText)}">
                            ${esc(taskText)}
                        </div>
                    </div>
                </div>
            `;
        });

        html += `
                </div>
            </div>
        `;
    });

    html += `
            </div>
            <div id="roster-no-match" style="display: none; text-align: center; padding: 24px; color: var(--text-muted); font-size: 0.8rem; font-style: italic;">
                No workers match the selected filter.
            </div>
        </div>
    `;

    body.innerHTML = html;

    const searchInput = document.getElementById('roster-quick-search');
    const noMatchEl = document.getElementById('roster-no-match');
    const sections = Array.from(body.querySelectorAll('.roster-area-section'));

    function applyFilter() {
        const activeChip = body.querySelector('.roster-filter-chip.active');
        const activeFilter = activeChip ? activeChip.dataset.filter : 'all';
        const query = searchInput ? searchInput.value.trim().toLowerCase() : '';

        let visibleTotal = 0;

        sections.forEach(section => {
            const cards = Array.from(section.querySelectorAll('.worker-roster-card'));
            let sectionVisibleCount = 0;

            cards.forEach(card => {
                const matchesFilter = (activeFilter === 'all' || 
                    (isWoodlandShops ? (card.dataset.shop === activeFilter || section.dataset.group === activeFilter) : card.dataset.trade === activeFilter));
                const matchesQuery = (!query || 
                    card.dataset.name.includes(query) || 
                    card.dataset.task.includes(query) || 
                    card.dataset.trade.toLowerCase().includes(query) ||
                    (card.dataset.shop && card.dataset.shop.toLowerCase().includes(query)) ||
                    (card.dataset.area && card.dataset.area.toLowerCase().includes(query)));

                if (matchesFilter && matchesQuery) {
                    card.style.display = 'flex';
                    sectionVisibleCount++;
                    visibleTotal++;
                } else {
                    card.style.display = 'none';
                }
            });

            if (sectionVisibleCount === 0) {
                section.style.display = 'none';
            } else {
                section.style.display = 'flex';
            }
        });

        if (noMatchEl) noMatchEl.style.display = visibleTotal === 0 ? 'block' : 'none';
    }

    body.querySelectorAll('.roster-filter-chip').forEach(chip => {
        chip.addEventListener('click', () => {
            body.querySelectorAll('.roster-filter-chip').forEach(c => {
                c.classList.remove('active');
                c.style.background = 'var(--card-bg)';
                c.style.color = 'var(--text-main)';
                const badge = c.querySelector('span:last-child');
                if (badge && isWoodlandShops) {
                    badge.style.background = 'var(--primary-light, #e0e7ff)';
                    badge.style.color = 'var(--primary-color, #4f46e5)';
                }
            });
            chip.classList.add('active');
            chip.style.background = 'var(--color-primary)';
            chip.style.color = '#ffffff';
            const activeBadge = chip.querySelector('span:last-child');
            if (activeBadge && isWoodlandShops) {
                activeBadge.style.background = 'rgba(255,255,255,0.2)';
                activeBadge.style.color = '#ffffff';
            }
            applyFilter();
        });
    });

    if (searchInput) {
        searchInput.addEventListener('input', applyFilter);
    }

    // If pre-filtered to a specific sub-department, trigger filter immediately
    if (initialFilter !== 'all') {
        applyFilter();
    }

    modal.classList.add('active');
    if (window.trapFocus) window.trapFocus(modal);
}

function openResidenceRosterModal(type, locationName) {
    const modal = document.getElementById('area-roster-modal');
    const title = document.getElementById('area-roster-title');
    const body = document.getElementById('area-roster-body');
    if (!modal || !body) return;

    const isWorkRes = type === 'workResidence';
    const isUnassigned = locationName.startsWith('Unassigned');
    const icon = isWorkRes ? '🛏️' : '🏠';
    const typeLabel = isWorkRes ? 'Work Residence' : 'Housing Cluster';

    const workers = (typeof workersData !== 'undefined' ? workersData : []).filter(w => {
        if (!w || w.isArchived) return false;
        if (isWorkRes) {
            const r = (w.workResidence || '').trim();
            return isUnassigned ? !r : r === locationName;
        } else {
            const h = (w.visitationHousing || (typeof detectResidence === 'function' ? detectResidence(w) : '') || '').trim();
            return isUnassigned ? !h : h === locationName;
        }
    });

    title.textContent = `${icon} ${typeLabel}: ${locationName} (${workers.length} volunteer${workers.length !== 1 ? 's' : ''})`;

    if (workers.length === 0) {
        body.innerHTML = `<div style="text-align:center; padding:36px 16px; color:var(--text-muted); font-style:italic; font-size:0.85rem;">No active volunteers assigned to this location today.</div>`;
        modal.classList.add('active');
        if (window.trapFocus) window.trapFocus(modal);
        return;
    }

    workers.sort((a, b) => (a.dailyStatus === 'Present' ? 0 : 1) - (b.dailyStatus === 'Present' ? 0 : 1) || (a.trade || '').localeCompare(b.trade || '') || (a.name || '').localeCompare(b.name || ''));

    const deptCounts = {};
    workers.forEach(w => {
        const d = w.department || (typeof inferWorkerDepartment === 'function' ? inferWorkerDepartment(w) : 'Job-site Construction');
        deptCounts[d] = (deptCounts[d] || 0) + 1;
    });

    const presentCount = workers.filter(w => w.dailyStatus === 'Present').length;
    const offsiteCount = workers.length - presentCount;

    let html = `
        <div style="display: flex; flex-direction: column; gap: 12px; margin-bottom: 14px;">
            <div class="roster-dept-banner" style="font-size: 0.74rem; line-height: 1.4; color: var(--text-muted); background: var(--bg-color); padding: 8px 12px; border-radius: 6px; border-left: 3px solid var(--color-primary); display: flex; align-items: flex-start; gap: 8px;">
                <span style="font-size: 0.95rem; flex-shrink: 0; margin-top: 1px;">ℹ️</span>
                <div>
                    <strong style="color: var(--text-main);">${typeLabel} Accommodation Roster:</strong>
                    Currently assigned to <strong>${esc(locationName)}</strong>. ${presentCount} present today, ${offsiteCount} off-site.
                </div>
            </div>

            <div style="display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 10px; padding-bottom: 10px; border-bottom: 1px solid var(--border-color);">
                <div style="display: flex; align-items: center; flex-wrap: wrap; gap: 6px;" id="roster-residence-filter-bar">
                    <button class="roster-filter-chip active" data-filter="all" style="padding: 3px 10px; font-size: 0.72rem; font-weight: 700; border-radius: var(--radius-full, 9999px); border: 1px solid var(--border-color); background: var(--color-primary); color: #ffffff; cursor: pointer; transition: all 0.15s;">
                        All (${workers.length})
                    </button>
                    ${Object.entries(deptCounts).map(([dept, count]) => {
                        const m = (typeof DEPARTMENT_META !== 'undefined' && DEPARTMENT_META[dept]) ? DEPARTMENT_META[dept] : { color: '#64748b' };
                        return `
                            <button class="roster-filter-chip" data-filter="${esc(dept)}" style="padding: 3px 10px; font-size: 0.72rem; font-weight: 700; border-radius: var(--radius-full, 9999px); border: 1px solid var(--border-color); background: var(--card-bg); color: var(--text-main); cursor: pointer; transition: all 0.15s; display: inline-flex; align-items: center; gap: 5px;">
                                <span style="width: 7px; height: 7px; border-radius: 50%; background: ${m.color};"></span>
                                <span>${esc(dept)}</span>
                                <span style="font-size: 0.65rem; padding: 0 5px; border-radius: 9999px; background: var(--primary-light, #e0e7ff); color: var(--primary-color, #4f46e5);">${count}</span>
                            </button>
                        `;
                    }).join('')}
                </div>
                <div style="position: relative; min-width: 180px; max-width: 240px; flex: 1;">
                    <input type="text" id="roster-quick-search" placeholder="Search resident, trade, task..." autofocus style="width: 100%; padding: 4px 10px 4px 26px; font-size: 0.75rem; border: 1px solid var(--border-color); border-radius: var(--radius-sm, 4px); background: var(--bg-color); color: var(--text-main);">
                    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="position: absolute; left: 8px; top: 50%; transform: translateY(-50%); color: var(--text-muted); pointer-events: none;"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/></svg>
                </div>
            </div>

            <div class="worker-roster-grid-4col" id="residence-roster-grid" style="display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 8px;">
    `;

    workers.forEach(w => {
        const style = getTradeStyle(w.trade);
        const avatarHtml = (w.photoUrl || w.photo)
            ? `<img src="${esc(w.photoUrl || w.photo)}" alt="${esc(w.name)}" style="width: 36px; height: 36px; border-radius: 50%; object-fit: cover; border: 1px solid var(--border-color); flex-shrink: 0;">`
            : `<div style="width: 36px; height: 36px; border-radius: 50%; background: ${style.bg || 'var(--primary-light)'}; color: ${style.text || 'var(--primary-color)'}; display: flex; align-items: center; justify-content: center; font-size: 0.8rem; font-weight: 700; flex-shrink: 0; border: 1px solid ${style.text || 'transparent'}; opacity: 0.95;">${(w.name || '?').charAt(0).toUpperCase()}</div>`;

        const statusBadge = w.dailyStatus === 'Present'
            ? `<span style="font-size:0.63rem; font-weight:700; color:var(--color-success); background:var(--color-success-bg); border:1px solid var(--color-success-border); padding:1px 5px; border-radius:3px;">Present</span>`
            : `<span style="font-size:0.63rem; font-weight:700; color:var(--color-danger); background:var(--color-danger-bg); border:1px solid var(--color-danger-border); padding:1px 5px; border-radius:3px;">${esc(w.dailyStatus)}</span>`;

        const workerAge = (typeof getWorkerAge === 'function') ? getWorkerAge(w) : null;
        const ageBadge = (workerAge != null && workerAge > 0)
            ? `<span class="worker-roster-age" style="font-size: 0.65rem; font-weight: 700; padding: 1px 5px; border-radius: 3px; background: var(--bg-color); color: var(--text-muted); border: 1px solid var(--border-color);" title="Age: ${workerAge} years old">${workerAge} yrs</span>`
            : '';

        const dept = w.department || (typeof inferWorkerDepartment === 'function' ? inferWorkerDepartment(w) : 'Job-site Construction');
        const deptMeta = (typeof DEPARTMENT_META !== 'undefined' && DEPARTMENT_META[dept]) ? DEPARTMENT_META[dept] : { icon: '📁' };
        const taskText = w.area ? (w.area + (w.specificTask ? ' · ' + w.specificTask : '')) : (w.specificTask || 'No Area Assigned');

        html += `
            <div class="hover-card worker-roster-card" data-dept="${esc(dept)}" data-trade="${esc(w.trade || '')}" data-name="${esc(w.name || '').toLowerCase()}" data-task="${esc(w.specificTask || '').toLowerCase()}" data-skill="${esc(w.skillLevel || '').toLowerCase()}" style="display: flex; align-items: flex-start; gap: 9px; background: var(--card-bg); border: 1px solid var(--border-color); padding: 8px 10px; border-radius: var(--radius-sm, 4px); box-shadow: var(--shadow-sm); min-width: 0; transition: transform 0.15s, box-shadow 0.15s, border-color 0.15s;">
                ${avatarHtml}
                <div style="flex: 1; min-width: 0; line-height: 1.25;">
                    <div style="display: flex; align-items: baseline; justify-content: space-between; gap: 6px;">
                        <span style="font-weight: 700; font-size: 0.8rem; color: var(--text-main); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;" title="${esc(w.name)}">${esc(w.name)}</span>
                        <div style="display: flex; align-items: center; gap: 4px; flex-shrink: 0;">
                            ${ageBadge}
                            ${w.aNumber != null ? `<span style="font-size: 0.65rem; font-weight: 700; color: var(--text-muted);">#${esc(w.aNumber)}</span>` : ''}
                        </div>
                    </div>
                    <div style="display: flex; align-items: center; gap: 4px; margin-top: 3px; flex-wrap: wrap;">
                        ${statusBadge}
                        <span style="font-size: 0.67rem; font-weight: 700; padding: 1px 6px; border-radius: 3px; background: ${style.bg}; color: ${style.text}; text-transform: uppercase; letter-spacing: 0.03em;">${esc(w.trade || 'General')}</span>
                        <span style="font-size: 0.65rem; font-weight: 700; padding: 1px 5px; border-radius: 3px; background: var(--bg-color); color: var(--text-muted); border: 1px solid var(--border-color);">${deptMeta.icon} ${esc(dept)}</span>
                    </div>
                    <div class="worker-roster-task" style="font-size: 0.68rem; color: var(--text-muted); margin-top: 4px; line-height: 1.35; word-break: break-word; white-space: normal;" title="${esc(taskText)}">
                        ${esc(taskText)}
                    </div>
                    ${w.phone ? `<div style="font-size: 0.66rem; color: var(--color-primary); font-weight: 600; margin-top: 2px;">📞 ${esc(w.phone)}</div>` : ''}
                </div>
            </div>
        `;
    });

    html += `
            </div>
            <div id="residence-roster-no-match" style="display: none; text-align: center; padding: 24px; color: var(--text-muted); font-size: 0.8rem; font-style: italic;">
                No resident volunteers match the selected filter.
            </div>
        </div>
    `;

    body.innerHTML = html;

    // Filter & search handlers
    const searchInput = document.getElementById('roster-quick-search');
    const noMatchEl = document.getElementById('residence-roster-no-match');
    const cards = Array.from(body.querySelectorAll('.worker-roster-card'));

    function applyResidenceFilter() {
        const activeChip = body.querySelector('#roster-residence-filter-bar .roster-filter-chip.active');
        const activeFilter = activeChip ? activeChip.dataset.filter : 'all';
        const query = searchInput ? searchInput.value.trim().toLowerCase() : '';

        let visibleCount = 0;
        cards.forEach(card => {
            const matchesFilter = activeFilter === 'all' || card.dataset.dept === activeFilter;
            const matchesQuery = !query || 
                card.dataset.name.includes(query) || 
                card.dataset.task.includes(query) || 
                card.dataset.trade.toLowerCase().includes(query) ||
                card.dataset.dept.toLowerCase().includes(query);

            if (matchesFilter && matchesQuery) {
                card.style.display = 'flex';
                visibleCount++;
            } else {
                card.style.display = 'none';
            }
        });

        if (noMatchEl) noMatchEl.style.display = visibleCount === 0 ? 'block' : 'none';
    }

    body.querySelectorAll('#roster-residence-filter-bar .roster-filter-chip').forEach(chip => {
        chip.addEventListener('click', () => {
            body.querySelectorAll('#roster-residence-filter-bar .roster-filter-chip').forEach(c => {
                c.classList.remove('active');
                c.style.background = 'var(--card-bg)';
                c.style.color = 'var(--text-main)';
            });
            chip.classList.add('active');
            chip.style.background = 'var(--color-primary)';
            chip.style.color = '#ffffff';
            applyResidenceFilter();
        });
    });

    if (searchInput) {
        searchInput.addEventListener('input', applyResidenceFilter);
    }

    modal.classList.add('active');
    if (window.trapFocus) window.trapFocus(modal);
}
return {
    renderDeployabilityBanner: renderDeployabilityBanner,
    renderDepartmentMatrix: renderDepartmentMatrix,
    renderDeptAnalytics: renderDeptAnalytics,
    renderHousingOverview: renderHousingOverview,
    openDepartmentRosterModal: openDepartmentRosterModal,
    openResidenceRosterModal: openResidenceRosterModal
};
})();

// ===== distribution.js (area roster) - verbatim from 01_Rust_WC_DB/js/distribution.js =====
var S5_distribution_2 = (function () {

function openAreaRosterModal(areaName) {
    const modal = document.getElementById('area-roster-modal');
    const title = document.getElementById('area-roster-title');
    const body = document.getElementById('area-roster-body');
    if (!modal || !body) return;

    const isAbsenteeGroup = (areaName === 'Absent and On Passes' || areaName === 'Absent Volunteers' || areaName === 'Volunteers on Passes');

    title.textContent = areaName === 'Absent and On Passes' ? 'Absent and On Passes' :
                        areaName === 'Absent Volunteers' ? 'Absent Personnel' :
                        areaName === 'Volunteers on Passes' ? 'Personnel On Pass' : `Roster for ${areaName}`;

    let workers = [];
    if (areaName === 'Absent and On Passes') {
        workers = workersData.filter(w => w && !w.isArchived && (w.dailyStatus === 'Absent' || w.dailyStatus === 'Passes'));
    } else if (areaName === 'Absent Volunteers') {
        workers = workersData.filter(w => w && !w.isArchived && w.dailyStatus === 'Absent');
    } else if (areaName === 'Volunteers on Passes') {
        workers = workersData.filter(w => w && !w.isArchived && w.dailyStatus === 'Passes');
    } else {
        workers = workersData.filter(w => {
            if (!w || w.isArchived) return false;
            const workerArea = w.area || 'Unassigned Area';
            return workerArea === areaName && w.dailyStatus !== 'Absent' && w.dailyStatus !== 'Passes';
        });
    }

    if (workers.length === 0) {
        body.innerHTML = `<div style="text-align: center; padding: 36px 16px; color: var(--text-muted); font-style: italic; font-size: 0.85rem;">No active workers assigned to this area today.</div>`;
        modal.classList.add('active');
        if (window.trapFocus) window.trapFocus(modal);
        return;
    }

    // Sort workers by trade then name for a cohesive, professional layout
    workers.sort((a, b) => (a.trade || '').localeCompare(b.trade || '') || (a.name || '').localeCompare(b.name || ''));

    // Count trades present in this area
    const tradeCounts = {};
    workers.forEach(w => {
        const tr = w.trade || 'Unassigned';
        tradeCounts[tr] = (tradeCounts[tr] || 0) + 1;
    });
    const sortedTrades = Object.keys(tradeCounts).sort();

    // Dynamic horizontal columns (1 to 4 columns based on worker count)
    const initialCols = Math.min(Math.max(workers.length, 1), 4);

    let html = `
        <div style="display: flex; flex-direction: column; gap: 12px; margin-bottom: 14px;">
            <!-- Top Controls Toolbar -->
            <div style="display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 10px; padding-bottom: 10px; border-bottom: 1px solid var(--border-color);">
                <div style="display: flex; align-items: center; flex-wrap: wrap; gap: 6px;" id="roster-trade-filter-bar">
                    <button class="roster-filter-chip active" data-filter="all" style="padding: 3px 10px; font-size: 0.72rem; font-weight: 700; border-radius: var(--radius-full, 9999px); border: 1px solid var(--border-color); background: var(--color-primary); color: #ffffff; cursor: pointer; transition: all 0.15s;">
                        All (${workers.length})
                    </button>
    `;

    sortedTrades.forEach(trade => {
        const style = getTradeStyle(trade);
        html += `
            <button class="roster-filter-chip" data-filter="${esc(trade)}" style="padding: 3px 10px; font-size: 0.72rem; font-weight: 700; border-radius: var(--radius-full, 9999px); border: 1px solid var(--border-color); background: var(--card-bg); color: var(--text-main); cursor: pointer; transition: all 0.15s; display: inline-flex; align-items: center; gap: 5px;">
                <span style="width: 7px; height: 7px; border-radius: 50%; background: ${style.text || 'var(--color-primary)'};"></span>
                <span>${esc(trade)}</span>
                <span style="font-size: 0.65rem; padding: 0 5px; border-radius: 9999px; background: ${style.bg || 'var(--primary-light)'}; color: ${style.text || 'var(--primary-color)'};">${tradeCounts[trade]}</span>
            </button>
        `;
    });

    html += `
                </div>
                <div style="position: relative; min-width: 180px; max-width: 240px; flex: 1;">
                    <input type="text" id="roster-quick-search" placeholder="Search name, trade, task..." autofocus style="width: 100%; padding: 4px 10px 4px 26px; font-size: 0.75rem; border: 1px solid var(--border-color); border-radius: var(--radius-sm, 4px); background: var(--bg-color); color: var(--text-main);">
                    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="position: absolute; left: 8px; top: 50%; transform: translateY(-50%); color: var(--text-muted); pointer-events: none;"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/></svg>
                </div>
            </div>

            <!-- Dynamic Horizontal Grid (Always spreads across horizontal space) -->
            <div class="worker-roster-grid-4col" id="area-roster-grid" style="display: grid; grid-template-columns: repeat(${initialCols}, minmax(0, 1fr)); gap: 8px;">
    `;

    workers.forEach(w => {
        const style = getTradeStyle(w.trade);
        const avatarHtml = (w.photoUrl || w.photo)
            ? `<img src="${esc(w.photoUrl || w.photo)}" alt="${esc(w.name)}" style="width: 36px; height: 36px; border-radius: 50%; object-fit: cover; border: 1px solid var(--border-color); flex-shrink: 0;">`
            : `<div style="width: 36px; height: 36px; border-radius: 50%; background: ${style.bg || 'var(--primary-light)'}; color: ${style.text || 'var(--primary-color)'}; display: flex; align-items: center; justify-content: center; font-size: 0.8rem; font-weight: 700; flex-shrink: 0; border: 1px solid ${style.text || 'transparent'}; opacity: 0.95;">${(w.name || '?').charAt(0).toUpperCase()}</div>`;

        html += `
            <div class="hover-card worker-roster-card" data-trade="${esc(w.trade || '')}" data-name="${esc(w.name || '').toLowerCase()}" data-task="${esc(w.specificTask || '').toLowerCase()}" data-skill="${esc(w.skillLevel || '').toLowerCase()}" style="display: flex; align-items: center; gap: 9px; background: var(--card-bg); border: 1px solid var(--border-color); padding: 8px 10px; border-radius: var(--radius-sm, 4px); box-shadow: var(--shadow-sm); min-width: 0; transition: transform 0.15s, box-shadow 0.15s, border-color 0.15s;">
                ${avatarHtml}
                <div style="flex: 1; min-width: 0; line-height: 1.25;">
                    <div style="display: flex; align-items: baseline; justify-content: space-between; gap: 6px;">
                        <span style="font-weight: 700; font-size: 0.8rem; color: var(--text-main); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;" title="${esc(w.name)}">${esc(w.name)}</span>
                        ${w.aNumber != null ? `<span style="font-size: 0.65rem; font-weight: 700; color: var(--text-muted); flex-shrink: 0;">#${esc(w.aNumber)}</span>` : ''}
                    </div>
                    <div style="display: flex; align-items: center; gap: 4px; margin-top: 3px; flex-wrap: wrap;">
                        <span style="font-size: 0.67rem; font-weight: 700; padding: 1px 6px; border-radius: 3px; background: ${style.bg}; color: ${style.text}; text-transform: uppercase; letter-spacing: 0.03em;">${esc(w.trade || 'General')}</span>
                        ${w.skillLevel ? `<span style="font-size: 0.65rem; font-weight: 700; padding: 1px 5px; border-radius: 3px; background: var(--color-surface-secondary, #f1f1ef); color: var(--text-muted); border: 1px solid var(--border-color);">Skill ${esc(w.skillLevel)}</span>` : ''}
                    </div>
                    ${!isAbsenteeGroup
                        ? `<div style="font-size: 0.68rem; color: var(--text-muted); margin-top: 3px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;" title="${esc(w.specificTask || 'General Task')}">${esc(w.specificTask || 'General Task')}</div>`
                        : `<div style="font-size: 0.67rem; color: var(--text-muted); margin-top: 2px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;" title="${esc(w.address || 'No Home Address')}">📍 ${esc(w.address || 'No Home Address')}</div>
                           <div style="font-size: 0.67rem; color: var(--color-primary); font-weight: 600; margin-top: 1px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">📞 ${esc(w.phone || 'No Phone')}</div>
                           ${w.dailyStatus === 'Absent' ? `<div style="font-size: 0.67rem; color: var(--color-danger); font-weight: 700; margin-top: 1px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;" title="${esc(w.absentReason || 'N/A')}">Reason: ${esc(w.absentReason || 'N/A')}</div>` : ''}`
                    }
                </div>
            </div>
        `;
    });

    html += `
            </div>
            <div id="roster-no-match" style="display: none; text-align: center; padding: 24px; color: var(--text-muted); font-size: 0.8rem; font-style: italic;">
                No workers match the selected filter.
            </div>
        </div>
    `;

    body.innerHTML = html;

    // Attach interactive filter logic
    const gridEl = document.getElementById('area-roster-grid');
    const filterChips = body.querySelectorAll('.roster-filter-chip');
    const searchInput = document.getElementById('roster-quick-search');
    const noMatchEl = document.getElementById('roster-no-match');
    const cards = Array.from(gridEl ? gridEl.querySelectorAll('.worker-roster-card') : []);

    function applyFilter() {
        const activeChip = body.querySelector('.roster-filter-chip.active');
        const activeFilter = activeChip ? activeChip.dataset.filter : 'all';
        const query = searchInput ? searchInput.value.trim().toLowerCase() : '';

        let visibleCount = 0;
        cards.forEach(card => {
            const trade = card.dataset.trade;
            const name = card.dataset.name;
            const task = card.dataset.task;
            const skill = card.dataset.skill;

            const matchesTrade = (activeFilter === 'all' || trade === activeFilter);
            const matchesQuery = !query || name.includes(query) || trade.toLowerCase().includes(query) || task.includes(query) || skill.includes(query);

            if (matchesTrade && matchesQuery) {
                card.style.display = 'flex';
                visibleCount++;
            } else {
                card.style.display = 'none';
            }
        });

        if (gridEl) {
            const cols = Math.min(Math.max(visibleCount, 1), 4);
            gridEl.style.gridTemplateColumns = `repeat(${cols}, minmax(0, 1fr))`;
        }

        if (noMatchEl) {
            noMatchEl.style.display = visibleCount === 0 ? 'block' : 'none';
        }
    }

    filterChips.forEach(chip => {
        chip.addEventListener('click', () => {
            filterChips.forEach(c => {
                c.classList.remove('active');
                c.style.background = 'var(--card-bg)';
                c.style.color = 'var(--text-main)';
            });
            chip.classList.add('active');
            chip.style.background = 'var(--color-primary)';
            chip.style.color = '#ffffff';
            applyFilter();
        });
    });

    if (searchInput) {
        searchInput.addEventListener('input', applyFilter);
    }

    if (window.trapFocus) window.trapFocus(modal);
    modal.classList.add('active');
}
return {
    openAreaRosterModal: openAreaRosterModal
};
})();

// ===== ui.js (department tabs) - verbatim from 01_Rust_WC_DB/js/ui.js =====
var S5_ui = (function () {

    function switchDistTab(tabName) {
        // Toggle tab button styles
        document.querySelectorAll('.dist-tab-btn').forEach(function (btn) {
            var isActive = btn.getAttribute('data-dist-tab') === tabName;
            btn.classList.toggle('active', isActive);
            btn.style.color = isActive ? 'var(--primary-color)' : 'var(--text-muted)';
            btn.style.background = isActive ? 'var(--primary-light)' : 'transparent';
            btn.style.boxShadow = isActive ? '0 1px 3px rgba(0,0,0,0.08)' : 'none';
        });
        // Toggle panes
        var deptPane = document.getElementById('dist-pane-departments');
        var housingPane = document.getElementById('dist-pane-housing');
        if (deptPane) deptPane.style.display = tabName === 'departments' ? 'flex' : 'none';
        if (housingPane) housingPane.style.display = tabName === 'housing' ? 'flex' : 'none';
        // Lazy-render housing when its tab is opened
        if (tabName === 'housing' && typeof window.renderHousingOverview === 'function') {
            var aw = (typeof workersData !== 'undefined') ? workersData.filter(function (w) { return w && !w.isArchived; }) : [];
            window.renderHousingOverview(aw);
        }
    }
return {
    switchDistTab: switchDistTab
};
})();

// ===== duty.js (overnight duty) - verbatim from 01_Rust_WC_DB/js/duty.js =====
var S5_duty_2 = (function () {

const DutyModule = (function () {
    // Current state
    let currentDate = new Date();
    let selectedAreaId = null;
    let currentViewMode = 'week'; // 'week' or 'month'

    // Assignment & replacement state variables
    let _pendingAssign = {};
    let _pendingReplace = {};
    let _pendingTag = {};
    let oldANumber = null;

    // Data caches
    let _areas = [];
    let _schedules = [];
    let _workers = [];
    let _dirty = true;

    // View elements
    let rootEl = null;

    /* ------ Init ------ */

    // Generate week schedule for the currently selected area (module scope so
    // it can be exposed for direct invocation by the header "Schedule Duty" button)
    async function requestWeekGeneration() {
        if (!selectedAreaId) { showToast('Select an area first.'); return; }
        if (selectedAreaId === 'combined') {
            showToast('Please select a specific area tab to generate its schedule.');
            return;
        }
        const area = _areas.find(a => a.id === selectedAreaId);
        if (!area) return;
        const minWorkers = area.workersNeeded || 1;
        if (!area.roster || area.roster.length < minWorkers) {
            showToast(`"${area.name}" roster needs at least ${minWorkers} workers.`);
            return;
        }
        if (await showConfirm(`Generate schedule for "${area.name}" this week? Existing manual assignments will be kept.`)) {
            await generateWeekSchedule(area);
        }
    }

    async function init() {
        rootEl = document.getElementById('duty-app-root');
        if (!rootEl) return;
        if (!window.dbStores || !window.dbStores.duty_schedules) {
            console.error('Duty stores not found in IndexedDB.');
            return;
        }
        if (!rootEl.dataset.initialized) {
            renderLayout();
            attachEventListeners();
            rootEl.dataset.initialized = 'true';
        }
        try {
            await loadData();
        } catch (err) {
            console.error('duty: loadData error:', err);
        }
    }

    /* ------ Layout ------ */

    function renderLayout() {
        rootEl.innerHTML = `
            <div style="display: flex; flex-direction: column; gap: 12px; height: 100%;">

                <!-- Area Tabs -->
                <div id="duty-area-tabs" class="duty-tab-bar">
                    <!-- Rendered by JS -->
                </div>

                <div id="duty-calendar-workspace" class="duty-calendar-workspace" style="display: flex; gap: 16px; flex: 1; min-height: 0;">

                    <!-- Sidebar: Area management -->
                    <div id="duty-sidebar" style="flex: 0 0 250px; display: flex; flex-direction: column; gap: 8px; overflow-y: auto; flex-shrink: 0;">
                        <div class="table-card" style="padding: 12px;">
                            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;">
                                <h3 style="font-size: 0.85rem; font-weight: 700; color: var(--text-main); margin-bottom: 0;">Areas</h3>
                                <button id="btn-add-duty-area" class="btn btn-secondary btn-sm" style="padding: 3px 8px; font-size: 0.7rem;" title="Add a new duty area">+ Add</button>
                            </div>
                            <div id="duty-areas-list" style="display: flex; flex-direction: column; gap: 6px;">
                                <!-- Populated by JS -->
                            </div>
                        </div>
                    </div>

                    <!-- Main Content: Calendar -->
                    <div class="table-card" style="flex: 1; display: flex; flex-direction: column; padding: 16px; overflow: hidden; min-width: 0;">
                        <!-- Calendar nav bar -->
                        <div class="duty-calendar-toolbar" style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px; flex-shrink: 0;">
                            <div style="display: flex; align-items: center; gap: 6px;">
                                <button id="duty-prev-date" class="btn btn-secondary btn-sm" style="display: flex; align-items: center; justify-content: center; width: 28px; height: 28px; padding: 0;" title="Previous">
                                    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 18 9 12 15 6"></polyline></svg>
                                </button>
                                <button id="duty-today" class="btn btn-secondary btn-sm" style="height: 28px; font-weight: 600; font-size: 0.72rem; padding: 0 10px;" title="This Week / Month">Today</button>
                                <button id="duty-next-date" class="btn btn-secondary btn-sm" style="display: flex; align-items: center; justify-content: center; width: 28px; height: 28px; padding: 0;" title="Next">
                                    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"></polyline></svg>
                                </button>
                                <h2 id="duty-current-date-label" style="font-size: 1.05rem; font-weight: 700; margin: 0 0 0 8px; color: var(--text-main);"></h2>
                            </div>
                            <div style="display: flex; align-items: center; gap: 8px;">
                                <div class="view-toggle" style="display: flex; background: var(--bg-body); padding: 3px; border-radius: 8px; border: 1px solid var(--border-color);">
                                    <button id="btn-view-week" class="btn btn-sm" style="background: var(--bg-color); border: 1px solid var(--border-color); color: var(--text-main); font-weight: 600; font-size: 0.75rem; padding: 4px 12px;">Week</button>
                                    <button id="btn-view-month" class="btn btn-sm" style="background: transparent; border: none; color: var(--text-muted); font-weight: 600; font-size: 0.75rem; padding: 4px 12px;">Month</button>
                                    <button id="btn-view-roster" class="btn btn-sm" style="background: transparent; border: none; color: var(--text-muted); font-weight: 600; font-size: 0.75rem; padding: 4px 12px;">Roster</button>
                                </div>
                                <button id="btn-toggle-expand" class="btn btn-secondary btn-sm" style="font-size: 0.72rem; padding: 4px 10px; display: flex; align-items: center; gap: 4px;" title="Toggle full-width calendar">
                                    <svg class="expand-icon" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                                        <polyline points="15 3 21 3 21 9"></polyline>
                                        <polyline points="9 21 3 21 3 15"></polyline>
                                        <line x1="21" y1="3" x2="14" y2="10"></line>
                                        <line x1="3" y1="21" x2="10" y2="14"></line>
                                    </svg>
                                    <span>Expand</span>
                                </button>
                                <button id="btn-gen-week" class="btn btn-primary btn-sm" style="font-size: 0.72rem; padding: 4px 10px;" title="Generate schedule for this week using area roster">Gen Week</button>
                                <button id="btn-clear-day" class="btn btn-secondary btn-sm" style="font-size: 0.72rem; padding: 4px 10px; color: var(--danger); border-color: rgba(220, 38, 38, 0.25);" title="Clear all scheduled volunteers for the active day">Clear Day</button>
                                <button id="btn-clear-week" class="btn btn-secondary btn-sm" style="font-size: 0.72rem; padding: 4px 10px; color: var(--danger); border-color: rgba(220, 38, 38, 0.25);" title="Clear all scheduled volunteers for the active week">Clear Week</button>
                                <button id="btn-print-duty" class="btn btn-secondary btn-sm" style="font-size: 0.72rem; padding: 4px 10px;" title="Print schedule">Print</button>
                            </div>
                        </div>

                        <!-- Selected area label -->
                        <div id="duty-selected-area-label" style="font-size: 0.8rem; font-weight: 600; color: var(--text-muted); margin-bottom: 6px; flex-shrink: 0;">
                            Select an area above.
                        </div>

                        <!-- Calendar grid -->
                        <div id="duty-calendar-grid" class="custom-scrollbar" style="flex: 1; overflow-y: auto; border: 1px solid var(--border-color); border-radius: var(--radius-sm); position: relative; min-height: 260px;">
                            <!-- Calendar populated by JS -->
                        </div>
                    </div>
                </div>
            </div>

            <!-- ------ Modals ------ -->
            <div id="duty-area-modal" class="modal-overlay">
                <div class="modal-content" style="max-width: 420px;">
                    <div class="modal-header">
                        <h2 id="duty-area-modal-title">Add Duty Area</h2>
                        <button class="close-btn" onclick="window.releaseFocusTrap();document.getElementById('duty-area-modal').classList.remove('active')">&times;</button>
                    </div>
                    <div class="modal-body">
                        <form id="duty-area-form">
                            <input type="hidden" id="duty-area-id">
                            <div class="form-group">
                                <label>Area Name</label>
                                <input type="text" id="duty-area-name" required>
                            </div>
                            <div class="form-group" style="display:flex;gap:12px;">
                                <div style="flex:1;">
                                    <label>Workers Needed/Shift</label>
                                    <input type="number" id="duty-area-workers" min="1" value="2" required>
                                </div>
                                <div style="flex:0 0 60px;">
                                    <label>Color</label>
                                    <input type="color" id="duty-area-color" value="#3b82f6" style="width:100%;height:36px;padding:2px;cursor:pointer;">
                                </div>
                            </div>
                            <div class="form-group">
                                <label>Shift Time</label>
                                <input type="text" id="duty-area-shift" value="22:00-06:00" placeholder="e.g. 22:00-06:00">
                            </div>
                            <div class="modal-actions">
                                <button type="button" class="btn btn-secondary" onclick="window.releaseFocusTrap();document.getElementById('duty-area-modal').classList.remove('active')">Cancel</button>
                                <button type="submit" class="btn btn-primary">Save Area</button>
                            </div>
                        </form>
                    </div>
                </div>
            </div>

            <div id="duty-roster-modal" class="modal-overlay">
                <div class="modal-content modal-4col-roster" style="max-width: 1200px; width: 96vw; display: flex; flex-direction: column; max-height: 88vh;">
                    <div class="modal-header">
                        <h2 id="duty-roster-modal-title">Manage Roster</h2>
                        <button class="close-btn" onclick="window.releaseFocusTrap();document.getElementById('duty-roster-modal').classList.remove('active')">&times;</button>
                    </div>
                    <div class="modal-body" style="flex: 1; overflow-y: hidden; display: flex; flex-direction: column; gap: 10px;">
                        <input type="text" id="duty-roster-search" class="search-bar" placeholder="Search workers..." style="width: 100%;" title="Type to filter workers">
                        <div id="duty-roster-selection-list" class="custom-scrollbar" style="flex: 1; overflow-y: auto; border: 1px solid var(--border-color); border-radius: var(--radius-sm); padding: 8px;">
                            <!-- Populated by JS -->
                        </div>
                    </div>
                    <div class="modal-actions" style="padding-top: 12px; border-top: 1px solid var(--border-color);">
                        <button type="button" class="btn btn-secondary" onclick="window.releaseFocusTrap();document.getElementById('duty-roster-modal').classList.remove('active')" title="Discard changes">Cancel</button>
                        <button type="button" id="btn-save-roster" class="btn btn-primary" title="Save which workers can be assigned to this area">Save Roster</button>
                    </div>
                </div>
            </div>

            <div id="duty-override-modal" class="modal-overlay">
                <div class="modal-content" style="max-width: 400px;">
                    <div class="modal-header">
                        <h2>Manual Override Reason</h2>
                        <button class="close-btn" onclick="if(window.__dutyCloseOverride) window.__dutyCloseOverride()">&times;</button>
                    </div>
                    <div class="modal-body">
                        <p style="font-size: 0.85rem; color: var(--text-muted); margin-bottom: 12px;" id="duty-override-info">Moving worker to a new schedule slot.</p>
                        <div class="form-group">
                            <label>Reason for override</label>
                            <textarea id="duty-override-reason" style="width: 100%; min-height: 80px; padding: 8px; border: 1px solid var(--border-color); border-radius: 6px; font-family: inherit; font-size: 0.85rem;" placeholder="e.g. Swapping with another worker, covering absence..." required></textarea>
                        </div>
                    </div>
                    <div class="modal-actions">
                        <button type="button" class="btn btn-secondary" onclick="if(window.__dutyCloseOverride) window.__dutyCloseOverride()">Cancel</button>
                        <button type="button" id="btn-override-confirm" class="btn btn-primary">Confirm Override</button>
                    </div>
                </div>
            </div>

            <!-- Assign Worker Modal (click empty slot) -->
            <div id="duty-assign-modal" class="modal-overlay">
                <div class="modal-content" style="max-width: 420px;">
                    <div class="modal-header">
                        <h2 id="duty-assign-modal-title">Assign Worker</h2>
                        <button class="close-btn" onclick="window.releaseFocusTrap();document.getElementById('duty-assign-modal').classList.remove('active')">&times;</button>
                    </div>
                    <div class="modal-body">
                        <p style="font-size: 0.85rem; color: var(--text-muted); margin-bottom: 12px;" id="duty-assign-date-label"></p>
                        <input type="text" id="duty-assign-search" class="search-bar" placeholder="Search roster..." style="width: 100%; margin-bottom: 8px;">
                        <div id="duty-assign-list" style="max-height: 250px; overflow-y: auto; display: flex; flex-direction: column; gap: 4px;"></div>
                    </div>
                </div>
            </div>

            <!-- Replace Worker Modal -->
            <div id="duty-replace-modal" class="modal-overlay">
                <div class="modal-content" style="max-width: 420px;">
                    <div class="modal-header">
                        <h2>Replace Worker</h2>
                        <button class="close-btn" onclick="window.releaseFocusTrap();document.getElementById('duty-replace-modal').classList.remove('active')">&times;</button>
                    </div>
                    <div class="modal-body">
                        <p style="font-size: 0.85rem; color: var(--text-muted); margin-bottom: 12px;" id="duty-replace-info"></p>
                        <input type="text" id="duty-replace-search" class="search-bar" placeholder="Search roster..." style="width: 100%; margin-bottom: 8px;">
                        <div id="duty-replace-list" style="max-height: 250px; overflow-y: auto; display: flex; flex-direction: column; gap: 4px;"></div>
                    </div>
                </div>
            </div>

            <!-- Worker Actions Popover (right-click on tag) -->
            <div id="duty-worker-popover" style="display:none; position:fixed; z-index: var(--z-system); background: var(--card-bg); border: 1px solid var(--border-color); border-radius: var(--radius-sm); box-shadow: var(--shadow-lg); padding: 4px; min-width: 160px;">
                <button class="popover-action" data-action="remove" style="display: flex; align-items: center; gap: 8px; width: 100%; padding: 8px 12px; border: none; background: transparent; cursor: pointer; font-size: 0.8rem; color: var(--danger); text-align: left; border-radius: 4px;">
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                    Remove
                </button>
                <button class="popover-action" data-action="replace" style="display: flex; align-items: center; gap: 8px; width: 100%; padding: 8px 12px; border: none; background: transparent; cursor: pointer; font-size: 0.8rem; color: var(--text-main); text-align: left; border-radius: 4px;">
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="15 3 21 3 21 9"></polyline><path d="M21 3l-7 7"></path><polyline points="9 21 3 21 3 15"></polyline><path d="M3 21l7-7"></path></svg>
                    Replace
                </button>
            </div>
        `;
    }

    /* ------ Event Listeners ------ */

    function attachEventListeners() {
        // Prev/Next date
        document.getElementById('duty-prev-date')?.addEventListener('click', () => {
            if (currentViewMode === 'week') {
                currentDate = new Date(currentDate);
                currentDate.setDate(currentDate.getDate() - 7);
            } else if (currentViewMode === 'roster') {
                currentDate = new Date(currentDate);
                currentDate.setDate(currentDate.getDate() - 1);
            } else {
                currentDate = new Date(currentDate);
                currentDate.setMonth(currentDate.getMonth() - 1);
            }
            renderCalendar();
        });

        document.getElementById('duty-next-date')?.addEventListener('click', () => {
            if (currentViewMode === 'week') {
                currentDate = new Date(currentDate);
                currentDate.setDate(currentDate.getDate() + 7);
            } else if (currentViewMode === 'roster') {
                currentDate = new Date(currentDate);
                currentDate.setDate(currentDate.getDate() + 1);
            } else {
                currentDate = new Date(currentDate);
                currentDate.setMonth(currentDate.getMonth() + 1);
            }
            renderCalendar();
        });

        document.getElementById('duty-today')?.addEventListener('click', () => {
            currentDate = new Date();
            renderCalendar();
        });

        // Week/Month toggle
        document.getElementById('btn-view-week')?.addEventListener('click', (e) => {
            if (currentViewMode === 'week') return;
            currentViewMode = 'week';
            const monthBtn = document.getElementById('btn-view-month');
            e.target.style.background = 'var(--bg-color)';
            e.target.style.border = '1px solid var(--border-color)';
            e.target.style.color = 'var(--text-main)';
            monthBtn.style.background = 'transparent';
            monthBtn.style.border = 'none';
            monthBtn.style.color = 'var(--text-muted)';
            renderCalendar();
        });

        document.getElementById('btn-view-month')?.addEventListener('click', (e) => {
            if (currentViewMode === 'month') return;
            currentViewMode = 'month';
            const weekBtn = document.getElementById('btn-view-week');
            e.target.style.background = 'var(--bg-color)';
            e.target.style.border = '1px solid var(--border-color)';
            e.target.style.color = 'var(--text-main)';
            weekBtn.style.background = 'transparent';
            weekBtn.style.border = 'none';
            weekBtn.style.color = 'var(--text-muted)';
            currentDate = new Date(currentDate);
            currentDate.setDate(1);
            renderCalendar();
        });

        document.getElementById('btn-view-roster')?.addEventListener('click', (e) => {
            if (currentViewMode === 'roster') return;
            currentViewMode = 'roster';
            const weekBtn = document.getElementById('btn-view-week');
            const monthBtn = document.getElementById('btn-view-month');
            e.target.style.background = 'var(--bg-color)';
            e.target.style.border = '1px solid var(--border-color)';
            e.target.style.color = 'var(--text-main)';
            if (weekBtn) {
                weekBtn.style.background = 'transparent';
                weekBtn.style.border = 'none';
                weekBtn.style.color = 'var(--text-muted)';
            }
            if (monthBtn) {
                monthBtn.style.background = 'transparent';
                monthBtn.style.border = 'none';
                monthBtn.style.color = 'var(--text-muted)';
            }
            renderCalendar();
        });

        // Expand/Collapse calendar view
        document.getElementById('btn-toggle-expand')?.addEventListener('click', (e) => {
            const workspace = document.getElementById('duty-calendar-workspace');
            const sidebar = document.getElementById('duty-sidebar');
            const btn = document.getElementById('btn-toggle-expand');
            if (!workspace || !sidebar || !btn) return;

            const isExpanded = workspace.classList.contains('expanded');
            const span = btn.querySelector('span');
            const svg = btn.querySelector('svg');

            if (isExpanded) {
                // Collapse / show sidebar
                workspace.classList.remove('expanded');
                sidebar.style.marginLeft = '0px';
                sidebar.style.opacity = '1';
                sidebar.style.pointerEvents = 'auto';
                if (span) span.textContent = 'Expand';
                if (svg) {
                    svg.innerHTML = `
                        <polyline points="15 3 21 3 21 9"></polyline>
                        <polyline points="9 21 3 21 3 15"></polyline>
                        <line x1="21" y1="3" x2="14" y2="10"></line>
                        <line x1="3" y1="21" x2="10" y2="14"></line>
                    `;
                }
            } else {
                // Expand / hide sidebar
                workspace.classList.add('expanded');
                sidebar.style.marginLeft = '-266px'; // 250px width + 16px gap
                sidebar.style.opacity = '0';
                sidebar.style.pointerEvents = 'none';
                if (span) span.textContent = 'Collapse';
                if (svg) {
                    svg.innerHTML = `
                        <polyline points="4 14 10 14 10 20"></polyline>
                        <polyline points="20 10 14 10 14 4"></polyline>
                        <line x1="14" y1="10" x2="21" y2="3"></line>
                        <line x1="10" y1="14" x2="3" y2="21"></line>
                    `;
                }
            }
        });

        // Add area
        document.getElementById('btn-add-duty-area')?.addEventListener('click', () => {
            const titleEl = document.getElementById('duty-area-modal-title');
            if (titleEl) titleEl.textContent = 'Add Duty Area';
            const formEl = document.getElementById('duty-area-form');
            if (formEl) formEl.reset();
            const idEl = document.getElementById('duty-area-id');
            if (idEl) idEl.value = '';
            const colorEl = document.getElementById('duty-area-color');
            if (colorEl) colorEl.value = _getNextColor();
            const shiftEl = document.getElementById('duty-area-shift');
            if (shiftEl) shiftEl.value = '22:00-06:00';
            const modalEl = document.getElementById('duty-area-modal');
            if (modalEl) { if (window.trapFocus) window.trapFocus(modalEl); modalEl.classList.add('active'); }
        });

        // Area form submit
        document.getElementById('duty-area-form')?.addEventListener('submit', async (e) => {
            e.preventDefault();
            try {
                const idEl = document.getElementById('duty-area-id');
                const nameEl = document.getElementById('duty-area-name');
                const workersEl = document.getElementById('duty-area-workers');
                const colorEl = document.getElementById('duty-area-color');
                const shiftEl = document.getElementById('duty-area-shift');
                const modalEl = document.getElementById('duty-area-modal');
                if (!idEl || !nameEl || !workersEl || !colorEl || !shiftEl || !modalEl) return;
                const isNew = !idEl.value;
                const id = idEl.value || safeUUID();
                const name = nameEl.value.trim();
                const workersNeeded = parseInt(workersEl.value, 10);
                const color = colorEl.value;
                const shiftTime = shiftEl.value.trim();

                // Preserve existing fields when editing
                let existing = null;
                if (!isNew) {
                    await window.dbStores.duty_areas.iterate(a => { if (a.id === id) existing = a; });
                }

                const area = {
                    id,
                    name,
                    workersNeeded,
                    color: color || '#3b82f6',
                    shiftTime: shiftTime || '22:00-06:00',
                    roster: existing?.roster || [],
                    lastRotationIndex: existing?.lastRotationIndex ?? 0
                };

                await window.dbStores.duty_areas.setItem(id, area);
                releaseFocusTrap();
                modalEl.classList.remove('active');
                if (window.markBadgesDirty) window.markBadgesDirty();
                await loadData();

                if (isNew) {
                    selectedAreaId = id;
                    await renderCalendar();
                    setTimeout(async () => {
                        if (await showConfirm(`Roster for "${name}" is empty. Set one up now?`)) {
                            openAreaRosterModal(id);
                        }
                    }, 300);
                }
            } catch (err) {
                console.error('Failed to save duty area:', err);
                showToast('Failed to save area. Check console for details.');
            }
        });

        // Roster search
        document.getElementById('duty-roster-search')?.addEventListener('input', debounce((e) => {
            const q = e.target.value.toLowerCase();
            document.querySelectorAll('.roster-candidate-item').forEach(el => {
                const text = el.textContent.toLowerCase();
                el.style.display = text.includes(q) ? 'flex' : 'none';
            });
        }, 150));

        // Save roster
        document.getElementById('btn-save-roster')?.addEventListener('click', saveAreaRoster);

        // Generate week schedule
        document.getElementById('btn-gen-week')?.addEventListener('click', requestWeekGeneration);

        // Print
        document.getElementById('btn-print-duty')?.addEventListener('click', () => {
            const html = _buildPrintHtml();
            if (html) safePrint(html);
        });

        // Clear Day
        document.getElementById('btn-clear-day')?.addEventListener('click', () => clearSchedules('day'));

        // Clear Week
        document.getElementById('btn-clear-week')?.addEventListener('click', () => clearSchedules('week'));

        // Delegated: area tab clicks
        document.getElementById('duty-area-tabs')?.addEventListener('click', (e) => {
            const tab = e.target.closest('.duty-tab');
            if (!tab) return;
            const areaId = tab.dataset.areaId;
            if (areaId) switchArea(areaId);
        });

        // Delegated: sidebar area clicks
        document.getElementById('duty-areas-list')?.addEventListener('click', (e) => {
            const btn = e.target.closest('button[data-area-id]');
            if (btn) {
                e.stopPropagation();
                const areaId = btn.dataset.areaId;
                const area = _areas.find(a => a.id === areaId);
                if (!area) return;

                if (btn.classList.contains('btn-area-roster')) {
                    openAreaRosterModal(areaId);
                } else if (btn.classList.contains('btn-area-edit')) {
                    const titleEl = document.getElementById('duty-area-modal-title');
                    if (titleEl) titleEl.textContent = 'Edit Duty Area';
                    const idEl = document.getElementById('duty-area-id');
                    if (idEl) idEl.value = area.id;
                    const nameEl = document.getElementById('duty-area-name');
                    if (nameEl) nameEl.value = area.name;
                    const workersEl = document.getElementById('duty-area-workers');
                    if (workersEl) workersEl.value = area.workersNeeded;
                    const colorEl = document.getElementById('duty-area-color');
                    if (colorEl) colorEl.value = area.color || '#3b82f6';
                    const shiftEl = document.getElementById('duty-area-shift');
                    if (shiftEl) shiftEl.value = area.shiftTime || '22:00-06:00';
                    const modalEl = document.getElementById('duty-area-modal');
                    if (modalEl) { if (window.trapFocus) window.trapFocus(modalEl); modalEl.classList.add('active'); }
                } else if (btn.classList.contains('btn-area-delete')) {
                    _deleteDutyArea(area);
                }
                return;
            }
            // Click on sidebar area row &rarr; switch area
            const row = e.target.closest('.duty-sidebar-area');
            if (row && row.dataset.areaId) {
                switchArea(row.dataset.areaId);
            }
        });

        // Click empty slot in calendar
        document.getElementById('duty-calendar-grid')?.addEventListener('click', (e) => {
            const slot = e.target.closest('.duty-cal-empty-slot');
            if (slot) {
                const dateStr = slot.dataset.date;
                const areaId = slot.dataset.areaId;
                _handleEmptySlotClick(dateStr, areaId);
                return;
            }
            const tag = e.target.closest('.duty-cal-worker-tag');
            if (tag && !e.target.closest('svg')) {
                const aNumber = parseInt(tag.dataset.anumber, 10);
                const dateStr = tag.dataset.date;
                _handleWorkerTagClick(aNumber, dateStr, e);
                return;
            }

            // Click month cell in combined view
            const monthCell = e.target.closest('.duty-cal-cell-month');
            if (monthCell) {
                const dateStr = monthCell.dataset.date;
                if (dateStr) {
                    currentDate = new Date(dateStr);
                    currentViewMode = 'week';

                    const weekBtn = document.getElementById('btn-view-week');
                    const monthBtn = document.getElementById('btn-view-month');
                    if (weekBtn && monthBtn) {
                        weekBtn.style.background = 'var(--bg-color)';
                        weekBtn.style.border = '1px solid var(--border-color)';
                        weekBtn.style.color = 'var(--text-main)';
                        monthBtn.style.background = 'transparent';
                        monthBtn.style.border = 'none';
                        monthBtn.style.color = 'var(--text-muted)';
                    }
                    renderCalendar();
                }
            }
        });

        // Left-click on empty slot or worker tag &rarr; action modal
        document.getElementById('duty-calendar-grid')?.addEventListener('dblclick', (e) => {
            const slot = e.target.closest('.duty-cal-empty-slot');
            if (slot) {
                e.preventDefault();
                const dateStr = slot.dataset.date;
                const areaId = slot.dataset.areaId;
                openAssignWorkerModal(dateStr, areaId);
                return;
            }
            const tag = e.target.closest('.duty-cal-worker-tag');
            if (tag && !e.target.closest('svg')) {
                e.preventDefault();
                const aNumber = parseInt(tag.dataset.anumber, 10);
                const dateStr = tag.dataset.date;
                _handleWorkerTagClick(aNumber, dateStr, e);
                return;
            }
        });

        // Assign modal: search + worker list
        document.getElementById('duty-assign-search')?.addEventListener('input', debounce((e) => {
            const q = e.target.value.toLowerCase();
            document.querySelectorAll('#duty-assign-list .assign-candidate-item').forEach(el => {
                const text = el.textContent.toLowerCase();
                el.style.display = text.includes(q) ? 'flex' : 'none';
            });
        }, 150));

        document.getElementById('duty-replace-search')?.addEventListener('input', debounce((e) => {
            const q = e.target.value.toLowerCase();
            document.querySelectorAll('#duty-replace-list .replace-candidate-item').forEach(el => {
                const text = el.textContent.toLowerCase();
                el.style.display = text.includes(q) ? 'flex' : 'none';
            });
        }, 150));

        // Delegated: assign/replace worker from modals
        document.getElementById('duty-assign-list')?.addEventListener('click', (e) => {
            const item = e.target.closest('.assign-candidate-item');
            if (item && !item.getAttribute('data-blocked')) {
                const aNumber = parseInt(item.dataset.aNumber, 10);
                const dateStr = _pendingAssign.dateStr;
                const areaId = _pendingAssign.areaId;
                closeAllModals(['duty-assign-modal']);
                assignWorker(dateStr, areaId, aNumber);
            }
        });

        document.getElementById('duty-replace-list')?.addEventListener('click', (e) => {
            const item = e.target.closest('.replace-candidate-item');
            if (item && !item.getAttribute('data-blocked')) {
                const newANumber = parseInt(item.dataset.aNumber, 10);
                oldANumber = _pendingReplace.anumber;
                const dateStr = _pendingReplace.dateStr;
                closeAllModals(['duty-replace-modal']);
                replaceWorker(dateStr, oldANumber, newANumber);
            }
        });

        // Worker popover actions (right-click or middle-click on tag)
        document.getElementById('duty-worker-popover')?.addEventListener('click', (e) => {
            const btn = e.target.closest('.popover-action');
            if (!btn) return;
            closePopover();
            const action = btn.dataset.action;
            if (action === 'remove') {
                removeWorker(_pendingTag.dateStr, _pendingTag.anumber);
            } else if (action === 'replace') {
                openReplaceModal(_pendingTag.dateStr, _pendingTag.anumber);
            }
        });

        // Close popover on scroll outside
        document.addEventListener('click', (e) => {
            const popover = document.getElementById('duty-worker-popover');
            if (popover && popover.style.display !== 'none' && !popover.contains(e.target)) {
                closePopover();
            }
        });
    }

    /* ------ Data Loading ------ */

    async function loadData() {
        _workers = typeof workersData !== 'undefined' ? workersData : [];

        // Load schedules from storage only when dirty (avoids IndexedDB read on every tab switch)
        if (_dirty || _schedules.length === 0) {
            _schedules = await window._storage.get('wc_duty_schedules') || [];
            _dirty = false;
        }

        // Always reload areas (they may have changed independently)
        _areas = [];
        await window.dbStores.duty_areas.iterate(a => {
            if (!a.color) a.color = '#3b82f6';
            if (!a.shiftTime) a.shiftTime = '22:00-06:00';
            if (a.lastRotationIndex === undefined) a.lastRotationIndex = 0;
            if (!a.roster) a.roster = [];
            _areas.push(a);
        });

        renderAreaTabs();
        renderSidebarAreas();

        // Auto-select if none selected or area was deleted
        if (!selectedAreaId || (selectedAreaId !== 'combined' && !_areas.find(a => a.id === selectedAreaId))) {
            selectedAreaId = _areas.length > 0 ? _areas[0].id : null;
        }

        renderCalendar();
    }

    /* ------ Area Tabs ------ */

    function renderAreaTabs() {
        const tabsEl = document.getElementById('duty-area-tabs');
        if (!tabsEl) return;

        if (_areas.length === 0) {
            tabsEl.innerHTML = '<div style="color: var(--text-muted); font-size: 0.8rem; padding: 8px 0;">No areas defined. Click "+ Add" to create one.</div>';
            return;
        }

        const isCombinedActive = selectedAreaId === 'combined';
        let html = `
            <button class="duty-tab ${isCombinedActive ? 'active' : ''}" data-area-id="combined" title="Combined calendar view of all areas">
                <span class="duty-tab-dot" style="background: #64748b;"></span>
                <span>All Areas</span>
            </button>
        `;

        html += [..._areas].sort((a, b) => window.areaPriorityComparator(a.name, b.name)).map(area => {
            const isActive = area.id === selectedAreaId;
            const rosterCount = (area.roster || []).length;
            const color = area.color || '#3b82f6';
            return `
                <button class="duty-tab ${isActive ? 'active' : ''}" data-area-id="${area.id}" title="${esc(area.name)} - ${rosterCount} worker${rosterCount !== 1 ? 's' : ''} in roster">
                    <span class="duty-tab-dot" style="background: ${color};"></span>
                    <span>${esc(area.name)}</span>
                    <span class="duty-tab-badge">${rosterCount}</span>
                </button>
            `;
        }).join('');

        tabsEl.innerHTML = html;
    }

    /* --- Sidebar Area List --- */

    function renderSidebarAreas() {
        const listEl = document.getElementById('duty-areas-list');
        if (!listEl) return;

        if (_areas.length === 0) {
            listEl.innerHTML = '<div style="color: var(--text-muted); font-size: 0.8rem; text-align: center; padding: 10px 0;">No areas defined.</div>';
            return;
        }

        listEl.innerHTML = [..._areas].sort((a, b) => window.areaPriorityComparator(a.name, b.name)).map(area => {
            const rosterCount = (area.roster || []).length;
            const hasRoster = rosterCount > 0;
            const isSelected = area.id === selectedAreaId;
            const color = area.color || '#3b82f6';
            return `
                <div class="duty-sidebar-area ${isSelected ? 'selected' : ''}" data-area-id="${area.id}">
                    <div style="display: flex; justify-content: space-between; align-items: center;">
                        <div style="display: flex; align-items: center; gap: 6px; min-width: 0; flex: 1;">
                            <span style="width: 8px; height: 8px; border-radius: 50%; background: ${color}; flex-shrink: 0;"></span>
                            <span style="font-size: 0.82rem; font-weight: 600; color: var(--text-main); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">${esc(area.name)}</span>
                        </div>
                        <div style="display: flex; gap: 3px; flex-shrink: 0;">
                            <button class="btn-area-roster" data-area-id="${area.id}" style="background: transparent; border: 1px solid var(--border-color); border-radius: 4px; padding: 1px 5px; cursor: pointer; color: var(--primary-color); display: inline-flex; align-items: center;" title="Select workers for this area" aria-label="Select workers for this area">${ctxGroupSvg}</button>
                            <button class="btn-area-edit" data-area-id="${area.id}" style="background: transparent; border: 1px solid var(--border-color); border-radius: 4px; padding: 1px 5px; cursor: pointer; color: var(--text-muted); display: inline-flex; align-items: center;" title="Edit area" aria-label="Edit area">${ctxEditSvg}</button>
                            <button class="btn-area-delete" data-area-id="${area.id}" style="background: transparent; border: 1px solid #fca5a5; border-radius: 4px; padding: 1px 5px; cursor: pointer; color: var(--danger); display: inline-flex; align-items: center;" title="Delete area" aria-label="Delete area">${ctxArchiveSvg}</button>
                        </div>
                    </div>
                    <div style="display: flex; align-items: center; gap: 4px; margin-top: 3px;">
                        <span style="width: 6px; height: 6px; border-radius: 50%; background: ${hasRoster ? '#22c55e' : '#94a3b8'}; flex-shrink: 0;"></span>
                        <span style="font-size: 0.65rem; color: var(--text-muted);">${hasRoster ? `${rosterCount} worker${rosterCount !== 1 ? 's' : ''}` : 'No roster'}</span>
                    </div>
                </div>
            `;
        }).join('');
    }

    /* --- Area Switch --- */

    function switchArea(areaId) {
        if (areaId === selectedAreaId) return;
        selectedAreaId = areaId;
        renderAreaTabs();
        renderSidebarAreas();
        renderCalendar();
    }

    /* --- Calendar View --- */

    async function renderCalendar() {
        try {
            const gridEl = document.getElementById('duty-calendar-grid');
            const labelEl = document.getElementById('duty-selected-area-label');
            const dateLabel = document.getElementById('duty-current-date-label');
            if (!gridEl) return;

            if (!selectedAreaId || _areas.length === 0) {
                gridEl.innerHTML = '<div style="display:flex;align-items:center;justify-content:center;height:100%;color:var(--text-muted);font-size:0.9rem;">Add a duty area to get started.</div>';
                if (labelEl) labelEl.textContent = '';
                if (dateLabel) dateLabel.textContent = '';
                return;
            }

            // Refresh schedules data
            _schedules = await window._storage.get('wc_duty_schedules') || [];

            if (selectedAreaId === 'combined') {
                if (labelEl) {
                    labelEl.innerHTML = `<span style="display:inline-block;width:10px;height:10px;border-radius:50%;background:#64748b;margin-right:6px;vertical-align:middle;"></span> Combined View - All Areas`;
                }
                if (currentViewMode === 'week') {
                    renderCombinedWeekView(gridEl, dateLabel);
                } else if (currentViewMode === 'roster') {
                    renderCombinedRosterView(gridEl, dateLabel);
                } else {
                    renderCombinedMonthView(gridEl, dateLabel);
                }
                return;
            }

            const area = _areas.find(a => a.id === selectedAreaId);
            if (!area) {
                gridEl.innerHTML = '<div style="display:flex;align-items:center;justify-content:center;height:100%;color:var(--text-muted);font-size:0.9rem;">Select an area.</div>';
                if (labelEl) labelEl.textContent = '';
                if (dateLabel) dateLabel.textContent = '';
                return;
            }

            if (labelEl) {
                const color = area.color || '#3b82f6';
                labelEl.innerHTML = `<span style="display:inline-block;width:10px;height:10px;border-radius:50%;background:${color};margin-right:6px;vertical-align:middle;"></span> ${esc(area.name)} - ${esc(area.shiftTime || '22:00-06:00')} (${area.workersNeeded} needed)`;
            }

            if (currentViewMode === 'week') {
                renderWeekView(gridEl, dateLabel, area);
            } else if (currentViewMode === 'roster') {
                renderRosterView(gridEl, dateLabel, area);
            } else {
                renderMonthView(gridEl, dateLabel, area);
            }
        } catch (err) {
            console.error('Failed to render duty calendar:', err);
            const gridEl = document.getElementById('duty-calendar-grid');
            if (gridEl) {
                gridEl.innerHTML = '<div style="display:flex;align-items:center;justify-content:center;height:100%;color:var(--danger-color);font-size:0.9rem;">Error loading calendar schedules.</div>';
            }
        }
    }

    function hexToRgba(hex, alpha) {
        const c = hex.replace('#', '');
        const r = parseInt(c.substring(0, 2), 16);
        const g = parseInt(c.substring(2, 4), 16);
        const b = parseInt(c.substring(4, 6), 16);
        return `rgba(${r},${g},${b},${alpha})`;
    }

    /* ------ Week View ------ */

    function renderWeekView(gridEl, dateLabel, area) {
        // Snap currentDate to Monday of its week
        const monday = _getMonday(currentDate);
        const dates = [];
        for (let i = 0; i < 7; i++) {
            const d = new Date(monday);
            d.setDate(monday.getDate() + i);
            dates.push(d);
        }

        // Build date label
        const first = dates[0];
        const last = dates[6];
        const monthNames = ["January", "February", "March", "April", "May", "June",
            "July", "August", "September", "October", "November", "December"
        ];
        const firstMonth = first.getMonth();
        const lastMonth = last.getMonth();
        let label;
        if (firstMonth === lastMonth) {
            label = `${first.getDate()} - ${last.getDate()} ${monthNames[firstMonth].substring(0,3)} ${first.getFullYear()}`;
        } else {
            label = `${first.getDate()} ${monthNames[firstMonth].substring(0,3)} - ${last.getDate()} ${monthNames[lastMonth].substring(0,3)} ${first.getFullYear()}`;
        }
        dateLabel.textContent = label;

        const dayNames = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
        const todayStr = todayLocal();
        const workerMap = _workers.reduce((m, w) => { m[w.aNumber] = w; return m; }, {});

        let html = '<div class="duty-cal-header">';
        html += '<div class="duty-cal-weekdays">';
        dayNames.forEach(d => { html += `<div class="duty-cal-weekday">${d}</div>`; });
        html += '</div></div>';

        html += '<div class="duty-cal-grid">';

        for (let i = 0; i < 7; i++) {
            const date = dates[i];
            const dateStr = getFormattedDateStr(date);
            const isWeekend = date.getDay() % 6 === 0;
            const isToday = dateStr === todayStr;

            const daySched = _schedules.find(s => s.dateStr === dateStr);
            const dayAssignments = daySched ? daySched.assignments.filter(a => a.areaId === area.id) : [];

            html += `<div class="duty-cal-cell ${isWeekend ? 'weekend' : ''} ${isToday ? 'duty-cal-cell-today' : ''}" style="border-top:2px solid ${area.color};" data-date="${dateStr}">`;
            html += `<div class="duty-cal-day-label">${date.getDate()}</div>`;
            html += `<div class="duty-cal-worker-list">`;

            // Show assigned workers
            var displayNames = _disambiguateSurnames(dayAssignments, workerMap);
            dayAssignments.forEach(a => {
                const w = workerMap[a.aNumber];
                const wName = w ? w.name : `#${a.aNumber}`;
                const initial = w ? w.name.charAt(0) : '?';
                const trade = w ? (w.trade || '') : '';
                const tradeStyle = getTradeStyle(trade);
                const isAuto = !a.overrideReason;
                var label = displayNames[a.aNumber] || wName.split(',')[0];
                html += `
                    <div class="duty-cal-worker-tag" data-anumber="${a.aNumber}" data-date="${dateStr}" data-area="${area.id}"
                         style="background: ${tradeStyle.bg}; color: ${tradeStyle.text};"
                         title="${esc(wName)} - ${isAuto ? 'Auto-assigned' : 'Manual: ' + esc(a.overrideReason)}">
                        <span style="width:16px;height:16px;border-radius:50%;background:${tradeStyle.text}22;display:flex;align-items:center;justify-content:center;font-size:0.55rem;font-weight:800;flex-shrink:0;">${initial}</span>
                        <span style="flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">${esc(label)}</span>
                        ${!isAuto ? '<span style="font-size:0.55rem;opacity:0.6;" title="Manual override">&#9998;</span>' : ''}
                    </div>
                `;
            });

            // Fill remaining slots (empty)
            const assignedCount = dayAssignments.length;
            const needed = area.workersNeeded;
            for (let j = assignedCount; j < needed; j++) {
                html += `<div class="duty-cal-empty-slot" data-date="${dateStr}" data-area-id="${area.id}" title="Click to assign a worker">+</div>`;
            }

            html += `</div></div>`;
        }

        html += '</div>';
        gridEl.innerHTML = html;
    }

    /* ------ Month View ------ */

    function renderMonthView(gridEl, dateLabel, area) {
        const year = currentDate.getFullYear();
        const month = currentDate.getMonth();
        const monthNames = ["January", "February", "March", "April", "May", "June",
            "July", "August", "September", "October", "November", "December"
        ];
        dateLabel.textContent = `${monthNames[month]} ${year}`;

        const firstDay = new Date(year, month, 1);
        const daysInMonth = new Date(year, month + 1, 0).getDate();
        const startOffset = (firstDay.getDay() + 6) % 7; // Monday=0

        const todayStr = todayLocal();
        const dayNames = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
        const workerMap = _workers.reduce((m, w) => { m[w.aNumber] = w; return m; }, {});

        let html = '<div class="duty-cal-header">';
        html += '<div class="duty-cal-weekdays">';
        dayNames.forEach(d => { html += `<div class="duty-cal-weekday">${d}</div>`; });
        html += '</div></div>';

        html += '<div class="duty-cal-grid">';

        // Empty cells before first day
        for (let i = 0; i < startOffset; i++) {
            html += `<div style="border:none;"></div>`;
        }

        for (let day = 1; day <= daysInMonth; day++) {
            const date = new Date(year, month, day);
            const dateStr = getFormattedDateStr(date);
            const isWeekend = date.getDay() % 6 === 0;
            const isToday = dateStr === todayStr;

            const daySched = _schedules.find(s => s.dateStr === dateStr);
            const dayAssignments = daySched ? daySched.assignments.filter(a => a.areaId === area.id) : [];
            const count = dayAssignments.length;
            const needed = area.workersNeeded;

            html += `<div class="duty-cal-cell duty-cal-cell-month ${isWeekend ? 'weekend' : ''} ${isToday ? 'duty-cal-cell-today' : ''}" style="border-top:2px solid ${area.color};" data-date="${dateStr}">`;
            html += `<div class="duty-cal-day-label">${day}</div>`;

            if (count > 0) {
                // Show first worker name, then "+N more"
                var monthDisplayNames = _disambiguateSurnames(dayAssignments, workerMap);
                const w = workerMap[dayAssignments[0].aNumber];
                const firstName = monthDisplayNames[dayAssignments[0].aNumber] || (w ? w.name.split(',')[0] : `#${dayAssignments[0].aNumber}`);
                html += `<div class="duty-cal-worker-name" title="${esc(w ? w.name : '')}">${esc(firstName)}</div>`;
                if (count > 1) {
                    html += `<div style="font-size:0.6rem;color:var(--text-muted);">+${count - 1} more</div>`;
                }
            } else {
                html += `<div class="duty-cal-count">${needed > 0 ? '0/' + needed : ''}</div>`;
            }

            html += `</div>`;
        }

        html += '</div>';
        gridEl.innerHTML = html;
    }

    /* ------ Helpers ------ */

    function _getMonday(date) {
        const d = new Date(date);
        const day = d.getDay();
        const diff = (day === 0 ? 6 : day - 1); // Mon=0
        d.setDate(d.getDate() - diff);
        d.setHours(0, 0, 0, 0);
        return d;
    }

    function _getNextColor() {
        const colors = ['#3b82f6', '#ef4444', '#22c55e', '#f59e0b', '#8b5cf6', '#ec4899', '#06b6d4', '#84cc16', '#f97316', '#6366f1'];
        const used = _areas.map(a => a.color).filter(Boolean);
        for (const c of colors) {
            if (!used.includes(c)) return c;
        }
        return colors[Math.floor(Math.random() * colors.length)];
    }

    function _disambiguateSurnames(assignments, workerMap) {
        const display = {};
        const groups = {};
        assignments.forEach(function(a) {
            const w = workerMap[a.aNumber];
            const fullName = w ? w.name : '#' + a.aNumber;
            var surnameRaw = fullName.split(',')[0].trim();
            var key = surnameRaw.toUpperCase();
            if (!groups[key]) groups[key] = { surname: surnameRaw, workers: [] };
            var given = fullName.indexOf(',') !== -1 ? fullName.split(',').slice(1).join(',').trim() : '';
            groups[key].workers.push({ assignment: a, given: given });
        });
        Object.keys(groups).forEach(function(key) {
            var g = groups[key];
            if (g.workers.length === 1) {
                display[g.workers[0].assignment.aNumber] = g.surname;
                return;
            }
            g.workers.forEach(function(w, idx) {
                var given = w.given;
                var label = g.surname;
                for (var len = 1; len <= given.length; len++) {
                    var prefix = given.substring(0, len);
                    var collision = false;
                    for (var oi = 0; oi < g.workers.length; oi++) {
                        if (oi !== idx && g.workers[oi].given.substring(0, len) === prefix) { collision = true; break; }
                    }
                    if (!collision) { label = g.surname + ' ' + prefix; break; }
                }
                if (label === g.surname && given.length > 0) label = g.surname + ' ' + given;
                display[w.assignment.aNumber] = label;
            });
        });
        return display;
    }

    function _handleEmptySlotClick(dateStr, areaId) {
        openAssignWorkerModal(dateStr, areaId);
    }

    function _handleWorkerTagClick(aNumber, dateStr, e) {
        _pendingTag = { anumber: aNumber, dateStr };
        const popover = document.getElementById('duty-worker-popover');
        if (!popover) return;

        const rect = e.target.getBoundingClientRect();
        popover.style.display = 'block';

        let left = rect.left + window.scrollX;
        let top = rect.bottom + window.scrollY;

        if (left + 160 > window.innerWidth) {
            left = rect.right - 160 + window.scrollX;
        }

        popover.style.left = `${left}px`;
        popover.style.top = `${top}px`;

        e.stopPropagation();
    }

    function closePopover() {
        const popover = document.getElementById('duty-worker-popover');
        if (popover) popover.style.display = 'none';
    }

    function closeAllModals(exclude = []) {
        const modals = ['duty-assign-modal', 'duty-replace-modal', 'duty-roster-modal', 'duty-area-modal', 'duty-override-modal'];
        modals.forEach(id => {
            if (!exclude.includes(id)) {
                releaseFocusTrap();
                document.getElementById(id)?.classList.remove('active');
            }
        });
    }

    function openAssignWorkerModal(dateStr, areaId) {
        _pendingAssign = { dateStr, areaId };
        const area = _areas.find(a => a.id === areaId);
        if (!area) return;

        const assignLabel = document.getElementById('duty-assign-date-label');
        if (assignLabel) assignLabel.textContent = `Assign worker for ${area.name} on ${dateStr}`;
        const assignSearch = document.getElementById('duty-assign-search');
        if (assignSearch) assignSearch.value = '';

        const listEl = document.getElementById('duty-assign-list');
        if (!listEl) return;

        const daySched = _schedules.find(s => s.dateStr === dateStr);
        const assignedANumbers = daySched ? daySched.assignments.map(a => a.aNumber) : [];

        const availableWorkers = _workers.filter(w => !assignedANumbers.includes(w.aNumber));

        const roster = area.roster || [];
        availableWorkers.sort((a, b) => {
            const inRosterA = roster.includes(a.aNumber);
            const inRosterB = roster.includes(b.aNumber);
            if (inRosterA && !inRosterB) return -1;
            if (!inRosterA && inRosterB) return 1;
            return a.name.localeCompare(b.name);
        });

        listEl.innerHTML = availableWorkers.map(w => {
            const inRoster = roster.includes(w.aNumber);
            const tradeStyle = getTradeStyle(w.trade);
            const blocked = window.isWorkerBlockedFromArea ? window.isWorkerBlockedFromArea(w.aNumber, area.name) : false;
            const status = window.getCovidStatusSync ? window.getCovidStatusSync(w.aNumber) : null;
            
            var statusBadge = '';
            var blockedAttr = '';
            var blockedStyle = '';
            if (blocked && status) {
                var stateLabel = status.state === 'isolated' ? 'Isolated' : status.state === 'pending_assessment' ? 'Pending Assessment' : 'Blocked';
                statusBadge = `<span style="font-size: 0.65rem; background: #fee2e2; color: #dc2626; padding: 2px 6px; border-radius: 4px; font-weight: 600; margin-left: auto;">${stateLabel}</span>`;
                blockedAttr = 'data-blocked="true"';
                blockedStyle = 'opacity: 0.55; cursor: not-allowed;';
            } else if (inRoster) {
                statusBadge = '<span style="font-size: 0.65rem; background: rgba(34, 197, 94, 0.15); color: #22c55e; padding: 2px 6px; border-radius: 4px; font-weight: 600;">Roster</span>';
            }

            return `
                <div class="assign-candidate-item" data-a-number="${w.aNumber}" ${blockedAttr} style="display: flex; align-items: center; justify-content: space-between; padding: 8px 12px; border: 1px solid var(--border-color); border-radius: var(--radius-sm); cursor: pointer; background: var(--card-bg); margin-bottom: 4px; ${blockedStyle}">
                    <div style="display: flex; align-items: center; gap: 8px; min-width: 0; flex: 1;">
                       <span style="width: 24px; height: 24px; border-radius: 50%; background: ${tradeStyle.bg}; color: ${tradeStyle.text}; display: flex; align-items: center; justify-content: center; font-size: 0.65rem; font-weight: 700; flex-shrink: 0;">${w.name.charAt(0)}</span>
                       <div style="min-width: 0; flex: 1;">
                           <div style="font-weight: 600; font-size: 0.82rem; color: var(--text-main); text-overflow: ellipsis; overflow: hidden; white-space: nowrap;">${esc(w.name)}</div>
                           <div style="font-size: 0.65rem; color: var(--text-muted);">${esc(w.trade || 'Volunteer')}</div>
                       </div>
                    </div>
                    ${statusBadge}
                </div>
            `;
        }).join('');

        if (availableWorkers.length === 0) {
            listEl.innerHTML = '<div style="color: var(--text-muted); text-align: center; padding: 12px; font-size: 0.8rem;">No available workers.</div>';
        }

        const assignModal = document.getElementById('duty-assign-modal');
        if (assignModal) { if (window.trapFocus) window.trapFocus(assignModal); assignModal.classList.add('active'); }
    }

    async function assignWorker(dateStr, areaId, aNumber) {
        try {
            const area = _areas.find(a => a.id === areaId);
            const areaName = area ? area.name : 'Unknown Area';
            await updateScheduleAssignment(dateStr, aNumber, areaId, false, 'Manual Assignment');
            await logAssignmentChange(aNumber, `Manually assigned to ${areaName} on ${dateStr}`);
            showToast(`Assigned to ${areaName}`);
        } catch (err) {
            console.error('Failed to assign worker:', err);
            showToast('Failed to assign worker. Database error.');
        }
    }

    async function removeWorker(dateStr, aNumber) {
        try {
            if (await showConfirm('Remove this worker from duty on this day?')) {
                await updateScheduleAssignment(dateStr, aNumber, null, true);
                await logAssignmentChange(aNumber, `Removed from duty on ${dateStr}`);
                showToast('Worker removed from schedule');
            }
        } catch (err) {
            console.error('Failed to remove worker:', err);
            showToast('Failed to remove worker. Database error.');
        }
    }

    function openReplaceModal(dateStr, aNumber) {
        _pendingReplace = { dateStr, anumber: aNumber };
        const worker = _workers.find(w => w.aNumber === aNumber);
        const workerName = worker ? worker.name : `#${aNumber}`;

        const daySched = _schedules.find(s => s.dateStr === dateStr);
        const assignment = daySched ? daySched.assignments.find(a => a.aNumber === aNumber) : null;
        const areaId = assignment ? assignment.areaId : null;
        const area = _areas.find(a => a.id === areaId);
        if (!area) {
            showToast('Assignment area not found.');
            return;
        }

        const replaceInfo = document.getElementById('duty-replace-info');
        if (replaceInfo) replaceInfo.textContent = `Replace ${workerName} for ${area.name} on ${dateStr}`;
        const replaceSearch = document.getElementById('duty-replace-search');
        if (replaceSearch) replaceSearch.value = '';

        const listEl = document.getElementById('duty-replace-list');
        if (!listEl) return;

        const assignedANumbers = daySched ? daySched.assignments.map(a => a.aNumber) : [];
        const availableWorkers = _workers.filter(w => w.aNumber !== aNumber && !assignedANumbers.includes(w.aNumber));

        const roster = area.roster || [];
        availableWorkers.sort((a, b) => {
            const inRosterA = roster.includes(a.aNumber);
            const inRosterB = roster.includes(b.aNumber);
            if (inRosterA && !inRosterB) return -1;
            if (!inRosterA && inRosterB) return 1;
            return a.name.localeCompare(b.name);
        });

        listEl.innerHTML = availableWorkers.map(w => {
            const inRoster = roster.includes(w.aNumber);
            const tradeStyle = getTradeStyle(w.trade);
            const blocked = window.isWorkerBlockedFromArea ? window.isWorkerBlockedFromArea(w.aNumber, area.name) : false;
            const status = window.getCovidStatusSync ? window.getCovidStatusSync(w.aNumber) : null;
            
            var statusBadge = '';
            var blockedAttr = '';
            var blockedStyle = '';
            if (blocked && status) {
                var stateLabel = status.state === 'isolated' ? 'Isolated' : status.state === 'pending_assessment' ? 'Pending Assessment' : 'Blocked';
                statusBadge = `<span style="font-size: 0.65rem; background: #fee2e2; color: #dc2626; padding: 2px 6px; border-radius: 4px; font-weight: 600; margin-left: auto;">${stateLabel}</span>`;
                blockedAttr = 'data-blocked="true"';
                blockedStyle = 'opacity: 0.55; cursor: not-allowed;';
            } else if (inRoster) {
                statusBadge = '<span style="font-size: 0.65rem; background: rgba(34, 197, 94, 0.15); color: #22c55e; padding: 2px 6px; border-radius: 4px; font-weight: 600;">Roster</span>';
            }

            return `
                <div class="replace-candidate-item" data-a-number="${w.aNumber}" ${blockedAttr} style="display: flex; align-items: center; justify-content: space-between; padding: 8px 12px; border: 1px solid var(--border-color); border-radius: var(--radius-sm); cursor: pointer; background: var(--card-bg); margin-bottom: 4px; ${blockedStyle}">
                    <div style="display: flex; align-items: center; gap: 8px; min-width: 0; flex: 1;">
                       <span style="width: 24px; height: 24px; border-radius: 50%; background: ${tradeStyle.bg}; color: ${tradeStyle.text}; display: flex; align-items: center; justify-content: center; font-size: 0.65rem; font-weight: 700; flex-shrink: 0;">${w.name.charAt(0)}</span>
                       <div style="min-width: 0; flex: 1;">
                           <div style="font-weight: 600; font-size: 0.82rem; color: var(--text-main); text-overflow: ellipsis; overflow: hidden; white-space: nowrap;">${esc(w.name)}</div>
                           <div style="font-size: 0.65rem; color: var(--text-muted);">${esc(w.trade || 'Volunteer')}</div>
                       </div>
                    </div>
                    ${statusBadge}
                </div>
            `;
        }).join('');

        if (availableWorkers.length === 0) {
            listEl.innerHTML = '<div style="color: var(--text-muted); text-align: center; padding: 12px; font-size: 0.8rem;">No available workers.</div>';
        }

        const replaceModal = document.getElementById('duty-replace-modal');
        if (replaceModal) { if (window.trapFocus) window.trapFocus(replaceModal); replaceModal.classList.add('active'); }
    }

    async function replaceWorker(dateStr, oldANumber, newANumber) {
        try {
            let schedules = await window._storage.get('wc_duty_schedules') || [];
            let daySched = schedules.find(s => s.dateStr === dateStr);
            if (!daySched) return;

            const assignmentIndex = daySched.assignments.findIndex(a => a.aNumber === oldANumber);
            if (assignmentIndex === -1) return;

            const areaId = daySched.assignments[assignmentIndex].areaId;
            const area = _areas.find(a => a.id === areaId);
            const areaName = area ? area.name : 'Unknown Area';

            const oldWorker = _workers.find(w => w.aNumber === oldANumber);
            const oldName = oldWorker ? oldWorker.name : `#${oldANumber}`;
            const newWorker = _workers.find(w => w.aNumber === newANumber);
            const newName = newWorker ? newWorker.name : `#${newANumber}`;

            const reason = await showOverrideReasonModal(newName, dateStr);
            if (reason === null) return;

            daySched.assignments[assignmentIndex].aNumber = newANumber;
            daySched.assignments[assignmentIndex].overrideReason = reason;
            daySched.assignments[assignmentIndex].assignedAt = new Date().toISOString();

            try {
            await window.dbStores.duty_schedules.setAll(schedules);
            } catch (e) {
                console.error('Failed to save schedule after replacement:', e);
                showToast('Failed to save replacement. Database error.');
                return;
            }

            if (window.markBadgesDirty) window.markBadgesDirty();
            _schedules = schedules;

            await logAssignmentChange(oldANumber, `Replaced by ${newName} on ${dateStr} in ${areaName}. Reason: ${reason}`);
            await logAssignmentChange(newANumber, `Replaced ${oldName} on ${dateStr} in ${areaName}. Reason: ${reason}`);

            showToast(`Replaced ${oldName} with ${newName}`);
            await renderCalendar();
        } catch (err) {
            console.error('Failed to replace worker on duty:', err);
            showToast('Failed to replace worker. Database error.');
        }
    }

    function renderCombinedWeekView(gridEl, dateLabel) {
        const monday = _getMonday(currentDate);
        const dates = [];
        for (let i = 0; i < 7; i++) {
            const d = new Date(monday);
            d.setDate(monday.getDate() + i);
            dates.push(d);
        }

        const first = dates[0];
        const last = dates[6];
        const monthNames = ["January", "February", "March", "April", "May", "June",
            "July", "August", "September", "October", "November", "December"
        ];
        const firstMonth = first.getMonth();
        const lastMonth = last.getMonth();
        let label;
        if (firstMonth === lastMonth) {
            label = `${first.getDate()} - ${last.getDate()} ${monthNames[firstMonth].substring(0,3)} ${first.getFullYear()}`;
        } else {
            label = `${first.getDate()} ${monthNames[firstMonth].substring(0,3)} - ${last.getDate()} ${monthNames[lastMonth].substring(0,3)} ${first.getFullYear()}`;
        }
        dateLabel.textContent = label;

        const dayNames = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
        const todayStr = todayLocal();
        const workerMap = _workers.reduce((m, w) => { m[w.aNumber] = w; return m; }, {});

        let html = '<div class="duty-cal-header">';
        html += '<div class="duty-cal-weekdays">';
        dayNames.forEach(d => { html += `<div class="duty-cal-weekday">${d}</div>`; });
        html += '</div></div>';

        html += '<div class="duty-cal-grid">';

        for (let i = 0; i < 7; i++) {
            const date = dates[i];
            const dateStr = getFormattedDateStr(date);
            const isWeekend = date.getDay() % 6 === 0;
            const isToday = dateStr === todayStr;

            const daySched = _schedules.find(s => s.dateStr === dateStr);

            html += `<div class="duty-cal-cell ${isWeekend ? 'weekend' : ''} ${isToday ? 'duty-cal-cell-today' : ''}" data-date="${dateStr}" style="padding: 6px; min-height: 220px; display: flex; flex-direction: column;">`;
            html += `<div class="duty-cal-day-label" style="font-weight: 700; margin-bottom: 6px;">${date.getDate()}</div>`;
            html += `<div class="custom-scrollbar" style="display: flex; flex-direction: column; gap: 8px; overflow-y: auto; flex: 1;">`;

            [..._areas].sort((a, b) => window.areaPriorityComparator(a.name, b.name)).forEach(area => {
                const dayAssignments = daySched ? daySched.assignments.filter(a => a.areaId === area.id) : [];
                const color = area.color || '#3b82f6';

                html += `
                    <div class="combined-area-group" style="border-left: 3px solid ${color}; background: ${hexToRgba(color, 0.06)}; padding: 4px 6px; margin-bottom: 4px; border-radius: 4px;">
                        <div style="font-size: 0.68rem; font-weight: 700; color: var(--text-main); margin-bottom: 2px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;" title="${esc(area.name)}">
                            ${esc(area.name)}
                        </div>
                        <div style="display: flex; flex-direction: column; gap: 3px;">
                `;

                var displayNames = _disambiguateSurnames(dayAssignments, workerMap);
                dayAssignments.forEach(a => {
                    const w = workerMap[a.aNumber];
                    const wName = w ? w.name : `#${a.aNumber}`;
                    const initial = w ? w.name.charAt(0) : '?';
                    const trade = w ? (w.trade || '') : '';
                    const tradeStyle = getTradeStyle(trade);
                    const isAuto = !a.overrideReason;
                    var label = displayNames[a.aNumber] || wName.split(',')[0];
                    html += `
                        <div class="duty-cal-worker-tag" data-anumber="${a.aNumber}" data-date="${dateStr}" data-area="${area.id}"
                             style="background: ${tradeStyle.bg}; color: ${tradeStyle.text}; padding: 2px 6px; font-size: 0.7rem; border-radius: 4px;"
                             title="${esc(wName)} - ${isAuto ? 'Auto-assigned' : 'Manual: ' + esc(a.overrideReason)}">
                            <span style="width:14px;height:14px;border-radius:50%;background:${tradeStyle.text}22;display:flex;align-items:center;justify-content:center;font-size:0.5rem;font-weight:800;flex-shrink:0;">${initial}</span>
                            <span style="flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">${esc(label)}</span>
                            ${!isAuto ? '<span style="font-size:0.5rem;opacity:0.6;" title="Manual override">&#9998;</span>' : ''}
                        </div>
                    `;
                });

                const assignedCount = dayAssignments.length;
                const needed = area.workersNeeded;
                for (let j = assignedCount; j < needed; j++) {
                    html += `<div class="duty-cal-empty-slot" data-date="${dateStr}" data-area-id="${area.id}" style="padding: 2px; font-size: 0.65rem; border: 1px dashed var(--border-color); text-align: center; border-radius: 4px; cursor: pointer; color: var(--text-muted);" title="Click to assign to ${esc(area.name)}">+</div>`;
                }

                html += `
                        </div>
                    </div>
                `;
            });

            html += `</div></div>`;
        }

        html += '</div>';
        gridEl.innerHTML = html;
    }

    function renderCombinedMonthView(gridEl, dateLabel) {
        const year = currentDate.getFullYear();
        const month = currentDate.getMonth();
        const monthNames = ["January", "February", "March", "April", "May", "June",
            "July", "August", "September", "October", "November", "December"
        ];
        dateLabel.textContent = `${monthNames[month]} ${year}`;

        const firstDay = new Date(year, month, 1);
        const daysInMonth = new Date(year, month + 1, 0).getDate();
        const startOffset = (firstDay.getDay() + 6) % 7;

        const todayStr = todayLocal();
        const dayNames = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];

        let html = '<div class="duty-cal-header">';
        html += '<div class="duty-cal-weekdays">';
        dayNames.forEach(d => { html += `<div class="duty-cal-weekday">${d}</div>`; });
        html += '</div></div>';

        html += '<div class="duty-cal-grid">';

        for (let i = 0; i < startOffset; i++) {
            html += `<div style="border:none;"></div>`;
        }

        for (let day = 1; day <= daysInMonth; day++) {
            const date = new Date(year, month, day);
            const dateStr = getFormattedDateStr(date);
            const isWeekend = date.getDay() % 6 === 0;
            const isToday = dateStr === todayStr;

            const daySched = _schedules.find(s => s.dateStr === dateStr);

            html += `<div class="duty-cal-cell duty-cal-cell-month ${isWeekend ? 'weekend' : ''} ${isToday ? 'duty-cal-cell-today' : ''}" data-date="${dateStr}" style="padding: 4px; min-height: 80px; cursor: pointer;">`;
            html += `<div class="duty-cal-day-label" style="font-weight: 700; margin-bottom: 4px;">${day}</div>`;
            html += `<div style="display: flex; flex-wrap: wrap; gap: 4px; align-content: flex-start;">`;

            const dots = [..._areas].sort((a, b) => window.areaPriorityComparator(a.name, b.name)).map(area => {
                const dayAssignments = daySched ? daySched.assignments.filter(a => a.areaId === area.id) : [];
                const count = dayAssignments.length;
                const needed = area.workersNeeded;
                const color = area.color || '#3b82f6';
                const pct = needed > 0 ? (count / needed) : 1;
                let borderStyle = '1px solid transparent';
                if (pct < 1) {
                    borderStyle = `1px dashed ${color}`;
                }
                return `
                    <span style="display: inline-flex; align-items: center; justify-content: center; width: 14px; height: 14px; border-radius: 50%; background: ${pct >= 1 ? color : 'transparent'}; border: ${borderStyle}; color: ${pct >= 1 ? '#ffffff' : color}; font-size: 0.55rem; font-weight: 800;"
                          title="${esc(area.name)}: ${count}/${needed} assigned">
                        ${count}
                    </span>
                `;
            }).join('');

            html += dots;
            html += `</div></div>`;
        }

        html += '</div>';
        gridEl.innerHTML = html;
    }

    /* ------ Roster Card View ------ */

    function renderRosterView(gridEl, dateLabel, area) {
        const dateStr = getFormattedDateStr(currentDate);
        const monthNames = ["January", "February", "March", "April", "May", "June",
            "July", "August", "September", "October", "November", "December"
        ];
        dateLabel.textContent = `${monthNames[currentDate.getMonth()].substring(0,3)} ${currentDate.getDate()}, ${currentDate.getFullYear()}`;

        const daySched = _schedules.find(s => s.dateStr === dateStr);
        const dayAssignments = daySched ? daySched.assignments.filter(a => a.areaId === area.id) : [];
        const workerMap = _workers.reduce((m, w) => { m[w.aNumber] = w; return m; }, {});

        let html = `<div style="font-size:0.82rem;color:var(--text-muted);margin-bottom:8px;">${esc(area.shiftTime || '22:00-06:00')} - ${area.workersNeeded} workers needed</div>`;
        html += '<div class="duty-roster-grid">';

        dayAssignments.forEach(a => {
            const w = workerMap[a.aNumber];
            if (w) {
                html += _renderRosterCard(w);
            }
        });

        const needed = area.workersNeeded;
        for (let j = dayAssignments.length; j < needed; j++) {
            html += '<div class="duty-roster-card duty-roster-empty" style="display:flex;align-items:center;justify-content:center;border:2px dashed var(--border-color);color:var(--text-muted);font-size:0.75rem;font-weight:600;">Empty</div>';
        }

        html += '</div>';
        gridEl.innerHTML = html;
    }

    function renderCombinedRosterView(gridEl, dateLabel) {
        const dateStr = getFormattedDateStr(currentDate);
        const monthNames = ["January", "February", "March", "April", "May", "June",
            "July", "August", "September", "October", "November", "December"
        ];
        dateLabel.textContent = `${monthNames[currentDate.getMonth()].substring(0,3)} ${currentDate.getDate()}, ${currentDate.getFullYear()}`;

        const daySched = _schedules.find(s => s.dateStr === dateStr);
        const workerMap = _workers.reduce((m, w) => { m[w.aNumber] = w; return m; }, {});

        let html = '<div class="duty-roster-combined">';

        [..._areas].sort((a, b) => window.areaPriorityComparator(a.name, b.name)).forEach(area => {
            const color = area.color || '#3b82f6';
            const dayAssignments = daySched ? daySched.assignments.filter(a => a.areaId === area.id) : [];
            const needed = area.workersNeeded;

            html += `
                <div class="duty-roster-area-section">
                    <div class="duty-roster-area-header" style="border-left: 3px solid ${color}; padding-left: 10px;">
                        <span style="font-weight:700;font-size:0.9rem;">${esc(area.name)}</span>
                        <span style="font-size:0.72rem;color:var(--text-muted);">${dayAssignments.length}/${needed} &middot; ${esc(area.shiftTime || '22:00-06:00')}</span>
                    </div>
                    <div class="duty-roster-grid">
            `;

            dayAssignments.forEach(a => {
                const w = workerMap[a.aNumber];
                if (w) {
                    html += _renderRosterCard(w);
                }
            });

            for (let j = dayAssignments.length; j < needed; j++) {
                html += '<div class="duty-roster-card duty-roster-empty" style="display:flex;align-items:center;justify-content:center;border:2px dashed var(--border-color);color:var(--text-muted);font-size:0.75rem;font-weight:600;">Empty</div>';
            }

            html += `
                    </div>
                </div>
            `;
        });

        html += '</div>';
        gridEl.innerHTML = html;
    }

    function _renderRosterCard(worker) {
        const tradeStyle = getTradeStyle(worker.trade);
        const photoHtml = worker.photoUrl
            ? `<img class="duty-roster-photo" src="${esc(worker.photoUrl)}" alt="${esc(worker.name)}" onerror="this.style.display='none';this.nextElementSibling.style.display='flex';">`
            : '';
        const fallbackStyle = worker.photoUrl
            ? 'display:none;'
            : 'display:flex;';
        const fallbackInitial = worker.name ? worker.name.charAt(0).toUpperCase() : '?';
        const age = typeof window.getAge === 'function' ? window.getAge(worker.birthday) : '';

        return `
            <div class="duty-roster-card" style="background:var(--card-bg);border:1px solid var(--border-color);">
                <div class="duty-roster-photo-container">
                    ${photoHtml}
                    <div class="duty-roster-photo-fallback" style="${fallbackStyle}">${fallbackInitial}</div>
                </div>
                <div class="duty-roster-info">
                    <div class="duty-roster-badge duty-roster-badge-anumber-big">#${worker.aNumber}</div>
                    <div class="duty-roster-name">${esc(worker.name)}</div>
                    <div class="duty-roster-badges">
                        <span class="duty-roster-badge duty-roster-badge-trade" style="background:${tradeStyle.bg};color:${tradeStyle.text};">${esc(worker.trade || 'Volunteer')}</span>
                        ${age ? `<span class="duty-roster-badge duty-roster-badge-age">${age}</span>` : ''}
                    </div>
                </div>
            </div>
        `;
    }

    /* ------ Roster Management ------ */

    async function openAreaRosterModal(areaId) {
        const area = _areas.find(a => a.id === areaId);
        if (!area) return;

        const roster = area.roster || [];
        const sorted = [..._workers].sort((a, b) => a.name.localeCompare(b.name));
        const listEl = document.getElementById('duty-roster-selection-list');

        const rosterTitle = document.getElementById('duty-roster-modal-title');
        if (rosterTitle) rosterTitle.textContent = `Roster for ${esc(area.name)}`;
        const rosterModal = document.getElementById('duty-roster-modal');
        if (rosterModal) rosterModal.dataset.areaId = areaId;

        if (listEl) {
            listEl.innerHTML = `<div class="worker-roster-grid-4col" style="gap: 6px;">` + sorted.map(w => {
                const avatar = (w.photoUrl || w.photo)
                    ? `<img src="${esc(w.photoUrl || w.photo)}" style="width:28px;height:28px;border-radius:50%;object-fit:cover;flex-shrink:0;border:1px solid var(--border-color);">`
                    : `<div style="width:28px;height:28px;border-radius:50%;background:var(--primary-light);color:var(--primary-color);display:flex;align-items:center;justify-content:center;font-size:0.75rem;font-weight:700;flex-shrink:0;">${(w.name || '?').charAt(0).toUpperCase()}</div>`;
                const isChecked = roster.includes(w.aNumber);
                return `
                <label class="roster-candidate-item" style="display: flex; align-items: center; gap: 8px; padding: 6px 8px; border: 1px solid var(--border-color); border-radius: 6px; cursor: pointer; background: var(--card-bg); ${isChecked ? 'border-color: var(--primary-color); background: rgba(59, 130, 246, 0.06);' : ''}" title="Add/remove ${esc(w.name)} from this area's roster">
                    <input type="checkbox" value="${w.aNumber}" ${isChecked ? 'checked' : ''} style="flex-shrink: 0; accent-color: var(--primary-color);">
                    ${avatar}
                    <div style="flex:1;min-width:0;line-height:1.25;">
                        <div style="font-weight:700;color:var(--text-main);font-size:0.78rem;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;">${esc(w.name)}</div>
                        <div style="font-size:0.67rem;color:var(--text-muted);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;"><span style="font-weight:600;color:var(--primary-color);">#${esc(w.aNumber)}</span> &middot; ${esc(w.trade || 'Volunteer')}</div>
                    </div>
                </label>
            `;
            }).join('') + `</div>`;
        }

        const rosterSearch = document.getElementById('duty-roster-search');
        if (rosterSearch) rosterSearch.value = '';
        if (rosterModal) { if (window.trapFocus) window.trapFocus(rosterModal); rosterModal.classList.add('active'); }
    }

    async function saveAreaRoster() {
        const rosterModal = document.getElementById('duty-roster-modal');
        if (!rosterModal) return;
        const areaId = rosterModal.dataset.areaId;
        if (!areaId) return;

        const checked = document.querySelectorAll('#duty-roster-selection-list input:checked');
        const selectedIds = Array.from(checked).map(cb => parseInt(cb.value, 10));

        const area = _areas.find(a => a.id === areaId);
        if (area) {
            area.roster = selectedIds;
            await window.dbStores.duty_areas.setItem(areaId, area);
        }

        releaseFocusTrap();
        rosterModal.classList.remove('active');
        await loadData();
    }

    /* ------ Week Schedule Generation ------ */

    async function generateWeekSchedule(area) {
        const monday = _getMonday(currentDate);
        const weekDates = [];
        for (let i = 0; i < 7; i++) {
            const d = new Date(monday);
            d.setDate(monday.getDate() + i);
            weekDates.push(d);
        }

        const roster = area.roster || [];
        if (roster.length === 0) {
            showToast(`"${area.name}" has no roster set.`);
            return;
        }

        const schedules = await window.dbStores.duty_schedules.getAll();
        let rotationIdx = area.lastRotationIndex || 0;

        for (const date of weekDates) {
            const dateStr = getFormattedDateStr(date);
            let daySched = schedules.find(s => s.dateStr === dateStr);
            if (!daySched) {
                daySched = { dateStr, assignments: [] };
                schedules.push(daySched);
            }

            // Count existing assignments for this area
            const existingForArea = daySched.assignments.filter(a => a.areaId === area.id);
            const needed = area.workersNeeded || 1;
            const toAdd = needed - existingForArea.length;

            if (toAdd > 0) {
                let attempts = 0;
                while (daySched.assignments.filter(a => a.areaId === area.id).length < needed && attempts < roster.length) {
                    const workerId = roster[rotationIdx % roster.length];
                    const alreadyAssigned = daySched.assignments.some(a => a.aNumber === workerId);
                    const blocked = window.isWorkerBlockedFromArea ? window.isWorkerBlockedFromArea(workerId, area.name) : false;
                    if (!alreadyAssigned && !blocked) {
                        daySched.assignments.push({
                            aNumber: workerId,
                            areaId: area.id,
                            overrideReason: null,
                            assignedAt: new Date().toISOString()
                        });
                    }
                    rotationIdx++;
                    attempts++;
                }
            }
        }

        // Save updated rotation index
        area.lastRotationIndex = rotationIdx;
        try {
            await window.dbStores.duty_areas.setItem(area.id, area);
            await window._storage.set('wc_duty_schedules', schedules);
        } catch (e) {
            console.error('Failed to save generated schedule:', e);
            showToast('Failed to save generated schedule.');
            return;
        }
        if (window.markBadgesDirty) window.markBadgesDirty();

        showToast(`Schedule generated for "${area.name}" this week.`);
        _schedules = schedules;
        await renderCalendar();
    }

    /* ------ Week Schedule Clearing ------ */

    async function clearSchedules(type) {
        const isCombined = selectedAreaId === 'combined';
        let targetAreaName = 'ALL areas';
        if (!isCombined) {
            const area = _areas.find(a => a.id === selectedAreaId);
            if (!area) return;
            targetAreaName = area.name;
        }

        let datesToClear = [];
        let confirmMsg = '';

        if (type === 'day') {
            const dateStr = getFormattedDateStr(currentDate);
            datesToClear.push(dateStr);
            confirmMsg = `Are you sure you want to clear all scheduled volunteers on ${dateStr} for ${targetAreaName}?`;
        } else if (type === 'week') {
            const monday = _getMonday(currentDate);
            for (let i = 0; i < 7; i++) {
                const d = new Date(monday);
                d.setDate(monday.getDate() + i);
                datesToClear.push(getFormattedDateStr(d));
            }
            confirmMsg = `Are you sure you want to clear all scheduled volunteers for the week of ${datesToClear[0]} to ${datesToClear[6]} for ${targetAreaName}?`;
        }

        if (!await showConfirm(confirmMsg, { danger: true, title: 'Clear Schedule', confirmText: 'Clear' })) return;

        let schedules = await window._storage.get('wc_duty_schedules') || [];
        const backup = structuredClone(schedules);

        try {
            let clearedCount = 0;
            const workersToLog = [];

            schedules.forEach(s => {
                if (datesToClear.includes(s.dateStr)) {
                    const before = s.assignments.length;
                    const removed = s.assignments.filter(a => isCombined || a.areaId === selectedAreaId);
                    removed.forEach(a => {
                        workersToLog.push({
                            aNumber: a.aNumber,
                            dateStr: s.dateStr,
                            areaId: a.areaId
                        });
                    });

                    s.assignments = s.assignments.filter(a => !(isCombined || a.areaId === selectedAreaId));
                    clearedCount += (before - s.assignments.length);
                }
            });

            if (clearedCount === 0) {
                showToast('No assignments found to clear.');
                return;
            }

            await window._storage.set('wc_duty_schedules', schedules);
            _schedules = schedules;

            // Log changes in historical logs
            const areaMap = _areas.reduce((m, a) => { m[a.id] = a.name; return m; }, {});
            for (const item of workersToLog) {
                const aName = areaMap[item.areaId] || 'Unknown Area';
                await logAssignmentChange(item.aNumber, `Cleared schedule for ${aName} on ${item.dateStr} via bulk clear`);
            }

            if (window.markBadgesDirty) window.markBadgesDirty();
            var restoreSchedules = structuredClone(backup);
            window.undoManager && window.undoManager.capture(function () {
                window._storage.set('wc_duty_schedules', restoreSchedules).then(function () {
                    _schedules = restoreSchedules;
                    renderCalendar();
                    showToast('Undo: schedules restored.');
                });
            }, { label: 'clear schedules' });
            window.showUndoToast('Cleared ' + clearedCount + ' volunteer slot(s).', function () {
                return window.undoManager && window.undoManager.undo();
            });
            await renderCalendar();
        } catch (e) {
            schedules = backup;
            await window._storage.set('wc_duty_schedules', backup);
            console.error('Schedule clearing failed, rolled back:', e);
            showToast('Failed to clear schedules. Changes reverted.');
        }
    }

    /* ------ Area Delete with Cascade ------ */

    async function _deleteDutyArea(area) {
        if (!await showConfirm(`Delete duty area "${area.name}"? This will remove its worker slots from all schedules.`, { danger: true, title: 'Delete Area', confirmText: 'Delete' })) return;
        let schedules = await window._storage.get('wc_duty_schedules') || [];
        let changed = false;
        schedules.forEach(s => {
            const before = s.assignments.length;
            s.assignments = s.assignments.filter(a => a.areaId !== area.id);
            if (s.assignments.length !== before) changed = true;
        });
        try {
            if (changed) await window._storage.set('wc_duty_schedules', schedules);
            const allAreas = await window._storage.get('wc_duty_areas') || [];
            const filtered = allAreas.filter(a => a.id !== area.id);
            await window._storage.set('wc_duty_areas', filtered);
        } catch (e) {
            console.error('Failed to delete duty area:', e);
            showToast('Failed to delete area due to a storage error.');
            return;
        }
        if (window.markBadgesDirty) window.markBadgesDirty();

        if (selectedAreaId === area.id) {
            selectedAreaId = null;
        }

        var restoreArea = structuredClone(area);
        var restoreSchedules = structuredClone(schedules);
        window.undoManager && window.undoManager.capture(function () {
            window._storage.get('wc_duty_areas').then(function (areas) {
                areas.push(restoreArea);
                window._storage.set('wc_duty_areas', areas);
            });
            window._storage.get('wc_duty_schedules').then(function (sched) {
                restoreSchedules.forEach(function (rs) {
                    var existing = sched.findIndex(function (s) { return s.dateStr === rs.dateStr; });
                    if (existing >= 0) sched[existing] = rs;
                    else sched.push(rs);
                });
                window._storage.set('wc_duty_schedules', sched);
            }).then(function () {
                showToast('Undo: area "' + restoreArea.name + '" restored.');
                if (window.markBadgesDirty) window.markBadgesDirty();
                _dirty = true;
                loadData();
            });
        }, { label: 'delete duty area' });
        window.showUndoToast('Area "' + area.name + '" deleted.', function () {
            return window.undoManager && window.undoManager.undo();
        });

        _dirty = true;
        await loadData();
    }

    /* ------ Date Utils ------ */

    function getFormattedDateStr(date) {
        const y = date.getFullYear();
        const m = String(date.getMonth() + 1).padStart(2, '0');
        const d = String(date.getDate()).padStart(2, '0');
        return `${y}-${m}-${d}`;
    }

    /* ------ Override Modal ------ */

    let _overrideResolve = null;

    function closeOverrideModal() {
        const overrideModal = document.getElementById('duty-override-modal');
        if (overrideModal) { releaseFocusTrap(); overrideModal.classList.remove('active'); }
        if (_overrideResolve) { const r = _overrideResolve; _overrideResolve = null; r(null); }
    }

    // Expose for inline onclick handlers in the template
    window.__dutyCloseOverride = closeOverrideModal;

    function showOverrideReasonModal(workerName, targetDate) {
        return new Promise((resolve) => {
            const modal = document.getElementById('duty-override-modal');
            const info = document.getElementById('duty-override-info');
            const reasonEl = document.getElementById('duty-override-reason');
            const confirmBtn = document.getElementById('btn-override-confirm');
            if (!modal || !info || !reasonEl || !confirmBtn) {
                resolve(null);
                return;
            }
            info.textContent = 'Moving ' + workerName + ' to schedule on ' + targetDate + '.';
            reasonEl.value = '';
            if (window.trapFocus) window.trapFocus(modal);
            modal.classList.add('active');
            setTimeout(function () { reasonEl.focus(); }, 100);
            _overrideResolve = resolve;
            var onConfirm = function () {
                var reason = reasonEl.value.trim();
                if (!reason) {
                    showToast('A reason is required for manual override.');
                    return;
                }
                releaseFocusTrap();
                modal.classList.remove('active');
                if (_overrideResolve) { var r = _overrideResolve; _overrideResolve = null; r(reason); }
                confirmBtn.removeEventListener('click', onConfirm);
            };
            confirmBtn.addEventListener('click', onConfirm);
        });
    }

    /* ------ Legacy: updateScheduleAssignment (kept for drag-drop compat) ------ */

    async function updateScheduleAssignment(dateStr, aNumber, newAreaId = null, remove = false, reason = '') {
        let schedules = await window._storage.get('wc_duty_schedules') || [];
        const backup = structuredClone(schedules);
        try {
            let daySched = schedules.find(s => s.dateStr === dateStr);
            if (!daySched) {
                daySched = { dateStr, assignments: [] };
                schedules.push(daySched);
            }

            if (remove) {
                daySched.assignments = daySched.assignments.filter(a => a.aNumber !== aNumber);
            } else {
                const existing = daySched.assignments.find(a => a.aNumber === aNumber);
                if (existing) {
                    existing.areaId = newAreaId;
                    if (reason) existing.overrideReason = reason;
                } else {
                    daySched.assignments.push({ aNumber, areaId: newAreaId, overrideReason: reason || null, assignedAt: new Date().toISOString() });
                }
            }

            await window._storage.set('wc_duty_schedules', schedules);
        } catch (e) {
            schedules = backup;
            await window._storage.set('wc_duty_schedules', backup);
            console.error('Schedule update failed, rolled back:', e);
            showToast('Failed to update schedule. Changes reverted.');
        }
        if (window.markBadgesDirty) window.markBadgesDirty();
        _schedules = await window._storage.get('wc_duty_schedules') || [];
        await renderCalendar();
    }

    async function logAssignmentChange(aNumber, note) {
        if (!window.dbStores || !window.dbStores.workers) return;
        let worker = null;
        await window.dbStores.workers.iterate(w => {
            if (w.aNumber === aNumber) worker = w;
        });
        if (worker && window.historyData) {
            window.historyData.push({
                id: safeUUID(),
                workerId: worker.aNumber,
                date: new Date().toISOString(),
                action: 'Duty Override',
                details: note
            });
            if (typeof window.saveHistory === 'function') window.saveHistory();
        }
    }

    // Public interface
    window.removeDutyAssignment = async (dateStr, aNumber) => {
        if (await showConfirm('Remove this worker from duty on this day?')) {
            await updateScheduleAssignment(dateStr, aNumber, null, true);
        }
    };

    function getRosteredTonight() {
        var todayStr = new Date().toISOString().slice(0, 10);
        var todaySched = _schedules.find(function (s) { return s.dateStr === todayStr; });
        if (!todaySched) return [];
        return (todaySched.assignments || []).map(function (a) { return a.aNumber; });
    }

    function _buildPrintHtml() {
        var dateStr = getFormattedDateStr(currentDate);
        var daySched = _schedules.find(function (s) { return s.dateStr === dateStr; });
        if (!daySched || daySched.assignments.length === 0) {
            showToast('No schedules found for tonight.');
            return null;
        }

        var workerMap = _workers.reduce(function (m, w) { m[w.aNumber] = w; return m; }, {});
        var monthNames = ["January", "February", "March", "April", "May", "June",
            "July", "August", "September", "October", "November", "December"
        ];
        var d = new Date(dateStr + 'T12:00:00');
        var headerDate = monthNames[d.getMonth()] + ' ' + d.getDate() + ', ' + d.getFullYear();
        var now = new Date();
        var genDate = monthNames[now.getMonth()] + ' ' + now.getDate() + ', ' + now.getFullYear();

        var sectionsHtml = '';
        var grandTotal = 0;
        var grandTrades = {};

        var sortedAreas = [..._areas].sort(function (a, b) { return window.areaPriorityComparator(a.name, b.name); });

        sortedAreas.forEach(function (area) {
            var dayAssignments = daySched.assignments.filter(function (a) { return a.areaId === area.id; });
            var needed = area.workersNeeded || 0;
            var color = area.color || '#3b82f6';
            var shiftTime = area.shiftTime || '22:00-06:00';

            if (dayAssignments.length === 0 && needed === 0) return;

            var cardsHtml = '';
            var areaTradeCounts = {};
            var areaTotal = 0;

            dayAssignments.forEach(function (a) {
                var w = workerMap[a.aNumber];
                if (!w) return;
                areaTotal++;
                grandTotal++;
                var t = w.trade || 'Volunteer';
                areaTradeCounts[t] = (areaTradeCounts[t] || 0) + 1;
                grandTrades[t] = (grandTrades[t] || 0) + 1;

                var tradeStyle = getTradeStyle(w.trade);
                var photo = w.photoUrl || w.photo;
                var hasPhoto = !!photo;
                var photoHtml = hasPhoto
                    ? '<img src="' + esc(photo) + '" alt="" style="width:100%;height:100%;object-fit:cover;display:block;">'
                    : '';
                var fallbackDisplay = hasPhoto ? 'none' : 'flex';
                var fallbackInitial = w.name ? w.name.charAt(0).toUpperCase() : '?';
                var age = typeof window.getAge === 'function' ? window.getAge(w.birthday) : '';

                cardsHtml += '<div class="dp-card" style="break-inside:avoid;page-break-inside:avoid;display:flex;align-items:center;gap:10px;padding:6px 10px;border:1px solid #e2e8f0;border-radius:6px;background:' + tradeStyle.bg + '30;border-left:3px solid ' + tradeStyle.text + ';">';
                cardsHtml += '<div style="width:48px;height:48px;border-radius:6px;overflow:hidden;border:1px solid #e2e8f0;flex-shrink:0;position:relative;background:linear-gradient(135deg,#2563eb,#1d4ed8);display:flex;align-items:center;justify-content:center;">';
                cardsHtml += photoHtml;
                cardsHtml += '<div style="display:' + fallbackDisplay + ';align-items:center;justify-content:center;width:100%;height:100%;font-size:1.3rem;font-weight:800;color:#fff;">' + fallbackInitial + '</div>';
                cardsHtml += '</div>';
                cardsHtml += '<div style="flex:1;min-width:0;">';
                cardsHtml += '<div style="display:flex;align-items:baseline;gap:6px;">';
                cardsHtml += '<span style="font-weight:700;font-size:0.9rem;color:#0f172a;">' + esc(w.name) + '</span>';
                cardsHtml += '<span style="font-size:0.65rem;color:#64748b;font-weight:600;">#' + w.aNumber + '</span>';
                cardsHtml += '</div>';
                cardsHtml += '<div style="display:flex;align-items:center;gap:4px;margin-top:2px;">';
                cardsHtml += '<span style="font-size:0.7rem;font-weight:600;padding:1px 6px;border-radius:4px;background:' + tradeStyle.bg + ';color:' + tradeStyle.text + ';">' + esc(w.trade || 'Volunteer') + '</span>';
                if (age) cardsHtml += '<span style="font-size:0.65rem;color:#64748b;">' + age + '</span>';
                cardsHtml += '</div></div></div>';
            });

            for (var j = dayAssignments.length; j < needed; j++) {
                cardsHtml += '<div style="break-inside:avoid;page-break-inside:avoid;display:flex;align-items:center;justify-content:center;padding:12px;border:2px dashed #cbd5e1;border-radius:6px;color:#94a3b8;font-size:0.75rem;font-weight:600;min-height:48px;">Empty</div>';
                areaTotal++;
                grandTotal++;
            }

            var tradeSummary = Object.keys(areaTradeCounts).sort().map(function (t) {
                return esc(t) + ' (' + areaTradeCounts[t] + ')';
            }).join(', ');

            sectionsHtml += '<div class="dp-section" style="break-inside:avoid;page-break-inside:avoid;border:1px solid #e2e8f0;border-radius:8px;overflow:hidden;margin-bottom:16px;">';
            sectionsHtml += '<div style="border-left:4px solid ' + color + ';background:#f8fafc;padding:8px 12px;display:flex;justify-content:space-between;align-items:center;">';
            sectionsHtml += '<div><span style="font-weight:800;font-size:0.95rem;color:#0f172a;">' + esc(area.name) + '</span>';
            sectionsHtml += '<span style="font-size:0.7rem;color:#64748b;margin-left:8px;">' + dayAssignments.length + '/' + needed + ' &middot; ' + esc(shiftTime) + '</span></div>';
            sectionsHtml += '</div>';
            sectionsHtml += '<div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(240px,1fr));gap:6px;padding:8px;">' + cardsHtml + '</div>';
            if (tradeSummary) {
                sectionsHtml += '<div style="font-size:0.65rem;color:#64748b;border-top:1px solid #e2e8f0;padding:4px 12px;">' + tradeSummary + ' &middot; Total: ' + areaTotal + '</div>';
            }
            sectionsHtml += '</div>';
        });

        var grandTradeSummary = Object.keys(grandTrades).sort().map(function (t) {
            return esc(t) + ' (' + grandTrades[t] + ')';
        }).join(', ');

        return '<!DOCTYPE html><html><head><meta charset="utf-8"><title>Night Duty Roster - ' + headerDate + '</title>' +
            '<style>' +
            '@page { size: auto; margin: 8mm 10mm; }' +
            '* { box-sizing: border-box; margin: 0; padding: 0; }' +
            'body { font-family: \'Inter\', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; color: #0f172a; background: #fff; padding: 0; }' +
            '.dp-header { text-align: center; border-bottom: 2px solid #1e40af; padding-bottom: 10px; margin-bottom: 16px; }' +
            '.dp-header h1 { font-size: 1.2rem; font-weight: 800; margin: 0 0 2px 0; letter-spacing: 0.02em; }' +
            '.dp-header .sub { font-size: 0.7rem; color: #64748b; text-transform: uppercase; letter-spacing: 0.1em; }' +
            '.dp-header .date { font-size: 0.85rem; color: #475569; margin-top: 2px; }' +
            '.dp-footer { text-align: center; font-size: 0.65rem; color: #94a3b8; border-top: 1px solid #e2e8f0; padding-top: 8px; margin-top: 16px; }' +
            '.dp-grand-summary { font-size: 0.7rem; color: #475569; text-align: center; margin-bottom: 16px; padding: 6px 12px; background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 6px; }' +
            '@media print { body { padding: 0 !important; margin: 0 auto !important; box-sizing: border-box !important; } .dp-section { break-inside: avoid !important; page-break-inside: avoid !important; } .dp-card { break-inside: avoid !important; page-break-inside: avoid !important; } img { image-rendering: -webkit-optimize-contrast; } }' +
            '</style></head><body>' +
            '<div class="dp-header">' +
            '<div class="sub">Service Command Center</div>' +
            '<h1>Night Duty Roster</h1>' +
            '<div class="date">' + headerDate + '</div>' +
            '</div>' +
            '<div class="dp-grand-summary">' + grandTradeSummary + ' &middot; Total: ' + grandTotal + '</div>' +
            sectionsHtml +
            '<div class="dp-footer">Generated on ' + genDate + '</div>' +
            '</body></html>';
    }

    return {
        init,
        loadData,
        getRosteredTonight,
        renderCalendar,
        requestWeekGeneration
    };
})();
return {
    DutyModule: DutyModule
};
})();

// ===== modals.js (focus trap) - verbatim from 01_Rust_WC_DB/js/modals.js =====
var S5_modals = (function () {

let lastFocusedElement = null;

let _focusTrapStack = [];

function restoreFocusAfterClose() {
    var f = lastFocusedElement;
    lastFocusedElement = null;
    if (f && typeof f.focus === 'function') setTimeout(function () { f.focus(); }, 50);
}

function _focusFirstElement(modalEl) {
    if (!modalEl) return;
    var auto = modalEl.querySelector('[autofocus]');
    if (auto) {
        setTimeout(function () { auto.focus(); }, 50);
        return;
    }
    var focusable = modalEl.querySelectorAll('input:not([type="hidden"]), select, textarea, button:not(.close-btn), [href], [tabindex]:not([tabindex="-1"])');
    if (focusable.length > 0) {
        setTimeout(function () { focusable[0].focus(); }, 50);
    }
}

function trapFocus(modalEl) {
    if (!modalEl) return;
    _focusTrapStack.push(modalEl);
    _focusFirstElement(modalEl);
}

function releaseFocusTrap() {
    _focusTrapStack.pop();
    if (_focusTrapStack.length > 0) {
        _focusFirstElement(_focusTrapStack[_focusTrapStack.length - 1]);
    }
}
return {
    trapFocus: trapFocus,
    releaseFocusTrap: releaseFocusTrap
};
})();

// ===== Bridge: SiteMonitor data in, Service 5 templates out =====

function api(path) {
  return fetch('/api' + path, { credentials: 'same-origin' }).then(function (r) {
    if (!r.ok) return r.json().catch(function () { return {}; }).then(function (b) { throw new Error(b.error || ('The server said ' + r.status)); });
    return r.json();
  });
}

// Records come back exactly as Service 5 stored them.
function records(name) { return api('/workforce/records/' + name).catch(function () { return []; }); }

function loadWorkers() {
  return api('/workforce/family-roster').then(function (list) {
    workersData = Array.isArray(list) ? list : [];
    return workersData;
  });
}

function load(what) {
  var jobs = [];
  if (what.indexOf('workers') >= 0) jobs.push(loadWorkers());
  if (what.indexOf('signatories') >= 0) jobs.push(records('signatories').then(function (l) { signatoriesData = l; }));
  if (what.indexOf('visitations') >= 0) jobs.push(records('visitations').then(function (l) { visitationsData = l; }));
  if (what.indexOf('visitation_groups') >= 0) jobs.push(records('visitation_groups').then(function (l) { visitationGroupsData = l; }));
  if (what.indexOf('history') >= 0) jobs.push(records('history').then(function (l) { historyData = l; }));
  if (what.indexOf('attendance_log') >= 0) jobs.push(records('attendance_log').then(function (l) { attendanceLog = l; }));
  if (what.indexOf('assignment_log') >= 0) jobs.push(records('assignment_log').then(function (l) { assignmentLog = l; }));
  // Group changes: Service 5's own log, plus the group moves SiteMonitor records in the assignment log.
  if (what.indexOf('group_changes') >= 0) jobs.push(Promise.all([records('group_changes'), records('assignment_log')]).then(function (r) {
    groupChanges = [];
    r[0].forEach(function (x) { if (Array.isArray(x)) groupChanges.push.apply(groupChanges, x); else if (x && Array.isArray(x.data)) groupChanges.push.apply(groupChanges, x.data); else if (x) groupChanges.push(x); });
    r[1].filter(function (e) { return e && e.field === 'projectGroup'; }).forEach(function (e) {
      var ts = typeof e.timestamp === 'number' ? new Date(e.timestamp).toISOString() : (e.timestamp || (e.date ? e.date + 'T08:00:00' : ''));
      groupChanges.push({ timestamp: ts, workerId: String(e.aNumber || ''), workerName: e.workerName || '', trade: e.trade || '',
        fromGroup: e.oldValue || '(none)', toGroup: e.newValue || '(none)', reason: e.reason || '' });
    });
  }));
  return Promise.all(jobs);
}

function indicesOf(ids) {
  var set = new Set();
  (ids || []).forEach(function (id) {
    var i = workersData.findIndex(function (w) { return w.id === id; });
    if (i >= 0) set.add(i);
  });
  return set;
}

// Service 5 opens some reports in a new tab. The tab is opened right away,
// while the click still counts, and filled once the data has arrived.
function popup() {
  var pre = null;
  try { pre = window.open('', '_blank'); } catch (e) { pre = null; }
  if (pre) {
    // A finished placeholder: Service 5's next write replaces it entirely.
    try { pre.document.open(); pre.document.write('<!DOCTYPE html><title>Preparing…</title><p style="font:14px Inter,system-ui,sans-serif;color:#64748b;padding:24px">Preparing the report…</p>'); pre.document.close(); } catch (e) { /* ignore */ }
  }
  return {
    run: function (fn) {
      var orig = window.open;
      window.open = function () { window.open = orig; return pre; };
      try { return fn(); } finally { window.open = orig; }
    },
    fail: function (msg) { if (pre) { try { pre.close(); } catch (e) { /* ignore */ } } showToast(msg); }
  };
}

// Service 5's on-screen print views (job orders, visit sheets) are shown in
// SiteMonitor inside a preview frame with Service 5's own stylesheet.
function previewDoc(title, inner, css) {
  return '<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"><title>' + esc(title) + '</title>' +
    '<link rel="stylesheet" href="/print/s5print.css">' +
    '<style>@font-face{font-family:"Inter";src:url("/fonts/inter-variable.woff2") format("woff2");font-weight:100 900;font-display:block}' +
    'body{font-family:"Inter",-apple-system,BlinkMacSystemFont,Arial,sans-serif;margin:0;background:#f1f5f9;color:#0f172a}</style>' +
    '<style id="global-page-spec">' + (css || '') + '</style></head><body>' +
    '<main id="print-view" class="view print-view active"><div id="print-container">' + inner + '</div></main></body></html>';
}

function fail(e) { showToast(e && e.message ? e.message : 'Could not prepare the report.'); throw e; }

window.S5Print = {
  // Job orders: Service 5's "Generate Forms" print view (4 per folio page).
  jobOrders: function (o) {
    o = o || {};
    return load(['workers', 'signatories']).then(function () {
      var D = S5_distribution;
      D.selectedWorkerIndices.clear();
      indicesOf(o.ids).forEach(function (i) { D.selectedWorkerIndices.add(i); });
      var box = document.createElement('div');
      D.setInputs(box, { value: o.date || '' }, { value: o.mode || 'all' }, { value: o.sig || ((signatoriesData[0] || {}).name || '') });
      D.generateFormsForPrinting();
      var spec = document.getElementById('global-page-spec');
      var css = spec ? spec.textContent : '';
      if (spec) spec.remove();
      return previewDoc('Job Orders', box.innerHTML, css);
    }).catch(fail);
  },

  // Personnel Directory: By Trade, By Project Group, By Housing.
  personnelReport: function (mode) {
    var p = popup();
    return loadWorkers().then(function () { p.run(function () { S5_personnel.printPersonnelReport(mode); }); }, function (e) { p.fail(e.message); });
  },

  // Volunteer & Family Roster (cards or table).
  familyRoster: function (o) {
    o = o || {};
    var p = popup();
    return loadWorkers().then(function () {
      var list = o.ids && o.ids.length ? Array.from(indicesOf(o.ids)).map(function (i) { return workersData[i]; }) : null;
      p.run(function () { S5_personnel.printVolunteerFamilyRoster(list, { layout: o.layout || 'cards', paperSize: o.paperSize || 'folio' }); });
    }, function (e) { p.fail(e.message); });
  },

  // Personnel info cards for the ticked people.
  idCards: function (ids) {
    var p = popup();
    return loadWorkers().then(function () {
      var set = indicesOf(ids);
      if (!set.size) return p.fail('Select at least one volunteer first.');
      p.run(function () { S5_personnel.printSelectedPersonnelCards(set); });
    }, function (e) { p.fail(e.message); });
  },

  // The Cards page's "Generate List".
  cardsList: function (ids) {
    return loadWorkers().then(function () {
      var C = S5_cards;
      C.cardsSelectedIndices.clear();
      indicesOf(ids).forEach(function (i) { C.cardsSelectedIndices.add(i); });
      C.printSelectedCardsList();
    }).catch(fail);
  },

  // Print & Export: table or profile layout with the chosen fields.
  printExport: function (o) {
    var p = popup();
    return loadWorkers().then(function () {
      var workers = Array.from(indicesOf(o.ids)).map(function (i) { return workersData[i]; }).filter(Boolean).sort(function (a, b) { return a.name.localeCompare(b.name); });
      if (!workers.length) return p.fail('No workers selected.');
      if (o.layout === 'roster' || o.layout === 'family-table') {
        return p.run(function () { S5_personnel.printVolunteerFamilyRoster(workers, { layout: o.layout === 'family-table' ? 'table' : 'cards' }); });
      }
      var html = o.layout === 'profile'
        ? S5_personnel.generateProfilePrintHTML(workers, o.fields, !!o.individual)
        : S5_personnel.generateTablePrintHTML(workers, o.fields, !!o.individual);
      p.run(function () { if (html) S5_personnel._openPrintWin(html); });
    }, function (e) { p.fail(e.message); });
  },

  // The profile's "Print Family Info".
  familyInfo: function (id) {
    return loadWorkers().then(function () {
      var i = workersData.findIndex(function (w) { return w.id === id; });
      if (i < 0) return showToast('No volunteer selected.');
      var html = S5_personnel.familyReportHTML(i);
      if (html) window.safePrint(html);
    }).catch(fail);
  },

  // The profile's attendance dossier (needs the person's attendance history).
  attendanceDossier: function (id) {
    return api('/workers/' + id + '/profile').then(function (w) {
      w.id = id;
      S5_personnel.printWorkerAttendanceDossier(w);
    }).catch(fail);
  },

  // Semi-annual records update forms.
  updateForms: function (o) {
    o = o || {};
    var options = o.options || {};
    // Service 5's blank form records, verbatim (records_update_form.js).
    function blanks(qty) {
      return Array.from({ length: qty }, () => ({
                    name: '',
                    aNumber: '',
                    trade: '',
                    skillLevel: '',
                    birthday: '',
                    civilStatus: '',
                    phone: '',
                    address: '',
                    workResidence: '',
                    visitationHousing: '',
                    assignedUnit: '',
                    bloodType: '',
                    covidStatus: '',
                    medical: '',
                    maintenanceMeds: [],
                    emergencyContact: '',
                    emergencyPhone: '',
                    spouseName: '',
                    spouseBirthday: '',
                    weddingDate: '',
                    spouseChurchDuty: '',
                    spouseContact: '',
                    spouseKasalSaIglesia: '',
                    spouseNagsasama: '',
                    children: [],
                    visitationGroup: '',
                    visitationRole: '',
                    projectGroup: '',
                    projectRole: ''
                }));
    }
    return loadWorkers().then(function () {
      var list = options.mode === 'blank' ? blanks(parseInt(o.blank, 10) || 10)
        : Array.from(indicesOf(o.ids)).map(function (i) { return workersData[i]; });
      if (!list.length) return showToast('No active volunteers found for printing.');
      S5_records_update_form.printUpdateForms(list, options);
    }).catch(fail);
  },

  // Visit sheets: one visit, every scheduled visit, or the last 7 days' completed visits.
  visitSheets: function (o) {
    o = o || {};
    return load(['workers', 'visitations']).then(function () {
      var V = S5_visitations;
      var pick;
      if (o.kind === 'one') {
        pick = visitationsData.filter(function (v) { return v.id === o.id; });
        return pick.length ? previewDoc('Visitation', V.generateMinisterSummaryHtml(pick[0].ministerName, pick), '') : '';
      }
      if (o.kind === 'week') {
        var now = new Date();
        var sevenDaysAgo = new Date(now);
        sevenDaysAgo.setDate(sevenDaysAgo.getDate() - 7);
        pick = visitationsData.filter(function (v) {
          if (v.status !== 'completed') return false;
          var d = new Date(v.completedAt || v.date);
          return d >= sevenDaysAgo && d <= now;
        });
        if (!pick.length) { showToast('No completed visits in the last 7 days.'); return ''; }
      } else {
        pick = visitationsData.filter(function (v) { return v.status === 'scheduled'; });
        if (!pick.length) { showToast('No scheduled visits to print.'); return ''; }
      }
      var grouped = {};
      pick.forEach(function (v) { (grouped[v.ministerName] = grouped[v.ministerName] || []).push(v); });
      var fullHtml = '';
      Object.keys(grouped).sort().forEach(function (ministerName) {
        var list = grouped[ministerName];
        for (var i = 0; i < list.length; i += 2) fullHtml += V.generateMinisterSummaryHtml(ministerName, list.slice(i, i + 2));
      });
      return previewDoc('Visitations', fullHtml, '');
    }).catch(fail);
  },

  // List & Attendance Generator.
  attendanceSheet: function (o) {
    return loadWorkers().then(function () {
      var A = S5_attendance_generator;
      var keys = Array.from(indicesOf(o.ids)).map(function (i) { return A.getWorkerKey(workersData[i]); });
      A.configure(Object.assign({}, o, { keys: keys }));
      A.generatePrintSheet();
    }).catch(fail);
  },

  // Visitation groups: one group's roster, or every group's summary (new tab).
  visitationRoster: function (groupName) {
    var p = popup();
    return load(['workers', 'visitation_groups']).then(function () {
      var g = visitationGroupsData.find(function (x) { return x.name === groupName || x.id === groupName; });
      if (!g) return p.fail('Group not found.');
      p.run(function () { S5_visitation_groups.printGroupRoster(g.id); });
    }, function (e) { p.fail(e.message); });
  },
  visitationSummary: function () {
    var p = popup();
    return load(['workers', 'visitation_groups']).then(function () {
      if (!visitationGroupsData.length) return p.fail('No visitation groups to print.');
      p.run(function () { S5_visitation_groups.printAllGroupsSummary(); });
    }, function (e) { p.fail(e.message); });
  },

  // Monthly attendance sheets (month is 0-11, as in Service 5).
  monthlySheet: function (o) {
    return load(['workers', 'visitation_groups']).then(function () {
      var V = S5_visitation_groups;
      if (o.all) return V.printAllMonthlyAttendanceSheets(o.year, o.month, o.paperSize || 'folio', o.officers !== false);
      var g = visitationGroupsData.find(function (x) { return x.name === o.group || x.id === o.group; });
      if (!g) return showToast('Group not found.');
      V.printMonthlyAttendanceSheet(g.id, o.year, o.month, o.paperSize || 'folio', o.officers !== false);
    }).catch(fail);
  },

  // Overtime: schedule by area, timesheet matrix, daily breakdown.
  overtime: function (o) {
    return load(['workers']).then(function () { return records('ot_entries'); }).then(function (entries) {
      var M = S5_ot_calculator.OtCalcModule;
      M._entries = entries || [];
      M._getEntryDateRange = function () { return { start: o.start || '', end: o.end || '' }; };
      if (o.kind === 'matrix') M._printSummary();
      else if (o.kind === 'daily') M._printDailyBreakdown();
      else M._printOvertimeSchedule();
    }).catch(fail);
  },

  // Project groups: one group, all groups, or the group change report.
  projectGroups: function (o) {
    return load(['workers', 'group_changes']).then(function () { return records('project_groups_meta'); }).then(function (meta) {
      var M = S5_project_groups.ProjectGroupsModule;
      M.getCustomAreaMeta = function () {
        var out = {};
        (meta || []).forEach(function (m) { if (m && m.name) out[m.name] = m; else if (m && typeof m === 'object') Object.assign(out, m); });
        return out;
      };
      M._loadGroups();
      if (o.kind === 'changes') { M._changeLogFilter = o.range || 'today'; return M._printChangeLog(); }
      if (o.kind === 'all') return M._printAllGroups();
      return M._printGroup(o.group);
    }).catch(fail);
  },

  // Tonight's overnight duty schedule.
  duty: function (o) {
    return load(['workers']).then(function () { return Promise.all([records('duty_areas'), records('duty_schedules')]); }).then(function (r) {
      var D = S5_duty;
      D.set(o.date ? new Date(o.date + 'T12:00:00') : new Date(), r[0] || [], r[1] || [], workersData);
      var html = D._buildPrintHtml();
      if (html) window.safePrint(html);
    }).catch(fail);
  },

  // Dependability analytics prints.
  analytics: function (o) {
    return load(['workers', 'history', 'attendance_log', 'assignment_log']).then(function () {
      var A = S5_analytics;
      if (o.kind === 'roster') return A.printDependabilityRoster(o.days || 90);
      if (o.kind === 'project') return A.printProjectDossier(o.project, o.days || 90);
      return A.printWorkerDossier(o.worker, o.days || 90);
    }).catch(fail);
  },

  // Dashboard: "Print All Present" (attendance sheet by area).
  presentRoster: function (ids) {
    return loadWorkers().then(function () {
      var list = ids && ids.length ? Array.from(indicesOf(ids)).map(function (i) { return workersData[i]; })
        : workersData.filter(function (w) { return w && !w.isArchived && w.dailyStatus === 'Present'; });
      S5_personnel.printPresentRoster(list);
    }).catch(fail);
  },

  // A person's attendance or assignment history page (new tab, with its own Print button).
  historyPage: function (o) {
    var p = popup();
    return load(['workers', 'history', 'attendance_log', 'assignment_log']).then(function () {
      var w = workersData.find(function (x) { return x.id === o.id; });
      if (!w) return p.fail('No volunteer selected.');
      return api('/workers/' + o.id + '/profile').then(function (full) {
        w.attendanceHistory = full.attendanceHistory || [];
        p.run(function () { if (o.kind === 'assignment') S5_table.showWorkerAssignmentPopup(w.name); else S5_table.showWorkerAttendancePopup(w.name); });
      });
    }, function (e) { p.fail(e.message); });
  },

  // Prints a preview frame (Service 5's "Print Now").
  printFrame: function (frame) {
    if (frame && frame.contentWindow) { frame.contentWindow.focus(); frame.contentWindow.print(); }
  }
};

// Service 5 keeps whole lists under storage keys; pages that edit (Overnight Duty)
// get a storage whose lists are SiteMonitor's records: each changed item is saved
// with PUT, each removed one deleted, so the server keeps its usual checks and log.
var STORE_LISTS = {
  wc_duty_schedules: ['duty_schedules', 'dateStr'], wc_duty_areas: ['duty_areas', 'id'], wc_ot_entries: ['ot_entries', 'id'],
  wc_signatories_data: ['signatories', 'id']
};
var storeSeen = {};
function storeKeyOf(field) { return function (x) { return String((x && (x[field] || x.id || x.name)) || ''); }; }
var siteStorage = {
  get: function (key) {
    if (key === STORAGE_KEY) return Promise.resolve(workersData);
    var c = STORE_LISTS[key];
    if (!c) return Promise.resolve(null);
    return api('/workforce/records/' + c[0]).then(function (l) {
      storeSeen[key] = JSON.parse(JSON.stringify(l));
      return l;
    });
  },
  set: function (key, list) {
    var c = STORE_LISTS[key];
    if (!c || !Array.isArray(list)) return Promise.resolve();
    var keyOf = storeKeyOf(c[1]);
    var before = storeSeen[key] || [];
    var send = function (method, id, body) {
      return fetch('/api/workforce/records/' + c[0] + '/' + encodeURIComponent(id), {
        method: method, credentials: 'same-origin', headers: { 'content-type': 'application/json' }, body: body ? JSON.stringify(body) : undefined
      });
    };
    var jobs = [];
    var kept = {};
    list.forEach(function (x) {
      var id = keyOf(x);
      if (!id) return;
      kept[id] = true;
      var old = before.find(function (b) { return keyOf(b) === id; });
      if (!old || JSON.stringify(old) !== JSON.stringify(x)) jobs.push(send('PUT', id, x));
    });
    before.forEach(function (b) { var id = keyOf(b); if (id && !kept[id]) jobs.push(send('DELETE', id)); });
    return Promise.all(jobs).then(function (rs) {
      var bad = rs.filter(function (r) { return !r.ok; })[0];
      if (bad) { showToast('Not saved: the server said ' + bad.status + '.'); throw new Error('not saved'); }
      storeSeen[key] = JSON.parse(JSON.stringify(list));
    });
  }
};

// Service 5 pages shown inside SiteMonitor (frontend/public/print/s5view-*.html):
// Service 5's markup and render code, fed SiteMonitor's records.
window.S5View = {
  // Service 5's shared helpers, where its page scripts expect them. Its local
  // copies (safeSetItem) are off: no personnel data is kept in the browser.
  globals: function () {
    window._storage = siteStorage;
    window.esc = esc;
    window.showConfirm = showConfirm;
    window.showErrorToast = function (m) { showToast(m); };
    window.trapFocus = S5_modals.trapFocus;
    window.releaseFocusTrap = S5_modals.releaseFocusTrap;
    window.SIG_STORAGE_KEY = SIG_STORAGE_KEY;
    window.defaultSignatories = defaultSignatories;
    window.safeSetItem = function () {};
    window.safeGetJSON = function (key, def) { return def; };
  },
  // Signatories: Service 5's signatories page script, saving to SiteMonitor's signatory records.
  signatories: function () {
    return Promise.all([loadWorkers(), siteStorage.get('wc_signatories_data')]).then(function (r) {
      signatoriesData = Array.isArray(r[1]) ? r[1] : [];
      if (typeof window.normalizeSignatoriesData === 'function') window.normalizeSignatoriesData();
      window.renderSignatories();
      document.body.setAttribute('data-ready', '1');
    }).catch(fail);
  },
  // OT Calculator / OT Entries: Service 5's OT module, saving to SiteMonitor's OT records.
  ot: function (entries) {
    window._storage = siteStorage;
    window.trapFocus = S5_modals.trapFocus;
    window.releaseFocusTrap = S5_modals.releaseFocusTrap;
    var M = S5_ot_calculator.OtCalcModule;
    window.OtCalcModule = M;
    // Service 5's sidebar button tells the module which page it is on.
    var nav = document.createElement('button');
    nav.id = 'nav-btn-ot-entries';
    nav.hidden = true;
    if (entries) nav.className = 'active';
    document.body.appendChild(nav);
    S5_ot_calculator.wireOtEvents();
    return loadWorkers().then(function () { return M.init(); }).then(function () {
      if (entries) M._renderEntriesPage();
      document.body.setAttribute('data-ready', '1');
    }).catch(fail);
  },
  // Overnight Duty: Service 5's duty module, saving to SiteMonitor's duty records.
  duty: function () {
    window._storage = siteStorage;
    window.trapFocus = S5_modals.trapFocus;
    window.releaseFocusTrap = S5_modals.releaseFocusTrap;
    // Service 5's "Schedule Duty" header button (ui.js) generates the week.
    var sched = document.getElementById('btn-duty-schedule');
    if (sched) sched.addEventListener('click', function () { S5_duty_2.DutyModule.requestWeekGeneration(); });
    return loadWorkers().then(function () { return S5_duty_2.DutyModule.init(); }).then(function () {
      document.body.setAttribute('data-ready', '1');
    }).catch(fail);
  },

  departments: function () {
    var D = S5_dashboard;
    // Service 5's buttons call these by name.
    ['renderDeployabilityBanner', 'renderDepartmentMatrix', 'renderDeptAnalytics', 'renderHousingOverview', 'openDepartmentRosterModal', 'openResidenceRosterModal']
      .forEach(function (k) { window[k] = D[k]; });
    window.openAreaRosterModal = S5_distribution_2.openAreaRosterModal;
    window.switchView = function () {};
    var modal = document.getElementById('area-roster-modal');
    var close = function () { if (modal) modal.classList.remove('active'); };
    var x = document.getElementById('btn-close-roster-modal');
    if (x) x.addEventListener('click', close);
    if (modal) modal.addEventListener('click', function (e) { if (e.target === modal) close(); });
    document.addEventListener('keydown', function (e) { if (e.key === 'Escape') close(); });
    var bar = document.getElementById('dist-tab-bar');
    if (bar) bar.addEventListener('click', function (e) {
      var b = e.target.closest('.dist-tab-btn');
      if (b) S5_ui.switchDistTab(b.getAttribute('data-dist-tab'));
    });
    return loadWorkers().then(function () {
      var aw = workersData.filter(function (w) { return w && !w.isArchived; });
      D.renderDeployabilityBanner(aw);
      D.renderDepartmentMatrix(aw);
      D.renderDeptAnalytics(aw);
      S5_ui.switchDistTab('departments');
      document.body.setAttribute('data-ready', '1');
    }).catch(fail);
  }
};
})();
