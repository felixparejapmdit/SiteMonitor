// SIGNATORY MANAGEMENT

const sigContainer = document.getElementById('signatories-grid');
const btnResetSigs = document.getElementById('btn-reset-sigs');
// Add Signatory Modal Elements
const addSigModal = document.getElementById('add-signatory-modal');
const btnCloseAddSig = document.getElementById('btn-close-add-sig');
const btnCancelAddSig = document.getElementById('btn-cancel-add-sig');
const addSigForm = document.getElementById('add-sig-form');
const newSigName = document.getElementById('new-sig-name');

// Bulk Assign Signatories to Project Groups Modal Elements
const bulkSigModal        = document.getElementById('bulk-assign-sig-groups-modal');
const bulkSigTbody        = document.getElementById('bulk-sig-groups-tbody');
const bulkQuickSigSelect  = document.getElementById('bulk-quick-sig-select');
const btnBulkAssignGroups = document.getElementById('btn-bulk-assign-groups');
const btnPgBulkSignatories= document.getElementById('btn-pg-bulk-signatories');
const btnCloseBulkSig     = document.getElementById('btn-close-bulk-sig-groups');
const btnCancelBulkSig    = document.getElementById('btn-cancel-bulk-sig-groups');
const btnBulkApplyAll     = document.getElementById('btn-bulk-apply-all-groups');
const btnBulkClearAll     = document.getElementById('btn-bulk-clear-all-groups');
const btnSaveBulkSig      = document.getElementById('btn-save-bulk-sig-groups');

function compressPngSignature(file, maxWidth = 480, maxHeight = 160) {
    return new Promise((resolve, reject) => {
        if (!file) return reject(new Error('No file provided'));
        if (file.type && file.type !== 'image/png') {
            return reject(new Error('Signature must be a transparent PNG file.'));
        }
        const reader = new FileReader();
        reader.onload = (e) => {
            const img = new Image();
            img.onload = () => {
                let w = img.width;
                let h = img.height;
                const ratio = Math.min(maxWidth / w, maxHeight / h, 1);
                w = Math.round(w * ratio);
                h = Math.round(h * ratio);

                const canvas = document.createElement('canvas');
                canvas.width = w;
                canvas.height = h;
                const ctx = canvas.getContext('2d');
                ctx.clearRect(0, 0, w, h);
                ctx.drawImage(img, 0, 0, w, h);
                resolve(canvas.toDataURL('image/png'));
            };
            img.onerror = () => reject(new Error('Failed to load image file.'));
            img.src = e.target.result;
        };
        reader.onerror = () => reject(new Error('Failed to read file.'));
        reader.readAsDataURL(file);
    });
}

function normalizeSignatoriesData() {
    if (!Array.isArray(signatoriesData)) return;
    const seenGroups = new Set();
    signatoriesData.forEach(sig => {
        if (!sig.projectGroups) {
            sig.projectGroups = sig.trades || [];
        }
        if (sig.photoUrl && !sig.signatureUrl) {
            sig.signatureUrl = sig.photoUrl;
        }
        delete sig.photoUrl;
        const exclusiveGroups = [];
        sig.projectGroups.forEach(g => {
            if (g && typeof g === 'string' && !seenGroups.has(g.trim())) {
                seenGroups.add(g.trim());
                exclusiveGroups.push(g.trim());
            }
        });
        sig.projectGroups = exclusiveGroups;
        sig.trades = sig.projectGroups;
    });
}

function getAllProjectGroups() {
    const set = new Set();
    if (typeof workersData !== 'undefined' && Array.isArray(workersData)) {
        workersData.forEach(w => {
            if (!w || w.isArchived) return;
            if (w.projectGroup && w.projectGroup.trim()) {
                set.add(w.projectGroup.trim());
            }
        });
    }
    return Array.from(set).sort();
}

function openAddSignatoryModal() {
    if (!addSigModal) return;
    if (newSigName) newSigName.value = '';
    addSigModal.classList.add('active');
    if (typeof trapFocus === 'function') trapFocus(addSigModal);
    setTimeout(() => newSigName?.focus(), 50);
}

function closeAddSignatoryModal() {
    if (!addSigModal) return;
    addSigModal.classList.remove('active');
    if (typeof releaseFocusTrap === 'function') releaseFocusTrap();
    if (newSigName) newSigName.value = '';
    const fileInput = document.getElementById('new-sig-file');
    if (fileInput) fileInput.value = '';
}

window.openAddSignatoryModal = openAddSignatoryModal;
window.closeAddSignatoryModal = closeAddSignatoryModal;

document.getElementById('btn-add-signatory')?.addEventListener('click', () => openAddSignatoryModal());
btnCloseAddSig?.addEventListener('click', () => closeAddSignatoryModal());
btnCancelAddSig?.addEventListener('click', () => closeAddSignatoryModal());

if (addSigForm) {
    addSigForm.addEventListener('submit', async (e) => {
        e.preventDefault();
        const nameVal = (newSigName?.value || '').trim();
        if (!nameVal) {
            if (typeof showErrorToast === 'function') showErrorToast('Signatory name is required.');
            return;
        }

        const fileInput = document.getElementById('new-sig-file');
        let signatureUrl = null;
        let signatureDate = null;
        if (fileInput && fileInput.files && fileInput.files[0]) {
            try {
                signatureUrl = await compressPngSignature(fileInput.files[0]);
                signatureDate = new Date().toISOString();
            } catch (err) {
                if (typeof showErrorToast === 'function') showErrorToast(err.message || 'Failed to process signature PNG.');
                return;
            }
        }

        signatoriesData.push({
            name: nameVal,
            projectGroups: [],
            signatureUrl: signatureUrl,
            signatureDate: signatureDate
        });

        saveSignatoriesToStorage();
        renderSignatories();
        closeAddSignatoryModal();
        if (typeof showToast === 'function') showToast(`Signatory "${nameVal}" added successfully.`);
    });
}

// BULK ASSIGN SIGNATORIES TO GROUPS

function getGroupAreaName(groupName) {
    if (window.ProjectGroupsModule && typeof window.ProjectGroupsModule.extractGroupArea === 'function') {
        return window.ProjectGroupsModule.extractGroupArea(groupName);
    }
    return 'General';
}

