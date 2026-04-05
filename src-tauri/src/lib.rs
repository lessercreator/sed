use sed_sdk::SedDocument;
use std::sync::Mutex;
use tauri::State;

struct AppState {
    doc: Mutex<Option<SedDocument>>,
    file_path: Mutex<Option<String>>,
}

// =============================================================================
// COMMANDS
// =============================================================================

#[tauri::command]
fn open_file(path: String, state: State<AppState>) -> Result<serde_json::Value, String> {
    let doc = SedDocument::open(&path).map_err(|e| e.to_string())?;
    let info = doc.info().map_err(|e| e.to_string())?;
    let result = serde_json::to_value(&info).map_err(|e| e.to_string())?;
    *state.doc.lock().unwrap() = Some(doc);
    *state.file_path.lock().unwrap() = Some(path);
    Ok(result)
}

#[tauri::command]
fn create_example(path: String, state: State<AppState>) -> Result<serde_json::Value, String> {
    sed_sdk::examples::create_skims_americana(&path).map_err(|e| e.to_string())?;
    open_file(path, state)
}

#[tauri::command]
fn get_info(state: State<AppState>) -> Result<serde_json::Value, String> {
    let guard = state.doc.lock().unwrap();
    let doc = guard.as_ref().ok_or("No document open")?;
    let info = doc.info().map_err(|e| e.to_string())?;
    serde_json::to_value(&info).map_err(|e| e.to_string())
}

#[tauri::command]
fn query(sql: String, state: State<AppState>) -> Result<serde_json::Value, String> {
    let guard = state.doc.lock().unwrap();
    let doc = guard.as_ref().ok_or("No document open")?;
    let rows = doc.query_raw(&sql).map_err(|e| e.to_string())?;
    let json_rows: Vec<serde_json::Value> = rows.into_iter().map(|row| {
        let mut obj = serde_json::Map::new();
        for (key, val) in row {
            obj.insert(key, serde_json::Value::String(val));
        }
        serde_json::Value::Object(obj)
    }).collect();
    Ok(serde_json::Value::Array(json_rows))
}

#[tauri::command]
fn get_spaces(state: State<AppState>) -> Result<serde_json::Value, String> {
    query("SELECT id, tag, name, level, space_type, scope, x, y FROM spaces ORDER BY level, tag".into(), state)
}

#[tauri::command]
fn get_placements(state: State<AppState>) -> Result<serde_json::Value, String> {
    query(
        "SELECT p.id, pt.tag, pt.domain, pt.category, pt.manufacturer, pt.model,
                p.cfm, p.status, p.level, p.phase, p.scope, p.x, p.y, p.notes,
                s.name as space_name, s.tag as space_tag, p.space_id, p.product_type_id
         FROM placements p
         JOIN product_types pt ON p.product_type_id = pt.id
         LEFT JOIN spaces s ON p.space_id = s.id
         ORDER BY p.level, pt.tag".into(),
        state,
    )
}

#[tauri::command]
fn get_product_types(state: State<AppState>) -> Result<serde_json::Value, String> {
    query("SELECT id, tag, domain, category, manufacturer, model, description, mounting FROM product_types ORDER BY tag".into(), state)
}

#[tauri::command]
fn get_systems(state: State<AppState>) -> Result<serde_json::Value, String> {
    query("SELECT id, tag, name, system_type, medium FROM systems ORDER BY tag".into(), state)
}

#[tauri::command]
fn get_notes(state: State<AppState>) -> Result<serde_json::Value, String> {
    query("SELECT id, key, text, discipline FROM keyed_notes ORDER BY key".into(), state)
}

#[tauri::command]
fn get_submittals(state: State<AppState>) -> Result<serde_json::Value, String> {
    query("SELECT id, description, status, date_submitted, submitted_by, company FROM submittals ORDER BY date_submitted".into(), state)
}

#[tauri::command]
fn update_element(table: String, id: String, field: String, value: Option<String>, state: State<AppState>) -> Result<serde_json::Value, String> {
    let guard = state.doc.lock().unwrap();
    let doc = guard.as_ref().ok_or("No document open")?;
    let rows = match table.as_str() {
        "spaces" => doc.update_space(&id, &field, value.as_deref()).map_err(|e| e.to_string())?,
        "placements" => doc.update_placement(&id, &field, value.as_deref()).map_err(|e| e.to_string())?,
        _ => return Err(format!("Table '{}' not supported for update", table)),
    };
    Ok(serde_json::json!({ "ok": true, "rows_affected": rows }))
}

#[tauri::command]
fn delete_element(table: String, id: String, state: State<AppState>) -> Result<serde_json::Value, String> {
    let guard = state.doc.lock().unwrap();
    let doc = guard.as_ref().ok_or("No document open")?;
    let rows = match table.as_str() {
        "spaces" => doc.delete_space(&id).map_err(|e| e.to_string())?,
        "placements" => doc.delete_placement(&id).map_err(|e| e.to_string())?,
        "product_types" => doc.delete_product_type(&id).map_err(|e| e.to_string())?,
        _ => return Err(format!("Table '{}' not supported for delete", table)),
    };
    Ok(serde_json::json!({ "ok": true, "rows_affected": rows }))
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
        })
        .invoke_handler(tauri::generate_handler![
            open_file,
            create_example,
            get_info,
            query,
            get_spaces,
            get_placements,
            get_product_types,
            get_systems,
            get_notes,
            get_submittals,
            update_element,
            delete_element,
        ])
        .run(tauri::generate_context!())
        .expect("error while running SED Editor");
}
