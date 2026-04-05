# SED — Structured Engineering Document

## Format Specification v0.3

---

## 1. Overview

A `.sed` file is a single SQLite database containing an entire building's (or facility's) engineered systems as structured, queryable data. It renders as traditional engineering drawings but every element — equipment, ductwork, piping, diffusers, rooms, notes — is a discrete, typed, addressable object.

**Purpose:** Replace PDF as the exchange format for engineering plans. Preserve the structured data that authoring tools (Revit, AutoCAD) create but PDF destroys.

**Design goals:**
- One file, emailable, no dependencies
- Opens in a free viewer, editable in the paid editor
- Every element queryable by AI or any tool
- Human-readable when rendered, machine-readable when queried
- All units stored in SI; display layer converts to locale (imperial/metric)

**Container:** SQLite 3, with WAL journaling and foreign keys enabled.

**Schema version:** Stored in `PRAGMA user_version`. Current: `3`.

---

## 2. Coordinate System

- Origin: project datum, typically southwest corner of the building footprint at ground level
- X axis: east (positive)
- Y axis: north (positive)
- Z axis: up (positive) — reserved for future 3D support
- Units: meters
- Rotation: degrees, counterclockwise from east

---

## 3. Flow and Measurement Units

Geometry (coordinates, lengths, areas) is stored in SI. Performance values (airflow, pressure, temperature) are stored in the project's design units as set by `units_display`.

**Always SI (not affected by units_display):**

| Quantity | Stored unit |
|---|---|
| Coordinates (x, y) | meters |
| Length (segment length, thickness) | meters |
| Diameter, width, height | meters |
| Area | m² |
| Weight | kg |

**Stored in project design units (determined by `units_display`):**

| Quantity | Imperial (default) | Metric |
|---|---|---|
| Airflow (cfm, flow_design) | CFM | L/s |
| Water flow (flow_design) | GPM | L/s |
| Pressure (static_pressure_pa) | in. WG | Pa |
| Temperature | °F | °C |
| Power | BTU/h or tons | kW |

The `units_display` meta key (`imperial` or `metric`) declares which unit system performance values are stored in. A conformant tool MUST read this key before interpreting flow, pressure, or temperature values.

---

## 4. Tables

### 4.1 `meta`

Project-level key-value metadata.

| Column | Type | Required | Description |
|---|---|---|---|
| key | TEXT PK | yes | Unique key name |
| value | TEXT | yes | Value |

**Required keys:**
- `sed_version` — format version (e.g., "0.3")
- `project_name` — project name
- `project_number` — project number
- `project_address` — site address
- `units_display` — "imperial" or "metric"
- `created_at` — ISO 8601 timestamp
- `modified_at` — ISO 8601 timestamp

### 4.2 `directory`

Project contacts.

| Column | Type | Required | Description |
|---|---|---|---|
| id | TEXT PK | yes | UUID |
| role | TEXT | yes | Role: owner, architect, engineer_mep, contractor_general, contractor_mech, etc. |
| company | TEXT | yes | Company name |
| contact | TEXT | no | Contact person name |
| email | TEXT | no | Email |
| phone | TEXT | no | Phone |
| address | TEXT | no | Mailing address |

### 4.3 `geometry_polygons`

Closed polygon boundaries (room outlines, zones).

| Column | Type | Required | Description |
|---|---|---|---|
| id | TEXT PK | yes | UUID |
| vertices | BLOB | yes | Packed float64 pairs: [x1,y1, x2,y2, ...] in little-endian |
| vertex_count | INTEGER | yes | Number of vertices |
| level | TEXT | yes | Level name |

### 4.4 `geometry_polylines`

Open polylines (walls, structural grid lines, architectural context).

| Column | Type | Required | Description |
|---|---|---|---|
| id | TEXT PK | yes | UUID |
| vertices | BLOB | yes | Packed float64 pairs |
| vertex_count | INTEGER | yes | Number of vertices |
| level | TEXT | yes | Level name |
| line_type | TEXT | no | wall, partition, curtain_wall, column_grid, storefront |
| weight | REAL | no | Line weight for rendering |
| properties | TEXT (JSON) | no | Extensible |

