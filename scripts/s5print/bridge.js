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
})();
