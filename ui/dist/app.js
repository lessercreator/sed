// SED Editor — Frontend
// Communicates with Rust backend via Tauri invoke()

const { invoke } = window.__TAURI__.core;

// =============================================================================
// STATE
// =============================================================================
const state = {
    info: {},
    spaces: [],
    placements: [],
    productTypes: [],
    systems: [],
    notes: [],
    submittals: [],
    roomGeometry: {},
    graph: { nodes: [], segments: [] },
    currentLevel: 'Level 1',
    levels: [],
    selectedElement: null,
    selectedTable: null,
    activeTool: 'select',
    dirty: true,
};

const view = {
    x: 0,
    y: 0,
    scale: 1,
    baseScale: 1,
};

const canvas = {
    el: null,
    ctx: null,
    width: 0,
    height: 0,
};

const mouse = {
    x: 0,
    y: 0,
    worldX: 0,
    worldY: 0,
    down: false,
    button: 0,
    dragStartX: 0,
    dragStartY: 0,
    viewStartX: 0,
    viewStartY: 0,
    draggingElement: null,
    dragElementStartX: 0,
    dragElementStartY: 0,
};

const roomDraw = {
    vertices: [],
    active: false,
};

const ductDraw = {
    nodes: [],
    systemId: null,
    active: false,
};

const equipPlace = {
    active: false,
    pickerX: 0,
    pickerY: 0,
    worldX: 0,
    worldY: 0,
};

let animFrameId = null;

// =============================================================================
// SHEET CONSTANTS (ARCH D — matches export.rs exactly)
// =============================================================================
const SHEET = {
    W: 914.4,       // 36" in mm
    H: 609.6,       // 24" in mm
    BORDER: 12.7,   // 1/2" border
    TB_W: 190.0,    // title block width
    TB_H: 76.0,     // title block height
};

// Cached model-to-sheet transform (recomputed each render)
const sheetXform = {
    drawLeft: SHEET.BORDER + 15,
    drawRight: SHEET.W - SHEET.BORDER - 15,
    drawTop: SHEET.BORDER + 15,
    drawBottom: SHEET.H - SHEET.BORDER - SHEET.TB_H - 15,
    drawW: SHEET.W - 2 * SHEET.BORDER - 30,
    drawH: SHEET.H - 2 * SHEET.BORDER - SHEET.TB_H - 30,
    modelXMin: 0, modelYMin: 0, modelXMax: 20, modelYMax: 20,
    scale: 20, xOff: 0, yOff: 0,
};

const SNAP_GRID = 0.25; // 25cm snap grid (approximately 1 foot = 0.3048m)

function snap(v) {
    return Math.round(v / SNAP_GRID) * SNAP_GRID;
}

// =============================================================================
// BOOT
// =============================================================================
document.getElementById('btn-new').addEventListener('click', () => {
    document.getElementById('new-project-dialog').classList.add('open');
    document.getElementById('np-name').focus();
});

document.getElementById('np-cancel').addEventListener('click', () => {
    document.getElementById('new-project-dialog').classList.remove('open');
});

document.getElementById('np-create').addEventListener('click', async () => {
    const name = document.getElementById('np-name').value.trim();
    const number = document.getElementById('np-number').value.trim();
    if (!name) return;
    const filename = (number || 'project').replace(/\s+/g, '-').toLowerCase() + '.sed';
    state.info = await invoke('new_file', { path: filename, projectName: name, projectNumber: number || '00-000' });
    document.getElementById('new-project-dialog').classList.remove('open');
    await loadDocument();
});

document.getElementById('btn-open').addEventListener('click', async () => {
    try {
        const result = await invoke('open_file', { path: 'skims-americana.sed' });
        state.info = result;
        await loadDocument();
    } catch (e) {
        const result = await invoke('create_example', { path: 'skims-americana.sed' });
        state.info = result;
        await loadDocument();
    }
});

document.getElementById('btn-example').addEventListener('click', async () => {
    const result = await invoke('create_example', { path: 'skims-americana.sed' });
    state.info = result;
    await loadDocument();
});

// =============================================================================
// LOAD DOCUMENT
// =============================================================================
async function loadDocument() {
    document.getElementById('welcome').classList.add('hidden');
    document.getElementById('editor').classList.remove('hidden');

    await reloadAll();
    setupCanvas();
    fitView();
    startRenderLoop();
}

async function reloadAll() {
    const [spaces, placements, productTypes, systems, notes, submittals] = await Promise.all([
        invoke('get_spaces'),
        invoke('get_placements'),
        invoke('get_product_types'),
        invoke('get_systems'),
        invoke('get_notes'),
        invoke('get_submittals'),
    ]);

    state.spaces = spaces;
    state.placements = placements;
    state.productTypes = productTypes;
    state.systems = systems;
    state.notes = notes;
    state.submittals = submittals;

    state.info = await invoke('get_info');
    state.levels = [...new Set(state.spaces.map(s => s.level))].sort();

    if (!state.levels.includes(state.currentLevel) && state.levels.length > 0) {
        state.currentLevel = state.levels[0];
    }

    updateHeader();
    buildLevelSelector();
    await loadLevelData();
    buildSidebar();
    state.dirty = true;
}

async function reloadAfterMutation() {
    const [spaces, placements, productTypes, systems] = await Promise.all([
        invoke('get_spaces'),
        invoke('get_placements'),
        invoke('get_product_types'),
        invoke('get_systems'),
    ]);
    state.spaces = spaces;
    state.placements = placements;
    state.productTypes = productTypes;
    state.systems = systems;
    state.info = await invoke('get_info');
    state.levels = [...new Set(state.spaces.map(s => s.level))].sort();

    await loadLevelData();
    updateHeader();
    buildSidebar();
    state.dirty = true;
}

async function loadLevelData() {
    const [geom, graph] = await Promise.all([
        invoke('get_room_geometry', { level: state.currentLevel }),
        invoke('get_graph', { level: state.currentLevel, systemTag: null }),
    ]);
    state.roomGeometry = {};
    geom.forEach(r => { state.roomGeometry[r.tag] = r; });
    state.graph = graph;
}

function updateHeader() {
    document.getElementById('project-name').textContent = state.info.project_name || 'Untitled';
    document.getElementById('project-meta').textContent =
        `#${state.info.project_number || ''} | ${state.info.placements || 0} placements`;
    const sc = document.getElementById('st-counts');
    sc.textContent = `${state.spaces.length} spaces | ${state.placements.length} placements | ${(state.graph.nodes || []).length} nodes`;
}

function buildLevelSelector() {
    const sel = document.getElementById('level-select');
    sel.innerHTML = '';
    state.levels.forEach(l => {
        const o = document.createElement('option');
        o.value = l;
        o.textContent = l;
        if (l === state.currentLevel) o.selected = true;
        sel.appendChild(o);
    });
    sel.onchange = async () => {
        state.currentLevel = sel.value;
        await loadLevelData();
        state.dirty = true;
    };
}

// =============================================================================
// SIDEBAR
// =============================================================================
function buildSidebar() {
    buildSpacesTab();
    buildDevicesTab();
    buildCatalogTab();
    buildSystemsTab();
}

function buildSpacesTab() {
    const container = document.getElementById('tab-spaces');
    container.innerHTML = '';
    state.levels.forEach(level => {
        const g = el('div', 'group');
        g.appendChild(elWithText('div', 'group-label', level));
        state.spaces.filter(s => s.level === level).forEach(s => {
            const devs = state.placements.filter(p => p.space_tag === s.tag);
            const cfm = devs.reduce((sum, p) => sum + (parseFloat(p.cfm) || 0), 0);
            const item = el('div', 'item' + (s.scope === 'nic' ? ' nic' : ''));
            item.innerHTML = `<span class="tag">${esc(s.tag)}</span><span class="name">${esc(s.name)}</span><span class="val">${cfm > 0 ? cfm + ' CFM' : ''}</span>`;
            item.onclick = () => selectElement(s, 'spaces');
            if (state.selectedElement && state.selectedElement.id === s.id) item.classList.add('selected');
            g.appendChild(item);
        });
        container.appendChild(g);
    });
}