### 4.5 `spaces`

Rooms, zones, areas.

| Column | Type | Required | Description |
|---|---|---|---|
| id | TEXT PK | yes | UUID |
| tag | TEXT | yes | Room tag: L1-01 |
| name | TEXT | yes | Room name: Sales Area |
| level | TEXT | yes | Level name: Level 1 |
| space_type | TEXT | no | retail, office, storage, restroom, corridor, mechanical, elevator |
| area_m2 | REAL | no | Floor area in m² |
| ceiling_ht_m | REAL | no | Ceiling height in meters |
| scope | TEXT | yes | in_contract, nic, by_others, existing |
| parent_id | TEXT FK→spaces | no | For zone grouping |
| boundary_id | TEXT FK→geometry_polygons | no | Room boundary polygon |
| x | REAL | no | Label placement X |
| y | REAL | no | Label placement Y |
| properties | TEXT (JSON) | no | Extensible |

### 4.6 `product_types`

The catalog. Each row is a class of equipment/device (e.g., "LD-1 = Titus FL-10 mud-in").

| Column | Type | Required | Description |
|---|---|---|---|
| id | TEXT PK | yes | UUID |
| tag | TEXT UNIQUE | yes | Type tag: LD-1, EXRTU, EF-1 |
| domain | TEXT | yes | air_device, equipment, accessory |
| category | TEXT | yes | supply_diffuser, rtu, exhaust_fan, fire_smoke_damper, vav_box, chiller, boiler, pump, etc. |
| manufacturer | TEXT | no | Manufacturer name |
| model | TEXT | no | Model number |
| description | TEXT | no | Human description |
| mounting | TEXT | no | mud-in, lay-in, surface, duct_mounted, sidewall, suspended |
| finish | TEXT | no | standard white, custom, etc. |
| size_nominal | TEXT | no | Nominal size: 10"x10", 8" |
| voltage | REAL | no | Rated voltage |
| phase | INTEGER | no | Electrical phase |
| hz | REAL | no | Frequency |
| submittal_id | TEXT FK→submittals | no | Associated submittal |
| properties | TEXT (JSON) | no | Extensible — use for COP, IPLV, efficiency, performance curves |

### 4.7 `placements`

Instances — each row is one physical thing in the building.

| Column | Type | Required | Description |
|---|---|---|---|
| id | TEXT PK | yes | UUID |
| instance_tag | TEXT | no | Instance-specific tag: EXRTU-1, VAV-2-03. Distinguishes multiple instances of same product_type. |
| product_type_id | TEXT FK→product_types | yes | What type this is |
| space_id | TEXT FK→spaces | no | What room it's in (NULL for roof/building-level equipment) |
| level | TEXT | yes | Level name |
| x | REAL | no | Position X in meters |
| y | REAL | no | Position Y in meters |
| rotation | REAL | no | Rotation in degrees |
| cfm | REAL | no | Design airflow (stored in m³/s) |
| cfm_balanced | REAL | no | As-balanced airflow (filled during TAB) |
| static_pressure_pa | REAL | no | External static pressure in Pa |
| status | TEXT | yes | new, existing_remain, existing_remove, existing_relocate |
| scope | TEXT | yes | in_contract, nic, by_others |
| phase | TEXT | yes | design, submitted, approved, installed, balanced, closed_out |
| weight_kg | REAL | no | Equipment weight |
| properties | TEXT (JSON) | no | Extensible |
| notes | TEXT | no | Free-text notes |

### 4.8 `systems`

Named distribution systems.

