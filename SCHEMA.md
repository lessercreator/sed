# SED Schema v0.1 — Structured Engineering Document

## Grounded in: SKIMS Americana at Brand (Project 25-161)
## Source data: Mechanical plans (KLH), AutoBid takeoffs (Air-Tec), Submittals (Toro-Aire, So-Cal)

---

## Design Principles

1. **Every element has a unique ID and a human-readable tag.** The ID is a UUID for machine use. The tag is what appears on the drawing (e.g., "EF-1", "LD-1").
2. **Units are SI internally, display units are per-locale.** CFM stored as m3/s, inches stored as meters. The viewer converts.
3. **The drawing is a view, not the source.** Sheet layout references elements by ID. Move the element in the model, the drawing updates.
4. **Submittals attach to elements.** Manufacturer data is linked, not separate.
5. **The schema is append-friendly.** New optional columns can be added without breaking existing files.

---

## SQLite Tables

### `meta`
Project-level metadata.

```sql
CREATE TABLE meta (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL
);

-- Required keys:
-- 'sed_version'      -> '0.1'
-- 'project_name'     -> 'SKIMS Americana at Brand'
-- 'project_number'   -> '25-161'
-- 'project_address'  -> '233 South Brand Blvd, Glendale, CA 92106'
-- 'client'           -> 'SKIMS Retail, LLC'
-- 'architect'        -> 'RDC'
-- 'engineer_mep'     -> 'KLH Engineers'
-- 'contractor_mech'  -> 'Air-Tec'
-- 'units_display'    -> 'imperial'  (or 'metric')
-- 'created_at'       -> ISO 8601 timestamp
-- 'modified_at'      -> ISO 8601 timestamp
```

### `directory`
Project contacts — everyone who touches the documents.

```sql
CREATE TABLE directory (
    id          TEXT PRIMARY KEY,  -- UUID
    role        TEXT NOT NULL,     -- 'owner', 'architect', 'engineer_mep', 'engineer_struct', 'contractor_general', 'contractor_mech', 'contractor_controls', 'code_official', etc.
    company     TEXT NOT NULL,
    contact     TEXT,
    email       TEXT,
    phone       TEXT,
    address     TEXT
);

-- From SKIMS:
-- role='owner',           company='Caruso',              contact='Ken Greenberg'
-- role='tenant',          company='SKIMS Retail, LLC',   contact='Robbie Zweig'
-- role='architect',       company='RDC',                 contact='James Botha'
-- role='engineer_mep',    company='KLH Engineers',       contact='Jordan Laycock'
-- role='engineer_struct', company='RMJ & Associates',    contact='Jayson Haines'
-- role='contractor_mech', company='Air-Tec',             contact=NULL
```

### `spaces`
Rooms, zones, and areas. The spatial hierarchy.