function buildDevicesTab() {
    const container = document.getElementById('tab-devices');
    container.innerHTML = '';
    const byTag = {};
    state.placements.forEach(p => {
        const key = p.tag;
        if (!byTag[key]) byTag[key] = [];
        byTag[key].push(p);
    });
    const groups = Object.entries(byTag).sort((a, b) => b[1].length - a[1].length);
    groups.forEach(([tag, items]) => {
        const first = items[0];
        const cfm = items.reduce((sum, p) => sum + (parseFloat(p.cfm) || 0), 0);
        const mfr = nv(first.manufacturer);
        const mdl = nv(first.model);
        const item = el('div', 'item');
        item.innerHTML = `<span class="tag">${esc(tag)}</span><span class="name">${items.length}x ${esc(mfr)} ${esc(mdl)}</span><span class="val">${cfm > 0 ? Math.round(cfm) + ' CFM' : ''}</span>`;
        item.onclick = () => showTypeGroupProps(tag, items);
        container.appendChild(item);
    });
}

async function createProductTypeDialog() {
    const result = await showDialog('New Product Type', [
        { name: 'tag', label: 'Tag', type: 'text', placeholder: 'e.g. LD-1, AHU, VAV' },
        { name: 'domain', label: 'Domain', type: 'select', default: 'air_device', options: [
            'air_device', 'equipment', 'accessory',
        ]},
        { name: 'category', label: 'Category', type: 'text', placeholder: 'e.g. supply_diffuser, rtu, vav_box' },
        { name: 'manufacturer', label: 'Manufacturer', type: 'text' },
        { name: 'model', label: 'Model', type: 'text' },
    ]);
    if (!result || !result.tag) return;
    await invoke('create_product_type', {
        tag: result.tag,
        domain: result.domain || 'air_device',
        category: result.category || 'air_device',
        manufacturer: result.manufacturer || null,
        model: result.model || null,
        description: null,
    });
    await reloadAfterMutation();
}

function buildCatalogTab() {
    const container = document.getElementById('tab-catalog');
    container.innerHTML = '';

    // "New Product Type" button
    const addBtn = el('div', 'item');
    addBtn.style.color = 'var(--accent)';
    addBtn.style.justifyContent = 'center';
    addBtn.style.fontWeight = '600';
    addBtn.innerHTML = '+ New Product Type';
    addBtn.onclick = () => createProductTypeDialog();
    container.appendChild(addBtn);

    const byDomain = {};
    state.productTypes.forEach(pt => {
        const d = pt.domain || 'other';
        if (!byDomain[d]) byDomain[d] = [];
        byDomain[d].push(pt);
    });
    Object.entries(byDomain).sort().forEach(([domain, types]) => {
        const g = el('div', 'group');
        g.appendChild(elWithText('div', 'group-label', domain));
        types.forEach(pt => {
            const item = el('div', 'item draggable');
            item.innerHTML = `<span class="tag">${esc(pt.tag)}</span><span class="name">${esc(nv(pt.manufacturer))} ${esc(nv(pt.model))}</span><span class="val">${esc(pt.category || '')}</span>`;
            item.draggable = true;
            item.addEventListener('dragstart', e => {
                e.dataTransfer.setData('text/plain', pt.id);
                e.dataTransfer.effectAllowed = 'copy';
            });
            item.onclick = () => selectElement(pt, 'product_types');
            g.appendChild(item);
        });
        container.appendChild(g);
    });
}

function buildSystemsTab() {
    const container = document.getElementById('tab-systems');
    container.innerHTML = '';
    const byType = {};
    state.systems.forEach(sys => {
        const t = sys.system_type || 'other';
        if (!byType[t]) byType[t] = [];
        byType[t].push(sys);
    });
    Object.entries(byType).sort().forEach(([type, systems]) => {
        const g = el('div', 'group');
        g.appendChild(elWithText('div', 'group-label', type));
        systems.forEach(sys => {
            const item = el('div', 'item');
            item.innerHTML = `<span class="tag">${esc(sys.tag)}</span><span class="name">${esc(sys.name)}</span><span class="val">${esc(sys.medium || '')}</span>`;
            item.onclick = () => selectElement(sys, 'systems');
            g.appendChild(item);
        });
        container.appendChild(g);
    });
}

// Tabs
document.querySelectorAll('.tab').forEach(t => {
    t.onclick = () => {
        document.querySelectorAll('.tab').forEach(x => x.classList.remove('active'));
        document.querySelectorAll('.tab-content').forEach(x => x.classList.remove('active'));
        t.classList.add('active');
        document.getElementById('tab-' + t.dataset.tab).classList.add('active');
    };
});

// =============================================================================
// PROPERTIES PANEL
// =============================================================================
function selectElement(elem, table) {
    state.selectedElement = elem;
    state.selectedTable = table;
    showProperties(elem, table);
    state.dirty = true;
}

function showProperties(elem, table) {
    const panel = document.getElementById('props');
    if (!elem) {
        panel.classList.remove('open');
        return;
    }
    panel.classList.add('open');

    if (table === 'spaces') showSpaceProps(elem);
    else if (table === 'placements') showPlacementProps(elem);
    else if (table === 'product_types') showProductTypeProps(elem);
    else if (table === 'systems') showSystemProps(elem);
    else if (table === 'nodes') showNodeProps(elem);
    else panel.classList.remove('open');
}

function showSpaceProps(s) {
    const devs = state.placements.filter(p => p.space_tag === s.tag);
    const cfm = devs.reduce((sum, p) => sum + (parseFloat(p.cfm) || 0), 0);

    let h = `<h3>${esc(s.tag)} - ${esc(s.name)}<span class="close" onclick="SED.closeProps()">&times;</span></h3>`;
    h += readonlyProp('ID', s.id);
    h += readonlyProp('Level', s.level);
    h += editProp('name', 'Name', s.name, 'spaces', s.id);
    h += editProp('space_type', 'Type', s.space_type, 'spaces', s.id);
    h += editProp('scope', 'Scope', s.scope, 'spaces', s.id);
    h += readonlyProp('Devices', devs.length);
    h += readonlyProp('Total CFM', cfm > 0 ? Math.round(cfm) : '-');

    if (devs.length) {
        h += `<div class="section"><div class="section-label">Devices</div>`;
        devs.forEach(d => {
            h += `<div class="pr" style="cursor:pointer" onclick="SED.selectPlacementById('${d.id}')"><span class="k" style="color:var(--accent)">${esc(d.tag)}</span><span class="v">${nv(d.cfm)} CFM</span></div>`;
        });
        h += '</div>';
    }

    h += `<div class="props-actions"><button class="btn" onclick="SED.deleteSelected()" style="color:var(--red);border-color:#f4433660">Delete Space</button></div>`;

    document.getElementById('props').innerHTML = h;
}

function showPlacementProps(p) {
    let h = `<h3>${esc(p.tag)}${p.instance_tag && p.instance_tag !== 'NULL' ? ' (' + esc(p.instance_tag) + ')' : ''}<span class="close" onclick="SED.closeProps()">&times;</span></h3>`;
    h += readonlyProp('ID', p.id);
    h += readonlyProp('Domain', p.domain);
    h += readonlyProp('Category', p.category);
    h += readonlyProp('Manufacturer', nv(p.manufacturer));
    h += readonlyProp('Model', nv(p.model));
    h += editProp('cfm', 'CFM', p.cfm, 'placements', p.id);
    h += editProp('status', 'Status', p.status, 'placements', p.id);
    h += editProp('phase', 'Phase', p.phase, 'placements', p.id);
    h += editProp('scope', 'Scope', p.scope, 'placements', p.id);
    h += readonlyProp('Room', nv(p.space_name));
    h += readonlyProp('Position', `${parseFloat(p.x || 0).toFixed(2)}, ${parseFloat(p.y || 0).toFixed(2)} m`);

    if (p.notes && p.notes !== 'NULL') {
        h += `<div class="section"><div class="section-label">Notes</div><div style="font-size:11px;color:var(--text2);line-height:1.5">${esc(p.notes)}</div></div>`;
    }

    h += `<div class="props-actions"><button class="btn" onclick="SED.deleteSelected()" style="color:var(--red);border-color:#f4433660">Delete Placement</button></div>`;

    document.getElementById('props').innerHTML = h;
}

function showProductTypeProps(pt) {
    const instances = state.placements.filter(p => p.product_type_id === pt.id);
    let h = `<h3>${esc(pt.tag)}<span class="close" onclick="SED.closeProps()">&times;</span></h3>`;
    h += readonlyProp('ID', pt.id);
    h += readonlyProp('Domain', pt.domain);
    h += readonlyProp('Category', pt.category);
    h += readonlyProp('Manufacturer', nv(pt.manufacturer));
    h += readonlyProp('Model', nv(pt.model));
    h += readonlyProp('Description', nv(pt.description));
    h += readonlyProp('Mounting', nv(pt.mounting));
    h += readonlyProp('Instances', instances.length);

    if (instances.length) {
        h += `<div class="section"><div class="section-label">Instances</div>`;
        instances.forEach(p => {
            h += `<div class="pr" style="cursor:pointer" onclick="SED.selectPlacementById('${p.id}')"><span class="k">${esc(nv(p.space_name))}</span><span class="v">${nv(p.cfm)} CFM</span></div>`;
        });
        h += '</div>';
    }

    document.getElementById('props').innerHTML = h;
}

