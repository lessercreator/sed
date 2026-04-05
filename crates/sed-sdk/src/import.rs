//! Import utilities for creating .sed files from external data sources.
//!
//! Supported formats:
//! - CSV equipment schedules (Revit schedule exports, manual spreadsheets)
//!
//! The import process:
//! 1. Read CSV with headers
//! 2. Map columns to SED fields using a column mapping
//! 3. Auto-detect product types from unique tag values
//! 4. Create placements for each row
//! 5. Infer spaces from room/level columns

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use crate::document::{SedDocument, generate_id};
use crate::types::*;

/// Column mapping — tells the importer which CSV columns map to which SED fields.
/// Uses case-insensitive substring matching against headers.
#[derive(Debug, Clone)]
pub struct ColumnMapping {
    pub tag: Vec<String>,           // column names that might contain the device tag: "Tag", "Mark", "Type"
    pub manufacturer: Vec<String>,  // "Manufacturer", "Mfr"
    pub model: Vec<String>,         // "Model", "Model Number"
    pub cfm: Vec<String>,           // "CFM", "Airflow", "Flow", "Design Airflow"
    pub room_name: Vec<String>,     // "Room", "Room Name", "Space", "Location"
    pub room_tag: Vec<String>,      // "Room Number", "Room Tag", "Space Number"
    pub level: Vec<String>,         // "Level", "Floor"
    pub size: Vec<String>,          // "Size", "Inlet Size", "Neck Size"
    pub status: Vec<String>,        // "Status", "Phase"
    pub notes: Vec<String>,         // "Notes", "Comments", "Remarks"
    pub domain: Vec<String>,        // "Domain", "Category", "Type" (equipment vs air_device)
}

impl Default for ColumnMapping {
    fn default() -> Self {
        ColumnMapping {
            tag: vec!["tag".into(), "mark".into(), "type mark".into(), "type".into(), "family".into()],
            manufacturer: vec!["manufacturer".into(), "mfr".into(), "vendor".into()],
            model: vec!["model".into(), "model number".into(), "catalog".into()],
            cfm: vec!["cfm".into(), "airflow".into(), "flow".into(), "design airflow".into(), "air flow".into()],
            room_name: vec!["room".into(), "room name".into(), "space".into(), "location".into(), "space name".into()],
            room_tag: vec!["room number".into(), "room tag".into(), "space number".into(), "room #".into()],
            level: vec!["level".into(), "floor".into(), "storey".into()],
            size: vec!["size".into(), "inlet size".into(), "neck size".into(), "duct size".into()],
            status: vec!["status".into(), "phase".into()],
            notes: vec!["notes".into(), "comments".into(), "remarks".into(), "description".into()],
            domain: vec!["domain".into(), "category".into(), "discipline".into()],
        }
    }
}

/// Import results
#[derive(Debug)]
pub struct ImportResult {
    pub rows_read: usize,
    pub product_types_created: usize,
    pub placements_created: usize,
    pub spaces_created: usize,
    pub unmapped_columns: Vec<String>,
}

impl std::fmt::Display for ImportResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Import complete:")?;
        writeln!(f, "  Rows read:          {}", self.rows_read)?;
        writeln!(f, "  Product types:      {}", self.product_types_created)?;
        writeln!(f, "  Placements:         {}", self.placements_created)?;
        writeln!(f, "  Spaces:             {}", self.spaces_created)?;
        if !self.unmapped_columns.is_empty() {
            writeln!(f, "  Unmapped columns:   {}", self.unmapped_columns.join(", "))?;
        }
        Ok(())
    }
}