```sql
CREATE TABLE spaces (
    id          TEXT PRIMARY KEY,  -- UUID
    tag         TEXT NOT NULL,     -- 'L1-01', 'L2-08'
    name        TEXT NOT NULL,     -- 'Sales Area', 'Managers Office'
    level       TEXT NOT NULL,     -- 'Level 1', 'Level 2', 'Roof'
    area_sf     REAL,             -- square feet (stored as m2 internally)
    ceiling_ht  REAL,             -- ceiling height
    space_type  TEXT,             -- 'retail', 'office', 'storage', 'restroom', 'corridor', 'mechanical', 'elevator'
    parent_id   TEXT REFERENCES spaces(id),  -- for zone grouping
    x           REAL,             -- centroid X for spatial indexing
    y           REAL,             -- centroid Y
    properties  TEXT              -- JSON for extensible attributes
);

-- From SKIMS plans:
-- tag='L1-01', name='Sales Area',        level='Level 1', space_type='retail'
-- tag='L1-02', name='Fit Room 1',        level='Level 1', space_type='retail'
-- tag='L1-03', name='Fit Room 2 ADA',    level='Level 1', space_type='retail'
-- tag='L1-04', name='Fit Room 3',        level='Level 1', space_type='retail'
-- tag='L1-05', name='Fit Room 4',        level='Level 1', space_type='retail'
-- tag='L1-06', name='Fit Room 5',        level='Level 1', space_type='retail'
-- tag='L1-07', name='Fit Room 6',        level='Level 1', space_type='retail'
-- tag='L1-08', name='Fit Room 7',        level='Level 1', space_type='retail'
-- tag='L1-09', name='Fit Room 8',        level='Level 1', space_type='retail'
-- tag='L1-10', name='Elevator Shaft',    level='Level 1', space_type='elevator'
-- tag='L1-11', name='Go-Backs',          level='Level 1', space_type='storage'
-- tag='L1-12', name='BOH Storage',       level='Level 1', space_type='storage'
-- tag='L1-13', name='Corridor 1',        level='Level 1', space_type='corridor'
-- tag='L2-01', name='Unused',            level='Level 2', space_type='storage'
-- tag='L2-02', name='Corridor 2',        level='Level 2', space_type='corridor'
-- tag='L2-03', name='BOH Storage',       level='Level 2', space_type='storage'
-- tag='L2-04', name='Corridor 3',        level='Level 2', space_type='corridor'
-- tag='L2-05', name='Break Area',        level='Level 2', space_type='office'
-- tag='L2-06', name='Restroom 1',        level='Level 2', space_type='restroom'
-- tag='L2-07', name='Restroom 2',        level='Level 2', space_type='restroom'
-- tag='L2-08', name='Managers Office',   level='Level 2', space_type='office'
-- tag='L2-09', name='BOH Storage',       level='Level 2', space_type='storage'
-- tag='L2-10', name='Riser Shaft',       level='Level 2', space_type='mechanical'
-- tag='L2-12', name='Mop Closet',        level='Level 2', space_type='storage'
```

### `equipment`
Major mechanical equipment.

```sql
CREATE TABLE equipment (
    id              TEXT PRIMARY KEY,
    tag             TEXT NOT NULL UNIQUE,  -- 'EXRTU-1', 'EXRTU-2', 'EF-1'
    category        TEXT NOT NULL,         -- 'rtu', 'exhaust_fan', 'split_system', 'chiller', 'boiler', 'pump', etc.
    status          TEXT NOT NULL,         -- 'new', 'existing_remain', 'existing_remove', 'existing_relocate'
    level           TEXT,
    space_id        TEXT REFERENCES spaces(id),
    
    -- Nameplate / performance
    manufacturer    TEXT,
    model           TEXT,
    cfm_supply      REAL,
    cfm_return      REAL,
    cfm_exhaust     REAL,
    cooling_cap     REAL,       -- total cooling capacity (stored as watts)
    heating_cap     REAL,       -- total heating capacity (stored as watts)
    static_pressure REAL,       -- external static pressure (stored as Pa)
    
    -- Electrical
    voltage         REAL,
    phase           INTEGER,
    hz              REAL,
    fla             REAL,
    mca             REAL,
    mocp            REAL,
    
    -- Physical
    weight_lbs      REAL,
    
    -- Coordinates for drawing placement
    x               REAL,
    y               REAL,
    rotation        REAL,       -- degrees
    
    properties      TEXT,       -- JSON for anything else
    notes           TEXT
);

-- From SKIMS:
-- tag='EXRTU-1', category='rtu', status='existing_remain', level='Roof', weight_lbs=1627
--   notes='Existing rooftop unit to remain. Balance to scheduled airflow. Clean and verify proper operation; clean cooling, heating coils, recharge refrigerant, replace belt, drive, and motor as required, replace filters. Check compressor and fans, replace/repair as required. Provide owner with reconditioning report prior to turnover.'
-- tag='EXRTU-2', category='rtu', status='existing_remain', level='Roof', weight_lbs=1627
--   notes=(same as EXRTU-1)
-- tag='EF-1', category='exhaust_fan', status='new', level='Level 2',
--   manufacturer='Broan-NuTone', model='L-400L', cfm_exhaust=210, static_pressure=0.5 inWG,
--   voltage=120, phase=1, hz=60
--   notes='New inline exhaust fan. Balance to 130 CFM. Extend new exhaust ductwork to existing exhaust main.'
```

