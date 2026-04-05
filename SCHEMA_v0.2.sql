-- =============================================================================
-- SED Schema v0.2 — Structured Engineering Document
-- Grounded in: SKIMS Americana at Brand (Project 25-161)
--
-- Design goal: Make engineered systems legible to a machine.
-- Give this file to AI. AI does the work.
-- =============================================================================

PRAGMA user_version = 2;  -- schema version tracking

-- =============================================================================
-- PROJECT METADATA
-- =============================================================================

CREATE TABLE meta (
    key     TEXT PRIMARY KEY,
    value   TEXT NOT NULL
);
-- Required: sed_version, project_name, project_number, project_address,
--           units_display ('imperial'|'metric'), created_at, modified_at

CREATE TABLE directory (
    id          TEXT PRIMARY KEY,
    role        TEXT NOT NULL,       -- 'owner','architect','engineer_mep','contractor_general','contractor_mech', etc.
    company     TEXT NOT NULL,
    contact     TEXT,
    email       TEXT,
    phone       TEXT,
    address     TEXT
);

-- =============================================================================
-- SPACES
-- =============================================================================

CREATE TABLE spaces (
    id          TEXT PRIMARY KEY,
    tag         TEXT NOT NULL,       -- 'L1-01'
    name        TEXT NOT NULL,       -- 'Sales Area'
    level       TEXT NOT NULL,       -- 'Level 1'
    space_type  TEXT,               -- 'retail','office','storage','restroom','corridor','mechanical'
    area_m2     REAL,
    ceiling_ht_m REAL,
    scope       TEXT NOT NULL DEFAULT 'in_contract',  -- 'in_contract','nic','by_others','existing'
    parent_id   TEXT REFERENCES spaces(id),
    x           REAL,
    y           REAL,
    properties  TEXT                -- JSON
);

-- =============================================================================
-- PRODUCT TYPES (the catalog)
-- A "type" is a class of thing: LD-1 means "Titus FL-10, mud-in, Border 22"
-- =============================================================================

CREATE TABLE product_types (
    id              TEXT PRIMARY KEY,
    tag             TEXT NOT NULL UNIQUE,  -- 'LD-1', 'SR-1', 'CD-2', 'EF-1', 'FSD-1'
    domain          TEXT NOT NULL,         -- 'air_device','equipment','accessory'
    category        TEXT NOT NULL,         -- 'supply_diffuser','exhaust_fan','fire_smoke_damper', etc.
    manufacturer    TEXT,
    model           TEXT,
    description     TEXT,                  -- 'Aluminum architectural linear slot diffuser, 1" single slot, insulated plenum'
    mounting        TEXT,                  -- 'mud-in','lay-in','surface','duct_mounted','sidewall','suspended'
    finish          TEXT,
    size_nominal    TEXT,                  -- '10"x10"' for FSD, '8"' for round devices

    -- Performance envelope (for the product, not the instance)
    voltage         REAL,
    phase           INTEGER,
    hz              REAL,

    -- Submittal linkage
    submittal_id    TEXT REFERENCES submittals(id),

    properties      TEXT                   -- JSON for anything product-specific
);

-- From SKIMS:
-- tag='LD-1', domain='air_device', category='supply_diffuser',
--   manufacturer='Titus', model='FL-10', mounting='mud-in',
--   description='Aluminum architectural linear slot, 1" single slot, insulated plenum, Border 22'
-- tag='LD-2', domain='air_device', category='supply_diffuser',
--   manufacturer='Titus', model='FL-10', mounting='offset', description='Border 14 offset'
-- tag='SR-1', domain='air_device', category='supply_register',
--   manufacturer='Titus', model='S300FL', mounting='duct_mounted',
--   description='Double deflection direct spiral duct mounted with ASD air scoop'
-- tag='EF-1', domain='equipment', category='exhaust_fan',
--   manufacturer='Broan-NuTone', model='L-400L', voltage=120, phase=1, hz=60
-- tag='FSD-1', domain='accessory', category='fire_smoke_damper',
--   manufacturer='Pottorff', model='FSD-352', size_nominal='10"x10"'
-- tag='EXRTU-1', domain='equipment', category='rtu'
--   (no manufacturer/model — existing unit, field verify)

-- =============================================================================
-- PLACEMENTS (instances of product types in the building)
-- Each row is one physical thing at one location.
-- =============================================================================

