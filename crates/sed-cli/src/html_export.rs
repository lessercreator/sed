use anyhow::Result;
use sed_sdk::SedDocument;

/// Export a fully self-contained interactive HTML mechanical plan viewer.
///
/// Loads ALL levels from the .sed file, embeds them as JSON, and renders
/// the specified `level` as the initial view. Level switching is instantaneous
/// since all data is already in the page.
pub fn export_html(file: &str, output: &str, level: &str) -> Result<()> {
    let doc = SedDocument::open(file)?;
    let info = doc.info()?;

    // ── Discover all levels with geometry or positioned placements ──
    let level_rows = doc.query_raw(
        "SELECT DISTINCT level FROM (
            SELECT s.level FROM spaces s JOIN geometry_polygons gp ON s.boundary_id = gp.id
            UNION
            SELECT p.level FROM placements p WHERE p.x IS NOT NULL
         ) sub ORDER BY level",
    )?;
    if level_rows.is_empty() {
        anyhow::bail!("No levels with positioned data found");
    }

    // ── Build per-level JSON ──
    let mut levels_json_parts = Vec::new();

    for level_row in &level_rows {
        let lvl = &level_row[0].1;

        let rooms = sed_sdk::geometry::get_room_geometry(&doc, lvl)?;
        let placements = doc.query_params(
            "SELECT pt.tag, pt.category, pt.domain, p.x, p.y, p.cfm,
                    COALESCE(p.instance_tag,'') as itag,
                    COALESCE(s.name,'') as room,
                    pt.manufacturer, pt.model
             FROM placements p
             JOIN product_types pt ON p.product_type_id = pt.id
             LEFT JOIN spaces s ON p.space_id = s.id
             WHERE p.level = ?1 AND p.x IS NOT NULL
             ORDER BY pt.tag",
            &[&lvl as &dyn rusqlite::types::ToSql],
        )?;
        let segments = doc.query_params(
            "SELECT n1.x as x1, n1.y as y1, n2.x as x2, n2.y as y2, seg.diameter_m
             FROM segments seg
             JOIN nodes n1 ON seg.from_node_id = n1.id
             JOIN nodes n2 ON seg.to_node_id = n2.id
             WHERE n1.level = ?1 AND n1.x IS NOT NULL AND n2.x IS NOT NULL",
            &[&lvl as &dyn rusqlite::types::ToSql],
        )?;
        let nodes = doc.query_params(
            "SELECT n.x, n.y, n.node_type,
                    COALESCE(n.fitting_type,'') as ft,
                    COALESCE(n.size_description,'') as sd
             FROM nodes n
             JOIN systems sys ON n.system_id = sys.id
             WHERE n.level = ?1 AND n.x IS NOT NULL",
            &[&lvl as &dyn rusqlite::types::ToSql],
        )?;

        // Rooms JSON
        let mut rooms_json = Vec::new();
        for r in &rooms {
            let verts: Vec<String> = r
                .vertices
                .iter()
                .map(|v| format!("{{\"x\":{},\"y\":{}}}", v.x, v.y))
                .collect();
            rooms_json.push(format!(
                "{{\"tag\":\"{}\",\"name\":\"{}\",\"scope\":\"{}\",\"vertices\":[{}]}}",
                esc(&r.tag),
                esc(&r.name),
                esc(&r.scope),
                verts.join(",")
            ));
        }

        // Placements JSON
        let mut place_json = Vec::new();
        for p in &placements {
            place_json.push(format!(
                "{{\"tag\":\"{}\",\"cat\":\"{}\",\"dom\":\"{}\",\"x\":{},\"y\":{},\"cfm\":{},\"itag\":\"{}\",\"room\":\"{}\",\"mfr\":\"{}\",\"model\":\"{}\"}}",
                esc(&p[0].1), esc(&p[1].1), esc(&p[2].1),
                &p[3].1, &p[4].1,
                if p[5].1 == "NULL" { "null" } else { &p[5].1 },
                esc(&p[6].1), esc(&p[7].1), esc(&p[8].1), esc(&p[9].1)
            ));
        }

        // Segments JSON (diameter converted from metres to inches)
        let mut seg_json = Vec::new();
        for s in &segments {
            let d: f64 = s[4].1.parse().unwrap_or(0.2);
            seg_json.push(format!(
                "{{\"x1\":{},\"y1\":{},\"x2\":{},\"y2\":{},\"d\":{:.1}}}",
                &s[0].1, &s[1].1, &s[2].1, &s[3].1, d / 0.0254
            ));
        }

        // Nodes JSON
        let mut node_json = Vec::new();
        for n in &nodes {
            node_json.push(format!(
                "{{\"x\":{},\"y\":{},\"type\":\"{}\",\"fitting\":\"{}\",\"size\":\"{}\"}}",
                &n[0].1, &n[1].1, esc(&n[2].1), esc(&n[3].1), esc(&n[4].1)
            ));
        }

        levels_json_parts.push(format!(
            "\"{}\":{{\"rooms\":[{}],\"placements\":[{}],\"segments\":[{}],\"nodes\":[{}]}}",
            esc(lvl),
            rooms_json.join(","),
            place_json.join(","),
            seg_json.join(","),
            node_json.join(","),
        ));
    }

    // ── Category summary across all levels ──
    let cat_rows = doc.query_raw(
        "SELECT pt.category, COUNT(*) as cnt, COALESCE(SUM(p.cfm),0) as total_cfm
         FROM placements p
         JOIN product_types pt ON p.product_type_id = pt.id
         WHERE p.x IS NOT NULL
         GROUP BY pt.category ORDER BY cnt DESC",
    )?;
    let mut cat_json = Vec::new();
    for row in &cat_rows {
        cat_json.push(format!(
            "{{\"cat\":\"{}\",\"cnt\":{},\"cfm\":{}}}",
            esc(&row[0].1),
            &row[1].1,
            &row[2].1
        ));
    }

    let total_placements: i64 = cat_rows
        .iter()
        .map(|r| r[1].1.parse::<i64>().unwrap_or(0))
        .sum();

    // ── Build final HTML ──
    let html = format!(
        r##"<!DOCTYPE html>
<html lang="en"><head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{project} — Mechanical Plan</title>
<style>
/* ═══════════════════════════════════════════════════════════════
   CSS Variables
   ═══════════════════════════════════════════════════════════════ */
:root {{
  --sidebar-w: 280px;
  --toolbar-h: 40px;
  --statusbar-h: 28px;
  --bg: #1a1a1e;
  --sidebar-bg: #1e1e24;
  --toolbar-bg: #252530;
  --status-bg: #1c1c22;
  --accent: #3b82f6;
  --accent-hover: #2563eb;
  --text: #d4d4d8;
  --text-dim: #71717a;
  --text-bright: #fafafa;
  --border: #2e2e38;
  --input-bg: #27272e;
  --panel-bg: #ffffffee;
  --panel-border: #d4d4d8;
  --select-blue: #3b82f6;
  --search-yellow: #f59e0b;
}}

/* ═══════════════════════════════════════════════════════════════
   Reset & Base
   ═══════════════════════════════════════════════════════════════ */
*, *::before, *::after {{ margin: 0; padding: 0; box-sizing: border-box; }}
html, body {{ height: 100%; overflow: hidden; font-family: 'Segoe UI', system-ui, -apple-system, sans-serif; background: var(--bg); color: var(--text); font-size: 13px; }}

/* ═══════════════════════════════════════════════════════════════
   Layout
   ═══════════════════════════════════════════════════════════════ */
#app {{ display: flex; height: 100vh; }}

/* Sidebar */
#sidebar {{
  width: var(--sidebar-w); min-width: var(--sidebar-w); background: var(--sidebar-bg);
  border-right: 1px solid var(--border); display: flex; flex-direction: column;
  overflow: hidden; z-index: 20;
}}
#sidebar-scroll {{ flex: 1; overflow-y: auto; padding: 16px; }}
#sidebar-scroll::-webkit-scrollbar {{ width: 6px; }}
#sidebar-scroll::-webkit-scrollbar-thumb {{ background: #444; border-radius: 3px; }}