| Column | Type | Required | Description |
|---|---|---|---|
| id | TEXT PK | yes | UUID |
| tag | TEXT UNIQUE | yes | System tag: RTU-1-SA, CHWS, EX-1 |
| name | TEXT | yes | Descriptive name |
| system_type | TEXT | yes | supply, return, exhaust, outside_air, pressurization |
| medium | TEXT | yes | air, chilled_water, hot_water, condenser_water, steam, refrigerant, gas |
| source_id | TEXT FK→placements | no | Equipment that drives this system |
| paired_system_id | TEXT FK→systems | no | Links supply to return in hydronic loops |
| properties | TEXT (JSON) | no | Extensible |

### 4.9 `placement_systems`

Junction table: placements can belong to multiple systems.

| Column | Type | Required | Description |
|---|---|---|---|
| placement_id | TEXT FK→placements | yes | PK part 1 |
| system_id | TEXT FK→systems | yes | PK part 2 |
| role | TEXT | yes | served_by, source, reheat, coil |

A VAV with hot water reheat has two rows: one for the air system (role='served_by'), one for the hot water system (role='reheat').

### 4.10 `nodes`

Graph junction points — where fittings, equipment connections, and terminals sit.

| Column | Type | Required | Description |
|---|---|---|---|
| id | TEXT PK | yes | UUID |
| system_id | TEXT FK→systems | yes | Which system this node belongs to |
| node_type | TEXT | yes | equipment_conn, branch, turn, transition, terminal, cap, damper, junction |
| placement_id | TEXT FK→placements | no | Links to a physical device at this node |
| fitting_type | TEXT | no | tap_45, elbow_45, elbow_90, reducer_concentric, end_cap, lateral_reducing |
| size_description | TEXT | no | Human-readable size: 28" trunk / 8" branch |
| level | TEXT | no | Level name |
| x | REAL | no | Position X |
| y | REAL | no | Position Y |
| properties | TEXT (JSON) | no | Extensible |

### 4.11 `segments`

Graph edges — duct runs, pipe runs between nodes.

| Column | Type | Required | Description |
|---|---|---|---|
| id | TEXT PK | yes | UUID |
| system_id | TEXT FK→systems | yes | Which system |
| from_node_id | TEXT FK→nodes | yes | Upstream node |
| to_node_id | TEXT FK→nodes | yes | Downstream node |
| shape | TEXT | yes | round, rectangular, oval, flex, pipe |
| width_m | REAL | no | Rectangular width |
| height_m | REAL | no | Rectangular height |
| diameter_m | REAL | no | Round/pipe diameter |
| length_m | REAL | no | Segment length |
| material | TEXT | yes | galvanized, aluminum, stainless, black_iron, copper, pvc, steel |
| gauge | INTEGER | no | Sheet metal gauge |
| pressure_class | TEXT | no | 2_in_wg, 4_in_wg, etc. |
| construction | TEXT | no | spiral_lock, longitudinal_seam, snap_lock, welded |
| exposure | TEXT | no | concealed, exposed |
| flow_design | REAL | no | Design flow in SI (m³/s for air, L/s for water) |
| flow_balanced | REAL | no | As-balanced flow |
| status | TEXT | yes | new, existing_remain, existing_remove |
| scope | TEXT | yes | in_contract, nic, by_others |
| properties | TEXT (JSON) | no | Extensible |

### 4.12 `insulation`

Insulation applied to segments.

| Column | Type | Required | Description |
|---|---|---|---|
| id | TEXT PK | yes | UUID |
| segment_id | TEXT FK→segments | no | Which segment |
| type | TEXT | yes | duct_wrap, acoustic_liner, pipe_insulation |
| manufacturer | TEXT | no | |
| product | TEXT | no | |
| thickness_m | REAL | no | Thickness in meters |
| r_value | REAL | no | R-value |
| facing | TEXT | no | fsk, psk, unfaced |
| code_reference | TEXT | no | CA Title 24, ASHRAE 90.1 |

### 4.13 `submittals`

Submittal tracking.