CREATE TABLE placements (
    id              TEXT PRIMARY KEY,
    product_type_id TEXT NOT NULL REFERENCES product_types(id),

    -- Location
    space_id        TEXT REFERENCES spaces(id),
    level           TEXT NOT NULL,
    x               REAL,
    y               REAL,
    rotation        REAL DEFAULT 0,

    -- Instance-specific performance
    cfm             REAL,               -- design airflow for this specific instance
    cfm_balanced    REAL,               -- filled in during TAB phase
    static_pressure_pa REAL,            -- if relevant (fans)

    -- Status
    status          TEXT NOT NULL,       -- 'new','existing_remain','existing_remove','existing_relocate'
    scope           TEXT NOT NULL DEFAULT 'in_contract',

    -- Lifecycle phase tracking
    phase           TEXT NOT NULL DEFAULT 'design',  -- 'design','submitted','approved','installed','balanced','closed_out'

    -- Physical (for existing equipment)
    weight_kg       REAL,

    -- Graph connections (what system is this part of)
    system_id       TEXT REFERENCES systems(id),

    properties      TEXT,
    notes           TEXT
);

-- From SKIMS, each row is ONE physical item:
-- product_type='LD-1', space='Sales Area L1-01', cfm=185, status='new', phase='design'
-- product_type='LD-1', space='Sales Area L1-01', cfm=180, status='new', phase='design'
-- product_type='LD-1', space='Sales Area L1-01', cfm=185, status='new', phase='design'
-- ... (~20 more LD-1 instances across the sales floor)
-- product_type='LD-2', space='Fit Room 3 L1-04', cfm=85, status='new', phase='design'
-- product_type='LD-2', space='Fit Room 4 L1-05', cfm=85, status='new', phase='design'
-- product_type='EXRTU-1', space=NULL, level='Roof', status='existing_remain', weight_kg=738
-- product_type='EF-1', space='BOH Storage L2-09', cfm=130, status='new'
--   notes='Balance to 130 CFM. Extend new exhaust ductwork to existing exhaust main.'

-- =============================================================================
-- SYSTEMS (named air/fluid systems — the top-level grouping)
-- =============================================================================

CREATE TABLE systems (
    id          TEXT PRIMARY KEY,
    tag         TEXT NOT NULL UNIQUE,    -- 'RTU-1-SA', 'RTU-1-RA', 'RTU-2-SA', 'EX-1'
    name        TEXT NOT NULL,           -- 'RTU-1 Supply Air', 'Exhaust System 1'
    system_type TEXT NOT NULL,           -- 'supply','return','exhaust','outside_air','mixed_air'
    medium      TEXT NOT NULL DEFAULT 'air',  -- 'air','chilled_water','hot_water','steam','refrigerant','gas','condensate'
    source_id   TEXT REFERENCES placements(id),  -- the equipment that drives this system (RTU, AHU, pump, etc.)
    properties  TEXT
);

-- From SKIMS:
-- tag='RTU-1-SA', name='EXRTU-1 Supply Air', system_type='supply', medium='air', source=EXRTU-1
-- tag='RTU-1-RA', name='EXRTU-1 Return Air', system_type='return', medium='air', source=EXRTU-1
-- tag='RTU-2-SA', name='EXRTU-2 Supply Air', system_type='supply', medium='air', source=EXRTU-2
-- tag='EX-1',     name='Exhaust System 1',   system_type='exhaust', medium='air', source=EF-1

-- =============================================================================
-- THE GRAPH: Segments and Nodes
--
-- The distribution network is a directed tree (or DAG).
-- A segment is a straight run of duct/pipe between two nodes.
-- A node is a junction point: fitting, branch, terminal, equipment connection.
--
-- To trace from EXRTU-1 to a specific LD-1 in Fit Room 4:
--   Start at the equipment node → follow segments downstream →
--   through fitting nodes → to the terminal node (the air device)
-- =============================================================================