### `air_devices`
Diffusers, registers, grilles, transfer grilles.

```sql
CREATE TABLE air_devices (
    id              TEXT PRIMARY KEY,
    tag             TEXT NOT NULL,         -- 'LD-1', 'SR-1', 'CD-1', 'ER-1', 'RG-1', 'TG-1'
    category        TEXT NOT NULL,         -- 'supply_diffuser', 'supply_register', 'return_grille', 'exhaust_register', 'transfer_grille', 'ceiling_diffuser'
    status          TEXT NOT NULL,         -- 'new', 'existing_remain', 'existing_remove'
    space_id        TEXT REFERENCES spaces(id),
    
    -- Performance
    cfm_design      REAL,                 -- design airflow
    cfm_balanced    REAL,                 -- as-balanced (filled in during TAB)
    neck_size       TEXT,                 -- '8"', '6"', '10"x10"'
    
    -- Product
    manufacturer    TEXT,
    model           TEXT,
    finish          TEXT,
    mounting        TEXT,                 -- 'lay-in', 'mud-in', 'surface', 'duct_mounted', 'sidewall'
    
    -- Drawing placement
    x               REAL,
    y               REAL,
    rotation        REAL,
    level           TEXT,
    
    properties      TEXT,
    notes           TEXT
);

-- From SKIMS plans + AD submittal:
-- tag='LD-1', category='supply_diffuser', cfm_design=180-185, manufacturer='Titus', model='FL-10',
--   mounting='mud-in', finish='standard white', notes='1" wide single slot with insulated plenum'
--   (appears ~20 times across Sales Area)
-- tag='LD-2', category='supply_diffuser', cfm_design=85-150, manufacturer='Titus', model='FL-10',
--   mounting='offset', finish='standard white'
--   (appears in fit rooms and corridors)
-- tag='SR-1', category='supply_register', cfm_design=90-135, manufacturer='Titus', model='S300FL',
--   mounting='duct_mounted', notes='with ASD air scoop'
-- tag='SR-2', category='supply_register', cfm_design=50-65, manufacturer='Titus', model='S300FL',
--   mounting='duct_mounted'
-- tag='CD-1', category='ceiling_diffuser', cfm_design=100, manufacturer='Titus', model='OMNI',
--   mounting='lay-in', notes='steel square plaque, border 3'
-- tag='CD-2', category='ceiling_diffuser', cfm_design=40-190, manufacturer='Titus', model='OMNI',
--   mounting='surface', notes='12x12 module, border 3, with TRM frame for hard lid'
-- tag='ER-1', category='exhaust_register', cfm_design=50-80, manufacturer='Titus', model='350FL',
--   mounting='surface'
-- tag='RG-1', category='return_grille', manufacturer='Titus', model='350FL', mounting='lay-in'
-- tag='RG-2', category='return_grille', manufacturer='Titus', model='350FL', mounting='surface'
-- tag='TG-1', category='transfer_grille', manufacturer='Titus', model='350FL', mounting='surface'
-- tag='TG-2', category='transfer_grille', manufacturer='Titus', model='350FL', mounting='surface'
-- tag='TG-3', category='transfer_grille', manufacturer='Titus', model='350FL', mounting='surface'
```

### `ducts`
Duct segments — the edges of the air distribution graph.