| Column | Type | Required | Description |
|---|---|---|---|
| id | TEXT PK | yes | UUID |
| number | TEXT | no | Sequential number |
| description | TEXT | yes | What this submittal covers |
| submitted_by | TEXT | no | Contact name |
| company | TEXT | no | Submitting company |
| date_submitted | TEXT | no | ISO 8601 date |
| status | TEXT | yes | for_approval, approved, approved_as_noted, revise_resubmit, rejected |
| reviewed_by | TEXT | no | |
| date_reviewed | TEXT | no | |
| spec_section | TEXT | no | CSI section: 233713 |
| attachment_id | TEXT FK→attachments | no | The actual submittal document |
| notes | TEXT | no | |

### 4.14 `attachments`

Embedded files.

| Column | Type | Required | Description |
|---|---|---|---|
| id | TEXT PK | yes | UUID |
| filename | TEXT | yes | Original filename |
| mime_type | TEXT | yes | MIME type |
| size_bytes | INTEGER | no | File size |
| data | BLOB | no | File contents |
| description | TEXT | no | |

### 4.15 `sheets`

Drawing sheets.

| Column | Type | Required | Description |
|---|---|---|---|
| id | TEXT PK | yes | UUID |
| number | TEXT UNIQUE | yes | Sheet number: M-001, M-101 |
| title | TEXT | yes | Sheet title |
| discipline | TEXT | yes | mechanical, electrical, plumbing, architectural, structural, fire_protection |
| sheet_size | TEXT | no | ARCH_D, 24x36, A1 |
| properties | TEXT (JSON) | no | |

### 4.16 `views`

Viewports within sheets.

| Column | Type | Required | Description |
|---|---|---|---|
| id | TEXT PK | yes | UUID |
| sheet_id | TEXT FK→sheets | yes | Parent sheet |
| view_type | TEXT | yes | plan, detail, section, schedule, legend, title_block |
| title | TEXT | no | View title |
| scale | TEXT | no | 1/4" = 1'-0", NTS |
| level | TEXT | no | For plan views: which level |
| vp_x, vp_y | REAL | no | Viewport position on sheet |
| vp_width, vp_height | REAL | no | Viewport size on sheet |
| model_x_min, model_y_min | REAL | no | Model region: min bounds |
| model_x_max, model_y_max | REAL | no | Model region: max bounds |
| parent_view_id | TEXT FK→views | no | For detail callouts: parent view |
| callout_x, callout_y | REAL | no | Callout bubble position |
| properties | TEXT (JSON) | no | |

### 4.17 `annotations`

Drawing annotations (visual communication layer).

| Column | Type | Required | Description |
|---|---|---|---|
| id | TEXT PK | yes | UUID |
| view_id | TEXT FK→views | yes | Which view this appears in |
| anno_type | TEXT | yes | dimension, leader, text, tag, revision_cloud, section_mark, matchline, north_arrow |
| ref_table | TEXT | no | What table the referenced element is in |
| ref_id | TEXT | no | UUID of referenced element |
| x1, y1, x2, y2 | REAL | no | Primary geometry points |
| geometry | BLOB | no | Additional geometry (cloud vertices, leader path) |
| text | TEXT | no | Display text |
| text_height | REAL | no | |
| text_rotation | REAL | no | |
| revision_id | TEXT FK→revisions | no | For revision clouds |
| properties | TEXT (JSON) | no | |

### 4.18 `general_notes`

Project-wide or discipline-wide notes.

| Column | Type | Required | Description |
|---|---|---|---|
| id | TEXT PK | yes | UUID |
| discipline | TEXT | no | NULL = all disciplines |
| text | TEXT | yes | Note text |
| sort_order | INTEGER | no | Display ordering |

### 4.19 `keyed_notes`

Notes referenced by symbol on drawings (H1, H2, etc.).

| Column | Type | Required | Description |
|---|---|---|---|
| id | TEXT PK | yes | UUID |
| key | TEXT UNIQUE | yes | Note key: H1, H2 |
| text | TEXT | yes | Note text |
| discipline | TEXT | no | |
| spec_section | TEXT | no | CSI reference |