CREATE TABLE nodes (
    id              TEXT PRIMARY KEY,
    system_id       TEXT NOT NULL REFERENCES systems(id),
    node_type       TEXT NOT NULL,
    -- Node types:
    --   'equipment_conn'  — connection point on equipment (supply outlet, return inlet)
    --   'branch'          — tee or tap where flow splits
    --   'turn'            — elbow (45, 90)
    --   'transition'      — reducer, concentric or nonconcentric
    --   'terminal'        — where duct meets an air device or ends
    --   'end_cap'         — dead end
    --   'damper'          — inline damper (MVD, MOD, FSD, backdraft)
    --   'junction'        — any other connection point

    -- What fitting/device is at this node (if any)
    placement_id    TEXT REFERENCES placements(id),  -- links to the physical damper, device, etc.

    -- Fitting details (for fittings that aren't separate placements)
    fitting_type    TEXT,        -- 'elbow_45','elbow_90_short','tap_45','reducer_concentric','lateral_reducing','end_cap'
    size_description TEXT,       -- '8"', '14", 12"' (for reducers)

    -- Position
    level           TEXT,
    x               REAL,
    y               REAL,

    properties      TEXT
);

CREATE TABLE segments (
    id              TEXT PRIMARY KEY,
    system_id       TEXT NOT NULL REFERENCES systems(id),

    -- Graph edges: directed from upstream to downstream
    from_node_id    TEXT NOT NULL REFERENCES nodes(id),
    to_node_id      TEXT NOT NULL REFERENCES nodes(id),

    -- Duct/pipe properties
    shape           TEXT NOT NULL,       -- 'round','rectangular','oval'
    width_m         REAL,               -- rectangular
    height_m        REAL,               -- rectangular
    diameter_m      REAL,               -- round/oval
    length_m        REAL,

    -- Construction
    material        TEXT NOT NULL DEFAULT 'galvanized',  -- 'galvanized','aluminum','stainless','black_iron','pvc','copper'
    gauge           INTEGER,
    pressure_class  TEXT,               -- '2_in_wg','4_in_wg'
    construction    TEXT,               -- 'spiral_lock','longitudinal_seam','snap_lock','welded'

    -- Condition
    exposure        TEXT,               -- 'concealed','exposed'
    lining          TEXT,               -- 'none','acoustic_liner'
    insulation_type TEXT,               -- 'none','duct_wrap'
    insulation_r    REAL,               -- R-value

    -- Status
    status          TEXT NOT NULL,       -- 'new','existing_remain','existing_remove'
    scope           TEXT NOT NULL DEFAULT 'in_contract',

    properties      TEXT
);

-- Example: tracing EXRTU-1 supply air on Level 1
--
-- [EXRTU-1 supply outlet] ——28" SA——> [branch node] ——26" SA——> [branch] ——24" SA——> ...
--                                          |
--                                     [tap_45 node] ——8" SA——> [elbow_45] ——flex——> [MVD] ——> [LD-1 @ 185 CFM]
--
-- The AutoBid audit trail maps directly:
--   Line 1: Break Connected          → equipment_conn node
--   Line 2: Tap-45 Straight          → branch node (tap)
--   Line 3: Elbow-45 Degree          → turn node
--   Line 4: Duct - Flex              → segment (flex duct)
--   Line 5: Casco Damper             → damper node
--   Line 6: Tap-45 Straight          → next branch node
--   ...

-- =============================================================================
-- INSULATION (applied to segments)
-- =============================================================================

CREATE TABLE insulation (
    id              TEXT PRIMARY KEY,
    segment_id      TEXT REFERENCES segments(id),
    manufacturer    TEXT,
    product         TEXT,
    thickness_m     REAL,
    r_value         REAL,
    facing          TEXT,               -- 'fsk','psk','unfaced'
    code_reference  TEXT                -- 'CA Title 24'
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
    date_submitted  TEXT,               -- ISO 8601
    status          TEXT NOT NULL,       -- 'for_approval','approved','approved_as_noted','revise_resubmit','rejected'
    reviewed_by     TEXT,
    date_reviewed   TEXT,
    spec_section    TEXT,               -- '233713','233423' — CSI reference
    attachment_id   TEXT REFERENCES attachments(id),
    notes           TEXT
);

-- product_types link to submittals via product_types.submittal_id

-- =============================================================================
-- SHEETS (drawing output)
-- =============================================================================

CREATE TABLE sheets (
    id          TEXT PRIMARY KEY,
    number      TEXT NOT NULL UNIQUE,   -- 'M-001','M-101'
    title       TEXT NOT NULL,
    discipline  TEXT NOT NULL,          -- 'mechanical','electrical','plumbing','architectural','structural','fire_protection'
    scale       TEXT,
    level       TEXT,
    sheet_size  TEXT,
    vp_x_min    REAL,
    vp_y_min    REAL,
    vp_x_max    REAL,
    vp_y_max    REAL,
    properties  TEXT
);

-- =============================================================================
-- NOTES
-- =============================================================================

-- General notes: apply to the whole project or a discipline
CREATE TABLE general_notes (
    id          TEXT PRIMARY KEY,
    discipline  TEXT,                   -- NULL = applies to all
    text        TEXT NOT NULL,
    sort_order  INTEGER
);

-- Keyed notes: referenced by symbol on drawings, may link to specific elements
CREATE TABLE keyed_notes (
    id          TEXT PRIMARY KEY,
    key         TEXT NOT NULL UNIQUE,   -- 'H1','H2'
    text        TEXT NOT NULL,
    discipline  TEXT,
    spec_section TEXT                   -- CSI section this note relates to
);

CREATE TABLE keyed_note_refs (
    note_id     TEXT REFERENCES keyed_notes(id),
    placement_id TEXT,                  -- which element this note points at (nullable — some are location-based)
    sheet_id    TEXT REFERENCES sheets(id),
    x           REAL,
    y           REAL,
    PRIMARY KEY (note_id, sheet_id, x, y)
);

-- =============================================================================
-- REVISIONS
-- =============================================================================

CREATE TABLE revisions (
    id          TEXT PRIMARY KEY,
    number      INTEGER NOT NULL,
    name        TEXT NOT NULL,          -- 'CD Issue','Bid Set','Addendum 1'
    date        TEXT NOT NULL,
    description TEXT,
    author      TEXT
);

CREATE TABLE revision_changes (
    id              TEXT PRIMARY KEY,
    revision_id     TEXT REFERENCES revisions(id),
    table_name      TEXT NOT NULL,      -- which table was affected
    element_id      TEXT NOT NULL,      -- UUID of the changed row
    change_type     TEXT NOT NULL,      -- 'added','modified','removed'
    field           TEXT,               -- column name (NULL for add/remove)
    old_value       TEXT,
    new_value       TEXT
);

-- =============================================================================
-- ATTACHMENTS (embedded files)
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
-- SCHEDULE DATA (HVAC loads, ventilation calcs — from engineer)
-- =============================================================================

CREATE TABLE schedule_data (
    id              TEXT PRIMARY KEY,
    space_id        TEXT REFERENCES spaces(id),
    equipment_id    TEXT REFERENCES placements(id),

    -- Airflows
    cfm_supply      REAL,
    cfm_return      REAL,
    cfm_exhaust     REAL,
    cfm_outside_air REAL,

    -- Loads
    sensible_w      REAL,
    total_w         REAL,

    -- Ventilation (62.1 / Title 24)
    occupancy       INTEGER,
    oa_per_person   REAL,
    oa_per_area     REAL,

    properties      TEXT
);

-- =============================================================================
-- SPATIAL INDEX (for viewport culling and proximity queries)
-- =============================================================================

CREATE VIRTUAL TABLE spatial_idx USING rtree(
    id,
    x_min, x_max,
    y_min, y_max
);

-- Every placed element registers its bounding box here.
-- Enables: "show me everything on screen" and "what's within 6 inches of this duct"

-- =============================================================================
-- EXAMPLE QUERIES — what AI can do with this
-- =============================================================================

-- "List every supply diffuser on Level 1 with its CFM and room"
-- SELECT pt.tag, p.cfm, s.name, s.tag as room_tag
-- FROM placements p
-- JOIN product_types pt ON p.product_type_id = pt.id
-- JOIN spaces s ON p.space_id = s.id
-- WHERE pt.category = 'supply_diffuser' AND p.level = 'Level 1';

-- "Total supply CFM per room"
-- SELECT s.name, s.tag, SUM(p.cfm) as total_supply_cfm
-- FROM placements p
-- JOIN product_types pt ON p.product_type_id = pt.id
-- JOIN spaces s ON p.space_id = s.id
-- WHERE pt.domain = 'air_device' AND pt.category LIKE 'supply%'
-- GROUP BY s.id;

-- "Trace the duct path from EXRTU-1 to Fit Room 4"
-- WITH RECURSIVE path AS (
--     SELECT n.id, n.node_type, 0 as depth
--     FROM nodes n
--     JOIN placements p ON n.placement_id = p.id
--     JOIN product_types pt ON p.product_type_id = pt.id
--     WHERE pt.tag = 'EXRTU-1' AND n.node_type = 'equipment_conn'
--     UNION ALL
--     SELECT n2.id, n2.node_type, path.depth + 1
--     FROM path
--     JOIN segments seg ON seg.from_node_id = path.id
--     JOIN nodes n2 ON n2.id = seg.to_node_id
-- )
-- SELECT * FROM path;

-- "All items with submittals still pending"
-- SELECT pt.tag, pt.manufacturer, pt.model, sub.status, sub.date_submitted
-- FROM product_types pt
-- JOIN submittals sub ON pt.submittal_id = sub.id
-- WHERE sub.status = 'for_approval';

-- "What changed between Bid Set and Addendum 1"
-- SELECT rc.table_name, rc.element_id, rc.change_type, rc.field,
--        rc.old_value, rc.new_value
-- FROM revision_changes rc
-- JOIN revisions r ON rc.revision_id = r.id
-- WHERE r.name = 'Addendum 1';

-- "Count of all new items by product type"
-- SELECT pt.tag, pt.category, COUNT(*) as qty
-- FROM placements p
-- JOIN product_types pt ON p.product_type_id = pt.id
-- WHERE p.status = 'new'
-- GROUP BY pt.id
-- ORDER BY qty DESC;

-- "All rooms with exhaust but no supply"
-- SELECT s.name, s.tag FROM spaces s
-- WHERE s.id IN (
--     SELECT p.space_id FROM placements p
--     JOIN product_types pt ON p.product_type_id = pt.id
--     WHERE pt.category = 'exhaust_register'
-- ) AND s.id NOT IN (
--     SELECT p.space_id FROM placements p
--     JOIN product_types pt ON p.product_type_id = pt.id
--     WHERE pt.category LIKE 'supply%'
-- );