function openBulkAssignGroupsModal() {
    if (!bulkSigModal) return;
    const allGroups = getAllProjectGroups();

    // Populate quick apply select
    if (bulkQuickSigSelect) {
        bulkQuickSigSelect.innerHTML = `<option value="">-- Choose Signatory --</option>` +
            signatoriesData.map(s => `<option value="${esc(s.name)}">${esc(s.name)}</option>`).join('');
    }

    // Compute member counts per project group
    const groupCounts = {};
    if (typeof workersData !== 'undefined' && Array.isArray(workersData)) {
        workersData.forEach(w => {
            if (!w || w.isArchived) return;
            const g = (w.projectGroup || '').trim();
            if (g) groupCounts[g] = (groupCounts[g] || 0) + 1;
        });
    }

    if (bulkSigTbody) {
        if (allGroups.length === 0) {
            bulkSigTbody.innerHTML = `
                <tr>
                    <td colspan="3" style="text-align: center; padding: 32px 16px; color: var(--text-muted); font-style: italic;">
                        No project groups found in the database.<br>
                        <span style="font-size: 0.75rem;">Assign volunteers to project groups first via Daily Assignments or Project Groups view.</span>
                    </td>
                </tr>`;
        } else {
            bulkSigTbody.innerHTML = allGroups.map(group => {
                const eGroup = esc(group);
                const count = groupCounts[group] || 0;
                const area = getGroupAreaName(group);
                // Find currently assigned signatory
                const currentSig = signatoriesData.find(s => (s.projectGroups || []).includes(group));
                const currentSigName = currentSig ? currentSig.name : '';

                const optionsHtml = signatoriesData.map(s => {
                    const isSelected = s.name === currentSigName;
                    return `<option value="${esc(s.name)}" ${isSelected ? 'selected' : ''}>${esc(s.name)}</option>`;
                }).join('');

                return `
                <tr style="border-bottom: 1px solid var(--border-color);">
                    <td style="padding: 10px 12px; font-weight: 700; color: var(--text-main);">
                        <div style="display: flex; align-items: center; gap: 8px;">
                            <span style="font-size: 0.68rem; font-weight: 700; background: rgba(30, 64, 175, 0.08); color: var(--primary-color); border: 1px solid rgba(30, 64, 175, 0.18); padding: 2px 6px; border-radius: 4px; white-space: nowrap;">
                                📍 ${esc(area)}
                            </span>
                            <span>${eGroup}</span>
                        </div>
                    </td>
                    <td style="padding: 10px 12px; text-align: center;">
                        <span style="font-size: 0.75rem; background: var(--bg-color); border: 1px solid var(--border-color); padding: 2px 8px; border-radius: 12px; font-weight: 600; color: var(--text-muted);">
                            ${count} volunteer${count !== 1 ? 's' : ''}
                        </span>
                    </td>
                    <td style="padding: 10px 12px;">
                        <select class="bulk-group-sig-select" data-group="${eGroup}" style="width: 100%; padding: 6px 8px; font-size: 0.8rem; border-radius: 4px; border: 1px solid var(--border-color); background: var(--card-bg);">
                            <option value="">-- Unassigned (Use Default) --</option>
                            ${optionsHtml}
                        </select>
                    </td>
                </tr>`;
            }).join('');
        }
    }

    bulkSigModal.classList.add('active');
    if (typeof trapFocus === 'function') trapFocus(bulkSigModal);
}

function closeBulkAssignGroupsModal() {
    if (!bulkSigModal) return;
    bulkSigModal.classList.remove('active');
    if (typeof releaseFocusTrap === 'function') releaseFocusTrap();
}

btnBulkAssignGroups?.addEventListener('click', () => openBulkAssignGroupsModal());
btnPgBulkSignatories?.addEventListener('click', () => openBulkAssignGroupsModal());
btnCloseBulkSig?.addEventListener('click', () => closeBulkAssignGroupsModal());
btnCancelBulkSig?.addEventListener('click', () => closeBulkAssignGroupsModal());

btnBulkApplyAll?.addEventListener('click', () => {
    const val = bulkQuickSigSelect?.value;
    if (!val) {
        if (typeof showToast === 'function') showToast('Please select a signatory first to apply to all groups.', 'warning');
        return;
    }
    const selects = bulkSigModal?.querySelectorAll('.bulk-group-sig-select');
    if (selects) {
        selects.forEach(s => { s.value = val; });
        if (typeof showToast === 'function') showToast(`Set all groups to "${val}". Click "Save Assignments" to commit.`);
    }
});

btnBulkClearAll?.addEventListener('click', () => {
    const selects = bulkSigModal?.querySelectorAll('.bulk-group-sig-select');
    if (selects) {
        selects.forEach(s => { s.value = ''; });
        if (typeof showToast === 'function') showToast('Cleared all group assignments. Click "Save Assignments" to commit.');
    }
});

btnSaveBulkSig?.addEventListener('click', () => {
    const selects = bulkSigModal?.querySelectorAll('.bulk-group-sig-select');
    if (!selects || selects.length === 0) {
        closeBulkAssignGroupsModal();
        return;
    }

    // Reset all signatories projectGroups
    signatoriesData.forEach(s => { s.projectGroups = []; });

    // Map according to selects
    let mappedCount = 0;
    selects.forEach(sel => {
        const group = sel.dataset.group;
        const sigName = sel.value;
        if (group && sigName) {
            const sig = signatoriesData.find(s => s.name === sigName);
            if (sig) {
                if (!sig.projectGroups) sig.projectGroups = [];
                sig.projectGroups.push(group);
                mappedCount++;
            }
        }
    });

    saveSignatoriesToStorage();
    renderSignatories();
    if (window.ProjectGroupsModule && typeof window.ProjectGroupsModule.init === 'function') {
        window.ProjectGroupsModule.init();
    }
    closeBulkAssignGroupsModal();
    if (typeof showToast === 'function') showToast(`Successfully updated signatory mappings for ${mappedCount} project group(s).`);
});

// CORE SIGNATORY DATA & RENDERING

async function loadSignatoryData() {
    try {
        const val = await window._storage.get(SIG_STORAGE_KEY);
        if (val) {
            signatoriesData = val;
            normalizeSignatoriesData();
            window.signatoriesData = signatoriesData;
            return;
        }
    } catch (e) { /* fallback to localStorage */ }
    signatoriesData = safeGetJSON(SIG_STORAGE_KEY, [...defaultSignatories]);
    normalizeSignatoriesData();
    window.signatoriesData = signatoriesData;
}