.sidebar-header {{ margin-bottom: 16px; }}
.sidebar-header h1 {{ font-size: 15px; font-weight: 600; color: var(--text-bright); line-height: 1.3; }}
.sidebar-header .pnum {{ font-size: 12px; color: var(--text-dim); margin-top: 2px; }}

.section-label {{
  font-size: 10px; text-transform: uppercase; letter-spacing: 1px;
  color: var(--text-dim); margin: 16px 0 6px; font-weight: 600;
}}
.section-label:first-of-type {{ margin-top: 0; }}

hr.sep {{ border: none; border-top: 1px solid var(--border); margin: 12px 0; }}

/* Level selector */
#level-select {{
  width: 100%; padding: 7px 10px; background: var(--input-bg); border: 1px solid var(--border);
  border-radius: 4px; color: var(--text-bright); font-size: 13px; outline: none; cursor: pointer;
  -webkit-appearance: none; appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6'%3E%3Cpath d='M0 0l5 6 5-6z' fill='%2371717a'/%3E%3C/svg%3E");
  background-repeat: no-repeat; background-position: right 10px center;
}}
#level-select:focus {{ border-color: var(--accent); }}
#level-select option {{ background: var(--input-bg); color: var(--text-bright); }}

/* Search */
#search {{
  width: 100%; padding: 7px 10px 7px 32px; background: var(--input-bg); border: 1px solid var(--border);
  border-radius: 4px; color: var(--text-bright); font-size: 13px; outline: none;
}}
#search:focus {{ border-color: var(--accent); }}
#search::placeholder {{ color: var(--text-dim); }}
.search-wrap {{ position: relative; }}
.search-wrap::before {{
  content: '\1F50D'; position: absolute; left: 9px; top: 50%; transform: translateY(-50%);
  font-size: 13px; opacity: 0.4; pointer-events: none;
}}

/* Layer toggles */
.layer-row {{
  display: flex; align-items: center; gap: 8px; padding: 4px 0; cursor: pointer; user-select: none;
}}
.layer-row:hover {{ color: var(--text-bright); }}
.layer-row input[type="checkbox"] {{ accent-color: var(--accent); cursor: pointer; }}
.layer-row .swatch {{
  width: 10px; height: 10px; border-radius: 2px; flex-shrink: 0;
}}

/* Device summary */
.summary-row {{
  display: flex; justify-content: space-between; padding: 3px 0;
  border-bottom: 1px solid var(--border);
}}
.summary-row .cnt {{ color: var(--text-bright); font-weight: 600; }}
.summary-row .cfm {{ color: var(--text-dim); font-size: 11px; margin-left: 4px; }}
.summary-total {{
  display: flex; justify-content: space-between; padding: 6px 0; margin-top: 4px;
  font-weight: 600; color: var(--text-bright); border-top: 2px solid var(--border);
}}

/* Main area */
#main {{ flex: 1; display: flex; flex-direction: column; overflow: hidden; }}

