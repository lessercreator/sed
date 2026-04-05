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
    scale: 40,
    baseScale: 40,
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
    const allVerts = [];
    Object.values(state.roomGeometry).forEach(r => {
        if (r.vertices) r.vertices.forEach(v => allVerts.push(v));
    });
    state.placements.filter(p => p.level === state.currentLevel).forEach(p => {
        const px = parseFloat(p.x), py = parseFloat(p.y);
        if (!isNaN(px) && !isNaN(py)) allVerts.push({ x: px, y: py });
    });
    (state.graph.nodes || []).forEach(n => {
        const nx = parseFloat(n.x), ny = parseFloat(n.y);
        if (!isNaN(nx) && !isNaN(ny)) allVerts.push({ x: nx, y: ny });
    });

    if (allVerts.length === 0) {
        view.x = canvas.width / 2;
        view.y = canvas.height / 2;
        view.scale = 40;
        state.dirty = true;
        return;
    }

    let xMin = Infinity, xMax = -Infinity, yMin = Infinity, yMax = -Infinity;
    allVerts.forEach(v => {
        if (v.x < xMin) xMin = v.x;
        if (v.x > xMax) xMax = v.x;
        if (v.y < yMin) yMin = v.y;
        if (v.y > yMax) yMax = v.y;
    });

    const padding = 60;
    const w = xMax - xMin || 1;
    const h = yMax - yMin || 1;
    const scaleX = (canvas.width - padding * 2) / w;
    const scaleY = (canvas.height - padding * 2) / h;
    view.scale = Math.min(scaleX, scaleY, 200);
    view.x = (canvas.width / 2) - ((xMin + xMax) / 2) * view.scale;
    view.y = (canvas.height / 2) - ((yMin + yMax) / 2) * view.scale;
    state.dirty = true;
}

// =============================================================================
// CANVAS INTERACTION
// =============================================================================
function screenToWorld(sx, sy) {
    const rect = canvas.el.getBoundingClientRect();
    const cx = sx - rect.left;
    const cy = sy - rect.top;
    return {
        x: (cx - view.x) / view.scale,
        y: (cy - view.y) / view.scale,
    };
}

