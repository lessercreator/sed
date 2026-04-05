// SED Editor — Frontend
// Communicates with Rust backend via Tauri invoke()

const { invoke } = window.__TAURI__.core;

// =============================================================================
// STATE
// =============================================================================
let info = {}, spaces = [], placements = [], productTypes = [], notes = [];
let currentLevel = 'Level 1';
let selectedId = null;
let canvas, ctx;
let vx = 0, vy = 0, vscale = 40; // 40 pixels per meter
let dragging = false, dsx = 0, dsy = 0;
let roomGeometry = {}; // tag -> { vertices: [{x,y},...], id, name, scope }

// =============================================================================
// BOOT
// =============================================================================
document.getElementById('btn-example').addEventListener('click', async () => {
    const result = await invoke('create_example', { path: 'skims-americana.sed' });
    info = result;
    await loadDocument();
});

document.getElementById('btn-open').addEventListener('click', async () => {
    // For now, open the example. TODO: file picker dialog
    try {
        const result = await invoke('open_file', { path: 'skims-americana.sed' });
        info = result;
        await loadDocument();
    } catch (e) {
        // File doesn't exist yet, create example
        const result = await invoke('create_example', { path: 'skims-americana.sed' });
        info = result;
        await loadDocument();
    }
});

async function loadDocument() {
    document.getElementById('welcome').style.display = 'none';
    document.getElementById('editor').style.display = 'flex';

    [spaces, placements, productTypes, notes] = await Promise.all([
        invoke('get_spaces'),
        invoke('get_placements'),
        invoke('get_product_types'),
        invoke('get_notes'),
    ]);

    document.getElementById('project-name').textContent = info.project_name;
    document.getElementById('project-meta').textContent = `#${info.project_number} | ${info.placements} placements`;
    document.getElementById('st-file').textContent = info.project_name;
    document.getElementById('st-counts').textContent = `${info.spaces} spaces | ${info.placements} placements`;

    const levels = [...new Set(spaces.map(s => s.level))].sort();
    const sel = document.getElementById('level-select');
    sel.innerHTML = '';
    levels.forEach(l => {
        const o = document.createElement('option');
        o.value = l; o.textContent = l;
        if (l === currentLevel) o.selected = true;
        sel.appendChild(o);
    });
    sel.onchange = async () => { currentLevel = sel.value; await loadRoomGeometry(); render(); };

    buildSidebar();
    setupCanvas();
    await loadRoomGeometry();
    fitView();
    render();
}

async function reload() {
    [info, spaces, placements] = await Promise.all([
        invoke('get_info'),
        invoke('get_spaces'),
        invoke('get_placements'),
    ]);
    document.getElementById('st-counts').textContent = `${info.spaces} spaces | ${info.placements} placements`;
    await loadRoomGeometry();
    buildSidebar();
    render();
}

async function loadRoomGeometry() {
    const geom = await invoke('get_room_geometry', { level: currentLevel });
    roomGeometry = {};
    geom.forEach(r => { roomGeometry[r.tag] = r; });
}

