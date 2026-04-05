-- =============================================================================
-- SED Schema v0.3 — Structured Engineering Document
-- Grounded in: SKIMS Americana at Brand (Project 25-161)
--
-- Thesis: A single file that contains an entire building's engineered systems
--         as a queryable database. Open it — you see the plans. Query it —
--         you get structured answers. Hand it to AI — it understands the building.
-- =============================================================================

PRAGMA user_version = 3;

-- =============================================================================
-- PROJECT METADATA
-- =============================================================================

CREATE TABLE meta (
    key     TEXT PRIMARY KEY,
    value   TEXT NOT NULL
);
-- Required keys:
--   sed_version, project_name, project_number, project_address,
--   units_display ('imperial'|'metric'), created_at, modified_at

CREATE TABLE directory (
    id          TEXT PRIMARY KEY,
    role        TEXT NOT NULL,
    company     TEXT NOT NULL,
    contact     TEXT,
    email       TEXT,
    phone       TEXT,
    address     TEXT
);

-- =============================================================================
-- GEOMETRY PRIMITIVES
--
-- Stored as compact binary (FlatBuffers BLOBs) for rendering performance,
-- with bounding box registered in the spatial index for viewport culling.
--
-- Coordinate system: project-local, meters, origin at building datum.
-- =============================================================================

-- Polygon geometries (room boundaries, zones, areas)
CREATE TABLE geometry_polygons (
    id              TEXT PRIMARY KEY,
    -- Vertices stored as packed float pairs: [x1,y1, x2,y2, ...]
    -- Closed polygon (last vertex connects to first)
    vertices        BLOB NOT NULL,
    vertex_count    INTEGER NOT NULL,
    level           TEXT NOT NULL
);

-- Polyline geometries (walls, architectural context lines)
CREATE TABLE geometry_polylines (
    id              TEXT PRIMARY KEY,
    vertices        BLOB NOT NULL,
    vertex_count    INTEGER NOT NULL,
    level           TEXT NOT NULL,
    line_type       TEXT,           -- 'wall','partition','curtain_wall','storefront','column_grid'
    weight          REAL,           -- line weight for rendering
    properties      TEXT
);

-- =============================================================================
-- SPACES
-- =============================================================================

CREATE TABLE spaces (
    id              TEXT PRIMARY KEY,
    tag             TEXT NOT NULL,       -- 'L1-01'
    name            TEXT NOT NULL,       -- 'Sales Area'
    level           TEXT NOT NULL,       -- 'Level 1'
    space_type      TEXT,               -- 'retail','office','storage','restroom','corridor','mechanical'
    area_m2         REAL,
    ceiling_ht_m    REAL,
    scope           TEXT NOT NULL DEFAULT 'in_contract',
    parent_id       TEXT REFERENCES spaces(id),
    boundary_id     TEXT REFERENCES geometry_polygons(id),  -- room boundary for rendering
    x               REAL,               -- label placement point
    y               REAL,
    properties      TEXT
);

-- =============================================================================
-- PRODUCT TYPES (the catalog)
-- =============================================================================

CREATE TABLE product_types (
    id              TEXT PRIMARY KEY,
    tag             TEXT NOT NULL UNIQUE,    -- 'LD-1','EXRTU-1','FSD-1'
    domain          TEXT NOT NULL,           -- 'air_device','equipment','accessory'
    category        TEXT NOT NULL,           -- 'supply_diffuser','rtu','fire_smoke_damper', etc.
    manufacturer    TEXT,
    model           TEXT,
    description     TEXT,
    mounting        TEXT,
    finish          TEXT,
    size_nominal    TEXT,

    voltage         REAL,
    phase           INTEGER,
    hz              REAL,

    submittal_id    TEXT REFERENCES submittals(id),
    properties      TEXT
);

-- =============================================================================
-- PLACEMENTS (instances in the building)
-- =============================================================================

CREATE TABLE placements (
    id              TEXT PRIMARY KEY,
    product_type_id TEXT NOT NULL REFERENCES product_types(id),

    space_id        TEXT REFERENCES spaces(id),
    level           TEXT NOT NULL,
    x               REAL,
    y               REAL,
    rotation        REAL DEFAULT 0,

    -- Instance-specific performance
    cfm             REAL,
    cfm_balanced    REAL,
    static_pressure_pa REAL,

    status          TEXT NOT NULL,       -- 'new','existing_remain','existing_remove','existing_relocate'
    scope           TEXT NOT NULL DEFAULT 'in_contract',
    phase           TEXT NOT NULL DEFAULT 'design',

    weight_kg       REAL,
    system_id       TEXT REFERENCES systems(id),

    properties      TEXT,
    notes           TEXT
);

-- =============================================================================
-- SYSTEMS
-- =============================================================================

CREATE TABLE systems (
    id          TEXT PRIMARY KEY,
    tag         TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    system_type TEXT NOT NULL,       -- 'supply','return','exhaust','outside_air'
    medium      TEXT NOT NULL DEFAULT 'air',
    source_id   TEXT REFERENCES placements(id),
    properties  TEXT
);

-- =============================================================================
-- THE GRAPH
-- =============================================================================