function renderSignatoriesStats(allGroups, unassignedGroups) {
    const statsBar = document.getElementById('signatories-stats-bar');
    if (!statsBar) return;

    const totalSignatories = signatoriesData.length;
    const totalGroups = allGroups.length;
    const mappedGroupsCount = totalGroups - unassignedGroups.length;
    const coveragePct = totalGroups > 0 ? Math.round((mappedGroupsCount / totalGroups) * 100) : 100;

    statsBar.innerHTML = `
    <div class="sig-kpi-bar">
        <div class="sig-kpi-card">
            <div class="sig-kpi-icon" style="background: rgba(59, 130, 246, 0.1); color: #2563eb;">
                <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"></path><circle cx="9" cy="7" r="4"></circle><path d="M23 21v-2a4 4 0 0 0-3-3.87"></path><path d="M16 3.13a4 4 0 0 1 0 7.75"></path></svg>
            </div>
            <div>
                <div style="font-size: 0.72rem; font-weight: 700; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.5px;">Active Signatories</div>
                <div style="font-size: 1.35rem; font-weight: 800; color: var(--text-main); line-height: 1.2;">${totalSignatories}</div>
                <div style="font-size: 0.72rem; color: var(--text-muted);">Job Order Releasing Officers</div>
            </div>
        </div>

        <div class="sig-kpi-card">
            <div class="sig-kpi-icon" style="background: rgba(16, 185, 129, 0.1); color: #059669;">
                <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"></circle><polyline points="12 6 12 12 16 14"></polyline></svg>
            </div>
            <div style="flex: 1;">
                <div style="display: flex; justify-content: space-between; align-items: baseline;">
                    <span style="font-size: 0.72rem; font-weight: 700; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.5px;">Group Coverage</span>
                    <span style="font-size: 0.75rem; font-weight: 700; color: #059669;">${coveragePct}%</span>
                </div>
                <div style="font-size: 1.35rem; font-weight: 800; color: var(--text-main); line-height: 1.2;">${mappedGroupsCount} <span style="font-size: 0.82rem; font-weight: 600; color: var(--text-muted);">/ ${totalGroups} mapped</span></div>
                <div style="width: 100%; height: 5px; background: rgba(0,0,0,0.06); border-radius: 9999px; margin-top: 4px; overflow: hidden;">
                    <div style="width: ${coveragePct}%; height: 100%; background: #10b981; border-radius: 9999px; transition: width 0.3s ease;"></div>
                </div>
            </div>
        </div>

        <div class="sig-kpi-card">
            <div class="sig-kpi-icon" style="background: ${unassignedGroups.length === 0 ? 'rgba(16, 185, 129, 0.1)' : 'rgba(245, 158, 11, 0.1)'}; color: ${unassignedGroups.length === 0 ? '#059669' : '#d97706'};">
                <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path><line x1="12" y1="9" x2="12" y2="13"></line><line x1="12" y1="17" x2="12.01" y2="17"></line></svg>
            </div>
            <div>
                <div style="font-size: 0.72rem; font-weight: 700; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.5px;">Unassigned Groups</div>
                <div style="font-size: 1.35rem; font-weight: 800; color: ${unassignedGroups.length === 0 ? '#059669' : '#d97706'}; line-height: 1.2;">${unassignedGroups.length}</div>
                <div style="font-size: 0.72rem; color: var(--text-muted);">${unassignedGroups.length === 0 ? 'All groups mapped to signatories' : 'Available for exclusive assignment'}</div>
            </div>
        </div>
    </div>
    `;
}

