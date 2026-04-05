use sed_sdk::SedDocument;
use sed_sdk::document::generate_id;
use sed_sdk::types::*;
use sed_sdk::undo::{UndoStack, Command};
use std::sync::Mutex;
use tauri::State;

struct AppState {
    doc: Mutex<Option<SedDocument>>,
    file_path: Mutex<Option<String>>,
    undo_stack: Mutex<UndoStack>,
}

// =============================================================================
// FILE OPS
// =============================================================================

#[tauri::command]
fn new_file(path: String, project_name: String, project_number: String, state: State<AppState>) -> Result<serde_json::Value, String> {
    let doc = SedDocument::create(&path).map_err(|e| e.to_string())?;
    doc.set_meta("sed_version", "0.3").map_err(|e| e.to_string())?;
    doc.set_meta("project_name", &project_name).map_err(|e| e.to_string())?;
    doc.set_meta("project_number", &project_number).map_err(|e| e.to_string())?;
    doc.set_meta("units_display", "imperial").map_err(|e| e.to_string())?;
    let info = doc.info().map_err(|e| e.to_string())?;
    let result = serde_json::to_value(&info).map_err(|e| e.to_string())?;
    *state.doc.lock().unwrap() = Some(doc);
    *state.file_path.lock().unwrap() = Some(path);
    *state.undo_stack.lock().unwrap() = UndoStack::new();
    Ok(result)
}

#[tauri::command]
fn open_file(path: String, state: State<AppState>) -> Result<serde_json::Value, String> {
    let doc = SedDocument::open(&path).map_err(|e| e.to_string())?;
    let info = doc.info().map_err(|e| e.to_string())?;
    let result = serde_json::to_value(&info).map_err(|e| e.to_string())?;
    *state.doc.lock().unwrap() = Some(doc);
    *state.file_path.lock().unwrap() = Some(path);
    *state.undo_stack.lock().unwrap() = UndoStack::new();
    Ok(result)
}

#[tauri::command]
fn create_example(path: String, state: State<AppState>) -> Result<serde_json::Value, String> {
    sed_sdk::examples::create_skims_americana(&path).map_err(|e| e.to_string())?;
    open_file(path, state)
}

// =============================================================================
// READ
// =============================================================================

#[tauri::command]
fn get_info(state: State<AppState>) -> Result<serde_json::Value, String> {
    with_doc(&state, |doc| {
        let info = doc.info()?;
        Ok(serde_json::to_value(&info)?)
    })
}

#[tauri::command]
fn query(sql: String, state: State<AppState>) -> Result<serde_json::Value, String> {
    with_doc(&state, |doc| {
        let rows = doc.query_raw(&sql)?;
        Ok(rows_to_json(rows))
    })
}

#[tauri::command]
fn get_spaces(state: State<AppState>) -> Result<serde_json::Value, String> {
    with_doc(&state, |doc| {
        let rows = doc.query_raw("SELECT id, tag, name, level, space_type, scope, x, y FROM spaces ORDER BY level, tag")?;
        Ok(rows_to_json(rows))
    })
}

#[tauri::command]
fn get_placements(state: State<AppState>) -> Result<serde_json::Value, String> {
    with_doc(&state, |doc| {
        let rows = doc.query_raw(
            "SELECT p.id, pt.tag, pt.domain, pt.category, pt.manufacturer, pt.model,
                    p.cfm, p.status, p.level, p.phase, p.scope, p.x, p.y, p.notes,
                    p.instance_tag, s.name as space_name, s.tag as space_tag,
                    p.space_id, p.product_type_id
             FROM placements p
             JOIN product_types pt ON p.product_type_id = pt.id
             LEFT JOIN spaces s ON p.space_id = s.id
             ORDER BY p.level, pt.tag"
        )?;
        Ok(rows_to_json(rows))
    })
}

#[tauri::command]
fn get_product_types(state: State<AppState>) -> Result<serde_json::Value, String> {
    with_doc(&state, |doc| {
        let rows = doc.query_raw("SELECT id, tag, domain, category, manufacturer, model, description, mounting FROM product_types ORDER BY tag")?;
        Ok(rows_to_json(rows))
    })
}

#[tauri::command]
fn get_systems(state: State<AppState>) -> Result<serde_json::Value, String> {
    with_doc(&state, |doc| {
        let rows = doc.query_raw("SELECT id, tag, name, system_type, medium FROM systems ORDER BY tag")?;
        Ok(rows_to_json(rows))
    })
}