function showSystemProps(sys) {
    let h = `<h3>${esc(sys.tag)}<span class="close" onclick="SED.closeProps()">&times;</span></h3>`;
    h += readonlyProp('ID', sys.id);
    h += readonlyProp('Name', sys.name);
    h += readonlyProp('Type', sys.system_type);
    h += readonlyProp('Medium', sys.medium);
    document.getElementById('props').innerHTML = h;
}

function showTypeGroupProps(tag, items) {
    const panel = document.getElementById('props');
    const first = items[0];
    const cfm = items.reduce((sum, p) => sum + (parseFloat(p.cfm) || 0), 0);
    let h = `<h3>${esc(tag)} (${items.length}x)<span class="close" onclick="SED.closeProps()">&times;</span></h3>`;
    h += readonlyProp('Category', first.category);
    h += readonlyProp('Manufacturer', nv(first.manufacturer));
    h += readonlyProp('Model', nv(first.model));
    h += readonlyProp('Total CFM', cfm > 0 ? Math.round(cfm) : '-');
    h += `<div class="section"><div class="section-label">Instances</div>`;
    items.forEach(p => {
        h += `<div class="pr" style="cursor:pointer" onclick="SED.selectPlacementById('${p.id}')"><span class="k">${esc(nv(p.space_name))}</span><span class="v">${nv(p.cfm)} CFM</span></div>`;
    });
    h += '</div>';
    panel.innerHTML = h;
    panel.classList.add('open');
}

function showNodeProps(n) {
    let h = `<h3>Node<span class="close" onclick="SED.closeProps()">&times;</span></h3>`;
    h += readonlyProp('ID', n.id);
    h += readonlyProp('Type', n.node_type);
    h += readonlyProp('Fitting', nv(n.fitting_type));
    h += readonlyProp('Size', nv(n.size_description));
    h += readonlyProp('System', nv(n.system_tag));
    h += readonlyProp('Position', `${parseFloat(n.x || 0).toFixed(2)}, ${parseFloat(n.y || 0).toFixed(2)} m`);
    document.getElementById('props').innerHTML = h;
}

function readonlyProp(k, v) {
    return `<div class="pr"><span class="k">${k}</span><span class="v readonly">${esc(String(v))}</span></div>`;
}

function editProp(field, label, val, table, id) {
    const v = (val && val !== 'NULL') ? val : '';
    return `<div class="pr"><span class="k">${label}</span><input value="${esc(v)}" onchange="SED.updateField('${table}','${id}','${field}',this.value)"></div>`;
}

function closeProps() {
    document.getElementById('props').classList.remove('open');
    state.selectedElement = null;
    state.selectedTable = null;
    state.dirty = true;
}

// =============================================================================
// CANVAS SETUP
// =============================================================================
function setupCanvas() {
    canvas.el = document.getElementById('plan');
    canvas.ctx = canvas.el.getContext('2d');
    resizeCanvas();

    window.addEventListener('resize', () => { resizeCanvas(); state.dirty = true; });

    canvas.el.addEventListener('mousedown', onCanvasMouseDown);
    canvas.el.addEventListener('mousemove', onCanvasMouseMove);
    canvas.el.addEventListener('mouseup', onCanvasMouseUp);
    canvas.el.addEventListener('mouseleave', onCanvasMouseLeave);
    canvas.el.addEventListener('wheel', onCanvasWheel, { passive: false });
    canvas.el.addEventListener('dblclick', onCanvasDoubleClick);
    canvas.el.addEventListener('contextmenu', e => e.preventDefault());

    canvas.el.addEventListener('dragover', e => { e.preventDefault(); e.dataTransfer.dropEffect = 'copy'; });
    canvas.el.addEventListener('drop', onCanvasDrop);
}

function resizeCanvas() {
    const wrap = document.getElementById('canvas-wrap');
    canvas.width = wrap.clientWidth;
    canvas.height = wrap.clientHeight;
    canvas.el.width = canvas.width * devicePixelRatio;
    canvas.el.height = canvas.height * devicePixelRatio;
    canvas.el.style.width = canvas.width + 'px';
    canvas.el.style.height = canvas.height + 'px';
    canvas.ctx.setTransform(devicePixelRatio, 0, 0, devicePixelRatio, 0, 0);
    state.dirty = true;
}

function fitView() {
    // Compute the sheet transform first so coordinate conversions work
    computeSheetTransform();
    // Fit the sheet into the viewport with padding
    const padding = 40;
    const scaleX = (canvas.width - padding * 2) / SHEET.W;
    const scaleY = (canvas.height - padding * 2) / SHEET.H;
    view.scale = Math.min(scaleX, scaleY);
    view.baseScale = view.scale;
    // Center the sheet (sheet origin is top-left at 0,0 in sheet-mm space)
    view.x = (canvas.width - SHEET.W * view.scale) / 2;
    view.y = (canvas.height - SHEET.H * view.scale) / 2;
    state.dirty = true;
}

// =============================================================================
// CANVAS INTERACTION
// =============================================================================
function screenToSheet(sx, sy) {
    // Screen pixels -> sheet mm
    const rect = canvas.el.getBoundingClientRect();
    const cx = sx - rect.left;
    const cy = sy - rect.top;
    return {
        x: (cx - view.x) / view.scale,
        y: (cy - view.y) / view.scale,
    };
}

function sheetToModel(smx, smy) {
    // Sheet mm -> model meters (inverse of model-to-sheet transform)
    // On canvas: sheet Y increases downward. Model Y increases upward.
    // So model yMin maps to drawBottom (high sheet Y), model yMax maps to drawTop (low sheet Y).
    const sf = sheetXform;
    const mx = (smx - sf.drawLeft) / sf.scale + sf.xOff;
    const my = (sf.drawBottom - smy) / sf.scale + sf.yOff;
    return { x: mx, y: my };
}

function modelToSheet(mx, my) {
    // Model meters -> sheet mm
    // Model Y up = sheet Y decreasing (toward top of page)
    const sf = sheetXform;
    return {
        x: sf.drawLeft + (mx - sf.xOff) * sf.scale,
        y: sf.drawBottom - (my - sf.yOff) * sf.scale,
    };
}

function screenToWorld(sx, sy) {
    const sm = screenToSheet(sx, sy);
    return sheetToModel(sm.x, sm.y);
}

function worldToScreen(wx, wy) {
    const sm = modelToSheet(wx, wy);
    return {
        x: sm.x * view.scale + view.x,
        y: sm.y * view.scale + view.y,
    };
}

function updateMouseWorld(e) {
    const rect = canvas.el.getBoundingClientRect();
    mouse.x = e.clientX - rect.left;
    mouse.y = e.clientY - rect.top;
    const w = screenToWorld(e.clientX, e.clientY);
    mouse.worldX = w.x;
    mouse.worldY = w.y;
    document.getElementById('st-coords').textContent = `${w.x.toFixed(2)}, ${w.y.toFixed(2)} m`;
}

function isPanning(e) {
    return e.button === 1 || (e.button === 0 && e.altKey);
}

function onCanvasMouseDown(e) {
    updateMouseWorld(e);
    mouse.down = true;
    mouse.button = e.button;

    if (isPanning(e)) {
        mouse.dragStartX = e.clientX;
        mouse.dragStartY = e.clientY;
        mouse.viewStartX = view.x;
        mouse.viewStartY = view.y;
        canvas.el.style.cursor = 'grabbing';
        return;
    }

    if (e.button !== 0) return;

    if (state.activeTool === 'select') {
        const hit = hitTest(mouse.worldX, mouse.worldY);
        if (hit) {
            selectElement(hit.element, hit.table);
            if (hit.table === 'placements' || hit.table === 'nodes') {
                mouse.draggingElement = hit;
                mouse.dragElementStartX = parseFloat(hit.element.x || 0);
                mouse.dragElementStartY = parseFloat(hit.element.y || 0);
                mouse.dragStartX = mouse.worldX;
                mouse.dragStartY = mouse.worldY;
                canvas.el.style.cursor = 'move';
            }
        } else {
            closeProps();
        }
    } else if (state.activeTool === 'room') {
        roomDraw.vertices.push({ x: snap(mouse.worldX), y: snap(mouse.worldY) });
        roomDraw.active = true;
        state.dirty = true;
    } else if (state.activeTool === 'equip') {
        equipPlace.worldX = snap(mouse.worldX);
        equipPlace.worldY = snap(mouse.worldY);
        showEquipmentPicker(e.clientX, e.clientY);
    } else if (state.activeTool === 'duct') {
        handleDuctClick(snap(mouse.worldX), snap(mouse.worldY));
    }
}