function renderSignatories() {
    if (!sigContainer) return;
    normalizeSignatoriesData();
    const allGroups = getAllProjectGroups();

    // Compute volunteer counts per project group
    const groupVolunteerCounts = {};
    if (typeof workersData !== 'undefined' && Array.isArray(workersData)) {
        workersData.forEach(w => {
            if (!w || w.isArchived) return;
            const g = (w.projectGroup || '').trim();
            if (g) groupVolunteerCounts[g] = (groupVolunteerCounts[g] || 0) + 1;
        });
    }

    // Determine assigned owners (mutual exclusivity)
    const assignedGroupOwners = {};
    signatoriesData.forEach((sig, idx) => {
        (sig.projectGroups || []).forEach(g => {
            assignedGroupOwners[g] = idx;
        });
    });

    const unassignedGroups = allGroups.filter(g => assignedGroupOwners[g] === undefined);

    // Render KPI stats bar
    renderSignatoriesStats(allGroups, unassignedGroups);

    sigContainer.innerHTML = signatoriesData.map((sig, idx) => {
        if (!sig.projectGroups) sig.projectGroups = [];
        const totalVolsCovered = sig.projectGroups.reduce((sum, g) => sum + (groupVolunteerCounts[g] || 0), 0);

        // Assigned chips - ONLY groups assigned to this signatory, with one-click deselect
        const assignedChipsHtml = sig.projectGroups.length === 0
            ? `<div class="sig-empty-groups-box">No project groups assigned to this signatory yet. Select an unassigned group below to assign.</div>`
            : sig.projectGroups.map(g => {
                const eg = esc(g);
                const vols = groupVolunteerCounts[g] || 0;
                return `
                <span class="sig-assigned-chip sig-group-tag" data-sig-idx="${idx}" data-sig-group="${eg}" title="Assigned exclusively to ${esc(sig.name)}. Click &times; to deselect.">
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" style="color: #2563eb; flex-shrink: 0;"><polyline points="20 6 9 17 4 12"></polyline></svg>
                    <span>${eg}</span>
                    <span style="font-size: 0.68rem; opacity: 0.85; background: rgba(37, 99, 235, 0.12); padding: 1px 6px; border-radius: 9999px; font-weight: 700;">${vols} vol${vols !== 1 ? 's' : ''}</span>
                    <button type="button" class="sig-chip-remove" onclick="event.stopPropagation(); window.deselectSignatoryGroup(${idx}, '${eg}')" title="Deselect and unbind ${eg}" aria-label="Deselect ${eg}">
                        &times;
                    </button>
                </span>`;
            }).join('');

        // Assign / Manage Groups Action Button (Dual Listbox Trigger)
        const assignControlHtml = `
        <div style="margin-top: 14px; padding-top: 12px; border-top: 1px dashed var(--border-color); display: flex; justify-content: space-between; align-items: center; gap: 8px;">
            <button type="button" class="btn btn-secondary btn-sm" onclick="window.openManageGroupsModal(${idx})" style="font-size: 0.76rem; padding: 5px 12px; font-weight: 700; display: inline-flex; align-items: center; gap: 6px;">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path></svg>
                + Assign / Manage Groups
            </button>
            <span style="font-size: 0.72rem; color: var(--text-muted); font-weight: 600;">
                ${unassignedGroups.length > 0 ? `${unassignedGroups.length} available` : 'All assigned'}
            </span>
        </div>`;

        return `
        <div class="sig-card-container">
            <!-- Header Row: Initials Avatar + Signatory Name Input + Delete Action -->
            <div style="display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-bottom: 12px;">
                <div style="display: flex; align-items: center; gap: 10px; flex: 1; min-width: 0;">
                    <div style="width: 38px; height: 38px; border-radius: 8px; background: linear-gradient(135deg, #1e40af 0%, #3b82f6 100%); color: #ffffff; display: flex; align-items: center; justify-content: center; font-weight: 800; font-size: 0.95rem; flex-shrink: 0; box-shadow: 0 2px 6px rgba(37,99,235,0.25);">
                        ${esc((sig.name || 'S').charAt(0).toUpperCase())}
                    </div>
                    <div style="flex: 1; min-width: 0;">
                        <input type="text" class="inline-input" placeholder="Signatory Name" value="${esc(sig.name)}" onchange="window.updateSignatoryName(${idx}, this.value)" style="font-weight: 800; font-size: 1.05rem; color: var(--text-main); width: 100%; padding: 4px 8px; border-radius: 6px;">
                        <div style="font-size: 0.72rem; color: var(--text-muted); padding-left: 8px; margin-top: 1px;">Authorized Job Order Signatory</div>
                    </div>
                </div>
                <button class="action-btn" title="Delete Signatory" style="border: none; background: transparent; color: #ef4444; padding: 6px; border-radius: 6px; cursor: pointer; flex-shrink: 0;" onclick="window.deleteSignatory(${idx})">
                    <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg>
                </button>
            </div>

            <!-- Stats Bar -->
            <div style="display: flex; justify-content: space-between; align-items: center; background: var(--bg-color); border: 1px solid var(--border-color); border-radius: 6px; padding: 6px 12px; margin-bottom: 12px; font-size: 0.72rem; color: var(--text-muted);">
                <span>🎯 <strong style="color: var(--text-main);">${sig.projectGroups.length}</strong> Groups Assigned</span>
                <span>👥 <strong style="color: var(--text-main);">${totalVolsCovered}</strong> Total Volunteers</span>
            </div>

            <!-- Digital Signature Section -->
            <div class="sig-signature-box" style="background: var(--bg-color); border: 1px solid var(--border-color); border-radius: 8px; padding: 12px; margin-bottom: 12px;">
                <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;">
                    <div style="display: flex; align-items: center; gap: 6px;">
                        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="color: var(--primary-color);"><path d="M12 20h9"></path><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"></path></svg>
                        <span style="font-size: 0.72rem; font-weight: 800; color: var(--text-main); text-transform: uppercase; letter-spacing: 0.3px;">Digital PNG Signature</span>
                    </div>
                    ${sig.signatureUrl ? `
                        <span style="font-size: 0.68rem; font-weight: 700; color: #059669; background: rgba(16, 185, 129, 0.1); border: 1px solid rgba(16, 185, 129, 0.2); padding: 2px 8px; border-radius: 9999px;">
                            🟢 Active Digital Seal
                        </span>
                    ` : `
                        <span style="font-size: 0.68rem; font-weight: 600; color: var(--text-muted); background: rgba(0, 0, 0, 0.04); padding: 2px 8px; border-radius: 9999px;">
                            Physical Ink Required
                        </span>
                    `}
                </div>

                ${sig.signatureUrl ? `
                    <div style="background: #ffffff; border: 1px dashed #cbd5e1; border-radius: 6px; padding: 8px 12px; display: flex; align-items: center; justify-content: center; min-height: 52px; position: relative; margin-bottom: 8px; overflow: hidden;">
                        <img src="${sig.signatureUrl}" alt="Signature of ${esc(sig.name)}" style="max-height: 48px; max-width: 100%; object-fit: contain; filter: contrast(1.15);">
                    </div>
                    <div style="display: flex; justify-content: space-between; align-items: center; font-size: 0.70rem; color: var(--text-muted); flex-wrap: wrap; gap: 6px;">
                        <span style="display: flex; align-items: center; gap: 4px;">
                            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"></circle><polyline points="12 6 12 12 16 14"></polyline></svg>
                            ${sig.signatureDate ? `Configured: ${new Date(sig.signatureDate).toLocaleDateString()}` : 'Date-locked with Job Orders'}
                        </span>
                        <div style="display: flex; gap: 6px;">
                            <button type="button" class="btn btn-secondary btn-sm" onclick="window.triggerUploadSignature(${idx})" style="font-size: 0.70rem; padding: 3px 8px;">
                                Change PNG
                            </button>
                            <button type="button" class="btn btn-danger btn-sm" onclick="window.removeSignatorySignature(${idx})" style="font-size: 0.70rem; padding: 3px 8px;">
                                Remove
                            </button>
                        </div>
                    </div>
                ` : `
                    <div style="display: flex; flex-direction: column; gap: 8px;">
                        <div style="display: flex; align-items: center; justify-content: space-between; gap: 8px; flex-wrap: wrap;">
                            <button type="button" class="btn btn-secondary btn-sm" onclick="window.triggerUploadSignature(${idx})" style="font-size: 0.74rem; font-weight: 700; padding: 6px 12px; display: inline-flex; align-items: center; gap: 6px; border: 1px dashed var(--color-primary, #2563eb);">
                                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path><polyline points="17 8 12 3 7 8"></polyline><line x1="12" y1="3" x2="12" y2="15"></line></svg>
                                Upload PNG Signature
                            </button>
                            ${((sig.name || '').toLowerCase().includes('philippe')) ? `
                                <button type="button" class="btn btn-sm" onclick="window.usePhilippeDefaultSignature(${idx})" style="font-size: 0.72rem; font-weight: 600; padding: 5px 10px; background: rgba(37, 99, 235, 0.08); color: #2563eb; border: 1px solid rgba(37, 99, 235, 0.2);">
                                    Load Philippe's Official PNG
                                </button>
                            ` : ''}
                        </div>
                        <div style="font-size: 0.68rem; color: var(--text-muted); line-height: 1.3;">
                            Upload transparent PNG. When printing Job Orders, the signature is sealed with <strong>tomorrow's Job Order date</strong> to prevent tampering.
                        </div>
                    </div>
                `}
                <input type="file" id="sig-file-input-${idx}" accept="image/png" style="display: none;" onchange="window.handleSignatoryFileUpload(${idx}, this)">
            </div>

            <!-- Mapped Groups Container -->
            <div style="background: var(--bg-color); padding: 14px; border-radius: 8px; border: 1px solid var(--border-color); flex: 1;">
                <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px;">
                    <span style="font-size: 0.72rem; font-weight: 800; color: var(--text-main); text-transform: uppercase; letter-spacing: 0.3px;">Mapped Project Groups</span>
                    <div style="display: flex; align-items: center; gap: 8px;">
                        <span style="font-size: 0.68rem; color: var(--text-muted); font-weight: 600;">${sig.projectGroups.length} bound</span>
                        ${sig.projectGroups.length > 0 ? `<button type="button" onclick="window.clearAllGroupsForSignatory(${idx})" style="font-size: 0.68rem; color: #ef4444; background: none; border: none; cursor: pointer; text-decoration: underline; padding: 0;">Unbind All</button>` : ''}
                    </div>
                </div>

                <div style="display: flex; flex-wrap: wrap; gap: 8px;">
                    ${assignedChipsHtml}
                </div>

                ${assignControlHtml}
            </div>
        </div>
        `;
    }).join('');
}