// =============================================================================
// SIDEBAR
// =============================================================================
function buildSidebar() {
    // Spaces
    const sd = document.getElementById('tab-spaces');
    sd.innerHTML = '';
    const levels = [...new Set(spaces.map(s => s.level))].sort();
    levels.forEach(level => {
        const g = el('div', 'group');
        g.innerHTML = `<div class="group-label">${level}</div>`;
        spaces.filter(s => s.level === level).forEach(s => {
            const devs = placements.filter(p => p.space_tag === s.tag);
            const cfm = devs.reduce((sum, p) => sum + (parseFloat(p.cfm) || 0), 0);
            const item = el('div', 'item' + (s.scope === 'nic' ? ' nic' : ''));
            item.innerHTML = `<span class="tag">${s.tag}</span><span class="name">${s.name}</span><span class="val">${cfm > 0 ? cfm + ' CFM' : ''}</span>`;
            item.onclick = () => selectSpace(s);
            g.appendChild(item);
        });
        sd.appendChild(g);
    });

    // Devices
    const dd = document.getElementById('tab-devices');
    dd.innerHTML = '';
    const byType = {};
    placements.forEach(p => {
        if (!byType[p.tag]) byType[p.tag] = { ...p, items: [] };
        byType[p.tag].items.push(p);
    });
    Object.values(byType).sort((a, b) => b.items.length - a.items.length).forEach(g => {
        const cfm = g.items.reduce((sum, p) => sum + (parseFloat(p.cfm) || 0), 0);
        const mfr = g.manufacturer !== 'NULL' ? g.manufacturer : '';
        const mdl = g.model !== 'NULL' ? g.model : '';
        const item = el('div', 'item');
        item.innerHTML = `<span class="tag">${g.tag}</span><span class="name">${g.items.length}x ${mfr} ${mdl}</span><span class="val">${cfm > 0 ? cfm + ' CFM' : ''}</span>`;
        item.onclick = () => showTypeProps(g);
        dd.appendChild(item);
    });

    // Submittals
    const sub = document.getElementById('tab-submittals');
    sub.innerHTML = '';
    invoke('get_submittals').then(subs => {
        subs.forEach(s => {
            const c = s.status === 'approved' ? 'var(--green)' : s.status === 'for_approval' ? 'var(--orange)' : 'var(--red)';
            const item = el('div', 'item');
            item.innerHTML = `<span class="name">${s.description}</span><span class="val" style="color:${c}">${s.status.replace(/_/g, ' ')}</span>`;
            sub.appendChild(item);
        });
    });

    // Notes
    const nd = document.getElementById('tab-notes');
    nd.innerHTML = '';
    notes.forEach(n => {
        const item = el('div', 'item');
        item.style.alignItems = 'flex-start';
        item.innerHTML = `<span class="tag" style="min-width:28px">${n.key}</span><span class="name" style="font-size:11px;line-height:1.4;white-space:normal">${n.text}</span>`;
        nd.appendChild(item);
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
// PROPERTIES
// =============================================================================
function selectSpace(s) {
    selectedId = s.id;
    const panel = document.getElementById('props');
    const devs = placements.filter(p => p.space_tag === s.tag);
    const cfm = devs.reduce((sum, p) => sum + (parseFloat(p.cfm) || 0), 0);

    let h = `<h3>${s.tag} - ${s.name}<span class="close" onclick="closeProps()">&times;</span></h3>`;
    h += rp('Level', s.level);
    h += ep('name', 'Name', s.name, 'spaces', s.id);
    h += ep('space_type', 'Type', s.space_type, 'spaces', s.id);
    h += ep('scope', 'Scope', s.scope, 'spaces', s.id);
    h += rp('Devices', devs.length);
    h += rp('Total CFM', cfm || '-');
    if (devs.length) {
        h += `<div class="section"><div class="section-label">Devices</div>`;
        devs.forEach(d => {
            h += `<div class="pr" style="cursor:pointer" onclick='selectPlacement("${d.id}")'><span class="k" style="color:var(--accent)">${d.tag}</span><span class="v">${nv(d.cfm)} CFM</span></div>`;
        });
        h += '</div>';
    }
    panel.innerHTML = h;
    panel.style.display = 'block';
    currentLevel = s.level;
    document.getElementById('level-select').value = currentLevel;
    render();
}

function selectPlacement(id) {
    const p = placements.find(x => x.id === id);
    if (!p) return;
    selectedId = p.id;
    const panel = document.getElementById('props');
    let h = `<h3>${p.tag}<span class="close" onclick="closeProps()">&times;</span></h3>`;
    h += rp('Category', p.category);
    h += rp('Manufacturer', nv(p.manufacturer));
    h += rp('Model', nv(p.model));
    h += ep('cfm', 'CFM', p.cfm, 'placements', p.id);
    h += ep('status', 'Status', p.status, 'placements', p.id);
    h += ep('phase', 'Phase', p.phase, 'placements', p.id);
    h += rp('Room', nv(p.space_name));
    if (p.notes && p.notes !== 'NULL') {
        h += `<div class="section"><div class="section-label">Notes</div><div style="font-size:11px;color:var(--text2);line-height:1.5">${p.notes}</div></div>`;
    }
    panel.innerHTML = h;
    panel.style.display = 'block';
    render();
}

function showTypeProps(g) {
    const panel = document.getElementById('props');
    const cfm = g.items.reduce((sum, p) => sum + (parseFloat(p.cfm) || 0), 0);
    let h = `<h3>${g.tag} (${g.items.length}x)<span class="close" onclick="closeProps()">&times;</span></h3>`;
    h += rp('Category', g.category);
    h += rp('Manufacturer', nv(g.manufacturer));
    h += rp('Model', nv(g.model));
    h += rp('Total CFM', cfm || '-');
    h += `<div class="section"><div class="section-label">Instances</div>`;
    g.items.forEach(p => {
        h += `<div class="pr" style="cursor:pointer" onclick='selectPlacement("${p.id}")'><span class="k">${nv(p.space_name)}</span><span class="v">${nv(p.cfm)} CFM</span></div>`;
    });
    h += '</div>';
    panel.innerHTML = h;
    panel.style.display = 'block';
}

function closeProps() { document.getElementById('props').style.display = 'none'; selectedId = null; render(); }

function rp(k, v) { return `<div class="pr"><span class="k">${k}</span><span class="v">${v}</span></div>`; }
function ep(field, label, val, table, id) {
    const v = val && val !== 'NULL' ? val : '';
    return `<div class="pr"><span class="k">${label}</span><input value="${v}" onchange="updateField('${table}','${id}','${field}',this.value)"></div>`;
}
function nv(v) { return v && v !== 'NULL' ? v : '-'; }

window.closeProps = closeProps;
window.selectPlacement = selectPlacement;

window.updateField = async function(table, id, field, value) {
    await invoke('update_element', { table, id, field, value: value || null });
    await reload();
};

// =============================================================================
// CANVAS
// =============================================================================
function setupCanvas() {
    canvas = document.getElementById('plan');
    ctx = canvas.getContext('2d');
    resizeCanvas();
    window.addEventListener('resize', () => { resizeCanvas(); render(); });

    canvas.addEventListener('mousedown', e => { dragging = true; dsx = e.clientX - vx; dsy = e.clientY - vy; canvas.style.cursor = 'grabbing'; });
    canvas.addEventListener('mousemove', e => { if (dragging) { vx = e.clientX - dsx; vy = e.clientY - dsy; render(); }});
    canvas.addEventListener('mouseup', () => { dragging = false; canvas.style.cursor = 'grab'; });
    canvas.addEventListener('mouseleave', () => { dragging = false; canvas.style.cursor = 'grab'; });
    canvas.addEventListener('wheel', e => {
        e.preventDefault();
        const f = e.deltaY < 0 ? 1.1 : 0.9;
        const r = canvas.getBoundingClientRect();
        const mx = e.clientX - r.left, my = e.clientY - r.top;
        vx = mx - (mx - vx) * f;
        vy = my - (my - vy) * f;
        vscale *= f;
        render();
        document.getElementById('st-zoom').textContent = `${(vscale / 40 * 100).toFixed(0)}%`;
    });

    // Double-click to select room
    canvas.addEventListener('dblclick', e => {
        const r = canvas.getBoundingClientRect();
        const mx = (e.clientX - r.left - vx) / vscale;
        const my = (e.clientY - r.top - vy) / vscale;
        // Check rooms using polygon geometry
        const ls = spaces.filter(s => s.level === currentLevel);
        for (const s of ls) {
            const geom = roomGeometry[s.tag];
            if (geom && pointInPolygon(mx, my, geom.vertices)) {
                selectSpace(s);
                return;
            }
        }
    });
}

function resizeCanvas() {
    const w = document.getElementById('canvas-wrap');
    canvas.width = w.clientWidth;
    canvas.height = w.clientHeight;
}

function fitView() {
    vscale = 40;
    vx = 80;
    vy = 80;
}

// =============================================================================
// RENDER — draws the plan from real coordinates
// =============================================================================
function render() {
    if (!ctx) return;
    const W = canvas.width, H = canvas.height;
    ctx.clearRect(0, 0, W, H);
    ctx.save();
    ctx.translate(vx, vy);
    ctx.scale(vscale, vscale);

    const ls = spaces.filter(s => s.level === currentLevel);
    const lp = placements.filter(p => p.level === currentLevel);

    // Grid (1m spacing)
    ctx.strokeStyle = '#1a1a1a';
    ctx.lineWidth = 0.02;
    for (let x = -5; x < 25; x++) { ctx.beginPath(); ctx.moveTo(x, -5); ctx.lineTo(x, 25); ctx.stroke(); }
    for (let y = -5; y < 25; y++) { ctx.beginPath(); ctx.moveTo(-5, y); ctx.lineTo(25, y); ctx.stroke(); }

    // Rooms (drawn from polygon vertices)
    ls.forEach(s => {
        const geom = roomGeometry[s.tag];
        if (!geom || !geom.vertices.length) return;
        const verts = geom.vertices;
        const isNic = s.scope === 'nic';
        const isSel = selectedId === s.id;

        // Draw polygon fill
        ctx.fillStyle = isNic ? '#15151580' : isSel ? '#1a2a3a80' : '#1a2a3a30';
        ctx.beginPath();
        ctx.moveTo(verts[0].x, verts[0].y);
        for (let i = 1; i < verts.length; i++) ctx.lineTo(verts[i].x, verts[i].y);
        ctx.closePath();
        ctx.fill();

        // Draw polygon border
        ctx.strokeStyle = isSel ? '#4a9eff' : isNic ? '#333333' : '#4a9eff30';
        ctx.lineWidth = isSel ? 0.06 : 0.03;
        ctx.beginPath();
        ctx.moveTo(verts[0].x, verts[0].y);
        for (let i = 1; i < verts.length; i++) ctx.lineTo(verts[i].x, verts[i].y);
        ctx.closePath();
        ctx.stroke();

        // Room tag (place at first vertex + small offset)
        const rx = verts[0].x, ry = verts[0].y;
        ctx.fillStyle = isNic ? '#444' : '#888';
        ctx.font = `${0.3}px ${getComputedStyle(document.body).fontFamily}`;
        ctx.fillText(s.tag, rx + 0.1, ry + 0.35);

        // Room name
        ctx.fillStyle = isNic ? '#555' : '#ccc';
        ctx.font = `bold ${0.35}px system-ui`;
        ctx.fillText(s.name, rx + 0.1, ry + 0.75);
    });

    // Devices
    lp.forEach(p => {
        const px = parseFloat(p.x), py = parseFloat(p.y);
        if (isNaN(px) || isNaN(py)) return;

        let color = '#4a9eff';
        if (p.category.includes('return')) color = '#4caf50';
        if (p.category.includes('exhaust')) color = '#f44336';
        if (p.category.includes('transfer')) color = '#ff9800';
        if (p.domain === 'equipment') color = '#e040fb';
        if (p.domain === 'accessory') color = '#ffeb3b';

        const isSel = selectedId === p.id;
        const r = 0.15;

        if (isSel) {
            ctx.strokeStyle = '#ffffff';
            ctx.lineWidth = 0.05;
            ctx.beginPath(); ctx.arc(px, py, r + 0.08, 0, Math.PI * 2); ctx.stroke();
        }

        ctx.fillStyle = color;
        if (p.domain === 'equipment') {
            // Diamond
            ctx.beginPath();
            ctx.moveTo(px, py - r * 1.3); ctx.lineTo(px + r * 1.3, py);
            ctx.lineTo(px, py + r * 1.3); ctx.lineTo(px - r * 1.3, py);
            ctx.closePath(); ctx.fill();
        } else {
            ctx.beginPath(); ctx.arc(px, py, r, 0, Math.PI * 2); ctx.fill();
        }

        // CFM label
        if (p.cfm && p.cfm !== 'NULL') {
            ctx.fillStyle = '#888';
            ctx.font = '0.2px system-ui';
            ctx.fillText(p.cfm, px + 0.2, py + 0.05);
        }
    });

    // Title
    ctx.fillStyle = '#444';
    ctx.font = 'bold 0.6px system-ui';
    ctx.fillText(currentLevel, -3, -1);

    // Legend
    const ly = -2;
    const legend = [['#4a9eff','Supply'],['#4caf50','Return'],['#f44336','Exhaust'],['#ff9800','Transfer'],['#e040fb','Equipment']];
    ctx.font = '0.25px system-ui';
    legend.forEach(([c, l], i) => {
        ctx.fillStyle = c;
        ctx.beginPath(); ctx.arc(-3 + i * 2.5, ly, 0.12, 0, Math.PI * 2); ctx.fill();
        ctx.fillStyle = '#888';
        ctx.fillText(l, -3 + i * 2.5 + 0.2, ly + 0.08);
    });

    ctx.restore();
}

// =============================================================================
// QUERY PANEL
// =============================================================================
document.getElementById('btn-sql').addEventListener('click', () => {
    document.getElementById('query-panel').classList.toggle('open');
    setTimeout(resizeCanvas, 10);
    setTimeout(render, 20);
});

document.getElementById('btn-fit').addEventListener('click', () => { fitView(); render(); });

document.getElementById('query-input').addEventListener('keydown', async e => {
    if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        const sql = e.target.value.trim();
        if (!sql) return;
        const rd = document.getElementById('query-result');
        try {
            const res = await invoke('query', { sql });
            if (res.error) { rd.textContent = 'Error: ' + res.error; }
            else if (Array.isArray(res) && res.length > 0) {
                const keys = Object.keys(res[0]);
                let out = keys.join('\t') + '\n' + keys.map(k => '-'.repeat(k.length)).join('\t') + '\n';
                res.forEach(row => { out += keys.map(k => row[k]).join('\t') + '\n'; });
                rd.textContent = out + `\n(${res.length} rows)`;
            } else { rd.textContent = '(no results)'; }
        } catch (err) { rd.textContent = 'Error: ' + err; }
    }
});

// =============================================================================
// KEYBOARD
// =============================================================================
document.addEventListener('keydown', e => {
    if (e.key === 'Escape') closeProps();
});

// =============================================================================
// HELPERS
// =============================================================================
function el(tag, cls) { const e = document.createElement(tag); if (cls) e.className = cls; return e; }

// Ray-casting point-in-polygon test
function pointInPolygon(px, py, verts) {
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