/* Toolbar */
#toolbar {{
  height: var(--toolbar-h); background: var(--toolbar-bg); border-bottom: 1px solid var(--border);
  display: flex; align-items: center; padding: 0 12px; gap: 2px; z-index: 15;
}}
.tool-btn {{
  background: none; border: 1px solid transparent; border-radius: 4px;
  color: var(--text); padding: 4px 10px; font-size: 12px; cursor: pointer;
  display: flex; align-items: center; gap: 4px; font-family: inherit; white-space: nowrap;
}}
.tool-btn:hover {{ background: #ffffff10; color: var(--text-bright); }}
.tool-btn.active {{ background: var(--accent); color: #fff; border-color: var(--accent); }}
.tool-sep {{ width: 1px; height: 20px; background: var(--border); margin: 0 6px; }}
.tool-btn kbd {{
  font-size: 10px; background: #ffffff15; padding: 1px 4px; border-radius: 2px;
  font-family: inherit;
}}

/* Canvas area */
#canvas-wrap {{ flex: 1; position: relative; overflow: hidden; background: var(--bg); }}
#canvas-wrap canvas {{ position: absolute; top: 0; left: 0; }}

/* Status bar */
#statusbar {{
  height: var(--statusbar-h); background: var(--status-bg); border-top: 1px solid var(--border);
  display: flex; align-items: center; padding: 0 12px; gap: 16px;
  font-size: 11px; color: var(--text-dim); z-index: 15;
}}
#statusbar .val {{ color: var(--text); font-weight: 500; }}

/* ═══════════════════════════════════════════════════════════════
   Properties Panel (right overlay)
   ═══════════════════════════════════════════════════════════════ */
#props {{
  display: none; position: absolute; right: 16px; top: 16px; width: 320px;
  background: var(--panel-bg); border: 1px solid var(--panel-border);
  border-radius: 8px; padding: 16px; font-size: 13px; color: #1a1a1e;
  max-height: calc(100% - 32px); overflow-y: auto;
  box-shadow: 0 8px 32px rgba(0,0,0,0.3); z-index: 25;
}}
#props h3 {{
  font-size: 15px; font-weight: 600; margin-bottom: 10px;
  display: flex; justify-content: space-between; align-items: center;
}}
#props .close {{
  width: 24px; height: 24px; border-radius: 4px; border: none; background: #f0f0f0;
  cursor: pointer; font-size: 16px; display: flex; align-items: center; justify-content: center;
  color: #666;
}}
#props .close:hover {{ background: #e0e0e0; color: #000; }}
.pr {{ display: flex; justify-content: space-between; padding: 4px 0; border-bottom: 1px solid #eee; }}
.pr .k {{ color: #888; font-size: 12px; }}
.pr .v {{ color: #1a1a1e; font-weight: 500; text-align: right; max-width: 190px; word-break: break-word; }}
.pr-section {{
  margin-top: 10px; padding-top: 8px; border-top: 1px solid #ddd;
  font-size: 10px; text-transform: uppercase; letter-spacing: 0.5px; color: #999; font-weight: 600;
}}

/* Measure overlay line */
#measure-overlay {{
  position: absolute; top: 0; left: 0; width: 100%; height: 100%;
  pointer-events: none; z-index: 10;
}}

/* ═══════════════════════════════════════════════════════════════
   Print styles
   ═══════════════════════════════════════════════════════════════ */
@media print {{
  #sidebar, #toolbar, #statusbar, #props {{ display: none !important; }}
  #app {{ display: block; }}
  #main {{ display: block; }}
  #canvas-wrap {{
    position: absolute; top: 0; left: 0; width: 100vw; height: 100vh;
  }}
  #canvas-wrap canvas {{
    width: 100vw !important; height: 100vh !important;
  }}
  body {{ background: white; }}
}}
</style>
</head>
<body>
<div id="app">

  <!-- ═══ Sidebar ═══ -->
  <div id="sidebar">
    <div id="sidebar-scroll">
      <div class="sidebar-header">
        <h1>{project}</h1>
        <div class="pnum">#{number}</div>
      </div>

      <div class="section-label">Level</div>
      <select id="level-select"></select>

      <hr class="sep">

      <div class="section-label">Search</div>
      <div class="search-wrap">
        <input id="search" type="text" placeholder="Filter by tag, name...">
      </div>
      <div id="search-count" style="font-size:11px;color:var(--text-dim);margin-top:4px;min-height:14px;"></div>

      <hr class="sep">

      <div class="section-label">Layers</div>
      <div id="layer-list"></div>

      <hr class="sep">

      <div class="section-label">Devices</div>
      <div id="cat-summary"></div>
    </div>
  </div>

  <!-- ═══ Main area ═══ -->
  <div id="main">

    <!-- Toolbar -->
    <div id="toolbar">
      <button class="tool-btn" id="btn-zin" title="Zoom In">+ Zoom</button>
      <button class="tool-btn" id="btn-zout" title="Zoom Out">&minus; Zoom</button>
      <button class="tool-btn" id="btn-fit" title="Fit to View">Fit <kbd>F</kbd></button>
      <div class="tool-sep"></div>
      <button class="tool-btn" id="btn-measure" title="Measure distance">Measure <kbd>M</kbd></button>
      <div class="tool-sep"></div>
      <button class="tool-btn" id="btn-print" title="Print sheet">Print <kbd>Ctrl+P</kbd></button>
    </div>

    <!-- Canvas -->
    <div id="canvas-wrap">
      <canvas id="c"></canvas>
      <svg id="measure-overlay"></svg>
      <div id="props"></div>
    </div>

    <!-- Status bar -->
    <div id="statusbar">
      <span>Zoom: <span class="val" id="stat-zoom">100%</span></span>
      <span>Cursor: <span class="val" id="stat-cursor">-</span></span>
      <span>Elements: <span class="val" id="stat-elements">-</span></span>
      <span>Level: <span class="val" id="stat-level">-</span></span>
    </div>
  </div>
</div>

<script>
/* ═══════════════════════════════════════════════════════════════
   DATA — embedded from .sed file at export time
   ═══════════════════════════════════════════════════════════════ */
var DATA = {{
  levels: {{{levels}}},
  project: "{project}",
  number: "{number}",
  categories: [{categories}],
  totalPlacements: {total_placements},
  initialLevel: "{initial_level}"
}};

/* ═══════════════════════════════════════════════════════════════
   State
   ═══════════════════════════════════════════════════════════════ */
var levelNames = Object.keys(DATA.levels);
var currentLevel = DATA.initialLevel && DATA.levels[DATA.initialLevel] ? DATA.initialLevel : levelNames[0] || "";
var searchTerm = "";

// View transform
var vx = 0, vy = 0, vs = 1;

// Interaction state
var dragging = false, dragStartX = 0, dragStartY = 0;
var selected = null;         // {{ type: 'room'|'placement'|'segment', data: ... }}
var dirty = true;            // redraw flag
var animFrameId = null;

// Measure tool
var measureActive = false;
var measureStart = null;     // {{x, y}} in model coords, or null
var measureEnd = null;

// Layer visibility
var layers = {{
  rooms: true, supply: true, return_: true, exhaust: true,
  equipment: true, duct: true, labels: true, nodes: true
}};

// Sheet constants (mm) — ARCH D
var SW = 914.4, SH = 609.6, SB = 12.7, TB_W = 190, TB_H = 76;
var drawL = SB + 15, drawR = SW - SB - 15, drawT = SB + 15, drawB = SH - SB - TB_H - 15;
var drawW = drawR - drawL, drawH = drawB - drawT;

// Model-to-sheet transform (recomputed per level)
var mxMin, myMin, mxMax, myMax, mScale;

/* ═══════════════════════════════════════════════════════════════
   DOM references
   ═══════════════════════════════════════════════════════════════ */
var canvas = document.getElementById('c');
var ctx = canvas.getContext('2d');
var W = 0, H = 0;

/* ═══════════════════════════════════════════════════════════════
   Coordinate Transforms
   ═══════════════════════════════════════════════════════════════ */

/** Recompute model bounds and model-to-sheet scale for current level. */
function computeTransform() {{
  var ld = DATA.levels[currentLevel];
  if (!ld) return;
  mxMin = Infinity; myMin = Infinity; mxMax = -Infinity; myMax = -Infinity;
  ld.rooms.forEach(function(r) {{
    r.vertices.forEach(function(v) {{
      if (v.x < mxMin) mxMin = v.x; if (v.y < myMin) myMin = v.y;
      if (v.x > mxMax) mxMax = v.x; if (v.y > myMax) myMax = v.y;
    }});
  }});
  ld.placements.forEach(function(p) {{
    if (p.x < mxMin) mxMin = p.x; if (p.y < myMin) myMin = p.y;
    if (p.x > mxMax) mxMax = p.x; if (p.y > myMax) myMax = p.y;
  }});
  ld.segments.forEach(function(s) {{
    if (s.x1 < mxMin) mxMin = s.x1; if (s.y1 < myMin) myMin = s.y1;
    if (s.x2 < mxMax) mxMax = s.x2; if (s.y2 > myMax) myMax = s.y2;
    if (s.x1 > mxMax) mxMax = s.x1; if (s.y1 > myMax) myMax = s.y1;
    if (s.x2 < mxMin) mxMin = s.x2; if (s.y2 < myMin) myMin = s.y2;
  }});
  if (mxMin === Infinity) {{ mxMin = 0; myMin = 0; mxMax = 20; myMax = 20; }}
  mxMin -= 1; myMin -= 1; mxMax += 1; myMax += 1;
  var mW = mxMax - mxMin, mH = myMax - myMin;
  mScale = Math.min(drawW / (mW * 1000), drawH / (mH * 1000)) * 1000;
}}

