"""Run: python scripts/s5print/build.py   (SERVICE5_DIR overrides ../01_Rust_WC_DB)

Builds frontend/public/print/s5print.js: Service 5's own print templates,
copied verbatim from ../01_Rust_WC_DB/js, plus a small bridge (bridge.js) that
feeds them SiteMonitor's data (which is stored in Service 5's exact shape).
Service 5 is only read, never changed."""
import os, sys, re
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from jsx import get, skip_block, ROOT, SERVICE5

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = SERVICE5
OUT = os.path.join(HERE, "..", "..", "frontend", "public", "print")

# (section title, source file, names copied verbatim, extra declarations, exports)
SECTIONS = [
    ("distribution.js (job orders)", "distribution.js",
     ["generateSigAuthToken", "buildWorkerFormDiv", "appendSignatoryDivider", "appendProjectGroupDivider", "renderJobOrderBatch", "generateFormsForPrinting"],
     "var printContainer = null, printDateInput = null, printBatchModeSelect = null, printGlobalSigSelect = null;\n"
     "var selectedWorkerIndices = new Set();",
     ["generateFormsForPrinting", "selectedWorkerIndices", "setInputs: function (c, d, b, s) { printContainer = c; printDateInput = d; printBatchModeSelect = b; printGlobalSigSelect = s; }"]),
    ("personnel.js", "personnel.js",
     ["_printPopupThenClose", "generatePrintableReport", "printPersonnelReport", "generateVolunteerFamilyRosterHTML", "printVolunteerFamilyRoster",
      "printSelectedPersonnelCards", "CATEGORIES", "FIELD_LABELS", "FIELD_WEIGHTS", "_fmtDate", "getFieldValue", "getColumnWidths", "_openPrintWin", "_printCSS",
      "generateTablePrintHTML", "generateProfilePrintHTML", "familyRows", "familyPrintCSS", "familyReportHTML", "printWorkerAttendanceDossier"],
     "var selectedPersonnel = new Set();",
     ["printPersonnelReport", "printVolunteerFamilyRoster", "printSelectedPersonnelCards", "generateTablePrintHTML", "generateProfilePrintHTML", "_openPrintWin",
      "familyReportHTML", "printWorkerAttendanceDossier", "printPresentRoster"]),
    ("table.js (history pages)", "table.js", ["showWorkerAttendancePopup", "showWorkerAssignmentPopup"], "",
     ["showWorkerAttendancePopup", "showWorkerAssignmentPopup"]),
    ("cards.js", "cards.js", ["printSelectedCardsList"], "var cardsSelectedIndices = new Set();",
     ["printSelectedCardsList", "cardsSelectedIndices"]),
    ("records_update_form.js", "records_update_form.js",
     ["esc", "getAge", "getMaintMedsSummary", "getChildStatusDisplay", "getChildResidingDisplay", "getChildDutyDisplay", "generateSingleSheetHTML",
      "generateFullPrintableDocumentHTML", "printUpdateForms"], "", ["printUpdateForms"]),
    ("visitations.js (minister summary sheets)", "visitations.js", ["generateMinisterSummaryHtml"], "", ["generateMinisterSummaryHtml"]),
    ("attendance_generator.js", "attendance_generator.js",
     ["_selectedWorkerIds", "_sheetTitle", "_sheetDate", "_sheetFacilitator", "_sheetVenue", "_sheetType", "_includePhotos", "_groupBy", "_sortBy", "_pageBreakPerGroup",
      "esc", "getWorkerKey", "getAge", "getPrintableResidence", "getGroupValue", "getGroupLabel", "sortWorkers", "renderRow", "renderSignOffGrid", "generatePrintSheet"], "",
     ["generatePrintSheet", "getWorkerKey",
      "configure: function (o) { _selectedWorkerIds = new Set(o.keys || []); _sheetTitle = o.title || _sheetTitle; _sheetDate = o.date || _sheetDate; _sheetFacilitator = o.facilitator || ''; _sheetVenue = o.venue || ''; _sheetType = o.type || 'meeting'; _includePhotos = o.photos !== false; _groupBy = o.groupBy || 'none'; _sortBy = o.sortBy || 'name'; _pageBreakPerGroup = o.pageBreak !== false; }"]),
    ("visitation_groups.js", "visitation_groups.js",
     ["_vmaCustomMarksCache", "MAX_MEMBERS_PER_GROUP", "formatPrayerTime", "getGroupsSummary", "getPrintableResidence", "getPrintableResidenceHtml", "printGroupRoster", "printAllGroupsSummary",
      "VMA_MONTH_NAMES", "VMA_DOW_INITIALS", "VMA_MARK_CYCLE", "getVmaStorageKey", "loadVmaMarks", "loadGroupsData", "getGroupRosterMembers", "generateSingleMonthlySheetHtml",
      "printMonthlyAttendanceSheet", "printAllMonthlyAttendanceSheets"], "",
     ["printGroupRoster", "printAllGroupsSummary", "printMonthlyAttendanceSheet", "printAllMonthlyAttendanceSheets"]),
    ("ot_calculator.js", "ot_calculator.js", ["formatSurnameFirst", "formatScheduleDate", "OtCalcModule"], "", ["OtCalcModule"]),
    ("project_groups.js", "project_groups.js", ["roleRank", "ProjectGroupsModule"], "", ["ProjectGroupsModule"]),
    ("duty.js (tonight's duty schedule)", "duty.js", ["getFormattedDateStr", "_buildPrintHtml"],
     "var currentDate = new Date(), _areas = [], _schedules = [], _workers = [];",
     ["_buildPrintHtml", "set: function (d, areas, schedules, workers) { currentDate = d; _areas = areas; _schedules = schedules; _workers = workers; }"]),
    ("analytics.js", "analytics.js",
     ["PERIODS", "STATUS_COLORS", "MONTH_NAMES", "_currentDays", "_currentProject", "_timelineSortCol", "_timelineSortDir", "parseDateComponents", "formatDayMonthYear", "_getSnapTimestamp",
      "_getSortedTimelineSnaps", "_filterSnaps", "_rankFreq", "_cleanProjectName", "computeStats", "computeAllStats", "getAvailableProjects", "computeProjectStats",
      "printWorkerDossier", "printDependabilityRoster", "printProjectDossier"], "",
     ["printWorkerDossier", "printDependabilityRoster", "printProjectDossier"]),
    # Service 5's Departments & Shops page (shown in SiteMonitor's Departments tab).
    ("dashboard.js (departments & shops)", "dashboard.js",
     ["renderDeployabilityBanner", "renderDepartmentMatrix", "buildDonutChart", "buildDonutLegend", "renderDeptAnalytics", "renderHousingOverview",
      "openDepartmentRosterModal", "openResidenceRosterModal"], "",
     ["renderDeployabilityBanner", "renderDepartmentMatrix", "renderDeptAnalytics", "renderHousingOverview", "openDepartmentRosterModal", "openResidenceRosterModal"]),
    ("distribution.js (area roster)", "distribution.js", ["openAreaRosterModal"], "", ["openAreaRosterModal"]),
    ("ui.js (department tabs)", "ui.js", ["switchDistTab"], "", ["switchDistTab"]),
    # Service 5's Overnight Duty page (saves through window._storage, which the bridge maps to SiteMonitor's records).
    ("duty.js (overnight duty)", "duty.js", ["DutyModule"], "", ["DutyModule"]),
    ("modals.js (focus trap)", "modals.js", ["lastFocusedElement", "_focusTrapStack", "restoreFocusAfterClose", "_focusFirstElement", "trapFocus", "releaseFocusTrap"], "", ["trapFocus", "releaseFocusTrap"]),
]