#[tauri::command]
fn get_notes(state: State<AppState>) -> Result<serde_json::Value, String> {
    with_doc(&state, |doc| {
        let rows = doc.query_raw("SELECT id, key, text, discipline FROM keyed_notes ORDER BY key")?;
        Ok(rows_to_json(rows))
    })
}

#[tauri::command]
fn get_submittals(state: State<AppState>) -> Result<serde_json::Value, String> {
    with_doc(&state, |doc| {
        let rows = doc.query_raw("SELECT id, description, status, date_submitted, submitted_by, company FROM submittals ORDER BY date_submitted")?;
        Ok(rows_to_json(rows))
    })
}

#[tauri::command]
fn get_room_geometry(level: String, state: State<AppState>) -> Result<serde_json::Value, String> {
    with_doc(&state, |doc| {
        let rooms = sed_sdk::geometry::get_room_geometry(doc, &level)?;
        Ok(serde_json::to_value(&rooms)?)
    })
}

#[tauri::command]
fn get_graph(level: String, system_tag: Option<String>, state: State<AppState>) -> Result<serde_json::Value, String> {
    with_doc(&state, |doc| {
        let mut node_sql = String::from(
            "SELECT n.id, n.node_type, n.fitting_type, n.size_description, n.x, n.y, sys.tag as system_tag
             FROM nodes n JOIN systems sys ON n.system_id = sys.id WHERE n.level = ?1"
        );
        let mut seg_sql = String::from(
            "SELECT seg.id, n1.x as x1, n1.y as y1, n2.x as x2, n2.y as y2, seg.shape, seg.diameter_m, seg.width_m, seg.flow_design, sys.tag as system_tag
             FROM segments seg
             JOIN nodes n1 ON seg.from_node_id = n1.id
             JOIN nodes n2 ON seg.to_node_id = n2.id
             JOIN systems sys ON seg.system_id = sys.id
             WHERE n1.level = ?1 AND n1.x IS NOT NULL"
        );
        if let Some(ref tag) = system_tag {
            node_sql += &format!(" AND sys.tag = '{}'", tag.replace('\'', "''"));
            seg_sql += &format!(" AND sys.tag = '{}'", tag.replace('\'', "''"));
        }
        let nodes = doc.query_params(&node_sql, &[&level as &dyn rusqlite::types::ToSql])?;
        let segs = doc.query_params(&seg_sql, &[&level as &dyn rusqlite::types::ToSql])?;
        Ok(serde_json::json!({ "nodes": rows_to_json(nodes), "segments": rows_to_json(segs) }))
    })
}

// =============================================================================
// CREATE
// =============================================================================

#[tauri::command]
fn create_space(tag: String, name: String, level: String, space_type: Option<String>, scope: Option<String>, vertices: Option<Vec<serde_json::Value>>, state: State<AppState>) -> Result<serde_json::Value, String> {
    with_doc(&state, |doc| {
        let id = generate_id();
        let scope = scope.unwrap_or_else(|| "in_contract".into());

        // If vertices provided, create a polygon
        let boundary_id = if let Some(verts) = &vertices {
            if verts.len() >= 3 {
                let poly_id = generate_id();
                let mut bytes = Vec::new();
                for v in verts {
                    let x = v.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let y = v.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    bytes.extend_from_slice(&x.to_le_bytes());
                    bytes.extend_from_slice(&y.to_le_bytes());
                }
                doc.create_polygon(&poly_id, &bytes, verts.len() as i32, &level)?;
                // Compute center
                let cx: f64 = verts.iter().map(|v| v.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0)).sum::<f64>() / verts.len() as f64;
                let cy: f64 = verts.iter().map(|v| v.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0)).sum::<f64>() / verts.len() as f64;
                doc.add_space(&Space {
                    id: id.clone(), tag: tag.clone(), name: name.clone(), level: level.clone(),
                    space_type: space_type.clone(), area_m2: None, ceiling_ht_m: None,
                    scope: scope.clone(), parent_id: None, boundary_id: Some(poly_id),
                    x: Some(cx), y: Some(cy),
                })?;
                return Ok(serde_json::json!({ "id": id }));
            }
            None
        } else {
            None
        };

        doc.add_space(&Space {
            id: id.clone(), tag, name, level,
            space_type, area_m2: None, ceiling_ht_m: None,
            scope, parent_id: None, boundary_id,
            x: None, y: None,
        })?;
        Ok(serde_json::json!({ "id": id }))
    })
}

#[tauri::command]
fn create_product_type(tag: String, domain: String, category: String, manufacturer: Option<String>, model: Option<String>, description: Option<String>, state: State<AppState>) -> Result<serde_json::Value, String> {
    with_doc(&state, |doc| {
        let id = generate_id();
        doc.add_product_type(&ProductType {
            id: id.clone(), tag, domain, category,
            manufacturer, model, description,
            mounting: None, finish: None, size_nominal: None,
            voltage: None, phase: None, hz: None, submittal_id: None,
        })?;
        Ok(serde_json::json!({ "id": id }))
    })
}