```sql
CREATE TABLE ducts (
    id              TEXT PRIMARY KEY,
    system          TEXT NOT NULL,         -- 'supply', 'return', 'exhaust'
    status          TEXT NOT NULL,         -- 'new', 'existing_remain', 'existing_remove'
    level           TEXT NOT NULL,
    
    -- Geometry
    shape           TEXT NOT NULL,         -- 'round', 'rectangular', 'oval'
    width           REAL,                 -- for rectangular (stored in meters)
    height          REAL,                 -- for rectangular
    diameter        REAL,                 -- for round/oval
    length          REAL,                 -- segment length
    
    -- Properties
    gauge           INTEGER,              -- metal gauge
    material        TEXT,                 -- 'galvanized', 'aluminum', 'stainless', 'black_iron'
    pressure_class  TEXT,                 -- '2_in_wg', '4_in_wg', etc.
    lining          TEXT,                 -- 'none', 'acoustic_liner', 'wrapped'
    insulation      TEXT,                 -- 'none', '1.5in_duct_wrap', '2in_duct_wrap'
    exposure        TEXT,                 -- 'concealed', 'exposed'
    
    -- Connection graph (what this duct connects)
    from_node_id    TEXT,                 -- UUID of upstream equipment/fitting/duct
    from_node_type  TEXT,                 -- 'equipment', 'air_device', 'fitting', 'duct'
    to_node_id      TEXT,                 -- UUID of downstream equipment/fitting/duct
    to_node_type    TEXT,
    
    -- Drawing
    x1              REAL,
    y1              REAL,
    x2              REAL,
    y2              REAL,
    
    properties      TEXT
);

-- From SKIMS AutoBid data:
-- Level 1 RTU Supply: 274 ft of round spiral lock (6", 8", 14", 16", 18", 20", 24")
--   26 gauge for 6"-8", 24 gauge for 14"-24"
--   Material: galvanized, 2" WG pressure class, SMACNA LP
--   Fittings: 60 spiral lock, 33 elbows-45, 14 elbows-90-short, 6 reducers, 32 taps-45
-- Level 1 RTU SA Exposed: round 8", 14", 16" (35 ft)
-- Level 1 RTU RA Exposed: rectangular 28"x22" (3.67 ft) with turning vanes
-- Level 2 Exhaust: round 6", 8" + rectangular (55 ft total)
-- Level 2 RTU SA Exposed: round 8"-28" (main trunk, 512 lbs)
```

### `fittings`
Duct fittings — elbows, tees, taps, reducers, etc.

```sql
CREATE TABLE fittings (
    id              TEXT PRIMARY KEY,
    category        TEXT NOT NULL,         -- 'elbow_45', 'elbow_90', 'tee', 'tap_45', 'reducer_concentric', 'reducer_nonconcentric', 'end_cap', 'lateral_reducing', 'transition'
    shape           TEXT NOT NULL,         -- 'round', 'rectangular'
    size_in         TEXT,                 -- '8"', '14", 12"' (for reducers)
    gauge           INTEGER,
    material        TEXT,
    
    -- Graph position
    duct_id         TEXT REFERENCES ducts(id),  -- which duct run this fitting is on
    position        REAL,                       -- distance along the duct from start
    
    x               REAL,
    y               REAL,
    rotation        REAL,
    level           TEXT,
    
    properties      TEXT
);
```

### `accessories`
Dampers, smoke detectors, access doors, etc.

```sql
CREATE TABLE accessories (
    id              TEXT PRIMARY KEY,
    tag             TEXT,                 -- 'FSD-1', 'SD-1'
    category        TEXT NOT NULL,        -- 'fire_smoke_damper', 'manual_volume_damper', 'backdraft_damper', 'motor_operated_damper', 'smoke_detector', 'access_door', 'turning_vanes', 'flex_connector'
    status          TEXT NOT NULL,
    
    -- Product
    manufacturer    TEXT,
    model           TEXT,
    size            TEXT,                 -- '10"x10"'
    
    -- For dampers
    fire_rating     TEXT,                 -- '3_hour'
    leakage_class   TEXT,                 -- 'class_2'
    actuator_model  TEXT,
    actuator_voltage TEXT,
    
    -- Location
    duct_id         TEXT REFERENCES ducts(id),
    space_id        TEXT REFERENCES spaces(id),
    level           TEXT,
    x               REAL,
    y               REAL,
    
    properties      TEXT,
    notes           TEXT
);

-- From SKIMS:
-- FSD-1: Pottorff FSD-352, 10"x10", 3-hour, UL Class 2, FSLF120 actuator 120V
-- 32x Casco manual volume dampers (6" and 8") on Level 1 supply
-- 3x Casco manual volume dampers (6") on Level 2 exhaust
-- Motor operated damper (1x, Level 2 corridor)
-- Smoke detector in supply duct (keyed note H5)
```

