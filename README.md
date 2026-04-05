# SED — Structured Engineering Document

A file format that replaces PDF for engineering plans. One file. Every element typed, connected, and queryable. Open it — you see the plan. Query it — you get structured answers. Hand it to AI — it understands the building.

## What is a `.sed` file?

A `.sed` file is a single SQLite database containing an entire building's engineered systems. Equipment, ductwork, piping, diffusers, rooms, zones, notes, submittals, and revisions — all as discrete, typed, addressable objects. The drawing is a view of the data, not the data itself.

```
$ sedtool info skims-americana.sed

SED v0.3
Project: SKIMS Americana at Brand (25-161)

  Spaces:           29
  Product Types:    15
  Placements:       61
  Systems:           4
  Graph Nodes:      18
  Graph Segments:   17
  Sheets:            6
  Submittals:        4
  Keyed Notes:      11
  Revisions:         2
```

```
$ sedtool report skims-americana.sed cfm

level    tag    name             total_supply_cfm  device_count
-------  -----  ---------------  ----------------  ------------
Level 1  L1-01  Sales Area       2920.00           16
Level 1  L1-02  Fit Room 1       185.00            1
Level 1  L1-03  Fit Room 2 ADA   95.00             1
...
```

## Why

Every mechanical plan in the world follows the same broken path: an engineer builds a rich model in Revit, then exports to PDF, destroying all structure. Every person downstream — contractor, controls integrator, commissioning agent — manually re-extracts information that already existed.

SED keeps the data alive.

## Architecture

```
┌─────────────────────────────────┐
│           .sed file             │
│         (SQLite DB)             │
├─────────────────────────────────┤
│  spaces          product_types  │
│  placements      systems        │
│  nodes           segments       │
│  insulation      submittals     │
│  sheets          views          │
│  annotations     keyed_notes    │
│  revisions       schedule_data  │
│  geometry_polygons              │
│  spatial_idx (R-tree)           │
└─────────────────────────────────┘
         │              │
    ┌────┘              └────┐
    ▼                        ▼
 sedtool CLI          SED Editor
 (query, validate,    (Tauri native
  report, export)      desktop app)
```

## Quick Start

```bash
# Build
cargo build

# Create an example .sed file
cargo run --bin sedtool -- example skims-americana.sed

# Query it
cargo run --bin sedtool -- report skims-americana.sed devices
cargo run --bin sedtool -- report skims-americana.sed cfm
cargo run --bin sedtool -- report skims-americana.sed equipment
cargo run --bin sedtool -- report skims-americana.sed submittals

# Run any SQL
cargo run --bin sedtool -- query skims-americana.sed \
  "SELECT pt.tag, COUNT(*) as qty, SUM(p.cfm) as total_cfm
   FROM placements p
   JOIN product_types pt ON p.product_type_id = pt.id
   GROUP BY pt.id ORDER BY qty DESC"

# Validate
cargo run --bin sedtool -- validate skims-americana.sed

# Create the office tower stress test (10-story, 208 placements)
cargo run --bin sedtool -- office office-tower.sed

# Run tests
cargo test
```

## Format Specification

See [SPECIFICATION.md](SPECIFICATION.md) for the complete format specification (v0.3).

Key properties:
- **Container:** SQLite 3 with WAL journaling
- **Coordinate system:** meters, origin at project datum
- **Units:** SI internally, display converts to imperial/metric
- **IDs:** UUID v4
- **Extensibility:** JSON `properties` column on every table

## Project Structure

```
crates/
  sed-sdk/          Rust SDK — schema, types, document API, geometry, undo/redo
  sed-cli/          CLI tool — sedtool
src-tauri/          Tauri desktop app — SED Editor
ui/                 Frontend for the editor
SPECIFICATION.md    Format specification v0.3
SCHEMA_v0.3.sql     SQL schema definition
```

## Example Projects

### SKIMS Americana at Brand
Real mechanical plans for a 10,780 SF retail tenant fit-out in Glendale, CA. Two existing RTUs, new ductwork distribution, 61 placements, duct graph with 8 branch taps.

### One Commerce Plaza
Simulated 10-story Class A office building. 2x 500-ton chillers, 4 AHUs, 108 VAV boxes, hydronic piping (CHW + HW + CW), exhaust systems, stairwell pressurization. 208 placements, 20 systems, 132 spaces.

## Status

**v0.3 — Schema stable, SDK tested, stress-tested against complex buildings.**

- Schema: 24 tables, survived a 10-story office tower with central plant
- SDK: typed API, private internals, 43 passing tests
- CLI: working (info, query, validate, report, example)
- Editor: Tauri app builds and runs, basic viewer
- Spec: complete format specification

## License

Format specification: CC BY 4.0
SDK and tools: MIT