#[tauri::command]
fn create_placement(product_type_id: String, level: String, x: Option<f64>, y: Option<f64>, cfm: Option<f64>, space_id: Option<String>, instance_tag: Option<String>, state: State<AppState>) -> Result<serde_json::Value, String> {
    with_doc(&state, |doc| {
        let id = generate_id();
        doc.add_placement(&Placement {
            id: id.clone(), instance_tag, product_type_id, space_id,
            level, x, y, rotation: None,
            cfm, cfm_balanced: None, static_pressure_pa: None,
            status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
            weight_kg: None, notes: None,
        })?;
        Ok(serde_json::json!({ "id": id }))
    })
}

#[tauri::command]
fn create_system(tag: String, name: String, system_type: String, medium: String, source_id: Option<String>, state: State<AppState>) -> Result<serde_json::Value, String> {
    with_doc(&state, |doc| {
        let id = generate_id();
        doc.add_system(&System {
            id: id.clone(), tag, name, system_type, medium,
            source_id, paired_system_id: None,
        })?;
        Ok(serde_json::json!({ "id": id }))
    })
}

#[tauri::command]
fn create_node(system_id: String, node_type: String, level: String, x: f64, y: f64, fitting_type: Option<String>, size_description: Option<String>, placement_id: Option<String>, state: State<AppState>) -> Result<serde_json::Value, String> {
    with_doc(&state, |doc| {
        let id = generate_id();
        doc.add_node(&Node {
            id: id.clone(), system_id, node_type, placement_id,
            fitting_type, size_description,
            level: Some(level), x: Some(x), y: Some(y),
        })?;
        Ok(serde_json::json!({ "id": id }))
    })
}

#[tauri::command]
fn create_segment(system_id: String, from_node_id: String, to_node_id: String, shape: String, diameter_m: Option<f64>, width_m: Option<f64>, height_m: Option<f64>, flow_design: Option<f64>, state: State<AppState>) -> Result<serde_json::Value, String> {
    with_doc(&state, |doc| {
        let id = generate_id();
        // Calculate length from node positions
        let n1 = doc.query_params("SELECT x, y FROM nodes WHERE id = ?1", &[&from_node_id as &dyn rusqlite::types::ToSql])?;
        let n2 = doc.query_params("SELECT x, y FROM nodes WHERE id = ?1", &[&to_node_id as &dyn rusqlite::types::ToSql])?;
        let nodes = vec![n1, n2].into_iter().filter_map(|r| r.into_iter().next()).collect::<Vec<_>>();
        let length = if nodes.len() == 2 {
            let x1: f64 = nodes[0][0].1.parse().unwrap_or(0.0);
            let y1: f64 = nodes[0][1].1.parse().unwrap_or(0.0);
            let x2: f64 = nodes[1][0].1.parse().unwrap_or(0.0);
            let y2: f64 = nodes[1][1].1.parse().unwrap_or(0.0);
            Some(((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt())
        } else { None };

        doc.add_segment(&Segment {
            id: id.clone(), system_id, from_node_id, to_node_id,
            shape, width_m, height_m, diameter_m, length_m: length,
            material: "galvanized".into(), gauge: None,
            pressure_class: None, construction: None, exposure: None,
            flow_design, flow_balanced: None,
            status: "new".into(), scope: "in_contract".into(),
        })?;
        Ok(serde_json::json!({ "id": id }))
    })
}

// =============================================================================
// UPDATE / DELETE
// =============================================================================

#[tauri::command]
fn update_element(table: String, id: String, field: String, value: Option<String>, state: State<AppState>) -> Result<serde_json::Value, String> {
    let guard = state.doc.lock().unwrap();
    let doc = guard.as_ref().ok_or("No document open")?;

    // Read old value for undo
    let old_value = doc.query_params(
        &format!("SELECT {} FROM {} WHERE id = ?1", field.replace('\'', ""), table.replace('\'', "")),
        &[&id as &dyn rusqlite::types::ToSql],
    ).ok().and_then(|rows| rows.first().and_then(|r| r.first().map(|(_, v)| v.clone())))
    .and_then(|v| if v == "NULL" { None } else { Some(v) });

    let rows = match table.as_str() {
        "spaces" => doc.update_space(&id, &field, value.as_deref()).map_err(|e| e.to_string())?,
        "placements" => doc.update_placement(&id, &field, value.as_deref()).map_err(|e| e.to_string())?,
        _ => return Err(format!("Table '{}' not supported for update", table)),
    };

    let mut undo = state.undo_stack.lock().unwrap();
    undo.push(Command::UpdateField { table, id, field, old_value, new_value: value });
    Ok(serde_json::json!({ "ok": true, "rows_affected": rows }))
}

#[tauri::command]
fn delete_element(table: String, id: String, state: State<AppState>) -> Result<serde_json::Value, String> {
    with_doc(&state, |doc| {
        let rows = match table.as_str() {
            "spaces" => doc.delete_space(&id)?,
            "placements" => doc.delete_placement(&id)?,
            "product_types" => doc.delete_product_type(&id)?,
            _ => anyhow::bail!("Table '{}' not supported for delete", table),
        };
        Ok(serde_json::json!({ "ok": true, "rows_affected": rows }))
    })
}

#[tauri::command]
fn move_element(table: String, id: String, x: f64, y: f64, state: State<AppState>) -> Result<serde_json::Value, String> {
    with_doc(&state, |doc| {
        match table.as_str() {
            "placements" => {
                doc.update_placement(&id, "x", Some(&x.to_string()))?;
                doc.update_placement(&id, "y", Some(&y.to_string()))?;
            }
            "nodes" => {
                doc.update_node_position(&id, x, y)?;
            }
            _ => anyhow::bail!("Can't move elements in table '{}'", table),
        }
        Ok(serde_json::json!({ "ok": true }))
    })
}

// =============================================================================
// DUPLICATE
// =============================================================================

#[tauri::command]
fn duplicate_placement(id: String, offset_x: f64, offset_y: f64, state: State<AppState>) -> Result<serde_json::Value, String> {
    with_doc(&state, |doc| {
        let new_id = sed_sdk::clipboard::duplicate_placement(doc, &id, offset_x, offset_y)?;
        Ok(serde_json::json!({ "id": new_id }))
    })
}

// =============================================================================
// UNDO / REDO
// =============================================================================

#[tauri::command]
fn undo(state: State<AppState>) -> Result<serde_json::Value, String> {
    let guard = state.doc.lock().unwrap();
    let doc = guard.as_ref().ok_or("No document open")?;
    let mut stack = state.undo_stack.lock().unwrap();
    let desc = stack.undo(doc).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "undone": desc }))
}