### `insulation`
Insulation specs applied to duct runs.

```sql
CREATE TABLE insulation (
    id              TEXT PRIMARY KEY,
    duct_id         TEXT REFERENCES ducts(id),  -- which duct segment
    type            TEXT NOT NULL,               -- 'duct_wrap', 'acoustic_liner', 'none'
    material        TEXT,                        -- 'fiberglass'
    manufacturer    TEXT,
    product         TEXT,
    thickness       REAL,                        -- stored in meters
    density         REAL,                        -- pcf or kg/m3
    r_value         REAL,
    facing          TEXT,                        -- 'fsk', 'psk', 'unfaced'
    code_reference  TEXT                         -- 'CA Title 24'
);

-- From SKIMS:
-- CertainTeed SoftTouch, 1.5" thick, 0.75 pcf, R-4.2, FSK facing
-- Applied to: new concealed unlined supply air duct
-- General note A: first 15 ft acoustically lined, remainder wrapped if concealed
```

### `submittals`
Submittal tracking — links products to elements.

```sql
CREATE TABLE submittals (
    id              TEXT PRIMARY KEY,
    submittal_number TEXT,
    description     TEXT NOT NULL,
    submitted_by    TEXT,                 -- contact name
    submitted_via   TEXT,                 -- company (rep)
    submitted_date  TEXT,                 -- ISO 8601
    status          TEXT NOT NULL,        -- 'for_approval', 'approved', 'approved_as_noted', 'revise_resubmit', 'rejected', 'for_record'
    reviewed_by     TEXT,
    reviewed_date   TEXT,
    
    -- Links to what this submittal covers
    -- (many-to-many via submittal_items)
    
    attachment_id   TEXT REFERENCES attachments(id),  -- the actual PDF/document
    notes           TEXT
);

CREATE TABLE submittal_items (
    submittal_id    TEXT REFERENCES submittals(id),
    element_type    TEXT NOT NULL,        -- 'equipment', 'air_device', 'accessory', 'insulation', 'duct'
    element_id      TEXT NOT NULL,        -- UUID of the element
    PRIMARY KEY (submittal_id, element_type, element_id)
);

-- From SKIMS:
-- Submittal: FSD, submitted by Dasha Perkins (Toro-Aire), 03/18/2026, status='for_approval'
--   covers: FSD-1 (accessory)
-- Submittal: AD, submitted by Dasha Perkins (So-Cal rep), 03/18/2026, status='for_approval'
--   covers: CD-1, CD-2, ER-1, RG-1, RG-2, TG-1, TG-2, TG-3, SR-1, SR-2, LD-1, LD-2, LR-1
-- Submittal: EF, submitted by Dasha Perkins (So-Cal rep), 03/18/2026, status='for_approval'
--   covers: EF-1 (equipment)
-- Submittal: Insulation, submitted by Mark Schaefer (So-Cal Insulation), status='for_approval'
--   covers: all new concealed supply ductwork
-- Submittal: Duct standards, 2" pressure class shop standards (Air-Tec)
--   covers: all new ductwork
```

### `attachments`
Embedded files — manufacturer cut sheets, photos, spec sections.

```sql
CREATE TABLE attachments (
    id              TEXT PRIMARY KEY,
    filename        TEXT NOT NULL,
    mime_type       TEXT NOT NULL,        -- 'application/pdf', 'image/png', etc.
    size_bytes      INTEGER,
    data            BLOB,                -- the actual file content
    description     TEXT
);
```

### `sheets`
Drawing sheets for visual output.