# Service 5 pages shown inside SiteMonitor: (file name, title, database.html line ranges kept verbatim, start call).
VIEWS = [
    ("s5view-departments.html", "Departments & Shops", [(909, 994), (3833, 3844)], "S5View.departments()"),
    ("s5view-duty.html", "Overnight Duty Management", [(1466, 1482), (3848, 3859)], "S5View.duty()"),
    # The OT module reads which page it is on from Service 5's sidebar button (#nav-btn-ot-entries.active).
    ("s5view-ot-calc.html", "Overtime Calculator", [(1674, 1775), (4189, 4227), (3848, 3859)], "S5View.ot(false)"),
    ("s5view-ot-entries.html", "Overtime Entries", [(1776, 1881), (4189, 4227), (3848, 3859)], "S5View.ot(true)"),
    # signatories.js is a page script: it runs after its markup and Service 5's shared helpers (S5View.globals()).
    ("s5view-signatories.html", "Signatories", [(995, 1032), (3030, 3185), (3848, 3859)], "S5View.signatories()"),
]

# Service 5 page scripts copied verbatim and loaded by a page (after S5View.globals()).
PAGE_SCRIPTS = {
    "s5view-signatories.html": ["signatories.js"],
}
CSS_FILES = ["design-tokens.css", "components/navigation.css", "components/empty-state.css", "components/skeleton.css",
             "components/worker-avatar.css", "utilities.css", "database.css"]