/** Model coordinates (m) to sheet coordinates (mm). */
function m2s(mx, my) {{
  return {{ x: drawL + (mx - mxMin) * mScale, y: drawB - (my - myMin) * mScale }};
}}

/** Screen pixel to sheet coordinates (mm). */
function screen2sheet(sx, sy) {{
  return {{ x: (sx - vx) / vs, y: (sy - vy) / vs }};
}}

/** Sheet coordinates (mm) to model coordinates (m). */
function s2m(sx, sy) {{
  return {{ x: (sx - drawL) / mScale + mxMin, y: (drawB - sy) / mScale + myMin }};
}}

/** Screen pixel to model coordinates (m). */
function screen2model(sx, sy) {{
  var sh = screen2sheet(sx, sy);
  return s2m(sh.x, sh.y);
}}

/* ═══════════════════════════════════════════════════════════════
   Canvas Sizing
   ═══════════════════════════════════════════════════════════════ */
function resize() {{
  var wrap = document.getElementById('canvas-wrap');
  W = wrap.clientWidth;
  H = wrap.clientHeight;
  canvas.width = W * devicePixelRatio;
  canvas.height = H * devicePixelRatio;
  canvas.style.width = W + 'px';
  canvas.style.height = H + 'px';
  ctx.setTransform(devicePixelRatio, 0, 0, devicePixelRatio, 0, 0);
  dirty = true;
}}

function fit() {{
  vs = Math.min(W / (SW + 40), H / (SH + 40));
  vx = (W - SW * vs) / 2;
  vy = (H - SH * vs) / 2;
  dirty = true;
}}

/* ═══════════════════════════════════════════════════════════════
   Render Loop (requestAnimationFrame with dirty flag)
   ═══════════════════════════════════════════════════════════════ */
function scheduleRender() {{
  if (animFrameId) return;
  animFrameId = requestAnimationFrame(function() {{
    animFrameId = null;
    if (dirty) {{
      dirty = false;
      render();
    }}
  }});
}}

function markDirty() {{
  dirty = true;
  scheduleRender();
}}

/* ═══════════════════════════════════════════════════════════════
   Main Render Function
   ═══════════════════════════════════════════════════════════════ */
function render() {{
  var ld = DATA.levels[currentLevel];
  if (!ld) return;
  var lowerSearch = searchTerm.toLowerCase();

  // Clear
  ctx.fillStyle = '#1a1a1e';
  ctx.fillRect(0, 0, W, H);
  ctx.save();
  ctx.translate(vx, vy);
  ctx.scale(vs, vs);

  // ── White sheet ──
  ctx.shadowColor = 'rgba(0,0,0,0.4)';
  ctx.shadowBlur = 20 / vs;
  ctx.shadowOffsetX = 4 / vs;
  ctx.shadowOffsetY = 4 / vs;
  ctx.fillStyle = '#fff';
  ctx.fillRect(0, 0, SW, SH);
  ctx.shadowColor = 'transparent';
  ctx.shadowBlur = 0;
  ctx.shadowOffsetX = 0;
  ctx.shadowOffsetY = 0;

  // ── Double border ──
  ctx.strokeStyle = '#000';
  ctx.lineWidth = 0.75;
  ctx.strokeRect(SB, SB, SW - 2 * SB, SH - 2 * SB);
  ctx.lineWidth = 0.25;
  ctx.strokeRect(SB + 2, SB + 2, SW - 2 * SB - 4, SH - 2 * SB - 4);

  // ── Title block ──
  renderTitleBlock();

  // ── Rooms ──
  if (layers.rooms) {{
    ld.rooms.forEach(function(r) {{
      if (r.vertices.length < 3) return;
      var sv = r.vertices.map(function(v) {{ return m2s(v.x, v.y); }});
      var isSel = selected && selected.type === 'room' && selected.data.tag === r.tag;
      var isSearch = lowerSearch && (r.tag.toLowerCase().indexOf(lowerSearch) !== -1 || r.name.toLowerCase().indexOf(lowerSearch) !== -1);

      // Fill
      if (isSel) {{
        ctx.fillStyle = 'rgba(59,130,246,0.12)';
        drawPoly(sv); ctx.fill();
      }} else if (isSearch) {{
        ctx.fillStyle = 'rgba(245,158,11,0.15)';
        drawPoly(sv); ctx.fill();
      }}

      // Stroke
      ctx.strokeStyle = isSel ? '#3b82f6' : isSearch ? '#f59e0b' : (r.scope === 'nic' ? '#bbb' : '#000');
      ctx.lineWidth = isSel ? 0.6 : isSearch ? 0.5 : (r.scope === 'nic' ? 0.15 : 0.35);
      drawPoly(sv); ctx.stroke();

      // Labels
      if (layers.labels) {{
        var cx = sv.reduce(function(s, v) {{ return s + v.x; }}, 0) / sv.length;
        var cy = sv.reduce(function(s, v) {{ return s + v.y; }}, 0) / sv.length;
        ctx.fillStyle = '#555';
        ctx.font = 'bold 5px Helvetica, sans-serif';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText(r.tag, cx, cy - 3);
        ctx.fillStyle = r.scope === 'nic' ? '#999' : '#000';
        ctx.font = '4px Helvetica, sans-serif';
        ctx.fillText(r.name, cx, cy + 3);
        ctx.textAlign = 'left';
        ctx.textBaseline = 'alphabetic';
      }}
    }});
  }}

  // ── Duct segments ──
  if (layers.duct) {{
    ld.segments.forEach(function(s) {{
      var p1 = m2s(s.x1, s.y1), p2 = m2s(s.x2, s.y2);
      var isSel = selected && selected.type === 'segment' && selected.data === s;
      ctx.strokeStyle = isSel ? '#3b82f6' : '#777';
      ctx.lineWidth = Math.max(s.d * mScale * 0.0254 * 0.8, 0.3);
      ctx.lineCap = 'round';
      ctx.beginPath(); ctx.moveTo(p1.x, p1.y); ctx.lineTo(p2.x, p2.y); ctx.stroke();
      // Size label at midpoint
      if (layers.labels) {{
        var mx = (p1.x + p2.x) / 2, my = (p1.y + p2.y) / 2;
        ctx.fillStyle = '#888';
        ctx.font = '3px Helvetica, sans-serif';
        ctx.textAlign = 'center';
        ctx.fillText(s.d + '"', mx, my - 1.5);
        ctx.textAlign = 'left';
      }}
    }});
  }}

  // ── Nodes ──
  if (layers.nodes) {{
    ld.nodes.forEach(function(n) {{
      var p = m2s(n.x, n.y);
      ctx.fillStyle = '#aaa';
      ctx.fillRect(p.x - 0.5, p.y - 0.5, 1.0, 1.0);
    }});
  }}

  // ── Placements ──
  ld.placements.forEach(function(p) {{
    // Layer filter
    if (!layerVisibleForPlacement(p)) return;

    var sp = m2s(p.x, p.y);
    var isSel = selected && selected.type === 'placement' && selected.data === p;
    var isSearch = lowerSearch && (
      p.tag.toLowerCase().indexOf(lowerSearch) !== -1 ||
      (p.itag && p.itag.toLowerCase().indexOf(lowerSearch) !== -1) ||
      (p.room && p.room.toLowerCase().indexOf(lowerSearch) !== -1)
    );
    var r = p.dom === 'equipment' ? 2.5 : 1.5;

    // Dim non-matching when searching
    if (lowerSearch && !isSearch && !isSel) {{
      ctx.globalAlpha = 0.12;
    }}

    // Shape
    ctx.strokeStyle = isSel ? '#3b82f6' : isSearch ? '#f59e0b' : '#000';
    ctx.lineWidth = isSel ? 0.5 : isSearch ? 0.6 : 0.3;
    if (p.dom === 'equipment') {{
      ctx.beginPath();
      ctx.moveTo(sp.x, sp.y - r); ctx.lineTo(sp.x + r, sp.y);
      ctx.lineTo(sp.x, sp.y + r); ctx.lineTo(sp.x - r, sp.y);
      ctx.closePath(); ctx.stroke();
    }} else {{
      ctx.beginPath();
      for (var i = 0; i < 8; i++) {{
        var a = Math.PI * 2 * i / 8;
        var ox = sp.x + r * Math.cos(a), oy = sp.y + r * Math.sin(a);
        if (i === 0) ctx.moveTo(ox, oy); else ctx.lineTo(ox, oy);
      }}
      ctx.closePath(); ctx.stroke();
    }}

    // Selection ring
    if (isSel) {{
      ctx.strokeStyle = 'rgba(59,130,246,0.5)';
      ctx.lineWidth = 0.3;
      ctx.setLineDash([0.5, 0.5]);
      ctx.beginPath(); ctx.arc(sp.x, sp.y, r + 1.5, 0, Math.PI * 2); ctx.stroke();
      ctx.setLineDash([]);
    }}

    // Search highlight ring
    if (isSearch && !isSel) {{
      ctx.strokeStyle = 'rgba(245,158,11,0.6)';
      ctx.lineWidth = 0.4;
      ctx.setLineDash([0.5, 0.5]);
      ctx.beginPath(); ctx.arc(sp.x, sp.y, r + 2, 0, Math.PI * 2); ctx.stroke();
      ctx.setLineDash([]);
    }}

    // Label
    if (layers.labels) {{
      ctx.fillStyle = '#333';
      ctx.font = '3.5px Helvetica, sans-serif';
      var label = p.cfm ? p.tag + ' ' + p.cfm + ' CFM' : p.tag;
      ctx.fillText(label, sp.x + r + 1, sp.y + 1);
    }}

    ctx.globalAlpha = 1.0;
  }});

  // ── Scale bar ──
  renderScaleBar();

  // ── North arrow ──
  renderNorthArrow();

  // ── Measure line ──
  if (measureStart && measureEnd) {{
    var ms = m2s(measureStart.x, measureStart.y);
    var me = m2s(measureEnd.x, measureEnd.y);
    ctx.strokeStyle = '#ef4444';
    ctx.lineWidth = 0.4;
    ctx.setLineDash([1, 1]);
    ctx.beginPath(); ctx.moveTo(ms.x, ms.y); ctx.lineTo(me.x, me.y); ctx.stroke();
    ctx.setLineDash([]);
    // Endpoints
    ctx.fillStyle = '#ef4444';
    ctx.beginPath(); ctx.arc(ms.x, ms.y, 0.8, 0, Math.PI * 2); ctx.fill();
    ctx.beginPath(); ctx.arc(me.x, me.y, 0.8, 0, Math.PI * 2); ctx.fill();
    // Distance label
    var dist = Math.sqrt(Math.pow(measureEnd.x - measureStart.x, 2) + Math.pow(measureEnd.y - measureStart.y, 2));
    var distFt = dist * 3.28084;
    var feet = Math.floor(distFt);
    var inches = Math.round((distFt - feet) * 12);
    if (inches === 12) {{ feet++; inches = 0; }}
    var distLabel = feet + "'-" + inches + '"  (' + dist.toFixed(2) + ' m)';
    var lx = (ms.x + me.x) / 2, ly = (ms.y + me.y) / 2;
    ctx.fillStyle = '#fff';
    ctx.fillRect(lx - 20, ly - 5, 40, 6);
    ctx.fillStyle = '#ef4444';
    ctx.font = 'bold 4px Helvetica, sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText(distLabel, lx, ly - 0.5);
    ctx.textAlign = 'left';
  }}

  ctx.restore();

  // Update status bar
  updateStatusBar();
}}