```sql
CREATE TABLE sheets (
    id              TEXT PRIMARY KEY,
    number          TEXT NOT NULL UNIQUE, -- 'M-001', 'M-101', 'M-102', 'M-103', 'M-104', 'M-105'
    title           TEXT NOT NULL,        -- 'Mechanical Cover Sheet', 'Mechanical Ductwork Plan - Level 1'
    discipline      TEXT NOT NULL,        -- 'mechanical', 'electrical', 'plumbing', 'architectural', 'structural'
    scale           TEXT,                 -- '1/4" = 1'-0"'
    level           TEXT,                 -- which level this sheet primarily shows
    sheet_size      TEXT,                 -- 'ARCH D', '24x36', 'A1'
    
    -- Viewport bounds (what area of the model this sheet shows)
    vp_x_min        REAL,
    vp_y_min        REAL,
    vp_x_max        REAL,
    vp_y_max        REAL,
    
    properties      TEXT
);

-- From SKIMS:
-- 'M-001', 'Mechanical Cover Sheet', 'mechanical'
-- 'M-101', 'Mechanical Ductwork Plan - Level 1', 'mechanical', '1/4" = 1'-0"', 'Level 1'
-- 'M-102', 'Mechanical Ductwork Plan - Level 2', 'mechanical', '1/4" = 1'-0"', 'Level 2'
-- 'M-103', 'Mechanical Ductwork Plan - Roof', 'mechanical', '1/4" = 1'-0"', 'Roof'
-- 'M-104', 'Mechanical Details', 'mechanical'
-- 'M-105', 'Mechanical Schedules', 'mechanical'
```

### `keyed_notes`
Drawing notes that reference specific conditions.

```sql
CREATE TABLE keyed_notes (
    id              TEXT PRIMARY KEY,
    key             TEXT NOT NULL UNIQUE, -- 'H1', 'H2', 'H3', ..., 'H11'
    text            TEXT NOT NULL,
    discipline      TEXT,
    sheet_id        TEXT REFERENCES sheets(id)  -- which sheet defines this note
);

-- From SKIMS:
-- 'H1': 'Refer to arch RCP for blank-off alignments with light fixtures & architectural elements.'
-- 'H2': 'Provide cable operated damper for MVD serving diffuser in inaccessible ceiling.'
-- 'H3': 'Provide birdscreen for return duct.'
-- 'H4': 'Install transfer grille as high as possible above ceiling.'
-- 'H5': 'Duct mounted smoke detector. Mechanical contractor shall install smoke detector in the supply air duct. Mechanical contractor shall provide wiring to fan interlock. E.C. shall provide wiring for connection to remote annunciator.'
-- 'H6': 'Provide new programmable thermostat in managers office with remote sensor in locations as indicated on plans in sales floor. Thermostats/sensors shall be same manufacturer as HVAC unit. Coordinate exact location with SKIMS project manager prior to installation.'
-- 'H7': 'Existing 24"x42" opening facing up on top of duct.'
-- 'H8': '1" door undercut.'
-- 'H9': 'Provide new inline exhaust fan. Balance to the scheduled airflow. Extend new exhaust ductwork to existing exhaust main. Field verify exact location prior to bid.'
-- 'H10': 'Existing rooftop unit to remain. Balance to the scheduled airflow. Clean and verify proper operation...'
-- 'H11': 'Existing exhaust roof penetrations and caps to remain. Field verify exact location prior to bid.'
```

### `keyed_note_refs`
Links keyed notes to specific elements or locations.

```sql
CREATE TABLE keyed_note_refs (
    note_id         TEXT REFERENCES keyed_notes(id),
    element_type    TEXT NOT NULL,        -- 'equipment', 'air_device', 'duct', 'accessory', 'space'
    element_id      TEXT NOT NULL,
    x               REAL,                -- location on drawing where note symbol appears
    y               REAL,
    sheet_id        TEXT REFERENCES sheets(id),
    PRIMARY KEY (note_id, element_type, element_id, sheet_id)
);
```

### `revisions`
Structured change tracking.