#[tauri::command]
fn redo(state: State<AppState>) -> Result<serde_json::Value, String> {
    let guard = state.doc.lock().unwrap();
    let doc = guard.as_ref().ok_or("No document open")?;
    let mut stack = state.undo_stack.lock().unwrap();
    let desc = stack.redo(doc).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "redone": desc }))
}

#[tauri::command]
fn undo_info(state: State<AppState>) -> Result<serde_json::Value, String> {
    let stack = state.undo_stack.lock().unwrap();
    Ok(serde_json::json!({
        "can_undo": stack.can_undo(),
        "can_redo": stack.can_redo(),
        "undo_count": stack.undo_count(),
        "redo_count": stack.redo_count(),
    }))
}

// =============================================================================
// HELPERS
// =============================================================================

fn with_doc<F>(state: &State<AppState>, f: F) -> Result<serde_json::Value, String>
where F: FnOnce(&SedDocument) -> anyhow::Result<serde_json::Value>
{
    let guard = state.doc.lock().unwrap();
    let doc = guard.as_ref().ok_or("No document open")?;
    f(doc).map_err(|e| e.to_string())
}

fn rows_to_json(rows: Vec<Vec<(String, String)>>) -> serde_json::Value {
    let json_rows: Vec<serde_json::Value> = rows.into_iter().map(|row| {
        let mut obj = serde_json::Map::new();
        for (key, val) in row {
            obj.insert(key, serde_json::Value::String(val));
        }
        serde_json::Value::Object(obj)
    }).collect();
    serde_json::Value::Array(json_rows)
}

// =============================================================================
// APP
// =============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            doc: Mutex::new(None),
            file_path: Mutex::new(None),
            undo_stack: Mutex::new(UndoStack::new()),
        })
        .invoke_handler(tauri::generate_handler![
            // File
            new_file, open_file, create_example,
            // Read
            get_info, query, get_spaces, get_placements, get_product_types,
            get_systems, get_notes, get_submittals, get_room_geometry, get_graph,
            // Create
            create_space, create_product_type, create_placement,
            create_system, create_node, create_segment,
            // Update
            update_element, move_element,
            // Delete / Duplicate
            delete_element,
            duplicate_placement,
            // Undo
            undo, redo, undo_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running SED Editor");
}