if (btnResetSigs) {
    btnResetSigs.addEventListener('click', async () => {
        if (await showConfirm("Reset all signatories to system defaults?")) {
            signatoriesData = [...defaultSignatories];
            normalizeSignatoriesData();
            window.signatoriesData = signatoriesData;
            saveSignatoriesToStorage();
            renderSignatories();
            if (window.ProjectGroupsModule && typeof window.ProjectGroupsModule.init === 'function') {
                window.ProjectGroupsModule.init();
            }
        }
    });
}

window.deleteSignatory = async (idx) => {
    if (await showConfirm("Are you sure you want to delete this specific signatory?")) {
        signatoriesData.splice(idx, 1);
        saveSignatoriesToStorage();
        renderSignatories();
        if (window.ProjectGroupsModule && typeof window.ProjectGroupsModule.init === 'function') {
            window.ProjectGroupsModule.init();
        }
    }
};

window.triggerUploadSignature = (idx) => {
    const input = document.getElementById(`sig-file-input-${idx}`);
    if (input) {
        input.value = '';
        input.click();
    }
};

window.handleSignatoryFileUpload = async (idx, input) => {
    const sig = signatoriesData[idx];
    if (!sig || !input.files || !input.files[0]) return;
    const file = input.files[0];
    try {
        const compressedDataUrl = await compressPngSignature(file);
        sig.signatureUrl = compressedDataUrl;
        sig.signatureDate = new Date().toISOString();
        saveSignatoriesToStorage();
        renderSignatories();
        if (typeof showToast === 'function') {
            showToast(`Digital signature attached to ${sig.name}.`);
        }
    } catch (err) {
        if (typeof showErrorToast === 'function') {
            showErrorToast(err.message || 'Failed to upload signature.');
        } else {
            alert(err.message);
        }
    }
};

window.removeSignatorySignature = async (idx) => {
    const sig = signatoriesData[idx];
    if (!sig) return;
    if (await showConfirm(`Remove digital signature for "${sig.name}"?\nForms will require a physical handwritten signature.`)) {
        sig.signatureUrl = null;
        sig.signatureDate = null;
        saveSignatoriesToStorage();
        renderSignatories();
        if (typeof showToast === 'function') {
            showToast(`Digital signature removed from ${sig.name}.`);
        }
    }
};

window.usePhilippeDefaultSignature = async (idx) => {
    const sig = signatoriesData[idx];
    if (!sig) return;
    sig.signatureUrl = 'assets/images/Philippe Signature (1).png';
    sig.signatureDate = new Date().toISOString();
    saveSignatoriesToStorage();
    renderSignatories();
    if (typeof showToast === 'function') {
        showToast(`Philippe's official PNG signature attached.`);
    }
};

window.deselectSignatoryGroup = (idx, groupName) => {
    const sig = signatoriesData[idx];
    if (!sig) return;
    if (!sig.projectGroups) sig.projectGroups = [];
    sig.projectGroups = sig.projectGroups.filter(g => g !== groupName);
    sig.trades = sig.projectGroups;
    saveSignatoriesToStorage();
    renderSignatories();
    if (window.ProjectGroupsModule && typeof window.ProjectGroupsModule.init === 'function') {
        window.ProjectGroupsModule.init();
    }
    if (typeof showToast === 'function') {
        showToast(`Deselected "${groupName}". It is now available for other signatories.`);
    }
};

window.assignGroupToSignatory = (idx, groupName) => {
    const sig = signatoriesData[idx];
    if (!sig || !groupName) return;
    if (!sig.projectGroups) sig.projectGroups = [];

    // Enforce mutual exclusivity across all signatories
    signatoriesData.forEach(s => {
        if (s.projectGroups) {
            s.projectGroups = s.projectGroups.filter(g => g !== groupName);
            s.trades = s.projectGroups;
        }
    });

    sig.projectGroups.push(groupName);
    sig.trades = sig.projectGroups;
    saveSignatoriesToStorage();
    renderSignatories();
    if (window.ProjectGroupsModule && typeof window.ProjectGroupsModule.init === 'function') {
        window.ProjectGroupsModule.init();
    }
    if (typeof showToast === 'function') {
        showToast(`Assigned "${groupName}" exclusively to ${sig.name}.`);
    }
};

window.assignGroupFromSelect = (idx, selectEl) => {
    const groupName = selectEl.value;
    if (!groupName) return;
    window.assignGroupToSignatory(idx, groupName);
};