/* ═══════════════════════════════════════════════════════════════
   Render Helpers
   ═══════════════════════════════════════════════════════════════ */

function drawPoly(sv) {{
  ctx.beginPath();
  ctx.moveTo(sv[0].x, sv[0].y);
  for (var i = 1; i < sv.length; i++) ctx.lineTo(sv[i].x, sv[i].y);
  ctx.closePath();
}}

function renderTitleBlock() {{
  var tbx = SW - SB - TB_W, tby = SH - SB - TB_H - 2;
  ctx.lineWidth = 0.5;
  ctx.strokeStyle = '#000';
  ctx.strokeRect(tbx, tby, TB_W - 2, TB_H);
  ctx.lineWidth = 0.15;
  [15, 30, 45, 60].forEach(function(dy) {{
    ctx.beginPath(); ctx.moveTo(tbx, tby + dy); ctx.lineTo(tbx + TB_W - 2, tby + dy); ctx.stroke();
  }});
  ctx.beginPath(); ctx.moveTo(tbx + 45, tby); ctx.lineTo(tbx + 45, tby + TB_H); ctx.stroke();

  ctx.fillStyle = '#888'; ctx.font = '5px Helvetica, sans-serif';
  ctx.fillText('PROJECT', tbx + 3, tby + 10);
  ctx.fillText('PROJECT NO.', tbx + 3, tby + 25);
  ctx.fillText('SHEET TITLE', tbx + 3, tby + 40);
  ctx.fillText('SHEET NO.', tbx + 3, tby + 55);
  ctx.fillText('FORMAT', tbx + 3, tby + 70);

  ctx.fillStyle = '#000'; ctx.font = 'bold 9px Helvetica, sans-serif';
  ctx.fillText(DATA.project, tbx + 48, tby + 11);
  ctx.font = '7px Helvetica, sans-serif';
  ctx.fillText(DATA.number, tbx + 48, tby + 26);
  ctx.font = 'bold 7px Helvetica, sans-serif';
  ctx.fillText(currentLevel, tbx + 48, tby + 41);
  ctx.font = 'bold 9px Helvetica, sans-serif';
  var sheetIdx = levelNames.indexOf(currentLevel) + 1;
  ctx.fillText('M-' + (100 + sheetIdx), tbx + 48, tby + 56);
  ctx.fillStyle = '#888'; ctx.font = '5px Helvetica, sans-serif';
  ctx.fillText('ARCH D  (36\u00d724)', tbx + 48, tby + 70);
}}