function onCanvasMouseMove(e) {
    updateMouseWorld(e);

    if (mouse.down && (isPanning({ button: mouse.button, altKey: e.altKey }) || mouse.button === 1)) {
        view.x = mouse.viewStartX + (e.clientX - mouse.dragStartX);
        view.y = mouse.viewStartY + (e.clientY - mouse.dragStartY);
        updateZoomDisplay();
        state.dirty = true;
        return;
    }

    if (mouse.draggingElement && mouse.down) {
        const dx = mouse.worldX - mouse.dragStartX;
        const dy = mouse.worldY - mouse.dragStartY;
        mouse.draggingElement.element.x = String(mouse.dragElementStartX + dx);
        mouse.draggingElement.element.y = String(mouse.dragElementStartY + dy);
        state.dirty = true;
        return;
    }

    if (state.activeTool === 'select' && !mouse.down) {
        const hit = hitTest(mouse.worldX, mouse.worldY);
        canvas.el.style.cursor = hit ? 'pointer' : 'default';
    }

    if (roomDraw.active || ductDraw.active) {
        state.dirty = true;
    }
}

function onCanvasMouseUp(e) {
    if (mouse.draggingElement) {
        const elem = mouse.draggingElement.element;
        const table = mouse.draggingElement.table;
        const x = snap(parseFloat(elem.x || 0));
        const y = snap(parseFloat(elem.y || 0));
        invoke('move_element', { table, id: elem.id, x, y }).then(() => reloadAfterMutation());
        mouse.draggingElement = null;
    }

    mouse.down = false;
    canvas.el.style.cursor = getCursorForTool();
}

function onCanvasMouseLeave() {
    mouse.down = false;
    mouse.draggingElement = null;
}

function onCanvasWheel(e) {
    e.preventDefault();
    const factor = e.deltaY < 0 ? 1.1 : 0.9;
    const rect = canvas.el.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;
    view.x = mx - (mx - view.x) * factor;
    view.y = my - (my - view.y) * factor;
    view.scale *= factor;
    updateZoomDisplay();
    state.dirty = true;
}

function onCanvasDoubleClick(e) {
    if (state.activeTool !== 'select') return;
    updateMouseWorld(e);
    const ls = state.spaces.filter(s => s.level === state.currentLevel);
    for (const s of ls) {
        const geom = state.roomGeometry[s.tag];
        if (geom && pointInPolygon(mouse.worldX, mouse.worldY, geom.vertices)) {
            selectElement(s, 'spaces');
            state.currentLevel = s.level;
            document.getElementById('level-select').value = state.currentLevel;
            return;
        }
    }
}

async function onCanvasDrop(e) {
    e.preventDefault();
    const productTypeId = e.dataTransfer.getData('text/plain');
    if (!productTypeId) return;
    const w = screenToWorld(e.clientX, e.clientY);
    await invoke('create_placement', {
        productTypeId,
        level: state.currentLevel,
        x: w.x,
        y: w.y,
        cfm: null,
        spaceId: null,
        instanceTag: null,
    });
    await reloadAfterMutation();
}

function updateZoomDisplay() {
    const base = view.baseScale || view.scale || 1;
    document.getElementById('st-zoom').textContent = `${(view.scale / base * 100).toFixed(0)}%`;
}

function getCursorForTool() {
    switch (state.activeTool) {
        case 'select': return 'default';
        case 'room': return 'crosshair';
        case 'equip': return 'crosshair';
        case 'duct': return 'crosshair';
        default: return 'default';
    }
}

// =============================================================================
// HIT TESTING
// =============================================================================
function hitTest(wx, wy) {
    const hitRadius = 8 / view.scale;

    const levelPlacements = state.placements.filter(p => p.level === state.currentLevel);
    for (const p of levelPlacements) {
        const px = parseFloat(p.x), py = parseFloat(p.y);
        if (isNaN(px) || isNaN(py)) continue;
        const dx = wx - px, dy = wy - py;
        if (Math.sqrt(dx * dx + dy * dy) < hitRadius) {
            return { element: p, table: 'placements' };
        }
    }

    const nodes = state.graph.nodes || [];
    for (const n of nodes) {
        const nx = parseFloat(n.x), ny = parseFloat(n.y);
        if (isNaN(nx) || isNaN(ny)) continue;
        const dx = wx - nx, dy = wy - ny;
        if (Math.sqrt(dx * dx + dy * dy) < hitRadius) {
            return { element: n, table: 'nodes' };
        }
    }

    const ls = state.spaces.filter(s => s.level === state.currentLevel);
    for (const s of ls) {
        const geom = state.roomGeometry[s.tag];
        if (geom && pointInPolygon(wx, wy, geom.vertices)) {
            return { element: s, table: 'spaces' };
        }
    }

    return null;
}