window.clearAllGroupsForSignatory = async (idx) => {
    const sig = signatoriesData[idx];
    if (!sig) return;
    const count = (sig.projectGroups || []).length;
    if (count === 0) return;
    if (await showConfirm(`Unbind all ${count} project group(s) from "${sig.name}"?\nThey will become available for assignment to other signatories.`)) {
        sig.projectGroups = [];
        sig.trades = [];
        saveSignatoriesToStorage();
        renderSignatories();
        if (window.ProjectGroupsModule && typeof window.ProjectGroupsModule.init === 'function') {
            window.ProjectGroupsModule.init();
        }
        if (typeof showToast === 'function') {
            showToast(`Unbound all project groups from "${sig.name}".`);
        }
    }
};

window.toggleSignatoryProjectGroup = (idx, groupName) => {
    const sig = signatoriesData[idx];
    if (!sig) return;
    if (!sig.projectGroups) sig.projectGroups = [];

    if (sig.projectGroups.includes(groupName)) {
        window.deselectSignatoryGroup(idx, groupName);
    } else {
        window.assignGroupToSignatory(idx, groupName);
    }
};

window.toggleSignatoryTrade = window.toggleSignatoryProjectGroup;

window.updateSignatoryName = (idx, newName) => {
    signatoriesData[idx].name = newName;
    saveSignatoriesToStorage();
    if (window.ProjectGroupsModule && typeof window.ProjectGroupsModule.init === 'function') {
        window.ProjectGroupsModule.init();
    }
};

function saveSignatoriesToStorage() {
    window.signatoriesData = signatoriesData;
    safeSetItem(SIG_STORAGE_KEY, JSON.stringify(signatoriesData));
    window._storage.set(SIG_STORAGE_KEY, signatoriesData).catch(e => {
        console.error('Failed to save signatories to storage:', e);
    });
}

// DUAL-LISTBOX BATCH TRANSFER MODAL (OPTION 2)

const manageSigModal = document.getElementById('sig-manage-groups-modal');
const btnCloseManageSigModal = document.getElementById('btn-close-sig-manage-modal');
const btnDoneManageSigModal = document.getElementById('btn-done-sig-manage');
const btnSigTransferRight = document.getElementById('btn-sig-transfer-right');
const btnSigSelectAllAvail = document.getElementById('btn-sig-select-all-avail');
const btnSigClearAvail = document.getElementById('btn-sig-clear-avail');
const btnSigUnbindAllModal = document.getElementById('btn-sig-unbind-all-modal');
const sigAvailSearch = document.getElementById('sig-transfer-avail-search');
const sigAssignedSearch = document.getElementById('sig-transfer-assigned-search');

let _activeManageSigIdx = 0;
let _checkedAvailableGroups = new Set();
let _manageAvailAreaFilter = 'all';
let _manageAvailSearchVal = '';
let _manageAssignedSearchVal = '';

window.setManageAvailAreaFilter = (area) => {
    _manageAvailAreaFilter = area;
    renderManageGroupsModal();
};

window.openManageGroupsModal = (sigIdx) => {
    if (!manageSigModal) return;
    _activeManageSigIdx = (sigIdx !== undefined && sigIdx >= 0 && sigIdx < signatoriesData.length) ? sigIdx : 0;
    _checkedAvailableGroups.clear();
    _manageAvailAreaFilter = 'all';
    _manageAvailSearchVal = '';
    _manageAssignedSearchVal = '';
    if (sigAvailSearch) sigAvailSearch.value = '';
    if (sigAssignedSearch) sigAssignedSearch.value = '';

    renderManageGroupsModal();
    manageSigModal.classList.add('active');
    if (typeof trapFocus === 'function') trapFocus(manageSigModal);
};

window.closeManageGroupsModal = () => {
    if (!manageSigModal) return;
    manageSigModal.classList.remove('active');
    if (typeof releaseFocusTrap === 'function') releaseFocusTrap();
    _checkedAvailableGroups.clear();
};

window.switchManageGroupsSignatory = (sigIdx) => {
    _activeManageSigIdx = sigIdx;
    _checkedAvailableGroups.clear();
    renderManageGroupsModal();
};