CREATE TABLE nodes (
    id              TEXT PRIMARY KEY,
    system_id       TEXT NOT NULL REFERENCES systems(id),
    node_type       TEXT NOT NULL,
    -- 'equipment_conn','branch','turn','transition','terminal',
    -- 'end_cap','damper','junction'

    placement_id    TEXT REFERENCES placements(id),
    fitting_type    TEXT,
    size_description TEXT,

    level           TEXT,
    x               REAL,
    y               REAL,
    properties      TEXT
);

CREATE TABLE segments (
    id              TEXT PRIMARY KEY,
    system_id       TEXT NOT NULL REFERENCES systems(id),

    from_node_id    TEXT NOT NULL REFERENCES nodes(id),
    to_node_id      TEXT NOT NULL REFERENCES nodes(id),

    -- Geometry
    shape           TEXT NOT NULL,       -- 'round','rectangular','oval','flex'
    width_m         REAL,
    height_m        REAL,
    diameter_m      REAL,
    length_m        REAL,

    -- Construction
    material        TEXT NOT NULL DEFAULT 'galvanized',
    gauge           INTEGER,
    pressure_class  TEXT,
    construction    TEXT,               -- 'spiral_lock','longitudinal_seam','snap_lock','welded'

    -- Condition
    exposure        TEXT,               -- 'concealed','exposed'

    -- Airflow / fluid flow — design intent at this point in the network
    flow_design     REAL,               -- design flow rate (CFM stored as m3/s)
    flow_balanced   REAL,               -- as-balanced (filled during TAB)

    -- Status
    status          TEXT NOT NULL,
    scope           TEXT NOT NULL DEFAULT 'in_contract',

    properties      TEXT
);

-- =============================================================================
-- INSULATION (applied to segments, not duplicated on segments table)
-- =============================================================================

CREATE TABLE insulation (
    id              TEXT PRIMARY KEY,
    segment_id      TEXT REFERENCES segments(id),
    type            TEXT NOT NULL,       -- 'duct_wrap','acoustic_liner','pipe_insulation'
    manufacturer    TEXT,
    product         TEXT,
    thickness_m     REAL,
    r_value         REAL,
    facing          TEXT,
    code_reference  TEXT
);

-- =============================================================================
-- SUBMITTALS
-- =============================================================================

CREATE TABLE submittals (
    id              TEXT PRIMARY KEY,
    number          TEXT,
    description     TEXT NOT NULL,
    submitted_by    TEXT,
    company         TEXT,
    date_submitted  TEXT,
    status          TEXT NOT NULL,
    reviewed_by     TEXT,
    date_reviewed   TEXT,
    spec_section    TEXT,
    attachment_id   TEXT REFERENCES attachments(id),
    notes           TEXT
);

-- =============================================================================
-- ATTACHMENTS
-- =============================================================================

CREATE TABLE attachments (
    id          TEXT PRIMARY KEY,
    filename    TEXT NOT NULL,
    mime_type   TEXT NOT NULL,
    size_bytes  INTEGER,
    data        BLOB,
    description TEXT
);

-- =============================================================================
-- SHEETS AND VIEWS
-- =============================================================================

CREATE TABLE sheets (
    id          TEXT PRIMARY KEY,
    number      TEXT NOT NULL UNIQUE,
    title       TEXT NOT NULL,
    discipline  TEXT NOT NULL,
    sheet_size  TEXT,                -- 'ARCH_D','24x36','A1'
    properties  TEXT
);

-- A sheet contains one or more views (viewports).
-- A plan view shows a region of a level at a scale.
-- A detail view shows a zoomed-in construction detail.
-- A schedule view shows a tabular data view.
CREATE TABLE views (
    id              TEXT PRIMARY KEY,
    sheet_id        TEXT NOT NULL REFERENCES sheets(id),
    view_type       TEXT NOT NULL,       -- 'plan','detail','section','schedule','legend','title_block'
    title           TEXT,                -- 'Mechanical Ductwork Plan - Level 1', 'Diffuser Installation Typical'
    scale           TEXT,                -- '1/4" = 1'-0"', '3/4" = 1'-0"', 'NTS'
    level           TEXT,                -- for plan views

    -- Viewport bounds within the sheet (in sheet coordinates, e.g. inches from bottom-left)
    vp_x            REAL,
    vp_y            REAL,
    vp_width        REAL,
    vp_height       REAL,

    -- Model bounds (what region of the building this view shows, in model coordinates)
    model_x_min     REAL,
    model_y_min     REAL,
    model_x_max     REAL,
    model_y_max     REAL,

    -- For detail/section views: reference geometry
    -- (callout bubble location on the parent view)
    parent_view_id  TEXT REFERENCES views(id),
    callout_x       REAL,
    callout_y       REAL,

    properties      TEXT
);

-- From SKIMS:
-- Sheet M-101 has 1 view: plan of Level 1 at 1/4" = 1'-0"
-- Sheet M-104 has multiple detail views:
--   'Diffuser Installation Typical' (233713.00-04), scale NTS
--   'Plenum/Linear Diffuser w/ Young Reg.' (233713.00-12), scale NTS
--   'Cabinet Inline Fan' (233423.00-03), scale NTS
--   'Manual Damper Detail' (233300.00-01), scale NTS

