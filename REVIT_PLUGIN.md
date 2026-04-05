# SED Revit Plugin — Export Specification

## Overview

A Revit add-in that exports the active MEP model to a `.sed` file. The engineer's workflow:

1. Design in Revit as usual
2. Click "Export to SED" on the Add-Ins ribbon
3. Choose output path
4. A `.sed` file is created containing every mechanical element, connection, room, and schedule

The PDF export button still exists. The `.sed` export sits beside it. Same effort, structured output.

## Technology

- **Language:** C# (.NET, Revit API)
- **Target:** Revit 2024-2027
- **Dependencies:** System.Data.SQLite (for writing the .sed file)
- **Installation:** .addin manifest + DLL in Revit add-ins folder

## Mapping: Revit → SED

### Spaces

| Revit | SED |
|---|---|
| Room | spaces row |
| Room.Number | spaces.tag |
| Room.Name | spaces.name |
| Room.Level.Name | spaces.level |
| Room.Area | spaces.area_m2 (convert from ft² to m²) |
| Room.UnboundedHeight | spaces.ceiling_ht_m |
| Room.GetBoundarySegments() | geometry_polygons (project to XY plane) |

### Product Types

Each unique Revit Family + Type combination becomes a product_type row.

| Revit | SED |
|---|---|
| FamilyInstance.Symbol.FamilyName + TypeName | product_types.tag (abbreviated) |
| FamilyInstance.Symbol.Family.FamilyCategory | product_types.domain + category |
| Type parameter "Manufacturer" | product_types.manufacturer |
| Type parameter "Model" | product_types.model |
| Type parameter "Description" | product_types.description |

**Domain mapping:**
- OST_MechanicalEquipment → equipment
- OST_DuctTerminal → air_device
- OST_DuctAccessory → accessory

**Category inference from Revit category + family name:**
- Air Terminal / "Supply Diffuser" → supply_diffuser
- Air Terminal / "Return Grille" → return_grille
- Mechanical Equipment / "RTU" or "Rooftop" → rtu
- Mechanical Equipment / "AHU" or "Air Handling" → ahu
- Mechanical Equipment / "VAV" → vav_box
- Mechanical Equipment / "Pump" → pump
- Mechanical Equipment / "Chiller" → chiller
- Mechanical Equipment / "Boiler" → boiler

### Placements

Each FamilyInstance becomes a placement row.

| Revit | SED |
|---|---|
| FamilyInstance.Id | (generate new UUID) |
| FamilyInstance.Symbol → product_type_id | placements.product_type_id |
| Mark parameter | placements.instance_tag |
| Room association | placements.space_id |
| Level.Name | placements.level |
| Location.Point (projected to XY, convert to meters) | placements.x, placements.y |
| Airflow parameter | placements.cfm |
| "New Construction" / "Existing" phase | placements.status |

### Systems

Each Revit MEP System becomes a system row.

| Revit | SED |
|---|---|
| MechanicalSystem.Name | systems.tag |
| MechanicalSystem.SystemType | systems.system_type (supply/return/exhaust) |
| "Air" / "Hydronic" | systems.medium |
| Base equipment | systems.source_id |

### Duct Graph

Revit's duct network maps to nodes + segments:

| Revit | SED |
|---|---|
| Duct element | segment (between two connectors) |
| Duct.Diameter / Width×Height | segment.diameter_m / width_m, height_m |
| Duct.Length | segment.length_m |
| Connector (on fitting) | node |
| FamilyInstance fitting type | node.fitting_type (elbow, tee, reducer, etc.) |
| Duct.MEPSystem.Flow | segment.flow_design |

**Walking the network:**
```
For each Duct element:
  Get Connector at each end
  Find the connected element (fitting, equipment, terminal)
  Create a node for each unique connector location
  Create a segment between the two nodes
```

### Pipe Graph (same pattern)

| Revit | SED |
|---|---|
| Pipe element | segment (shape='pipe') |
| Pipe.Diameter | segment.diameter_m |
| PipingSystem.Name | systems.tag |
| PipingSystem.SystemType | systems.medium (chilled_water, hot_water, etc.) |

### Sheets & Views

| Revit | SED |
|---|---|
| ViewSheet.SheetNumber | sheets.number |
| ViewSheet.Name | sheets.title |
| ViewSheet.GetAllPlacedViews() | views (one per viewport) |
| Viewport.GetBoxOutline() | view bounds |
| View.Scale | views.scale |

### Schedules

| Revit | SED |
|---|---|
| ScheduleDefinition fields | schedule_data columns |
| HVAC loads from spaces | schedule_data.cfm_supply, sensible_w, etc. |

### Keyed Notes

| Revit | SED |
|---|---|
| KeynoteTable entries referenced on views | keyed_notes |
| TextNote elements | annotations |

### Submittals

Not in Revit — the `.sed` file is created with empty submittals table. Contractor fills these in downstream.

## What the plugin does NOT export

- 3D geometry (SED is 2D plan-view for now)
- Rendered views (the `.sed` renderer generates its own views)
- Electrical or plumbing (future — schema supports it, plugin maps need extension)
- Fabrication-level detail (shop drawings, hanger locations)

## Export workflow

```
1. User clicks "Export to SED"
2. Plugin enumerates all Rooms → spaces + geometry_polygons
3. Plugin enumerates all FamilyInstance types → product_types
4. Plugin enumerates all FamilyInstance placements → placements
5. Plugin walks MEP systems → systems
6. Plugin walks duct/pipe networks → nodes + segments
7. Plugin reads sheets → sheets + views
8. Plugin reads schedules → schedule_data
9. Plugin writes all to SQLite .sed file
10. Plugin runs sed_sdk::validate equivalent (FK checks, graph integrity)
11. Done — user has a .sed file alongside their .rvt
```

## File size estimate

A typical 50,000 SF office building:
- 200 rooms, 500 placements, 10 systems, 800 nodes, 750 segments
- Estimated .sed file: 2-5 MB (mostly graph data)
- Export time: 5-15 seconds

## Development plan

1. **Phase 1:** Rooms + equipment + air devices + basic placement data. No graph. (~2 weeks)
2. **Phase 2:** Duct graph walking. System extraction. (~2 weeks)
3. **Phase 3:** Piping graph. Schedules. Views/sheets. (~2 weeks)
4. **Phase 4:** Testing with real projects. Bug fixes. Performance. (~2 weeks)

Total: ~8 weeks to a production plugin.

## Distribution

- Free download from GitHub / website
- Works with any Revit version 2024+
- No Autodesk App Store dependency (direct .addin install)
- MIT licensed