# Click handlers copied verbatim as named functions: (file, anchor, name, params, into section).
SNIPPETS = [
    ("dashboard.js", "printBtn.addEventListener('click', function () {", "printPresentRoster", "presentWorkers", "personnel.js"),
]


# File-level code copied verbatim from a marker to the end, wrapped as a function the page calls.
TAILS = [("ot_calculator.js", "// ---- EVENT WIRING ----", "wireOtEvents")]


def tail(file, marker, name):
    t = open(os.path.join(ROOT, file), encoding="utf-8").read()
    return "// verbatim: " + file + " from " + marker + chr(10) + "function " + name + "() {" + chr(10) + t[t.index(marker):] + chr(10) + "}"


def snippet(file, anchor, name, params):
    t = open(os.path.join(ROOT, file), encoding="utf-8").read()
    i = t.index(anchor) + len(anchor) - 1
    j = skip_block(t, i)
    return "// verbatim: the body of " + file + "'s handler" + chr(10) + "function " + name + "(" + params + ") " + t[i:j + 1]


# Fixes applied to the copies, each because Service 5's own code fails there.
PATCHES = [
    # Service 5 never defines `clearance`, so its dossier print throws; show its own clearance label.
    ("personnel.js", "${escapeHtml(clearance)}", "${escapeHtml(getWorkerClearanceState(worker).label)}"),
    # ...nor `restrictions`; show the person's own restrictions list.
    ("personnel.js", "${restrictions.length ? escapeHtml(restrictions.join(', ')) : 'None'}",
     "${(Array.isArray(worker.restrictions) && worker.restrictions.length) ? escapeHtml(worker.restrictions.join(', ')) : 'None'}"),
    # Module objects are kept inside the kit instead of on window.
    ("ot_calculator.js", "window.OtCalcModule = {", "var OtCalcModule = {"),
    ("project_groups.js", "window.ProjectGroupsModule = {", "var ProjectGroupsModule = {"),
]

CORE = ["esc", "parseDateSafe", "formatDateLocal", "todayLocal", "getAge", "getFormattedDate", "displayDate", "WORKER_DEPARTMENTS", "inferWorkerDepartment",
        "getWorkerStatusForDate", "checkJobOrderEligibility", "getWorkerClearanceState", "isEligibleForJobOrder", "tradeThemeColors", "getTradeStyle",
        "getCombinedMedicalCondition", "waitForImagesThenCall", "safePrint", "queryWorkerSnapshots", "todayStr", "areaPriorityComparator",
        "DEPARTMENT_META", "getWorkerDeployability", "areaThemeColors", "getAreaStyle", "getResidenceStyle",
        "STORAGE_KEY", "debounce", "showConfirm", "dbStores", "table.js:ctxEditSvg", "table.js:ctxGroupSvg", "table.js:ctxArchiveSvg", "toDateStr", "guardClick", "showUndoToast", "SIG_STORAGE_KEY", "defaultSignatories"]


CORE_RAW = [("core.js", "// ---- Confirm modal (replaces blocking confirm()) ----", "function showConfirm")]

USED = {}


def section(title, file, names, pre, exports):
    body = "\n\n".join(get(file, n) for n in names)
    for f, a, b in PATCHES:
        if f == file:
            assert a in body, (file, a)
            body = body.replace(a, b)
    for f, a, n, prm, into in SNIPPETS:
        if into == file:
            body += "\n\n" + snippet(f, a, n, prm)
    for f, marker, n in TAILS:
        if f == file:
            body += "\n\n" + tail(f, marker, n)
            exports = exports + [n]
    exp = ",\n    ".join(e if ":" in e else f"{e}: {e}" for e in exports)
    var = "S5_" + re.sub(r"\W", "_", file.split('.')[0])
    # A second section from the same file gets its own name (S5_distribution_2), never overwriting the first.
    USED[var] = USED.get(var, 0) + 1
    if USED[var] > 1:
        var += f"_{USED[var]}"
    return "\n".join([f"// ===== {title} - verbatim from 01_Rust_WC_DB/js/{file} =====",
                      f"var {var} = (function () {{", pre, body, f"return {{\n    {exp}\n}};", "})();", ""])