-- =============================================================================
-- ANNOTATIONS
--
-- Drawing annotations live in view-space. They are the "ink" layer —
-- dimensions, leaders, text labels, revision clouds, section marks.
-- They reference model elements but exist for visual communication.
-- =============================================================================

CREATE TABLE annotations (
    id              TEXT PRIMARY KEY,
    view_id         TEXT NOT NULL REFERENCES views(id),
    anno_type       TEXT NOT NULL,
    -- 'dimension'         — distance measurement between two points
    -- 'leader'            — line from a note to an element
    -- 'text'              — standalone text label
    -- 'tag'               — element tag (auto-generated from element data)
    -- 'revision_cloud'    — clouded region indicating a change
    -- 'section_mark'      — section/detail callout symbol
    -- 'matchline'         — where the drawing continues on another sheet
    -- 'north_arrow'
    -- 'graphic_scale'

    -- What element this annotation references (nullable — some are standalone)
    ref_table       TEXT,               -- 'placements','segments','nodes','spaces'
    ref_id          TEXT,

    -- Geometry (view coordinates)
    -- For dimensions: two endpoints + offset
    -- For leaders: anchor point + elbow + endpoint
    -- For text: insertion point
    -- For clouds: bounding polygon
    x1              REAL,
    y1              REAL,
    x2              REAL,
    y2              REAL,
    geometry        BLOB,               -- additional geometry if needed (cloud vertices, leader path)

    -- Content
    text            TEXT,               -- display text (for text/tags: the label; for dimensions: override text or NULL for auto)
    text_height     REAL,
    text_rotation   REAL,

    -- Revision tracking
    revision_id     TEXT REFERENCES revisions(id),  -- for revision clouds: which revision

    properties      TEXT
);

-- =============================================================================
-- NOTES
-- =============================================================================

CREATE TABLE general_notes (
    id          TEXT PRIMARY KEY,
    discipline  TEXT,
    text        TEXT NOT NULL,
    sort_order  INTEGER
);

CREATE TABLE keyed_notes (
    id          TEXT PRIMARY KEY,
    key         TEXT NOT NULL UNIQUE,
    text        TEXT NOT NULL,
    discipline  TEXT,
    spec_section TEXT
);

CREATE TABLE keyed_note_refs (
    note_id     TEXT REFERENCES keyed_notes(id),
    placement_id TEXT,
    view_id     TEXT REFERENCES views(id),
    x           REAL,
    y           REAL,
    PRIMARY KEY (note_id, view_id, x, y)
);

-- =============================================================================
-- REVISIONS
-- =============================================================================

CREATE TABLE revisions (
    id          TEXT PRIMARY KEY,
    number      INTEGER NOT NULL,
    name        TEXT NOT NULL,
    date        TEXT NOT NULL,
    description TEXT,
    author      TEXT
);

CREATE TABLE revision_changes (
    id              TEXT PRIMARY KEY,
    revision_id     TEXT REFERENCES revisions(id),
    table_name      TEXT NOT NULL,
    element_id      TEXT NOT NULL,
    change_type     TEXT NOT NULL,
    field           TEXT,
    old_value       TEXT,
    new_value       TEXT
);

-- =============================================================================
-- SCHEDULE DATA
-- =============================================================================

CREATE TABLE schedule_data (
    id              TEXT PRIMARY KEY,
    space_id        TEXT REFERENCES spaces(id),
    equipment_id    TEXT REFERENCES placements(id),

    cfm_supply      REAL,
    cfm_return      REAL,
    cfm_exhaust     REAL,
    cfm_outside_air REAL,

    sensible_w      REAL,
    total_w         REAL,

    occupancy       INTEGER,
    oa_per_person   REAL,
    oa_per_area     REAL,

    properties      TEXT
);

-- =============================================================================
-- SPATIAL INDEX
-- =============================================================================

CREATE VIRTUAL TABLE spatial_idx USING rtree(
    id,
    x_min, x_max,
    y_min, y_max
);

-- =============================================================================
-- WHAT THIS FILE CAN NOW DO
--
-- RENDER: Spaces have boundary polygons. Segments trace through nodes with
--   coordinates. Placements have positions. Annotations provide dimensions,
--   tags, and leaders. Views define what to show at what scale on each sheet.
--   A renderer reads this and draws a mechanical plan.
--
-- QUERY: Every element is typed, located, and connected. AI can answer:
--   "Total supply CFM on Level 1?"
--   "Trace the duct from EXRTU-1 to Fit Room 4."
--   "What submittals are still pending?"
--   "What changed in the bid set?"
--   "Show me all rooms with exhaust but no return path."
--
-- EVOLVE: The phase field on placements tracks the lifecycle. The file
--   starts as design intent and accumulates reality — submitted products,
--   approved submittals, installed conditions, balanced airflows.
--
-- INTEROP: The format is SQLite. Any language can read it. Any tool can
--   query it. The spec is open. The SDK is MIT. The viewer is free.
-- =============================================================================