function worldToScreen(wx, wy) {
    return {
        x: wx * view.scale + view.x,
        y: wy * view.scale + view.y,
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
    document.getElementById('st-zoom').textContent = `${(view.scale / view.baseScale * 100).toFixed(0)}%`;
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

function render() {
    const ctx = canvas.ctx;
    if (!ctx) return;
    const W = canvas.width, H = canvas.height;
    ctx.clearRect(0, 0, W, H);
    ctx.save();
    ctx.translate(view.x, view.y);
    ctx.scale(view.scale, view.scale);

    drawGrid(ctx, W, H);
    drawRooms(ctx);
    drawSegments(ctx);
    drawNodes(ctx);
    drawPlacements(ctx);
    drawRoomDrawPreview(ctx);
    drawDuctDrawPreview(ctx);
    drawLevelLabel(ctx);

    ctx.restore();
}

function drawGrid(ctx, W, H) {
    const invScale = 1 / view.scale;
    const x0 = -view.x * invScale;
    const y0 = -view.y * invScale;
    const x1 = x0 + W * invScale;
    const y1 = y0 + H * invScale;

    let step = 1;
    if (view.scale < 10) step = 5;
    if (view.scale < 3) step = 10;

    ctx.strokeStyle = '#1a1a1a';
    ctx.lineWidth = 0.02;
    ctx.beginPath();
    const gx0 = Math.floor(x0 / step) * step;
    const gy0 = Math.floor(y0 / step) * step;
    for (let x = gx0; x <= x1; x += step) {
        ctx.moveTo(x, y0);
        ctx.lineTo(x, y1);
    }
    for (let y = gy0; y <= y1; y += step) {
        ctx.moveTo(x0, y);
        ctx.lineTo(x1, y);
    }
    ctx.stroke();

    if (view.scale > 20) {
        ctx.strokeStyle = '#222';
        ctx.lineWidth = 0.01;
        ctx.beginPath();
        const majorStep = step * 5;
        const mx0 = Math.floor(x0 / majorStep) * majorStep;
        const my0 = Math.floor(y0 / majorStep) * majorStep;
        for (let x = mx0; x <= x1; x += majorStep) {
            ctx.moveTo(x, y0);
            ctx.lineTo(x, y1);
        }
        for (let y = my0; y <= y1; y += majorStep) {
            ctx.moveTo(x0, y);
            ctx.lineTo(x1, y);
        }
        ctx.stroke();
    }
}

function drawRooms(ctx) {
    const ls = state.spaces.filter(s => s.level === state.currentLevel);

    ls.forEach(s => {
        const geom = state.roomGeometry[s.tag];
        if (!geom || !geom.vertices || !geom.vertices.length) return;
        const verts = geom.vertices;
        const isNic = s.scope === 'nic';
        const isSel = state.selectedElement && state.selectedElement.id === s.id;

        ctx.fillStyle = isNic ? '#15151580' : isSel ? '#1a2a3a80' : '#4a9eff08';
        ctx.beginPath();
        ctx.moveTo(verts[0].x, verts[0].y);
        for (let i = 1; i < verts.length; i++) ctx.lineTo(verts[i].x, verts[i].y);
        ctx.closePath();
        ctx.fill();

        ctx.strokeStyle = isSel ? '#4a9eff' : isNic ? '#333333' : '#4a9eff30';
        ctx.lineWidth = isSel ? 0.06 : 0.03;
        ctx.beginPath();
        ctx.moveTo(verts[0].x, verts[0].y);
        for (let i = 1; i < verts.length; i++) ctx.lineTo(verts[i].x, verts[i].y);
        ctx.closePath();
        ctx.stroke();

        const cx = verts.reduce((s, v) => s + v.x, 0) / verts.length;
        const cy = verts.reduce((s, v) => s + v.y, 0) / verts.length;

        ctx.fillStyle = isNic ? '#444' : '#888';
        ctx.font = `${0.25}px system-ui`;
        ctx.textAlign = 'center';
        ctx.fillText(s.tag, cx, cy - 0.1);

        ctx.fillStyle = isNic ? '#555' : '#ccc';
        ctx.font = `bold ${0.3}px system-ui`;
        ctx.fillText(s.name, cx, cy + 0.25);
        ctx.textAlign = 'left';
    });
}

function drawSegments(ctx) {
    const segs = state.graph.segments || [];
    segs.forEach(seg => {
        const x1 = parseFloat(seg.x1), y1 = parseFloat(seg.y1);
        const x2 = parseFloat(seg.x2), y2 = parseFloat(seg.y2);
        if (isNaN(x1) || isNaN(y1) || isNaN(x2) || isNaN(y2)) return;

        const diam = parseFloat(seg.diameter_m) || 0.2;
        const lineW = Math.max(diam * 0.8, 0.04);

        const sysTag = seg.system_tag || '';
        if (sysTag.toLowerCase().includes('exhaust') || sysTag.toLowerCase().includes('ex')) {
            ctx.strokeStyle = '#f4433660';
        } else if (sysTag.toLowerCase().includes('return') || sysTag.toLowerCase().includes('ra')) {
            ctx.strokeStyle = '#4caf5060';
        } else {
            ctx.strokeStyle = '#66666680';
        }
        ctx.lineWidth = lineW;
        ctx.lineCap = 'round';
        ctx.beginPath();
        ctx.moveTo(x1, y1);
        ctx.lineTo(x2, y2);
        ctx.stroke();

        const mx = (x1 + x2) / 2;
        const my = (y1 + y2) / 2;
        const angle = Math.atan2(y2 - y1, x2 - x1);
        const arrowLen = 0.15;
        ctx.fillStyle = ctx.strokeStyle;
        ctx.beginPath();
        ctx.moveTo(mx + Math.cos(angle) * arrowLen, my + Math.sin(angle) * arrowLen);
        ctx.lineTo(mx + Math.cos(angle + 2.5) * arrowLen * 0.5, my + Math.sin(angle + 2.5) * arrowLen * 0.5);
        ctx.lineTo(mx + Math.cos(angle - 2.5) * arrowLen * 0.5, my + Math.sin(angle - 2.5) * arrowLen * 0.5);
        ctx.closePath();
        ctx.fill();
    });
}

function drawNodes(ctx) {
    const nodes = state.graph.nodes || [];
    nodes.forEach(n => {
        const nx = parseFloat(n.x), ny = parseFloat(n.y);
        if (isNaN(nx) || isNaN(ny)) return;

        const isSel = state.selectedElement && state.selectedElement.id === n.id;
        const size = 0.08;

        if (isSel) {
            ctx.strokeStyle = '#ffffff';
            ctx.lineWidth = 0.04;
            ctx.strokeRect(nx - size * 1.5, ny - size * 1.5, size * 3, size * 3);
        }

        ctx.fillStyle = '#888';
        ctx.fillRect(nx - size, ny - size, size * 2, size * 2);
    });
}

function drawPlacements(ctx) {
    const lp = state.placements.filter(p => p.level === state.currentLevel);

    lp.forEach(p => {
        const px = parseFloat(p.x), py = parseFloat(p.y);
        if (isNaN(px) || isNaN(py)) return;

        let color = '#4a9eff';
        const cat = (p.category || '').toLowerCase();
        const dom = (p.domain || '').toLowerCase();
        if (cat.includes('return') || cat.includes('return_grille')) color = '#4caf50';
        if (cat.includes('exhaust')) color = '#f44336';
        if (cat.includes('transfer')) color = '#ff9800';
        if (dom === 'equipment') color = '#e040fb';
        if (dom === 'accessory') color = '#ffeb3b';

        const cfmVal = parseFloat(p.cfm) || 0;
        const baseR = 0.12;
        const r = cfmVal > 0 ? baseR + Math.min(cfmVal / 2000, 0.15) : baseR;

        const isSel = state.selectedElement && state.selectedElement.id === p.id;

        if (isSel) {
            ctx.strokeStyle = '#ffffff';
            ctx.lineWidth = 0.05;
            ctx.beginPath();
            ctx.arc(px, py, r + 0.08, 0, Math.PI * 2);
            ctx.stroke();
        }

        ctx.fillStyle = color;
        if (dom === 'equipment') {
            ctx.beginPath();
            ctx.moveTo(px, py - r * 1.3);
            ctx.lineTo(px + r * 1.3, py);
            ctx.lineTo(px, py + r * 1.3);
            ctx.lineTo(px - r * 1.3, py);
            ctx.closePath();
            ctx.fill();
        } else {
            ctx.beginPath();
            ctx.arc(px, py, r, 0, Math.PI * 2);
            ctx.fill();
        }

        if (cfmVal > 0) {
            ctx.fillStyle = '#888';
            ctx.font = '0.18px system-ui';
            ctx.textAlign = 'left';
            ctx.fillText(String(Math.round(cfmVal)), px + r + 0.08, py + 0.06);
        }
    });
}

function drawRoomDrawPreview(ctx) {
    if (!roomDraw.active || roomDraw.vertices.length === 0) return;

    ctx.strokeStyle = '#4a9eff';
    ctx.lineWidth = 0.04;
    ctx.setLineDash([0.1, 0.1]);
    ctx.beginPath();
    ctx.moveTo(roomDraw.vertices[0].x, roomDraw.vertices[0].y);
    for (let i = 1; i < roomDraw.vertices.length; i++) {
        ctx.lineTo(roomDraw.vertices[i].x, roomDraw.vertices[i].y);
    }
    ctx.lineTo(mouse.worldX, mouse.worldY);
    ctx.stroke();
    ctx.setLineDash([]);

    roomDraw.vertices.forEach(v => {
        ctx.fillStyle = '#4a9eff';
        ctx.beginPath();
        ctx.arc(v.x, v.y, 0.06, 0, Math.PI * 2);
        ctx.fill();
    });

    if (roomDraw.vertices.length >= 3) {
        ctx.fillStyle = '#4a9eff15';
        ctx.beginPath();
        ctx.moveTo(roomDraw.vertices[0].x, roomDraw.vertices[0].y);
        for (let i = 1; i < roomDraw.vertices.length; i++) {
            ctx.lineTo(roomDraw.vertices[i].x, roomDraw.vertices[i].y);
        }
        ctx.closePath();
        ctx.fill();
    }
}

function drawDuctDrawPreview(ctx) {
    if (!ductDraw.active || ductDraw.nodes.length === 0) return;
    const lastNode = (state.graph.nodes || []).find(n => n.id === ductDraw.nodes[ductDraw.nodes.length - 1]);
    if (!lastNode) return;

    const lx = parseFloat(lastNode.x), ly = parseFloat(lastNode.y);
    if (isNaN(lx) || isNaN(ly)) return;

    ctx.strokeStyle = '#4a9eff80';
    ctx.lineWidth = 0.04;
    ctx.setLineDash([0.1, 0.1]);
    ctx.beginPath();
    ctx.moveTo(lx, ly);
    ctx.lineTo(mouse.worldX, mouse.worldY);
    ctx.stroke();
    ctx.setLineDash([]);
}

function drawLevelLabel(ctx) {
    ctx.fillStyle = '#333';
    ctx.font = 'bold 0.5px system-ui';
    ctx.textAlign = 'left';
    const invScale = 1 / view.scale;
    const x = (-view.x) * invScale + 0.5;
    const y = (-view.y) * invScale + 0.8;
    ctx.fillText(state.currentLevel, x, y);
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