function renderManageGroupsModal() {
    normalizeSignatoriesData();
    const allGroups = getAllProjectGroups();
    const currentSig = signatoriesData[_activeManageSigIdx];
    if (!currentSig) return;
    if (!currentSig.projectGroups) currentSig.projectGroups = [];

    // Calculate volunteer counts per group
    const groupVolunteerCounts = {};
    if (typeof workersData !== 'undefined' && Array.isArray(workersData)) {
        workersData.forEach(w => {
            if (!w || w.isArchived) return;
            const g = (w.projectGroup || '').trim();
            if (g) groupVolunteerCounts[g] = (groupVolunteerCounts[g] || 0) + 1;
        });
    }

    // Determine ownership across all signatories
    const assignedGroupOwners = {};
    signatoriesData.forEach((sig, idx) => {
        (sig.projectGroups || []).forEach(g => {
            assignedGroupOwners[g] = idx;
        });
    });

    // Available groups = groups not assigned to ANY signatory
    const rawAvailable = allGroups.filter(g => assignedGroupOwners[g] === undefined);

    // Compute distinct areas among rawAvailable
    const availAreas = new Set();
    rawAvailable.forEach(g => {
        availAreas.add(getGroupAreaName(g));
    });
    const sortedAvailAreas = Array.from(availAreas).sort();

    // Render Area Filter Pills for available groups
    const pillsEl = document.getElementById('sig-transfer-avail-area-pills');
    if (pillsEl) {
        if (sortedAvailAreas.length <= 1) {
            pillsEl.style.display = 'none';
        } else {
            pillsEl.style.display = 'flex';
            const isAll = _manageAvailAreaFilter === 'all';
            let pillsHtml = `<button type="button" class="btn btn-xs ${isAll ? 'btn-primary' : 'btn-secondary'}" onclick="window.setManageAvailAreaFilter('all')" style="font-size: 0.68rem; padding: 2px 8px; border-radius: 9999px; white-space: nowrap; font-weight: 700;">All (${rawAvailable.length})</button>`;
            sortedAvailAreas.forEach(a => {
                const isActive = _manageAvailAreaFilter === a;
                const countInArea = rawAvailable.filter(g => getGroupAreaName(g) === a).length;
                pillsHtml += `<button type="button" class="btn btn-xs ${isActive ? 'btn-primary' : 'btn-secondary'}" onclick="window.setManageAvailAreaFilter('${esc(a)}')" style="font-size: 0.68rem; padding: 2px 8px; border-radius: 9999px; white-space: nowrap; font-weight: 700;">📍 ${esc(a)} (${countInArea})</button>`;
            });
            pillsEl.innerHTML = pillsHtml;
        }
    }

    const filteredAvailable = rawAvailable.filter(g => {
        const matchArea = _manageAvailAreaFilter === 'all' || getGroupAreaName(g) === _manageAvailAreaFilter;
        const matchSearch = !_manageAvailSearchVal || g.toLowerCase().includes(_manageAvailSearchVal.toLowerCase());
        return matchArea && matchSearch;
    });

    // Current signatory assigned groups
    const rawAssigned = currentSig.projectGroups;
    const filteredAssigned = rawAssigned.filter(g =>
        !_manageAssignedSearchVal || g.toLowerCase().includes(_manageAssignedSearchVal.toLowerCase())
    );

    // 1. Render Signatory Switcher Tab Bar
    const tabBar = document.getElementById('sig-manage-tab-bar');
    if (tabBar) {
        tabBar.innerHTML = signatoriesData.map((sig, idx) => {
            const isActive = idx === _activeManageSigIdx;
            const count = (sig.projectGroups || []).length;
            return `
            <button type="button" class="sig-tab-pill ${isActive ? 'active' : ''}" onclick="window.switchManageGroupsSignatory(${idx})">
                <span style="font-weight: 700;">${esc(sig.name)}</span>
                <span style="font-size: 0.65rem; opacity: 0.85; padding: 1px 6px; border-radius: 9999px; background: ${isActive ? 'rgba(255,255,255,0.25)' : 'var(--bg-color)'}; border: ${isActive ? 'none' : '1px solid var(--border-color)'}; font-weight: 700;">
                    ${count} group${count !== 1 ? 's' : ''}
                </span>
            </button>`;
        }).join('');
    }

    // 2. Render Left Column (Available Groups)
    const availListEl = document.getElementById('sig-transfer-avail-list');
    const availCountEl = document.getElementById('sig-transfer-avail-count');
    if (availCountEl) availCountEl.textContent = `${rawAvailable.length}`;

    if (availListEl) {
        if (filteredAvailable.length === 0) {
            availListEl.innerHTML = `
                <div style="padding: 32px 16px; text-align: center; color: var(--text-muted); font-size: 0.8rem; font-style: italic;">
                    ${rawAvailable.length === 0 ? 'All project groups are assigned across signatories.' : 'No available groups match your area or search filter.'}
                </div>`;
        } else {
            availListEl.innerHTML = filteredAvailable.map(g => {
                const isChecked = _checkedAvailableGroups.has(g);
                const vols = groupVolunteerCounts[g] || 0;
                const eg = esc(g);
                const area = getGroupAreaName(g);
                return `
                <label class="sig-transfer-item ${isChecked ? 'selected' : ''}" style="cursor: pointer;">
                    <div style="display: flex; align-items: center; gap: 8px; flex: 1; min-width: 0;">
                        <input type="checkbox" class="sig-avail-checkbox" value="${eg}" ${isChecked ? 'checked' : ''} onchange="window.toggleAvailableGroupCheck('${eg}', this.checked)" style="margin: 0; cursor: pointer;">
                        <span style="font-size: 0.65rem; background: rgba(30, 64, 175, 0.08); color: var(--primary-color); border: 1px solid rgba(30, 64, 175, 0.18); padding: 1px 5px; border-radius: 4px; font-weight: 700; white-space: nowrap;">
                            📍 ${esc(area)}
                        </span>
                        <span style="font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;" title="${eg}">${eg}</span>
                    </div>
                    <span style="font-size: 0.68rem; color: var(--text-muted); background: var(--bg-color); border: 1px solid var(--border-color); padding: 1px 6px; border-radius: 9999px; white-space: nowrap; margin-left: 6px;">
                        ${vols} vol${vols !== 1 ? 's' : ''}
                    </span>
                </label>`;
            }).join('');
        }
    }

    // Update center button hint
    const hintEl = document.getElementById('sig-transfer-selected-hint');
    if (hintEl) hintEl.textContent = `${_checkedAvailableGroups.size} selected`;
    if (btnSigTransferRight) btnSigTransferRight.disabled = _checkedAvailableGroups.size === 0;

    // 3. Render Right Column (Assigned Groups)
    const assignedListEl = document.getElementById('sig-transfer-assigned-list');
    const assignedCountEl = document.getElementById('sig-transfer-assigned-count');
    const assignedTitleEl = document.getElementById('sig-transfer-assigned-title');

    if (assignedTitleEl) assignedTitleEl.textContent = `Assigned to ${currentSig.name}`;
    if (assignedCountEl) assignedCountEl.textContent = `${rawAssigned.length}`;

    if (assignedListEl) {
        if (filteredAssigned.length === 0) {
            assignedListEl.innerHTML = `
                <div style="padding: 32px 16px; text-align: center; color: var(--text-muted); font-size: 0.8rem; font-style: italic;">
                    ${rawAssigned.length === 0 ? `No groups assigned to ${esc(currentSig.name)} yet. Check available groups on the left and click "Assign Selected".` : 'No assigned groups match filter.'}
                </div>`;
        } else {
            assignedListEl.innerHTML = filteredAssigned.map(g => {
                const eg = esc(g);
                const vols = groupVolunteerCounts[g] || 0;
                const area = getGroupAreaName(g);
                return `
                <div class="sig-transfer-item" style="cursor: default;">
                    <div style="display: flex; align-items: center; gap: 8px; flex: 1; min-width: 0;">
                        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" style="color: #2563eb; flex-shrink: 0;"><polyline points="20 6 9 17 4 12"></polyline></svg>
                        <span style="font-size: 0.65rem; background: rgba(30, 64, 175, 0.08); color: var(--primary-color); border: 1px solid rgba(30, 64, 175, 0.18); padding: 1px 5px; border-radius: 4px; font-weight: 700; white-space: nowrap;">
                            📍 ${esc(area)}
                        </span>
                        <span style="font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;" title="${eg}">${eg}</span>
                    </div>
                    <div style="display: flex; align-items: center; gap: 8px;">
                        <span style="font-size: 0.68rem; background: rgba(37,99,235,0.1); color: #1e40af; padding: 1px 6px; border-radius: 9999px; font-weight: 700; white-space: nowrap;">
                            ${vols} vol${vols !== 1 ? 's' : ''}
                        </span>
                        <button type="button" class="sig-chip-remove" onclick="window.removeGroupFromModal('${eg}')" title="Remove ${eg} from ${esc(currentSig.name)}" aria-label="Remove ${eg}">
                            &times;
                        </button>
                    </div>
                </div>`;
            }).join('');
        }
    }

    // 4. Update Summary Footer
    const summaryFooter = document.getElementById('sig-transfer-summary-footer');
    if (summaryFooter) {
        const totalVols = rawAssigned.reduce((sum, g) => sum + (groupVolunteerCounts[g] || 0), 0);
        summaryFooter.innerHTML = `
            <strong>${esc(currentSig.name)}</strong>: <span style="color: #1e40af; font-weight: 700;">${rawAssigned.length} group(s) bound</span> &bull; <strong>${totalVols}</strong> total volunteers covered
        `;
    }
}