/// Import a CSV file into a .sed document.
pub fn import_csv(
    csv_path: &str,
    sed_path: &str,
    project_name: &str,
    project_number: &str,
    mapping: &ColumnMapping,
) -> Result<ImportResult> {
    let doc = if Path::new(sed_path).exists() {
        SedDocument::open(sed_path)?
    } else {
        let d = SedDocument::create(sed_path)?;
        d.set_meta("sed_version", "0.3")?;
        d.set_meta("project_name", project_name)?;
        d.set_meta("project_number", project_number)?;
        d.set_meta("units_display", "imperial")?;
        d.set_meta("created_at", &chrono_now())?;
        d.set_meta("modified_at", &chrono_now())?;
        d
    };

    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_path(csv_path)
        .with_context(|| format!("Failed to open CSV: {}", csv_path))?;

    let headers: Vec<String> = reader.headers()?.iter().map(|h| h.to_string()).collect();
    let header_lower: Vec<String> = headers.iter().map(|h| h.to_lowercase()).collect();

    // Map header indices
    let idx_tag = find_col(&header_lower, &mapping.tag);
    let idx_mfr = find_col(&header_lower, &mapping.manufacturer);
    let idx_model = find_col(&header_lower, &mapping.model);
    let idx_cfm = find_col(&header_lower, &mapping.cfm);
    let idx_room_name = find_col(&header_lower, &mapping.room_name);
    let idx_room_tag = find_col(&header_lower, &mapping.room_tag);
    let idx_level = find_col(&header_lower, &mapping.level);
    let idx_size = find_col(&header_lower, &mapping.size);
    let idx_status = find_col(&header_lower, &mapping.status);
    let idx_notes = find_col(&header_lower, &mapping.notes);

    // Track unmapped columns
    let mapped_indices: Vec<usize> = [idx_tag, idx_mfr, idx_model, idx_cfm, idx_room_name, idx_room_tag, idx_level, idx_size, idx_status, idx_notes]
        .iter().filter_map(|i| *i).collect();
    let unmapped: Vec<String> = headers.iter().enumerate()
        .filter(|(i, _)| !mapped_indices.contains(i))
        .map(|(_, h)| h.clone())
        .collect();

    // Caches
    let mut product_types: HashMap<String, String> = HashMap::new(); // tag -> id
    let mut spaces: HashMap<String, String> = HashMap::new(); // tag -> id

    // Load existing product types and spaces
    for pt in doc.list_product_types()? {
        product_types.insert(pt.tag.clone(), pt.id.clone());
    }
    for s in doc.list_spaces()? {
        spaces.insert(s.tag.clone(), s.id.clone());
    }

    let mut rows_read = 0;
    let mut pt_created = 0;
    let mut p_created = 0;
    let mut s_created = 0;

    for result in reader.records() {
        let record = result?;
        rows_read += 1;

        let tag = get_field(&record, idx_tag).unwrap_or_else(|| format!("ITEM-{}", rows_read));
        let mfr = get_field(&record, idx_mfr);
        let model = get_field(&record, idx_model);
        let cfm: Option<f64> = get_field(&record, idx_cfm).and_then(|s| s.replace(',', "").parse().ok());
        let room_name = get_field(&record, idx_room_name);
        let room_tag = get_field(&record, idx_room_tag);
        let level = get_field(&record, idx_level).unwrap_or_else(|| "Level 1".into());
        let size = get_field(&record, idx_size);
        let status = get_field(&record, idx_status).unwrap_or_else(|| "new".into());
        let notes = get_field(&record, idx_notes);

        // Get or create product type
        let pt_id = if let Some(existing) = product_types.get(&tag) {
            existing.clone()
        } else {
            let id = generate_id();
            let domain = if tag.starts_with("AHU") || tag.starts_with("RTU") || tag.starts_with("EF")
                || tag.starts_with("CH-") || tag.starts_with("B-") || tag.starts_with("P-")
                || tag.starts_with("CT-") || tag.starts_with("VAV") {
                "equipment"
            } else {
                "air_device"
            };
            let category = infer_category(&tag, domain);
            doc.add_product_type(&ProductType {
                id: id.clone(), tag: tag.clone(), domain: domain.into(),
                category, manufacturer: mfr.clone(), model: model.clone(),
                description: None, mounting: None, finish: None,
                size_nominal: size.clone(), voltage: None, phase: None,
                hz: None, submittal_id: None,
            })?;
            product_types.insert(tag.clone(), id.clone());
            pt_created += 1;
            id
        };

        // Get or create space
        let space_id = if let Some(rt) = &room_tag {
            if let Some(existing) = spaces.get(rt) {
                Some(existing.clone())
            } else {
                let id = generate_id();
                doc.add_space(&Space {
                    id: id.clone(), tag: rt.clone(),
                    name: room_name.unwrap_or_else(|| rt.clone()),
                    level: level.clone(), space_type: None,
                    area_m2: None, ceiling_ht_m: None,
                    scope: "in_contract".into(), parent_id: None,
                    boundary_id: None, x: None, y: None,
                })?;
                spaces.insert(rt.clone(), id.clone());
                s_created += 1;
                Some(spaces[rt].clone())
            }
        } else {
            None
        };

        // Create placement
        doc.add_placement(&Placement {
            id: generate_id(), instance_tag: None,
            product_type_id: pt_id, space_id,
            level, x: None, y: None, rotation: None,
            cfm, cfm_balanced: None, static_pressure_pa: None,
            status, scope: "in_contract".into(), phase: "design".into(),
            weight_kg: None, notes,
        })?;
        p_created += 1;
    }

    doc.set_meta("modified_at", &chrono_now())?;

    Ok(ImportResult {
        rows_read,
        product_types_created: pt_created,
        placements_created: p_created,
        spaces_created: s_created,
        unmapped_columns: unmapped,
    })
}