```sql
CREATE TABLE revisions (
    id              TEXT PRIMARY KEY,
    number          INTEGER NOT NULL,     -- 1, 2, 3...
    name            TEXT NOT NULL,        -- 'CD Issue', 'Bid Set', 'Addendum 1', 'Bulletin 1'
    date            TEXT NOT NULL,        -- ISO 8601
    description     TEXT,
    author          TEXT
);

CREATE TABLE revision_changes (
    id              TEXT PRIMARY KEY,
    revision_id     TEXT REFERENCES revisions(id),
    element_type    TEXT NOT NULL,
    element_id      TEXT NOT NULL,
    change_type     TEXT NOT NULL,        -- 'added', 'modified', 'removed'
    field           TEXT,                 -- which field changed (NULL for add/remove)
    old_value       TEXT,
    new_value       TEXT
);

-- From SKIMS:
-- Rev 1: 'CD Issue', 12/04/2025
-- Rev 2: 'Bid Set', 01/20/2026
```

### `schedules`
HVAC load and ventilation schedules — computed views but stored for reference.

```sql
CREATE TABLE schedule_data (
    id              TEXT PRIMARY KEY,
    schedule_type   TEXT NOT NULL,        -- 'hvac_load', 'air_device', 'equipment', 'ventilation'
    space_id        TEXT REFERENCES spaces(id),
    equipment_id    TEXT REFERENCES equipment(id),
    
    -- HVAC load data (from M-105 schedule)
    area_sf         REAL,
    ceiling_height  REAL,
    air_changes     REAL,
    cfm_supply      REAL,
    cfm_return      REAL,
    cfm_exhaust     REAL,
    cfm_outside_air REAL,
    sensible_load   REAL,
    total_load      REAL,
    
    -- Ventilation per ASHRAE 62.1 or Title 24
    occupancy       INTEGER,
    oa_per_person   REAL,
    oa_per_area     REAL,
    
    properties      TEXT
);
```

---

## Spatial Index (R-tree)

```sql
CREATE VIRTUAL TABLE spatial_index USING rtree(
    id,
    x_min, x_max,
    y_min, y_max
);
```

All placed elements (equipment, air_devices, ducts, fittings, accessories) register their bounding box here. This enables fast viewport culling for rendering and spatial queries like "show me everything within 6 inches of this duct."

---

## Example Queries (what this schema enables)

```sql
-- Material takeoff: all air devices by type with CFM
SELECT tag, category, cfm_design, manufacturer, model, mounting,
       s.name as room, s.level
FROM air_devices ad
JOIN spaces s ON ad.space_id = s.id
ORDER BY s.level, s.tag;

-- Total supply CFM per level
SELECT s.level, SUM(ad.cfm_design) as total_cfm
FROM air_devices ad
JOIN spaces s ON ad.space_id = s.id
WHERE ad.category LIKE 'supply%'
GROUP BY s.level;

-- All equipment needing submittals
SELECT e.tag, e.category, e.manufacturer, e.model,
       sub.status as submittal_status
FROM equipment e
LEFT JOIN submittal_items si ON si.element_id = e.id AND si.element_type = 'equipment'
LEFT JOIN submittals sub ON sub.id = si.submittal_id;

-- Structured diff between revisions
SELECT rc.element_type, rc.element_id, rc.change_type, rc.field,
       rc.old_value, rc.new_value
FROM revision_changes rc
WHERE rc.revision_id = (SELECT id FROM revisions WHERE number = 2);

-- Find all duct runs serving a specific room
SELECT d.* FROM ducts d
JOIN air_devices ad ON d.to_node_id = ad.id
JOIN spaces s ON ad.space_id = s.id
WHERE s.name = 'Managers Office';

-- Submittal status dashboard
SELECT sub.description, sub.status, sub.submitted_date,
       COUNT(si.element_id) as items_covered
FROM submittals sub
JOIN submittal_items si ON si.submittal_id = sub.id
GROUP BY sub.id;
```

---

## What this does NOT yet include (future versions)

- Piping (hydronic, plumbing, fire protection, gas)
- Electrical (panels, circuits, conduit)
- Controls / BAS points and sequences
- 3D elevation data (this is 2D plan-view for now)
- Clash detection geometry
- Detailed drawing symbology definitions
- Specification sections (Div 23)
- RFI tracking
- Cost data / bid information