window.toggleAvailableGroupCheck = (groupName, checked) => {
    if (checked) {
        _checkedAvailableGroups.add(groupName);
    } else {
        _checkedAvailableGroups.delete(groupName);
    }
    const hintEl = document.getElementById('sig-transfer-selected-hint');
    if (hintEl) hintEl.textContent = `${_checkedAvailableGroups.size} selected`;
    if (btnSigTransferRight) btnSigTransferRight.disabled = _checkedAvailableGroups.size === 0;

    const checkboxes = document.querySelectorAll('.sig-avail-checkbox');
    checkboxes.forEach(cb => {
        if (cb.value === groupName) {
            const item = cb.closest('.sig-transfer-item');
            if (item) {
                if (checked) item.classList.add('selected');
                else item.classList.remove('selected');
            }
        }
    });
};

window.selectAllManageGroupsAvailable = (checked) => {
    const listEl = document.getElementById('sig-transfer-avail-list');
    const checkboxes = listEl ? listEl.querySelectorAll('.sig-avail-checkbox') : document.querySelectorAll('.sig-avail-checkbox');
    checkboxes.forEach(cb => {
        cb.checked = checked;
        if (checked) _checkedAvailableGroups.add(cb.value);
        else _checkedAvailableGroups.delete(cb.value);
        const item = cb.closest('.sig-transfer-item');
        if (item) {
            if (checked) item.classList.add('selected');
            else item.classList.remove('selected');
        }
    });
    const hintEl = document.getElementById('sig-transfer-selected-hint');
    if (hintEl) hintEl.textContent = `${_checkedAvailableGroups.size} selected`;
    if (btnSigTransferRight) btnSigTransferRight.disabled = _checkedAvailableGroups.size === 0;
};

window.transferSelectedToActiveSignatory = () => {
    if (_checkedAvailableGroups.size === 0) return;
    const currentSig = signatoriesData[_activeManageSigIdx];
    if (!currentSig) return;
    if (!currentSig.projectGroups) currentSig.projectGroups = [];

    const toAdd = Array.from(_checkedAvailableGroups);

    // Remove toAdd from any other signatory to ensure mutual exclusivity
    signatoriesData.forEach(s => {
        if (s.projectGroups) {
            s.projectGroups = s.projectGroups.filter(g => !toAdd.includes(g));
            s.trades = s.projectGroups;
        }
    });

    toAdd.forEach(g => {
        if (!currentSig.projectGroups.includes(g)) {
            currentSig.projectGroups.push(g);
        }
    });
    currentSig.trades = currentSig.projectGroups;

    saveSignatoriesToStorage();
    _checkedAvailableGroups.clear();
    renderManageGroupsModal();
    renderSignatories();
    if (window.ProjectGroupsModule && typeof window.ProjectGroupsModule.init === 'function') {
        window.ProjectGroupsModule.init();
    }
    if (typeof showToast === 'function') {
        showToast(`Assigned ${toAdd.length} group(s) to ${currentSig.name}.`);
    }
};

window.removeGroupFromModal = (groupName) => {
    const currentSig = signatoriesData[_activeManageSigIdx];
    if (!currentSig) return;
    if (!currentSig.projectGroups) currentSig.projectGroups = [];

    currentSig.projectGroups = currentSig.projectGroups.filter(g => g !== groupName);
    currentSig.trades = currentSig.projectGroups;

    saveSignatoriesToStorage();
    renderManageGroupsModal();
    renderSignatories();
    if (window.ProjectGroupsModule && typeof window.ProjectGroupsModule.init === 'function') {
        window.ProjectGroupsModule.init();
    }
    if (typeof showToast === 'function') {
        showToast(`Removed "${groupName}" from ${currentSig.name}.`);
    }
};

window.removeAllGroupsFromActiveSignatoryModal = async () => {
    const currentSig = signatoriesData[_activeManageSigIdx];
    if (!currentSig || !currentSig.projectGroups || currentSig.projectGroups.length === 0) return;
    if (await showConfirm(`Unbind all ${currentSig.projectGroups.length} group(s) from ${currentSig.name}?`)) {
        currentSig.projectGroups = [];
        currentSig.trades = [];
        saveSignatoriesToStorage();
        renderManageGroupsModal();
        renderSignatories();
        if (window.ProjectGroupsModule && typeof window.ProjectGroupsModule.init === 'function') {
            window.ProjectGroupsModule.init();
        }
        if (typeof showToast === 'function') {
            showToast(`Cleared all groups from ${currentSig.name}.`);
        }
    }
};

// Event listener bindings
btnCloseManageSigModal?.addEventListener('click', () => window.closeManageGroupsModal());
btnDoneManageSigModal?.addEventListener('click', () => window.closeManageGroupsModal());
btnSigTransferRight?.addEventListener('click', () => window.transferSelectedToActiveSignatory());
btnSigSelectAllAvail?.addEventListener('click', () => window.selectAllManageGroupsAvailable(true));
btnSigClearAvail?.addEventListener('click', () => window.selectAllManageGroupsAvailable(false));
btnSigUnbindAllModal?.addEventListener('click', () => window.removeAllGroupsFromActiveSignatoryModal());

sigAvailSearch?.addEventListener('input', (e) => {
    _manageAvailSearchVal = e.target.value.trim();
    renderManageGroupsModal();
});

sigAssignedSearch?.addEventListener('input', (e) => {
    _manageAssignedSearchVal = e.target.value.trim();
    renderManageGroupsModal();
});

manageSigModal?.addEventListener('click', (e) => {
    if (e.target === manageSigModal) window.closeManageGroupsModal();
});