function renderScaleBar() {{
  var barM = Math.ceil(5 / mScale * 1000);
  if (barM < 1) barM = 1;
  var barMM = barM * mScale / 1000;
  ctx.strokeStyle = '#000'; ctx.lineWidth = 0.5;
  ctx.beginPath(); ctx.moveTo(drawL, drawB + 8); ctx.lineTo(drawL + barMM, drawB + 8); ctx.stroke();
  for (var i = 0; i <= barM; i++) {{
    var tx = drawL + i / barM * barMM;
    ctx.beginPath(); ctx.moveTo(tx, drawB + 6); ctx.lineTo(tx, drawB + 10); ctx.stroke();
  }}
  ctx.fillStyle = '#000'; ctx.font = '4px Helvetica, sans-serif';
  ctx.fillText('0', drawL, drawB + 14);
  ctx.fillText(barM + ' m', drawL + barMM - 6, drawB + 14);
}}

function renderNorthArrow() {{
  var nax = drawR - 10, nay = drawT + 12;
  ctx.fillStyle = '#000';
  ctx.beginPath();
  ctx.moveTo(nax, nay - 8); ctx.lineTo(nax + 3, nay);
  ctx.lineTo(nax, nay - 2); ctx.lineTo(nax - 3, nay);
  ctx.closePath(); ctx.fill();
  ctx.font = 'bold 5px Helvetica, sans-serif';
  ctx.textAlign = 'center';
  ctx.fillText('N', nax, nay - 10);
  ctx.textAlign = 'left';
}}

/** Check if a placement should be visible given current layer state. */
function layerVisibleForPlacement(p) {{
  if (!layers.equipment && p.dom === 'equipment') return false;
  if (!layers.supply && p.dom !== 'equipment' && p.cat && p.cat.toLowerCase().indexOf('supply') !== -1) return false;
  if (!layers.return_ && p.dom !== 'equipment' && p.cat && p.cat.toLowerCase().indexOf('return') !== -1) return false;
  if (!layers.exhaust && p.dom !== 'equipment' && p.cat && p.cat.toLowerCase().indexOf('exhaust') !== -1) return false;
  return true;
}}

/* ═══════════════════════════════════════════════════════════════
   Status Bar
   ═══════════════════════════════════════════════════════════════ */
function updateStatusBar() {{
  var ld = DATA.levels[currentLevel];
  var zoomPct = Math.round(vs * 100);
  document.getElementById('stat-zoom').textContent = zoomPct + '%';
  document.getElementById('stat-level').textContent = currentLevel;
  if (ld) {{
    var total = ld.rooms.length + ld.placements.length + ld.segments.length + ld.nodes.length;
    document.getElementById('stat-elements').textContent = total;
  }}
}}

/* ═══════════════════════════════════════════════════════════════
   Hit Testing
   ═══════════════════════════════════════════════════════════════ */

function pointInPoly(px, py, vs) {{
  var inside = false;
  for (var i = 0, j = vs.length - 1; i < vs.length; j = i++) {{
    if ((vs[i].y > py) !== (vs[j].y > py) &&
        px < (vs[j].x - vs[i].x) * (py - vs[i].y) / (vs[j].y - vs[i].y) + vs[i].x)
      inside = !inside;
  }}
  return inside;
}}

/** Distance from point (px,py) to line segment (ax,ay)-(bx,by). */
function pointSegDist(px, py, ax, ay, bx, by) {{
  var dx = bx - ax, dy = by - ay;
  var len2 = dx * dx + dy * dy;
  if (len2 === 0) return Math.sqrt((px - ax) * (px - ax) + (py - ay) * (py - ay));
  var t = Math.max(0, Math.min(1, ((px - ax) * dx + (py - ay) * dy) / len2));
  var nx = ax + t * dx, ny = ay + t * dy;
  return Math.sqrt((px - nx) * (px - nx) + (py - ny) * (py - ny));
}}

function hitTest(screenX, screenY) {{
  var ld = DATA.levels[currentLevel];
  if (!ld) return null;
  var sh = screen2sheet(screenX, screenY);
  var sx = sh.x, sy = sh.y;

  // Hit test placements first (smallest targets, highest priority)
  for (var pi = 0; pi < ld.placements.length; pi++) {{
    var p = ld.placements[pi];
    if (!layerVisibleForPlacement(p)) continue;
    var sp = m2s(p.x, p.y);
    var dx = sx - sp.x, dy = sy - sp.y;
    if (Math.sqrt(dx * dx + dy * dy) < 4) {{
      return {{ type: 'placement', data: p }};
    }}
  }}

  // Hit test duct segments
  if (layers.duct) {{
    for (var si = 0; si < ld.segments.length; si++) {{
      var seg = ld.segments[si];
      var p1 = m2s(seg.x1, seg.y1), p2 = m2s(seg.x2, seg.y2);
      if (pointSegDist(sx, sy, p1.x, p1.y, p2.x, p2.y) < 2) {{
        return {{ type: 'segment', data: seg }};
      }}
    }}
  }}

  // Hit test rooms
  if (layers.rooms) {{
    for (var ri = 0; ri < ld.rooms.length; ri++) {{
      var rm = ld.rooms[ri];
      if (rm.vertices.length < 3) continue;
      var sv = rm.vertices.map(function(v) {{ return m2s(v.x, v.y); }});
      if (pointInPoly(sx, sy, sv)) {{
        return {{ type: 'room', data: rm }};
      }}
    }}
  }}

  return null;
}}

/* ═══════════════════════════════════════════════════════════════
   Properties Panel
   ═══════════════════════════════════════════════════════════════ */

function showProps(hit) {{
  if (!hit) {{ closeProps(); return; }}
  selected = hit;
  if (hit.type === 'placement') showPlacementProps(hit.data);
  else if (hit.type === 'room') showRoomProps(hit.data);
  else if (hit.type === 'segment') showSegmentProps(hit.data);
  markDirty();
}}

function showPlacementProps(p) {{
  var ld = DATA.levels[currentLevel];
  var panel = document.getElementById('props');
  var devs = ld.placements.filter(function(d) {{ return d.room === p.room && p.room; }});
  var roomCfm = devs.reduce(function(s, d) {{ return s + (d.cfm || 0); }}, 0);

  var h = '<h3>' + esc(p.itag || p.tag) + '<button class="close" onclick="closeProps()">&times;</button></h3>';
  h += pr('Tag', p.tag);
  if (p.itag) h += pr('Instance Tag', p.itag);
  h += pr('Category', p.cat);
  h += pr('Domain', p.dom);
  h += pr('Manufacturer', p.mfr || '\u2014');
  h += pr('Model', p.model || '\u2014');
  h += pr('CFM', p.cfm || '\u2014');
  h += pr('Room', p.room || '\u2014');
  h += pr('Position', p.x.toFixed(3) + ', ' + p.y.toFixed(3) + ' m');
  if (p.room && roomCfm) h += pr('Room Total CFM', roomCfm);
  panel.innerHTML = h;
  panel.style.display = 'block';
}}

