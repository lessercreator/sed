//! Markup/annotation support for SED documents.
//!
//! Contractors can add redline markups to a .sed file:
//! - Text notes at specific locations
//! - Cloud/revision markups around areas of concern
//! - Measurement annotations
//! - Approval stamps
//!
//! Markups are stored in the annotations table and can be
//! filtered by author, date, and type.

use anyhow::Result;
use crate::document::{SedDocument, generate_id};

pub struct Markup {
    pub id: String,
    pub markup_type: String,   // "text", "cloud", "measurement", "stamp"
    pub author: String,
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub x2: Option<f64>,       // for measurements: endpoint
    pub y2: Option<f64>,
    pub level: String,
    pub created_at: String,
}

/// Add a text note markup at a position.
pub fn add_text_note(doc: &SedDocument, level: &str, x: f64, y: f64, text: &str, author: &str) -> Result<String> {
    let id = generate_id();
    // Find or create a view for this level
    let view_id = get_or_create_view(doc, level)?;

    doc.conn.execute(
        "INSERT INTO annotations (id, view_id, anno_type, x1, y1, text, properties)
         VALUES (?1, ?2, 'text', ?3, ?4, ?5, ?6)",
        rusqlite::params![
            id, view_id, x, y, text,
            format!("{{\"author\":\"{}\",\"created\":\"2026-04-05\",\"markup_type\":\"redline\"}}", author)
        ],
    )?;
    Ok(id)
}

/// Add a cloud markup around an area.
pub fn add_cloud(doc: &SedDocument, level: &str, x: f64, y: f64, width: f64, height: f64, text: &str, author: &str) -> Result<String> {
    let id = generate_id();
    let view_id = get_or_create_view(doc, level)?;

    doc.conn.execute(
        "INSERT INTO annotations (id, view_id, anno_type, x1, y1, x2, y2, text, properties)
         VALUES (?1, ?2, 'revision_cloud', ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            id, view_id, x, y, x + width, y + height, text,
            format!("{{\"author\":\"{}\",\"created\":\"2026-04-05\"}}", author)
        ],
    )?;
    Ok(id)
}

/// Add a measurement annotation between two points.
pub fn add_measurement(doc: &SedDocument, level: &str, x1: f64, y1: f64, x2: f64, y2: f64, author: &str) -> Result<String> {
    let id = generate_id();
    let view_id = get_or_create_view(doc, level)?;

    let dist_m = ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt();
    let dist_ft = dist_m * 3.28084;
    let feet = dist_ft.floor() as i32;
    let inches = ((dist_ft - feet as f64) * 12.0).round() as i32;
    let text = format!("{}'- {}\"  ({:.2}m)", feet, inches, dist_m);

    doc.conn.execute(
        "INSERT INTO annotations (id, view_id, anno_type, x1, y1, x2, y2, text, properties)
         VALUES (?1, ?2, 'dimension', ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            id, view_id, x1, y1, x2, y2, text,
            format!("{{\"author\":\"{}\",\"created\":\"2026-04-05\",\"distance_m\":{:.4}}}", author, dist_m)
        ],
    )?;
    Ok(id)
}

/// List all markups/annotations for a level.
pub fn list_markups(doc: &SedDocument, level: &str) -> Result<Vec<Vec<(String, String)>>> {
    doc.query_params(
        "SELECT a.id, a.anno_type, a.x1, a.y1, a.x2, a.y2, a.text, a.properties
         FROM annotations a
         JOIN views v ON a.view_id = v.id
         WHERE v.level = ?1
         ORDER BY a.anno_type",
        &[&level as &dyn rusqlite::types::ToSql],
    )
}

/// Get or create a view for a level (needed for annotations).
fn get_or_create_view(doc: &SedDocument, level: &str) -> Result<String> {
    let rows = doc.query_params(
        "SELECT v.id FROM views v WHERE v.level = ?1 LIMIT 1",
        &[&level as &dyn rusqlite::types::ToSql],
    )?;
    if let Some(row) = rows.first() {
        return Ok(row[0].1.clone());
    }

    // Create a sheet and view for this level
    let sheet_rows = doc.query_params(
        "SELECT id FROM sheets WHERE title LIKE ?1 LIMIT 1",
        &[&format!("%{}%", level) as &dyn rusqlite::types::ToSql],
    )?;
    let sheet_id = if let Some(row) = sheet_rows.first() {
        row[0].1.clone()
    } else {
        let sid = generate_id();
        doc.add_sheet(&crate::types::Sheet {
            id: sid.clone(),
            number: format!("M-{}", level.replace("Level ", "")),
            title: format!("Mechanical Plan — {}", level),
            discipline: "mechanical".into(),
            sheet_size: Some("ARCH D".into()),
        })?;
        sid
    };

    let view_id = generate_id();
    doc.add_view(&crate::types::View {
        id: view_id.clone(),
        sheet_id,
        view_type: "plan".into(),
        title: Some(level.into()),
        scale: Some("1/4\" = 1'-0\"".into()),
        level: Some(level.into()),
        vp_x: None, vp_y: None, vp_width: None, vp_height: None,
        model_x_min: None, model_y_min: None, model_x_max: None, model_y_max: None,
    })?;

    Ok(view_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_list_markups() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        drop(tmp);
        crate::examples::create_skims_americana(&path).unwrap();
        let doc = SedDocument::open(&path).unwrap();

        add_text_note(&doc, "Level 1", 5.0, 10.0, "Verify duct routing in field", "John Smith").unwrap();
        add_cloud(&doc, "Level 1", 3.0, 2.0, 10.0, 8.0, "Revised per Addendum 1", "John Smith").unwrap();
        add_measurement(&doc, "Level 1", 3.0, 13.0, 13.0, 13.0, "John Smith").unwrap();

        let markups = list_markups(&doc, "Level 1").unwrap();
        assert_eq!(markups.len(), 3);

        // Check measurement text has feet-inches
        let dim = markups.iter().find(|m| m[1].1 == "dimension").unwrap();
        assert!(dim[6].1.contains("'"), "Measurement should be in feet-inches: {}", dim[6].1);
    }
}