fn find_col(headers: &[String], candidates: &[String]) -> Option<usize> {
    for candidate in candidates {
        for (i, header) in headers.iter().enumerate() {
            if header.contains(candidate.as_str()) {
                return Some(i);
            }
        }
    }
    None
}

fn get_field(record: &csv::StringRecord, idx: Option<usize>) -> Option<String> {
    idx.and_then(|i| record.get(i))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn infer_category(tag: &str, domain: &str) -> String {
    if domain == "equipment" {
        if tag.starts_with("AHU") { return "ahu".into(); }
        if tag.starts_with("RTU") { return "rtu".into(); }
        if tag.starts_with("EF") { return "exhaust_fan".into(); }
        if tag.starts_with("CH") { return "chiller".into(); }
        if tag.starts_with("B-") { return "boiler".into(); }
        if tag.starts_with("CT") { return "cooling_tower".into(); }
        if tag.starts_with("VAV") { return "vav_box".into(); }
        if tag.starts_with("P-") || tag.contains("PUMP") { return "pump".into(); }
        return "equipment".into();
    }
    if tag.starts_with("LD") || tag.starts_with("SD") { return "supply_diffuser".into(); }
    if tag.starts_with("SR") { return "supply_register".into(); }
    if tag.starts_with("CD") { return "ceiling_diffuser".into(); }
    if tag.starts_with("RG") { return "return_grille".into(); }
    if tag.starts_with("ER") { return "exhaust_register".into(); }
    if tag.starts_with("TG") { return "transfer_grille".into(); }
    if tag.starts_with("FSD") { return "fire_smoke_damper".into(); }
    "air_device".into()
}

fn chrono_now() -> String {
    // Simple ISO 8601 without chrono dependency
    "2026-04-05T00:00:00Z".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_basic_csv() {
        // Create a test CSV
        let csv_content = "Tag,Manufacturer,Model,CFM,Room Number,Room Name,Level\nLD-1,Titus,FL-10,185,L1-01,Sales Area,Level 1\nLD-1,Titus,FL-10,180,L1-01,Sales Area,Level 1\nSR-1,Titus,S300FL,90,L1-12,BOH Storage,Level 1\nEF-1,Broan,L-400L,210,L2-09,BOH Storage,Level 2\n";

        let csv_path = "test_import.csv";
        let sed_path = "test_import.sed";
        std::fs::write(csv_path, csv_content).unwrap();

        let result = import_csv(csv_path, sed_path, "Test Import", "TI-001", &ColumnMapping::default()).unwrap();

        assert_eq!(result.rows_read, 4);
        assert_eq!(result.product_types_created, 3); // LD-1, SR-1, EF-1
        assert_eq!(result.placements_created, 4);
        assert_eq!(result.spaces_created, 3); // L1-01, L1-12, L2-09

        // Verify the file
        let doc = SedDocument::open(sed_path).unwrap();
        let info = doc.info().unwrap();
        assert_eq!(info.project_name, "Test Import");
        assert_eq!(info.product_types, 3);
        assert_eq!(info.placements, 4);
        assert_eq!(info.spaces, 3);

        // EF-1 should be equipment domain
        let types = doc.list_product_types().unwrap();
        let ef = types.iter().find(|t| t.tag == "EF-1").unwrap();
        assert_eq!(ef.domain, "equipment");
        assert_eq!(ef.category, "exhaust_fan");

        std::fs::remove_file(csv_path).ok();
        std::fs::remove_file(sed_path).ok();
    }
}