### 4.20 `keyed_note_refs`

Links keyed notes to locations on drawings.

| Column | Type | Required | Description |
|---|---|---|---|
| note_id | TEXT FK→keyed_notes | yes | PK part 1 |
| placement_id | TEXT | no | Element this note points at |
| view_id | TEXT FK→views | yes | PK part 2 |
| x, y | REAL | yes | PK parts 3-4: position of note symbol |

### 4.21 `revisions`

Document revision history.

| Column | Type | Required | Description |
|---|---|---|---|
| id | TEXT PK | yes | UUID |
| number | INTEGER | yes | Sequential revision number |
| name | TEXT | yes | CD Issue, Bid Set, Addendum 1 |
| date | TEXT | yes | ISO 8601 date |
| description | TEXT | no | |
| author | TEXT | no | |

### 4.22 `revision_changes`

Structured diff per revision.

| Column | Type | Required | Description |
|---|---|---|---|
| id | TEXT PK | yes | UUID |
| revision_id | TEXT FK→revisions | yes | Which revision |
| table_name | TEXT | yes | Affected table |
| element_id | TEXT | yes | UUID of changed element |
| change_type | TEXT | yes | added, modified, removed |
| field | TEXT | no | Column name (NULL for add/remove) |
| old_value | TEXT | no | |
| new_value | TEXT | no | |

### 4.23 `schedule_data`

HVAC load and ventilation calculations.

| Column | Type | Required | Description |
|---|---|---|---|
| id | TEXT PK | yes | UUID |
| space_id | TEXT FK→spaces | no | |
| equipment_id | TEXT FK→placements | no | |
| cfm_supply, cfm_return, cfm_exhaust, cfm_outside_air | REAL | no | In m³/s |
| sensible_w, total_w | REAL | no | Loads in watts |
| occupancy | INTEGER | no | |
| oa_per_person, oa_per_area | REAL | no | Ventilation rates |
| properties | TEXT (JSON) | no | |

### 4.24 `spatial_idx`

R-tree spatial index for viewport culling and proximity queries.

| Column | Type | Description |
|---|---|---|
| id | INTEGER PK | Sequential |
| x_min, x_max | REAL | Bounding box X |
| y_min, y_max | REAL | Bounding box Y |

---

## 5. Extensibility

Every table with a `properties` column accepts arbitrary JSON. This allows vendors, firms, and disciplines to add custom data without schema modification.

**Convention for performance data in `product_types.properties`:**
```json
{
  "cop": 5.8,
  "iplv": 0.55,
  "pump_curve": "100gpm@25ft, 80gpm@35ft",
  "thermal_efficiency": 0.96
}
```

---

## 6. Versioning

The schema version is stored in `PRAGMA user_version`.

- **Minor changes** (new optional columns, new tables): increment user_version, use `ALTER TABLE ADD COLUMN` for migration.
- **Major changes** (column removal, type changes): increment major version in `sed_version` meta key. Provide migration tooling.

A conformant reader MUST check user_version and refuse to open files with a higher major version than it supports.

---

## 7. Conformance

A valid `.sed` file MUST:
1. Be a valid SQLite 3 database
2. Have `PRAGMA user_version` set to a recognized schema version
3. Have all required meta keys present
4. Have all required columns on all tables
5. Pass foreign key integrity checks

A conformant viewer MUST:
1. Open any valid `.sed` file without error
2. Render plan views from spaces, placements, nodes, and segments
3. Display properties for any selected element
4. Support pan, zoom, and level switching

A conformant editor MUST additionally:
1. Preserve all existing data when saving
2. Generate UUIDs for all new elements
3. Record changes in `revision_changes` when creating a new revision
4. Maintain foreign key integrity

---

## 8. License

This specification is published under CC BY 4.0. Anyone may implement readers, writers, and tools for `.sed` files without restriction.