function showRoomProps(r) {{
  var ld = DATA.levels[currentLevel];
  var panel = document.getElementById('props');
  var roomDevs = ld.placements.filter(function(p) {{ return p.room === r.name; }});
  var totalCfm = roomDevs.reduce(function(s, d) {{ return s + (d.cfm || 0); }}, 0);

  var h = '<h3>' + esc(r.tag) + ' \u2014 ' + esc(r.name) + '<button class="close" onclick="closeProps()">&times;</button></h3>';
  h += pr('Tag', r.tag);
  h += pr('Name', r.name);
  h += pr('Scope', r.scope);
  h += pr('Devices', roomDevs.length);
  h += pr('Total CFM', totalCfm || '\u2014');

  if (roomDevs.length) {{
    h += '<div class="pr-section">Devices in Room</div>';
    roomDevs.forEach(function(d) {{
      h += pr(d.tag, (d.cfm || '\u2014') + ' CFM');
    }});
  }}
  panel.innerHTML = h;
  panel.style.display = 'block';
}}

function showSegmentProps(s) {{
  var panel = document.getElementById('props');
  var len = Math.sqrt(Math.pow(s.x2 - s.x1, 2) + Math.pow(s.y2 - s.y1, 2));
  var lenFt = len * 3.28084;
  var feet = Math.floor(lenFt);
  var inches = Math.round((lenFt - feet) * 12);
  if (inches === 12) {{ feet++; inches = 0; }}

  var h = '<h3>Duct Segment<button class="close" onclick="closeProps()">&times;</button></h3>';
  h += pr('From', s.x1.toFixed(3) + ', ' + s.y1.toFixed(3) + ' m');
  h += pr('To', s.x2.toFixed(3) + ', ' + s.y2.toFixed(3) + ' m');
  h += pr('Diameter', s.d + '"');
  h += pr('Length', len.toFixed(2) + ' m (' + feet + "'-" + inches + '")');
  panel.innerHTML = h;
  panel.style.display = 'block';
}}

function pr(k, v) {{
  return '<div class="pr"><span class="k">' + k + '</span><span class="v">' + v + '</span></div>';
}}

function esc(s) {{
  return s ? String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;') : '';
}}

function closeProps() {{
  selected = null;
  document.getElementById('props').style.display = 'none';
  markDirty();
}}

/* ═══════════════════════════════════════════════════════════════
   Sidebar: Level Selector
   ═══════════════════════════════════════════════════════════════ */

function buildLevelSelector() {{
  var sel = document.getElementById('level-select');
  sel.innerHTML = '';
  levelNames.forEach(function(name) {{
    var opt = document.createElement('option');
    var ld = DATA.levels[name];
    var count = ld ? ld.placements.length + ld.rooms.length : 0;
    opt.value = name;
    opt.textContent = name + '  (' + count + ' elements)';
    if (name === currentLevel) opt.selected = true;
    sel.appendChild(opt);
  }});
}}

function switchLevel(name) {{
  currentLevel = name;
  selected = null;
  measureStart = null;
  measureEnd = null;
  document.getElementById('props').style.display = 'none';
  computeTransform();
  fit();
  buildLevelSelector();
  updateDeviceSummary();
  updateSearchCount();
  markDirty();
}}

/* ═══════════════════════════════════════════════════════════════
   Sidebar: Layers
   ═══════════════════════════════════════════════════════════════ */

var layerDefs = [
  {{ key: 'rooms', label: 'Rooms', color: '#000' }},
  {{ key: 'supply', label: 'Supply', color: '#3b82f6' }},
  {{ key: 'return_', label: 'Return', color: '#10b981' }},
  {{ key: 'exhaust', label: 'Exhaust', color: '#ef4444' }},
  {{ key: 'equipment', label: 'Equipment', color: '#8b5cf6' }},
  {{ key: 'duct', label: 'Duct', color: '#777' }},
  {{ key: 'nodes', label: 'Nodes', color: '#aaa' }},
  {{ key: 'labels', label: 'Labels', color: '#555' }}
];

function buildLayers() {{
  var container = document.getElementById('layer-list');
  container.innerHTML = '';
  layerDefs.forEach(function(def) {{
    var row = document.createElement('label');
    row.className = 'layer-row';
    var cb = document.createElement('input');
    cb.type = 'checkbox';
    cb.checked = layers[def.key];
    cb.addEventListener('change', function() {{
      layers[def.key] = cb.checked;
      markDirty();
    }});
    var swatch = document.createElement('span');
    swatch.className = 'swatch';
    swatch.style.background = def.color;
    var label = document.createElement('span');
    label.textContent = def.label;
    row.appendChild(cb);
    row.appendChild(swatch);
    row.appendChild(label);
    container.appendChild(row);
  }});
}}

/* ═══════════════════════════════════════════════════════════════
   Sidebar: Device Summary
   ═══════════════════════════════════════════════════════════════ */

function updateDeviceSummary() {{
  var container = document.getElementById('cat-summary');
  container.innerHTML = '';
  var ld = DATA.levels[currentLevel];
  if (!ld) return;

  // Count by category for current level
  var cats = {{}};
  var totalCfm = 0;
  ld.placements.forEach(function(p) {{
    if (!cats[p.cat]) cats[p.cat] = {{ cnt: 0, cfm: 0 }};
    cats[p.cat].cnt++;
    cats[p.cat].cfm += (p.cfm || 0);
    totalCfm += (p.cfm || 0);
  }});

  var keys = Object.keys(cats).sort(function(a, b) {{ return cats[b].cnt - cats[a].cnt; }});
  keys.forEach(function(cat) {{
    var row = document.createElement('div');
    row.className = 'summary-row';
    row.innerHTML = '<span>' + esc(cat) + '</span><span><span class="cnt">' + cats[cat].cnt + '</span>' +
      (cats[cat].cfm ? '<span class="cfm">' + cats[cat].cfm + ' CFM</span>' : '') + '</span>';
    container.appendChild(row);
  }});

  var total = document.createElement('div');
  total.className = 'summary-total';
  total.innerHTML = '<span>Total</span><span>' + ld.placements.length + (totalCfm ? ' / ' + totalCfm + ' CFM' : '') + '</span>';
  container.appendChild(total);
}}

/* ═══════════════════════════════════════════════════════════════
   Search
   ═══════════════════════════════════════════════════════════════ */

function updateSearchCount() {{
  var el = document.getElementById('search-count');
  if (!searchTerm) {{ el.textContent = ''; return; }}
  var ld = DATA.levels[currentLevel];
  if (!ld) {{ el.textContent = ''; return; }}
  var lower = searchTerm.toLowerCase();
  var cnt = 0;
  ld.placements.forEach(function(p) {{
    if (p.tag.toLowerCase().indexOf(lower) !== -1 ||
        (p.itag && p.itag.toLowerCase().indexOf(lower) !== -1) ||
        (p.room && p.room.toLowerCase().indexOf(lower) !== -1)) cnt++;
  }});
  ld.rooms.forEach(function(r) {{
    if (r.tag.toLowerCase().indexOf(lower) !== -1 ||
        r.name.toLowerCase().indexOf(lower) !== -1) cnt++;
  }});
  el.textContent = cnt + ' match' + (cnt !== 1 ? 'es' : '');
}}