def build():
    # "file.js:name" takes a shared helper from another Service 5 file.
    core = "\n\n".join(get(*n.split(":")) if ":" in n else get("core.js", n) for n in CORE)
    # Service 5's built-in default signatories are real people: keep their names out of SiteMonitor's code.
    core = re.sub(r"let defaultSignatories = \[.*?\];", "let defaultSignatories = []; // Service 5's defaults are real people; left out", core, count=1, flags=re.S)
    # File-level code kept verbatim between two markers (e.g. what the confirm dialog's buttons do).
    for f, a, b in CORE_RAW:
        t = open(os.path.join(ROOT, f), encoding="utf-8").read()
        i = t.index(a)
        core += "\n\n// verbatim: " + f + "\n" + t[i:t.index(b, i)]
    parts = [open(os.path.join(HERE, "head.js"), encoding="utf-8").read(),
             "// ===== core.js helpers - verbatim from 01_Rust_WC_DB/js/core.js =====", core,
             "function safePrint(html, options) { return window.safePrint(html, options); }",
             "var waitForImagesThenCall = window.waitForImagesThenCall, getCombinedMedicalCondition = window.getCombinedMedicalCondition;", ""]
    parts += [section(*s) for s in SECTIONS]
    parts.append(open(os.path.join(HERE, "bridge.js"), encoding="utf-8").read())
    os.makedirs(OUT, exist_ok=True)
    js = "\n".join(parts)
    open(os.path.join(OUT, "s5print.js"), "w", encoding="utf-8", newline="\n").write(js)
    # Service 5's print stylesheet and its job-order rules, verbatim.
    css = open(os.path.join(SRC, "css", "print.css"), encoding="utf-8").read()
    db = open(os.path.join(SRC, "css", "database.css"), encoding="utf-8").read().split("\n")
    jo = "\n".join(db[3564:3653]) + "\n\n" + "\n".join(db[4294:4656])
    open(os.path.join(OUT, "s5print.css"), "w", encoding="utf-8", newline="\n").write(
        "/* Service 5 print styles, verbatim: css/print.css, then css/database.css lines 3565-3653 and 4295-4656. */\n" + css + "\n\n" + jo + "\n")
    # Service 5's stylesheets, verbatim, for the pages shown inside SiteMonitor.
    for f in CSS_FILES:
        dst = os.path.join(OUT, "s5css", f)
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        open(dst, "w", encoding="utf-8", newline="\n").write(open(os.path.join(SRC, "css", f), encoding="utf-8").read())
    page = open(os.path.join(SRC, "database.html"), encoding="utf-8").read().split("\n")
    for name, title, ranges, start in VIEWS:
        body = "\n".join("\n".join(page[a - 1:b]) for a, b in ranges)
        body = body.replace('class="view"', 'class="view active"', 1)  # Service 5 shows only the active view
        links = "\n".join(f'<link rel="stylesheet" href="/print/s5css/{f}">' for f in CSS_FILES)
        scripts = ""
        for s in PAGE_SCRIPTS.get(name, []):
            dst = os.path.join(OUT, "s5src", s)
            os.makedirs(os.path.dirname(dst), exist_ok=True)
            open(dst, "w", encoding="utf-8", newline="\n").write(open(os.path.join(ROOT, s), encoding="utf-8").read())
            scripts += f'<script src="/print/s5src/{s}"></script>\n'
        if scripts:
            scripts = "<script>S5View.globals();</script>\n" + scripts
        open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n").write(
            "<!DOCTYPE html>\n<!-- GENERATED by scripts/s5print/build.py: Service 5's own page markup (database.html), styles and code. -->\n"
            f'<html lang="en"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>{title}</title>\n'
            '<style>@font-face{font-family:"Inter";src:url("/fonts/inter-variable.woff2") format("woff2");font-weight:100 900}</style>\n'
            f"{links}\n<style>body{{margin:0}} .global-clock{{display:none}}</style>\n</head><body>\n"
            f"{body}\n<script src=\"/print/s5print.js\"></script>\n{scripts}<script>{start};</script>\n</body></html>\n")
    print("wrote", len(js.splitlines()), "lines")


if __name__ == "__main__":
    build()