function pointInPolygon(px, py, verts) {
    if (!verts || verts.length < 3) return false;
    let inside = false;
    for (let i = 0, j = verts.length - 1; i < verts.length; j = i++) {
        const xi = verts[i].x, yi = verts[i].y;
        const xj = verts[j].x, yj = verts[j].y;
        if ((yi > py) !== (yj > py) && px < (xj - xi) * (py - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
    }
    return inside;
}

// =============================================================================
// TOOL: DRAW ROOM
// =============================================================================
async function finishRoom() {
    if (roomDraw.vertices.length < 3) {
        roomDraw.vertices = [];
        roomDraw.active = false;
        state.dirty = true;
        return;
    }

    const result = await showDialog('New Room', [
        { name: 'tag', label: 'Room Tag', type: 'text', placeholder: 'e.g. L1-14' },
        { name: 'name', label: 'Room Name', type: 'text', placeholder: 'e.g. Main Lobby' },
        { name: 'spaceType', label: 'Space Type', type: 'select', default: 'office', options: [
            'retail', 'office', 'storage', 'restroom', 'corridor', 'mechanical',
        ]},
    ]);

    if (!result || !result.tag) {
        roomDraw.vertices = [];
        roomDraw.active = false;
        state.dirty = true;
        return;
    }

    await invoke('create_space', {
        tag: result.tag,
        name: result.name || 'Unnamed',
        level: state.currentLevel,
        spaceType: result.spaceType || 'office',
        scope: 'in_contract',
        vertices: roomDraw.vertices.map(v => ({ x: v.x, y: v.y })),
    });

    roomDraw.vertices = [];
    roomDraw.active = false;
    await reloadAfterMutation();
}

// =============================================================================
// TOOL: PLACE EQUIPMENT
// =============================================================================
function showEquipmentPicker(screenX, screenY) {
    const picker = document.getElementById('equip-picker');
    const list = document.getElementById('equip-picker-list');
    list.innerHTML = '';

    if (state.productTypes.length === 0) {
        const msg = el('div', '');
        msg.style.cssText = 'padding:8px;color:var(--text3);font-size:12px';
        msg.textContent = 'No product types in catalog.';
        list.appendChild(msg);
        const createBtn = el('div', 'item');
        createBtn.style.cssText = 'color:var(--accent);justify-content:center;font-weight:600;margin-top:4px';
        createBtn.textContent = '+ Create One';
        createBtn.onclick = async () => {
            picker.classList.add('hidden');
            await createProductTypeDialog();
        };
        list.appendChild(createBtn);
    } else {
        state.productTypes.forEach(pt => {
            const item = el('div', 'item');
            item.innerHTML = `<span class="tag">${esc(pt.tag)}</span><span class="name">${esc(pt.category || '')}</span>`;
            item.onclick = async () => {
                picker.classList.add('hidden');
                await invoke('create_placement', {
                    productTypeId: pt.id,
                    level: state.currentLevel,
                    x: equipPlace.worldX,
                    y: equipPlace.worldY,
                    cfm: null,
                    spaceId: null,
                    instanceTag: null,
                });
                await reloadAfterMutation();
            };
            list.appendChild(item);
        });
    }

    const wrap = document.getElementById('canvas-wrap');
    const wrapRect = wrap.getBoundingClientRect();
    let px = screenX - wrapRect.left;
    let py = screenY - wrapRect.top;
    if (px + 270 > wrapRect.width) px = wrapRect.width - 270;
    if (py + 310 > wrapRect.height) py = wrapRect.height - 310;
    picker.style.left = px + 'px';
    picker.style.top = py + 'px';
    picker.classList.remove('hidden');
}

// =============================================================================
// TOOL: ROUTE DUCT
// =============================================================================
async function handleDuctClick(wx, wy) {
    if (!ductDraw.systemId) {
        if (state.systems.length === 0) {
            const result = await showDialog('Create System', [
                { name: 'tag', label: 'System Tag', type: 'text', placeholder: 'e.g. RTU-1-SA' },
                { name: 'name', label: 'System Name', type: 'text', placeholder: 'e.g. RTU-1 Supply Air' },
                { name: 'systemType', label: 'System Type', type: 'select', default: 'supply', options: [
                    'supply', 'return', 'exhaust',
                ]},
            ]);
            if (!result || !result.tag) return;
            const sysResult = await invoke('create_system', {
                tag: result.tag,
                name: result.name || result.tag,
                systemType: result.systemType || 'supply',
                medium: 'air',
                sourceId: null,
            });
            await reloadAfterMutation();
            ductDraw.systemId = sysResult.id;
        } else {
            const sysOptions = state.systems.map(s => ({
                value: s.id,
                label: `${s.tag} (${s.system_type})`,
            }));
            sysOptions.push({ value: '__new__', label: '+ Create New System' });
            const result = await showDialog('Select System', [
                { name: 'systemId', label: 'System', type: 'select', options: sysOptions },
            ]);
            if (!result) return;
            if (result.systemId === '__new__') {
                const newSys = await showDialog('Create System', [
                    { name: 'tag', label: 'System Tag', type: 'text', placeholder: 'e.g. RTU-1-SA' },
                    { name: 'name', label: 'System Name', type: 'text', placeholder: 'e.g. RTU-1 Supply Air' },
                    { name: 'systemType', label: 'System Type', type: 'select', default: 'supply', options: [
                        'supply', 'return', 'exhaust',
                    ]},
                ]);
                if (!newSys || !newSys.tag) return;
                const sysResult = await invoke('create_system', {
                    tag: newSys.tag,
                    name: newSys.name || newSys.tag,
                    systemType: newSys.systemType || 'supply',
                    medium: 'air',
                    sourceId: null,
                });
                await reloadAfterMutation();
                ductDraw.systemId = sysResult.id;
            } else {
                ductDraw.systemId = result.systemId;
            }
        }
    }

    if (ductDraw.nodes.length === 0) {
        const diamResult = await showDialog('Duct Size', [
            { name: 'diameter', label: 'Diameter (inches)', type: 'number', default: '8', placeholder: 'e.g. 8, 12, 14' },
        ]);
        if (!diamResult || !diamResult.diameter) return;
        ductDraw.diameterInches = parseFloat(diamResult.diameter);
        ductDraw.diameter = String(ductDraw.diameterInches);
    }

    const diamInches = ductDraw.diameterInches;
    const diamM = diamInches * 0.0254;

    const nodeResult = await invoke('create_node', {
        systemId: ductDraw.systemId,
        nodeType: 'junction',
        level: state.currentLevel,
        x: wx,
        y: wy,
        fittingType: null,
        sizeDescription: `${diamInches}"`,
        placementId: null,
    });

    if (ductDraw.nodes.length > 0) {
        const prevNodeId = ductDraw.nodes[ductDraw.nodes.length - 1];
        await invoke('create_segment', {
            systemId: ductDraw.systemId,
            fromNodeId: prevNodeId,
            toNodeId: nodeResult.id,
            shape: 'round',
            diameterM: diamM,
            widthM: null,
            heightM: null,
            flowDesign: null,
        });
    }

    ductDraw.nodes.push(nodeResult.id);
    ductDraw.active = true;
    await reloadAfterMutation();
}

function finishDuct() {
    ductDraw.nodes = [];
    ductDraw.systemId = null;
    ductDraw.active = false;
    ductDraw.diameter = null;
    ductDraw.diameterInches = 0;
    state.dirty = true;
}

// =============================================================================
// RENDER LOOP
// =============================================================================
function startRenderLoop() {
    function frame() {
        if (state.dirty) {
            render();
            state.dirty = false;
        }
        animFrameId = requestAnimationFrame(frame);
    }
    frame();
}

function computeSheetTransform() {
    // Drawing area within the sheet (mm) — matches export.rs
    // In export.rs (PDF, Y up): draw_left=B+15, draw_bottom=B+TB_H+15, draw_right=W-B-15, draw_top=H-B-15
    // On canvas (Y down): we map PDF Y to canvas Y by: canvas_y = SHEET.H - pdf_y
    const B = SHEET.BORDER;
    const sf = sheetXform;
    sf.drawLeft = B + 15;                           // same as PDF
    sf.drawRight = SHEET.W - B - 15;                // same as PDF
    sf.drawTop = B + 15;                             // canvas top = SHEET.H - (H-B-15) = B+15
    sf.drawBottom = SHEET.H - B - SHEET.TB_H - 15;  // canvas bottom = SHEET.H - (B+TB_H+15)
    sf.drawW = sf.drawRight - sf.drawLeft;
    sf.drawH = sf.drawBottom - sf.drawTop;

    // Gather model bounds
    let xMin = Infinity, yMin = Infinity, xMax = -Infinity, yMax = -Infinity;
    Object.values(state.roomGeometry).forEach(r => {
        if (r.vertices) r.vertices.forEach(v => {
            xMin = Math.min(xMin, v.x); yMin = Math.min(yMin, v.y);
            xMax = Math.max(xMax, v.x); yMax = Math.max(yMax, v.y);
        });
    });
    state.placements.filter(p => p.level === state.currentLevel).forEach(p => {
        const px = parseFloat(p.x), py = parseFloat(p.y);
        if (!isNaN(px) && !isNaN(py)) {
            xMin = Math.min(xMin, px); yMin = Math.min(yMin, py);
            xMax = Math.max(xMax, px); yMax = Math.max(yMax, py);
        }
    });
    (state.graph.nodes || []).forEach(n => {
        const nx = parseFloat(n.x), ny = parseFloat(n.y);
        if (!isNaN(nx) && !isNaN(ny)) {
            xMin = Math.min(xMin, nx); yMin = Math.min(yMin, ny);
            xMax = Math.max(xMax, nx); yMax = Math.max(yMax, ny);
        }
    });
    if (xMin === Infinity) { xMin = 0; yMin = 0; xMax = 20; yMax = 20; }

    // Padding in model meters
    const pad = 1.0;
    xMin -= pad; yMin -= pad; xMax += pad; yMax += pad;

    sf.modelXMin = xMin; sf.modelYMin = yMin;
    sf.modelXMax = xMax; sf.modelYMax = yMax;

    const modelW = xMax - xMin;
    const modelH = yMax - yMin;

    // Scale: mm per meter. Match export.rs logic:
    // scale = min(drawW / (modelW * 1000), drawH / (modelH * 1000)) * 1000
    // Simplifies to: min(drawW / modelW, drawH / modelH) in mm/m
    // But export.rs multiplies model in meters * 1000 to get mm, then fits.
    // Actually looking at export.rs: model_w is in meters (f32), scale computed as:
    //   scale = min(draw_w / (model_w * 1000.0), draw_h / (model_h * 1000.0)) * 1000.0
    // Which gives: scale = min(draw_w / model_w, draw_h / model_h)  (mm per meter)
    // Then tx(x) = draw_left + (x - x_off) * scale
    sf.scale = Math.min(sf.drawW / modelW, sf.drawH / modelH);
    sf.xOff = xMin;
    sf.yOff = yMin;
}

function render() {
    const ctx = canvas.ctx;
    if (!ctx) return;
    const W = canvas.width, H = canvas.height;

    // Dark background
    ctx.fillStyle = '#1a1a1e';
    ctx.fillRect(0, 0, W, H);

    // Compute model-to-sheet transform
    computeSheetTransform();

    ctx.save();
    ctx.translate(view.x, view.y);
    ctx.scale(view.scale, view.scale);

    // White sheet
    ctx.fillStyle = '#ffffff';
    ctx.fillRect(0, 0, SHEET.W, SHEET.H);

    drawSheetBorder(ctx);
    drawTitleBlock(ctx);
    drawScaleBar(ctx);
    drawNorthArrow(ctx);
    drawSheetRooms(ctx);
    drawSheetSegments(ctx);
    drawSheetNodes(ctx);
    drawSheetPlacements(ctx);
    drawRoomDrawPreview(ctx);
    drawDuctDrawPreview(ctx);

    ctx.restore();
}

// -------------------------------------------------------------------------
// SHEET BORDER (double line, matches export.rs)
// -------------------------------------------------------------------------
function drawSheetBorder(ctx) {
    const B = SHEET.BORDER;

    // Outer border
    ctx.strokeStyle = '#000000';
    ctx.lineWidth = 0.75;
    ctx.strokeRect(B, B, SHEET.W - 2 * B, SHEET.H - 2 * B);

    // Inner border
    ctx.lineWidth = 0.25;
    ctx.strokeRect(B + 2, B + 2, SHEET.W - 2 * B - 4, SHEET.H - 2 * B - 4);
}

// -------------------------------------------------------------------------
// TITLE BLOCK (bottom right, matches export.rs)
// -------------------------------------------------------------------------
function drawTitleBlock(ctx) {
    const B = SHEET.BORDER;
    const tbX = SHEET.W - B - SHEET.TB_W;
    const tbY = SHEET.H - B - SHEET.TB_H - 2;  // near bottom of sheet (high Y = bottom on screen)

    // Title block border
    ctx.strokeStyle = '#000000';
    ctx.lineWidth = 0.5;
    ctx.strokeRect(tbX, tbY, SHEET.TB_W - 2, SHEET.TB_H);

    // Horizontal dividers
    const rowH = 15;
    const rows = [tbY + rowH, tbY + rowH * 2, tbY + rowH * 3, tbY + rowH * 4];
    ctx.lineWidth = 0.15;
    rows.forEach(ry => {
        ctx.beginPath();
        ctx.moveTo(tbX, ry);
        ctx.lineTo(tbX + SHEET.TB_W - 2, ry);
        ctx.stroke();
    });

    // Vertical divider
    const labelW = 45;
    ctx.beginPath();
    ctx.moveTo(tbX + labelW, tbY);
    ctx.lineTo(tbX + labelW, tbY + SHEET.TB_H);
    ctx.stroke();

    // Text sizes in mm
    const small = 5, medium = 7, large = 10;

    // Row 1: Project name
    ctx.fillStyle = '#808080';
    ctx.font = `${small}px Helvetica, Arial, sans-serif`;
    ctx.textAlign = 'left';
    ctx.textBaseline = 'top';
    ctx.fillText('PROJECT', tbX + 3, tbY + 3);
    ctx.fillStyle = '#000000';
    ctx.font = `bold ${large}px Helvetica, Arial, sans-serif`;
    ctx.fillText(state.info.project_name || 'Untitled', tbX + labelW + 3, tbY + 3);

    // Row 2: Project number
    ctx.fillStyle = '#808080';
    ctx.font = `${small}px Helvetica, Arial, sans-serif`;
    ctx.fillText('PROJECT NO.', tbX + 3, rows[0] + 3);
    ctx.fillStyle = '#000000';
    ctx.font = `${medium}px Helvetica, Arial, sans-serif`;
    ctx.fillText(state.info.project_number || '', tbX + labelW + 3, rows[0] + 4);

    // Row 3: Sheet title
    ctx.fillStyle = '#808080';
    ctx.font = `${small}px Helvetica, Arial, sans-serif`;
    ctx.fillText('SHEET TITLE', tbX + 3, rows[1] + 3);
    ctx.fillStyle = '#000000';
    ctx.font = `bold ${medium}px Helvetica, Arial, sans-serif`;
    ctx.fillText(state.currentLevel, tbX + labelW + 3, rows[1] + 4);

    // Row 4: Sheet number
    ctx.fillStyle = '#808080';
    ctx.font = `${small}px Helvetica, Arial, sans-serif`;
    ctx.fillText('SHEET NO.', tbX + 3, rows[2] + 3);
    ctx.fillStyle = '#000000';
    ctx.font = `bold ${large}px Helvetica, Arial, sans-serif`;
    const sheetIdx = state.levels.indexOf(state.currentLevel);
    const sheetNum = 'M-' + String(101 + (sheetIdx >= 0 ? sheetIdx : 0));
    ctx.fillText(sheetNum, tbX + labelW + 3, rows[2] + 3);

    // Row 5: Generated
    ctx.fillStyle = '#808080';
    ctx.font = `${small}px Helvetica, Arial, sans-serif`;
    ctx.fillText('GENERATED', tbX + 3, rows[3] + 3);
    ctx.fillStyle = '#000000';
    ctx.fillText('From .sed file \u2014 Structured Engineering Document', tbX + labelW + 3, rows[3] + 3);

    ctx.textBaseline = 'alphabetic';
}

// -------------------------------------------------------------------------
// SCALE BAR (bottom left of drawing area, matches export.rs)
// -------------------------------------------------------------------------
function drawScaleBar(ctx) {
    const sf = sheetXform;
    const scaleM = 1.0 / sf.scale; // meters per mm
    const barLenM = Math.ceil(5.0 * scaleM);
    const barLenMm = barLenM / scaleM;
    const barX = sf.drawLeft;
    const barY = SHEET.H - SHEET.BORDER - SHEET.TB_H - 2 - 8; // above title block

    ctx.strokeStyle = '#000000';
    ctx.lineWidth = 0.5;
    ctx.beginPath();
    ctx.moveTo(barX, barY);
    ctx.lineTo(barX + barLenMm, barY);
    ctx.stroke();

    // Tick marks
    for (let i = 0; i <= barLenM; i++) {
        const tickX = barX + (i / barLenM) * barLenMm;
        ctx.beginPath();
        ctx.moveTo(tickX, barY - 1.5);
        ctx.lineTo(tickX, barY + 1.5);
        ctx.stroke();
    }

    ctx.fillStyle = '#4d4d4d';
    ctx.font = '5px Helvetica, Arial, sans-serif';
    ctx.textAlign = 'left';
    ctx.textBaseline = 'bottom';
    ctx.fillText('0', barX - 1, barY - 2);
    ctx.fillText(barLenM + 'm', barX + barLenMm - 3, barY - 2);
    ctx.textBaseline = 'alphabetic';
}

// -------------------------------------------------------------------------
// NORTH ARROW (top right of drawing area, matches export.rs)
// -------------------------------------------------------------------------
function drawNorthArrow(ctx) {
    const sf = sheetXform;
    const naX = sf.drawRight - 15;
    const naY = sf.drawTop + 15;

    ctx.fillStyle = '#000000';
    ctx.strokeStyle = '#000000';
    ctx.lineWidth = 0.3;

    // Arrow pointing up (negative Y on screen = up on sheet)
    ctx.beginPath();
    ctx.moveTo(naX, naY - 10);      // tip (top)
    ctx.lineTo(naX + 3, naY);        // bottom right
    ctx.lineTo(naX, naY - 3);        // notch
    ctx.lineTo(naX - 3, naY);        // bottom left
    ctx.closePath();
    ctx.fill();

    ctx.font = 'bold 7px Helvetica, Arial, sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'bottom';
    ctx.fillText('N', naX, naY - 11);
    ctx.textBaseline = 'alphabetic';
    ctx.textAlign = 'left';
}

// -------------------------------------------------------------------------
// DRAW ROOMS on sheet
// -------------------------------------------------------------------------
function drawSheetRooms(ctx) {
    const ls = state.spaces.filter(s => s.level === state.currentLevel);

    ls.forEach(s => {
        const geom = state.roomGeometry[s.tag];
        if (!geom || !geom.vertices || !geom.vertices.length) return;
        const verts = geom.vertices;
        const isNic = s.scope === 'nic';
        const isSel = state.selectedElement && state.selectedElement.id === s.id;

        // Convert all vertices to sheet mm
        const sv = verts.map(v => modelToSheet(v.x, v.y));

        // Selection highlight fill
        if (isSel) {
            ctx.fillStyle = '#d0e4ff40';
            ctx.beginPath();
            ctx.moveTo(sv[0].x, sv[0].y);
            for (let i = 1; i < sv.length; i++) ctx.lineTo(sv[i].x, sv[i].y);
            ctx.closePath();
            ctx.fill();
        }

        // Outline
        const color = isNic ? '#b0b0b0' : '#000000';
        ctx.strokeStyle = isSel ? '#2060c0' : color;
        ctx.lineWidth = isSel ? 0.6 : (isNic ? 0.15 : 0.35);
        ctx.beginPath();
        ctx.moveTo(sv[0].x, sv[0].y);
        for (let i = 1; i < sv.length; i++) ctx.lineTo(sv[i].x, sv[i].y);
        ctx.closePath();
        ctx.stroke();

        // Room label at centroid
        const cx = sv.reduce((s, v) => s + v.x, 0) / sv.length;
        const cy = sv.reduce((s, v) => s + v.y, 0) / sv.length;

        ctx.fillStyle = '#4d4d4d';
        ctx.font = 'bold 5px Helvetica, Arial, sans-serif';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText(s.tag, cx, cy - 3);

        ctx.fillStyle = '#000000';
        ctx.font = '4px Helvetica, Arial, sans-serif';
        ctx.fillText(s.name, cx, cy + 3);
        ctx.textAlign = 'left';
        ctx.textBaseline = 'alphabetic';
    });
}

// -------------------------------------------------------------------------
// DRAW DUCT SEGMENTS on sheet
// -------------------------------------------------------------------------
function drawSheetSegments(ctx) {
    const segs = state.graph.segments || [];
    const sf = sheetXform;

    segs.forEach(seg => {
        const mx1 = parseFloat(seg.x1), my1 = parseFloat(seg.y1);
        const mx2 = parseFloat(seg.x2), my2 = parseFloat(seg.y2);
        if (isNaN(mx1) || isNaN(my1) || isNaN(mx2) || isNaN(my2)) return;

        const s1 = modelToSheet(mx1, my1);
        const s2 = modelToSheet(mx2, my2);

        const diam = parseFloat(seg.diameter_m) || 0.2;
        // Line width proportional to diameter, in sheet-mm
        const lineW = Math.max(diam * sf.scale * 0.8, 0.2);

        ctx.strokeStyle = '#666666';
        ctx.lineWidth = lineW;
        ctx.lineCap = 'round';
        ctx.beginPath();
        ctx.moveTo(s1.x, s1.y);
        ctx.lineTo(s2.x, s2.y);
        ctx.stroke();

        // Flow direction arrow
        const smx = (s1.x + s2.x) / 2;
        const smy = (s1.y + s2.y) / 2;
        const angle = Math.atan2(s2.y - s1.y, s2.x - s1.x);
        const arrowLen = Math.max(lineW * 2, 2);
        ctx.fillStyle = '#666666';
        ctx.beginPath();
        ctx.moveTo(smx + Math.cos(angle) * arrowLen, smy + Math.sin(angle) * arrowLen);
        ctx.lineTo(smx + Math.cos(angle + 2.5) * arrowLen * 0.5, smy + Math.sin(angle + 2.5) * arrowLen * 0.5);
        ctx.lineTo(smx + Math.cos(angle - 2.5) * arrowLen * 0.5, smy + Math.sin(angle - 2.5) * arrowLen * 0.5);
        ctx.closePath();
        ctx.fill();
    });
}

// -------------------------------------------------------------------------
// DRAW NODES on sheet
// -------------------------------------------------------------------------
function drawSheetNodes(ctx) {
    const nodes = state.graph.nodes || [];
    nodes.forEach(n => {
        const nx = parseFloat(n.x), ny = parseFloat(n.y);
        if (isNaN(nx) || isNaN(ny)) return;

        const sn = modelToSheet(nx, ny);
        const isSel = state.selectedElement && state.selectedElement.id === n.id;
        const size = 0.8; // mm

        if (isSel) {
            ctx.strokeStyle = '#2060c0';
            ctx.lineWidth = 0.4;
            ctx.strokeRect(sn.x - size * 1.5, sn.y - size * 1.5, size * 3, size * 3);
        }

        ctx.fillStyle = '#888888';
        ctx.fillRect(sn.x - size, sn.y - size, size * 2, size * 2);
    });
}

// -------------------------------------------------------------------------
// DRAW PLACEMENTS on sheet (octagons for devices, diamonds for equipment)
// -------------------------------------------------------------------------
function drawSheetPlacements(ctx) {
    const lp = state.placements.filter(p => p.level === state.currentLevel);

    lp.forEach(p => {
        const px = parseFloat(p.x), py = parseFloat(p.y);
        if (isNaN(px) || isNaN(py)) return;

        const sp = modelToSheet(px, py);
        const dom = (p.domain || '').toLowerCase();
        const cat = (p.category || '').toLowerCase();
        const cfmVal = parseFloat(p.cfm) || 0;

        // Radius in sheet mm
        const r = dom === 'equipment' ? 2.5 : 1.5;

        const isSel = state.selectedElement && state.selectedElement.id === p.id;

        // Color matching export.rs greyscale logic
        let color;
        if (dom === 'equipment') {
            color = '#000000';
        } else if (dom === 'accessory') {
            color = '#4d4d4d';
        } else if (cat.includes('return') || cat.includes('exhaust')) {
            color = '#333333';
        } else {
            color = '#000000';
        }

        ctx.strokeStyle = isSel ? '#2060c0' : color;
        ctx.lineWidth = isSel ? 0.5 : 0.3;

        if (dom === 'equipment') {
            // Diamond
            ctx.beginPath();
            ctx.moveTo(sp.x, sp.y - r);
            ctx.lineTo(sp.x + r, sp.y);
            ctx.lineTo(sp.x, sp.y + r);
            ctx.lineTo(sp.x - r, sp.y);
            ctx.closePath();
            ctx.stroke();
        } else {
            // Octagon (8-sided circle approximation, matching export.rs)
            const n = 8;
            ctx.beginPath();
            for (let i = 0; i < n; i++) {
                const angle = Math.PI * 2 * i / n;
                const ox = sp.x + r * Math.cos(angle);
                const oy = sp.y + r * Math.sin(angle);
                if (i === 0) ctx.moveTo(ox, oy);
                else ctx.lineTo(ox, oy);
            }
            ctx.closePath();
            ctx.stroke();
        }

        // Selection ring
        if (isSel) {
            ctx.strokeStyle = '#2060c0';
            ctx.lineWidth = 0.3;
            ctx.setLineDash([1, 1]);
            ctx.beginPath();
            ctx.arc(sp.x, sp.y, r + 1.5, 0, Math.PI * 2);
            ctx.stroke();
            ctx.setLineDash([]);
        }

        // Tag + CFM label
        ctx.fillStyle = '#333333';
        ctx.font = '3.5px Helvetica, Arial, sans-serif';
        ctx.textAlign = 'left';
        ctx.textBaseline = 'middle';
        const tag = p.tag || '';
        if (cfmVal > 0) {
            ctx.fillText(tag, sp.x + r + 1, sp.y - 1.5);
            ctx.fillText(Math.round(cfmVal) + ' CFM', sp.x + r + 1, sp.y + 2);
        } else {
            ctx.fillText(tag, sp.x + r + 1, sp.y);
        }
        ctx.textBaseline = 'alphabetic';
    });
}

// -------------------------------------------------------------------------
// DRAW ROOM PREVIEW (during room-draw tool) — in sheet coordinates
// -------------------------------------------------------------------------
function drawRoomDrawPreview(ctx) {
    if (!roomDraw.active || roomDraw.vertices.length === 0) return;

    const sv = roomDraw.vertices.map(v => modelToSheet(v.x, v.y));
    const sm = modelToSheet(mouse.worldX, mouse.worldY);

    ctx.strokeStyle = '#2060c0';
    ctx.lineWidth = 0.4;
    ctx.setLineDash([2, 2]);
    ctx.beginPath();
    ctx.moveTo(sv[0].x, sv[0].y);
    for (let i = 1; i < sv.length; i++) ctx.lineTo(sv[i].x, sv[i].y);
    ctx.lineTo(sm.x, sm.y);
    ctx.stroke();
    ctx.setLineDash([]);

    sv.forEach(v => {
        ctx.fillStyle = '#2060c0';
        ctx.beginPath();
        ctx.arc(v.x, v.y, 0.8, 0, Math.PI * 2);
        ctx.fill();
    });

    if (sv.length >= 3) {
        ctx.fillStyle = '#2060c018';
        ctx.beginPath();
        ctx.moveTo(sv[0].x, sv[0].y);
        for (let i = 1; i < sv.length; i++) ctx.lineTo(sv[i].x, sv[i].y);
        ctx.closePath();
        ctx.fill();
    }
}

// -------------------------------------------------------------------------
// DRAW DUCT PREVIEW (during duct-draw tool) — in sheet coordinates
// -------------------------------------------------------------------------
function drawDuctDrawPreview(ctx) {
    if (!ductDraw.active || ductDraw.nodes.length === 0) return;
    const lastNode = (state.graph.nodes || []).find(n => n.id === ductDraw.nodes[ductDraw.nodes.length - 1]);
    if (!lastNode) return;

    const lx = parseFloat(lastNode.x), ly = parseFloat(lastNode.y);
    if (isNaN(lx) || isNaN(ly)) return;

    const s1 = modelToSheet(lx, ly);
    const s2 = modelToSheet(mouse.worldX, mouse.worldY);

    ctx.strokeStyle = '#2060c080';
    ctx.lineWidth = 0.4;
    ctx.setLineDash([2, 2]);
    ctx.beginPath();
    ctx.moveTo(s1.x, s1.y);
    ctx.lineTo(s2.x, s2.y);
    ctx.stroke();
    ctx.setLineDash([]);
}

// =============================================================================
// TOOLBAR
// =============================================================================
document.querySelectorAll('[data-tool]').forEach(btn => {
    btn.addEventListener('click', () => setTool(btn.dataset.tool));
});

function setTool(toolName) {
    cancelActiveTool();
    state.activeTool = toolName;
    document.querySelectorAll('[data-tool]').forEach(b => b.classList.remove('active'));
    const active = document.querySelector(`[data-tool="${toolName}"]`);
    if (active) active.classList.add('active');
    canvas.el.style.cursor = getCursorForTool();
    document.getElementById('st-tool').textContent =
        ({ select: 'Select', room: 'Draw Room', equip: 'Place Equipment', duct: 'Route Duct' })[toolName] || toolName;
}

function cancelActiveTool() {
    if (roomDraw.active) {
        roomDraw.vertices = [];
        roomDraw.active = false;
    }
    if (ductDraw.active) {
        finishDuct();
    }
    document.getElementById('equip-picker').classList.add('hidden');
    state.dirty = true;
}

document.getElementById('btn-fit').addEventListener('click', () => { fitView(); updateZoomDisplay(); });

document.getElementById('btn-sql').addEventListener('click', () => {
    document.getElementById('query-panel').classList.toggle('open');
    setTimeout(() => { resizeCanvas(); state.dirty = true; }, 20);
});

document.getElementById('btn-undo').addEventListener('click', async () => {
    await invoke('undo');
    await reloadAfterMutation();
});

document.getElementById('btn-redo').addEventListener('click', async () => {
    await invoke('redo');
    await reloadAfterMutation();
});

// =============================================================================
// SQL QUERY PANEL
// =============================================================================
document.getElementById('query-input').addEventListener('keydown', async e => {
    if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        const sql = e.target.value.trim();
        if (!sql) return;
        const rd = document.getElementById('query-result');
        try {
            const res = await invoke('query', { sql });
            if (res.error) {
                rd.textContent = 'Error: ' + res.error;
            } else if (Array.isArray(res) && res.length > 0) {
                const keys = Object.keys(res[0]);
                let out = keys.join('\t') + '\n' + keys.map(k => '-'.repeat(k.length)).join('\t') + '\n';
                res.forEach(row => { out += keys.map(k => row[k] ?? '').join('\t') + '\n'; });
                rd.textContent = out + `\n(${res.length} rows)`;
            } else {
                rd.textContent = '(no results)';
            }
        } catch (err) {
            rd.textContent = 'Error: ' + err;
        }
    }
});

// =============================================================================
// KEYBOARD SHORTCUTS
// =============================================================================
document.addEventListener('keydown', async e => {
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA' || e.target.tagName === 'SELECT') return;

    if (e.ctrlKey && e.key === 'z') {
        e.preventDefault();
        await invoke('undo');
        await reloadAfterMutation();
        return;
    }
    if (e.ctrlKey && e.key === 'y') {
        e.preventDefault();
        await invoke('redo');
        await reloadAfterMutation();
        return;
    }

    if (e.key === 'Escape') {
        if (roomDraw.active && roomDraw.vertices.length > 0) {
            roomDraw.vertices = [];
            roomDraw.active = false;
            state.dirty = true;
        } else if (ductDraw.active) {
            finishDuct();
        } else {
            closeProps();
            setTool('select');
        }
        document.getElementById('equip-picker').classList.add('hidden');
        return;
    }

    if (e.key === 'Delete' || e.key === 'Backspace') {
        if (state.selectedElement && state.selectedTable) {
            await deleteSelected();
        }
        return;
    }

    if (e.key === 'Enter' && state.activeTool === 'room' && roomDraw.active) {
        await finishRoom();
        return;
    }

    if (e.key === 'Enter' && state.activeTool === 'duct' && ductDraw.active) {
        finishDuct();
        return;
    }

    switch (e.key.toLowerCase()) {
        case 's': setTool('select'); break;
        case 'r': setTool('room'); break;
        case 'e': setTool('equip'); break;
        case 'd': setTool('duct'); break;
    }
});

// =============================================================================
// DELETE
// =============================================================================
async function deleteSelected() {
    if (!state.selectedElement || !state.selectedTable) return;

    const deleteTable = state.selectedTable;
    const deleteId = state.selectedElement.id;

    if (deleteTable === 'nodes' || deleteTable === 'systems' || deleteTable === 'product_types') {
        return;
    }

    await invoke('delete_element', { table: deleteTable, id: deleteId });
    closeProps();
    await reloadAfterMutation();
}

// =============================================================================
// UPDATE FIELD
// =============================================================================
async function updateField(table, id, field, value) {
    await invoke('update_element', { table, id, field, value: value || null });
    await reloadAfterMutation();
    if (state.selectedElement && state.selectedElement.id === id) {
        const updated = (table === 'spaces' ? state.spaces : state.placements).find(e => e.id === id);
        if (updated) selectElement(updated, table);
    }
}

function selectPlacementById(id) {
    const p = state.placements.find(x => x.id === id);
    if (p) selectElement(p, 'placements');
}

// =============================================================================
// HELPERS
// =============================================================================
function el(tag, cls) {
    const e = document.createElement(tag);
    if (cls) e.className = cls;
    return e;
}

function elWithText(tag, cls, text) {
    const e = el(tag, cls);
    e.textContent = text;
    return e;
}

function nv(v) {
    return (v && v !== 'NULL') ? v : '-';
}

function esc(s) {
    if (s == null) return '';
    return String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

// =============================================================================
// MODAL DIALOG
// =============================================================================
function showDialog(title, fields) {
    return new Promise((resolve) => {
        const overlay = document.getElementById('input-dialog');
        const titleEl = document.getElementById('input-dialog-title');
        const fieldsEl = document.getElementById('input-dialog-fields');
        const okBtn = document.getElementById('input-dialog-ok');
        const cancelBtn = document.getElementById('input-dialog-cancel');

        titleEl.textContent = title;
        fieldsEl.innerHTML = '';

        fields.forEach(f => {
            const label = document.createElement('label');
            label.textContent = f.label;
            fieldsEl.appendChild(label);

            let input;
            if (f.type === 'select') {
                input = document.createElement('select');
                (f.options || []).forEach(opt => {
                    const o = document.createElement('option');
                    if (typeof opt === 'object') {
                        o.value = opt.value;
                        o.textContent = opt.label;
                    } else {
                        o.value = opt;
                        o.textContent = opt;
                    }
                    input.appendChild(o);
                });
                if (f.default != null) input.value = f.default;
            } else {
                input = document.createElement('input');
                input.type = f.type || 'text';
                if (f.default != null) input.value = f.default;
                if (f.placeholder) input.placeholder = f.placeholder;
            }
            input.dataset.fieldName = f.name;
            fieldsEl.appendChild(input);
        });

        overlay.classList.add('open');

        const firstInput = fieldsEl.querySelector('input, select');
        if (firstInput) firstInput.focus();

        function cleanup() {
            overlay.classList.remove('open');
            okBtn.removeEventListener('click', onOk);
            cancelBtn.removeEventListener('click', onCancel);
            overlay.removeEventListener('keydown', onKeydown);
        }

        function gather() {
            const result = {};
            fieldsEl.querySelectorAll('[data-field-name]').forEach(inp => {
                result[inp.dataset.fieldName] = inp.value;
            });
            return result;
        }

        function onOk() {
            cleanup();
            resolve(gather());
        }

        function onCancel() {
            cleanup();
            resolve(null);
        }

        function onKeydown(e) {
            if (e.key === 'Enter') {
                e.preventDefault();
                onOk();
            } else if (e.key === 'Escape') {
                e.preventDefault();
                onCancel();
            }
        }

        okBtn.addEventListener('click', onOk);
        cancelBtn.addEventListener('click', onCancel);
        overlay.addEventListener('keydown', onKeydown);
    });
}

// =============================================================================
// GLOBAL BINDINGS (for onclick handlers in innerHTML)
// =============================================================================
window.SED = {
    closeProps,
    updateField,
    selectPlacementById,
    deleteSelected,
};