/* ═══════════════════════════════════════════════════════════════
   Event Listeners
   ═══════════════════════════════════════════════════════════════ */

// ── Level selector ──
document.getElementById('level-select').addEventListener('change', function(e) {{
  switchLevel(e.target.value);
}});

// ── Search ──
document.getElementById('search').addEventListener('input', function(e) {{
  searchTerm = e.target.value;
  updateSearchCount();
  markDirty();
}});

// ── Canvas: pan (left drag or middle drag) ──
canvas.addEventListener('mousedown', function(e) {{
  if (measureActive) return;
  if (e.button === 0 || e.button === 1) {{
    dragging = true;
    dragStartX = e.clientX - vx;
    dragStartY = e.clientY - vy;
    canvas.style.cursor = 'grabbing';
    e.preventDefault();
  }}
}});

canvas.addEventListener('mousemove', function(e) {{
  // Update cursor position in status bar
  var rect = canvas.getBoundingClientRect();
  var mc = screen2model(e.clientX - rect.left, e.clientY - rect.top);
  var ft = mc.x * 3.28084;
  var feetX = Math.floor(Math.abs(ft));
  var inchX = Math.round((Math.abs(ft) - feetX) * 12);
  ft = mc.y * 3.28084;
  var feetY = Math.floor(Math.abs(ft));
  var inchY = Math.round((Math.abs(ft) - feetY) * 12);
  document.getElementById('stat-cursor').textContent =
    mc.x.toFixed(2) + ', ' + mc.y.toFixed(2) + ' m';

  if (dragging) {{
    vx = e.clientX - dragStartX;
    vy = e.clientY - dragStartY;
    markDirty();
  }}
}});

canvas.addEventListener('mouseup', function() {{
  dragging = false;
  canvas.style.cursor = measureActive ? 'crosshair' : 'grab';
}});

canvas.addEventListener('mouseleave', function() {{
  dragging = false;
  canvas.style.cursor = measureActive ? 'crosshair' : 'grab';
}});

// ── Canvas: zoom (scroll wheel, centered on cursor) ──
canvas.addEventListener('wheel', function(e) {{
  e.preventDefault();
  var factor = e.deltaY < 0 ? 1.12 : 1 / 1.12;
  var rect = canvas.getBoundingClientRect();
  var mx = e.clientX - rect.left, my = e.clientY - rect.top;
  vx = mx - (mx - vx) * factor;
  vy = my - (my - vy) * factor;
  vs *= factor;
  markDirty();
}}, {{ passive: false }});

// ── Canvas: click to select ──
canvas.addEventListener('click', function(e) {{
  if (measureActive) {{
    var rect = canvas.getBoundingClientRect();
    var mc = screen2model(e.clientX - rect.left, e.clientY - rect.top);
    if (!measureStart) {{
      measureStart = mc;
      measureEnd = null;
    }} else {{
      measureEnd = mc;
      // Measurement done — deactivate after showing
      markDirty();
    }}
    return;
  }}
  var rect = canvas.getBoundingClientRect();
  var hit = hitTest(e.clientX - rect.left, e.clientY - rect.top);
  showProps(hit);
}});

// ── Keyboard shortcuts ──
document.addEventListener('keydown', function(e) {{
  // Escape: close panel, cancel measure
  if (e.key === 'Escape') {{
    if (measureActive) {{
      measureActive = false;
      measureStart = null;
      measureEnd = null;
      document.getElementById('btn-measure').classList.remove('active');
      canvas.style.cursor = 'grab';
      markDirty();
    }} else {{
      closeProps();
    }}
    return;
  }}

  // F: fit to view
  if (e.key === 'f' || e.key === 'F') {{
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'SELECT') return;
    fit();
    markDirty();
    return;
  }}

  // M: measure tool
  if (e.key === 'm' || e.key === 'M') {{
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'SELECT') return;
    toggleMeasure();
    return;
  }}
}});

// ── Toolbar buttons ──
document.getElementById('btn-zin').addEventListener('click', function() {{
  var factor = 1.3;
  vx = W / 2 - (W / 2 - vx) * factor;
  vy = H / 2 - (H / 2 - vy) * factor;
  vs *= factor;
  markDirty();
}});

document.getElementById('btn-zout').addEventListener('click', function() {{
  var factor = 1 / 1.3;
  vx = W / 2 - (W / 2 - vx) * factor;
  vy = H / 2 - (H / 2 - vy) * factor;
  vs *= factor;
  markDirty();
}});

document.getElementById('btn-fit').addEventListener('click', function() {{
  fit();
  markDirty();
}});

document.getElementById('btn-measure').addEventListener('click', function() {{
  toggleMeasure();
}});

document.getElementById('btn-print').addEventListener('click', function() {{
  window.print();
}});

function toggleMeasure() {{
  measureActive = !measureActive;
  measureStart = null;
  measureEnd = null;
  document.getElementById('btn-measure').classList.toggle('active', measureActive);
  canvas.style.cursor = measureActive ? 'crosshair' : 'grab';
  markDirty();
}}

// ── Window resize ──
window.addEventListener('resize', function() {{
  resize();
  markDirty();
}});

/* ═══════════════════════════════════════════════════════════════
   Initialization
   ═══════════════════════════════════════════════════════════════ */
computeTransform();
resize();
fit();
buildLevelSelector();
buildLayers();
updateDeviceSummary();
render();
</script>
</body></html>"##,
        project = esc_html(&info.project_name),
        number = esc_html(&info.project_number),
        levels = levels_json_parts.join(","),
        categories = cat_json.join(","),
        total_placements = total_placements,
        initial_level = esc_html(level),
    );

    let size_kb = html.len() as f64 / 1024.0;
    std::fs::write(output, html)?;
    println!("Exported: {} ({:.1} KB)", output, size_kb);
    Ok(())
}

/// Export all levels with the first available level as the initial view.
pub fn export_html_all(file: &str, output: &str) -> Result<()> {
    let doc = SedDocument::open(file)?;
    let level_rows = doc.query_raw(
        "SELECT DISTINCT level FROM (
            SELECT s.level FROM spaces s JOIN geometry_polygons gp ON s.boundary_id = gp.id
            UNION
            SELECT p.level FROM placements p WHERE p.x IS NOT NULL
         ) sub ORDER BY level",
    )?;
    if level_rows.is_empty() {
        anyhow::bail!("No levels with positioned data found");
    }
    let first_level = level_rows[0][0].1.clone();
    drop(doc);
    export_html(file, output, &first_level)
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', " ")
}

fn esc_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
