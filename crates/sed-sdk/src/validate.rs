//! Validation rules for SED documents.
//!
//! Returns structured errors and warnings that can be consumed by CLI, UI, or AI.

use anyhow::Result;
use serde::Serialize;
use crate::document::SedDocument;

#[derive(Debug, Clone, Serialize)]
pub struct ValidationResult {
    pub errors: Vec<ValidationItem>,
    pub warnings: Vec<ValidationItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationItem {
    pub code: String,
    pub message: String,
    pub table: Option<String>,
    pub element_id: Option<String>,
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

impl std::fmt::Display for ValidationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.errors.is_empty() && self.warnings.is_empty() {
            writeln!(f, "VALID — no errors, no warnings")?;
            return Ok(());
        }
        for e in &self.errors {
            writeln!(f, "ERROR [{}]: {}", e.code, e.message)?;
        }
        for w in &self.warnings {
            writeln!(f, "WARN  [{}]: {}", w.code, w.message)?;
        }
        writeln!(f, "\n{} error(s), {} warning(s)", self.errors.len(), self.warnings.len())?;
        Ok(())
    }
}

pub fn validate(doc: &SedDocument) -> Result<ValidationResult> {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // =========================================================================
    // META
    // =========================================================================

    for key in ["sed_version", "project_name", "project_number"] {
        if doc.get_meta(key)?.is_none() {
            errors.push(ValidationItem {
                code: "META_MISSING".into(),
                message: format!("Required meta key '{}' is missing", key),
                table: Some("meta".into()),
                element_id: None,
            });
        }
    }

    // =========================================================================
    // REFERENTIAL INTEGRITY
    // =========================================================================

    // Orphaned placements (product_type_id doesn't exist)
    let count = count_query(doc, "SELECT COUNT(*) FROM placements p LEFT JOIN product_types pt ON p.product_type_id = pt.id WHERE pt.id IS NULL")?;
    if count > 0 {
        errors.push(item("REF_ORPHAN_PLACEMENT", format!("{} placements reference non-existent product types", count), "placements"));
    }

    // Orphaned nodes (system_id doesn't exist)
    let count = count_query(doc, "SELECT COUNT(*) FROM nodes n LEFT JOIN systems sys ON n.system_id = sys.id WHERE sys.id IS NULL")?;
    if count > 0 {
        errors.push(item("REF_ORPHAN_NODE", format!("{} nodes reference non-existent systems", count), "nodes"));
    }

    // Orphaned segments (from_node or to_node doesn't exist)
    let count = count_query(doc, "SELECT COUNT(*) FROM segments seg LEFT JOIN nodes n1 ON seg.from_node_id = n1.id WHERE n1.id IS NULL")?;
    if count > 0 {
        errors.push(item("REF_ORPHAN_SEGMENT", format!("{} segments have missing from_node", count), "segments"));
    }
    let count = count_query(doc, "SELECT COUNT(*) FROM segments seg LEFT JOIN nodes n2 ON seg.to_node_id = n2.id WHERE n2.id IS NULL")?;
    if count > 0 {
        errors.push(item("REF_ORPHAN_SEGMENT", format!("{} segments have missing to_node", count), "segments"));
    }

    // =========================================================================
    // GRAPH INTEGRITY
    // =========================================================================

    // Disconnected nodes (not referenced by any segment)
    let seg_count = count_query(doc, "SELECT COUNT(*) FROM segments")?;
    if seg_count > 0 {
        let count = count_query(doc, "SELECT COUNT(*) FROM nodes n WHERE n.id NOT IN (SELECT from_node_id FROM segments) AND n.id NOT IN (SELECT to_node_id FROM segments)")?;
        if count > 0 {
            warnings.push(item("GRAPH_DISCONNECTED_NODE", format!("{} graph nodes are not connected to any segment", count), "nodes"));
        }
    }

    // Segments with zero or negative length
    let count = count_query(doc, "SELECT COUNT(*) FROM segments WHERE length_m IS NOT NULL AND length_m <= 0")?;
    if count > 0 {
        warnings.push(item("GRAPH_ZERO_LENGTH", format!("{} segments have zero or negative length", count), "segments"));
    }

    // Self-referencing segments (from == to)
    let count = count_query(doc, "SELECT COUNT(*) FROM segments WHERE from_node_id = to_node_id")?;
    if count > 0 {
        errors.push(item("GRAPH_SELF_REF", format!("{} segments connect a node to itself", count), "segments"));
    }

    // =========================================================================
    // DATA QUALITY
    // =========================================================================

    // New placements without a space assignment
    let count = count_query(doc, "SELECT COUNT(*) FROM placements WHERE space_id IS NULL AND status = 'new'")?;
    if count > 0 {
        warnings.push(item("DATA_NO_SPACE", format!("{} new placements have no assigned space", count), "placements"));
    }

    // Placements without coordinates
    let total_placements = count_query(doc, "SELECT COUNT(*) FROM placements")?;
    let positioned = count_query(doc, "SELECT COUNT(*) FROM placements WHERE x IS NOT NULL AND y IS NOT NULL")?;
    if total_placements > 0 && positioned < total_placements {
        let unpositioned = total_placements - positioned;
        warnings.push(item("DATA_NO_COORDS", format!("{} of {} placements have no coordinates", unpositioned, total_placements), "placements"));
    }

    // Equipment without instance tags
    let count = count_query(doc, "SELECT COUNT(*) FROM placements p JOIN product_types pt ON p.product_type_id = pt.id WHERE pt.domain = 'equipment' AND p.instance_tag IS NULL")?;
    if count > 0 {
        warnings.push(item("DATA_NO_INSTANCE_TAG", format!("{} equipment placements have no instance tag", count), "placements"));
    }

    // Product types with no placements (unused catalog entries)
    let count = count_query(doc, "SELECT COUNT(*) FROM product_types pt WHERE pt.id NOT IN (SELECT product_type_id FROM placements)")?;
    if count > 0 {
        warnings.push(item("DATA_UNUSED_TYPE", format!("{} product types have no placements", count), "product_types"));
    }

    // =========================================================================
    // SUBMITTALS
    // =========================================================================

    // Product types with no submittal
    let count = count_query(doc, "SELECT COUNT(*) FROM product_types WHERE submittal_id IS NULL AND domain != 'accessory'")?;
    if count > 0 {
        warnings.push(item("SUB_MISSING", format!("{} product types (non-accessory) have no submittal linked", count), "product_types"));
    }

    // =========================================================================
    // SHEETS AND VIEWS
    // =========================================================================

    // Sheets with no views
    let count = count_query(doc, "SELECT COUNT(*) FROM sheets s WHERE s.id NOT IN (SELECT sheet_id FROM views)")?;
    if count > 0 {
        warnings.push(item("SHEET_NO_VIEWS", format!("{} sheets have no views defined", count), "sheets"));
    }

    // =========================================================================
    // SPATIAL INDEX
    // =========================================================================

    let spatial_count = count_query(doc, "SELECT COUNT(*) FROM spatial_idx")?;
    if positioned > 0 && spatial_count == 0 {
        warnings.push(item("SPATIAL_EMPTY", "Spatial index is empty but positioned elements exist".into(), "spatial_idx"));
    }

    // Orphaned spatial_map entries
    let count = count_query(doc, "SELECT COUNT(*) FROM spatial_map sm LEFT JOIN spatial_idx si ON sm.spatial_id = si.id WHERE si.id IS NULL")?;
    if count > 0 {
        warnings.push(item("SPATIAL_ORPHAN", format!("{} spatial_map entries reference missing spatial_idx rows", count), "spatial_map"));
    }

    // =========================================================================
    // HYDRONIC PAIR INTEGRITY
    // =========================================================================

    // Systems with paired_system_id that doesn't point back
    let count = count_query(doc, "SELECT COUNT(*) FROM systems s1 JOIN systems s2 ON s1.paired_system_id = s2.id WHERE s2.paired_system_id IS NULL OR s2.paired_system_id != s1.id")?;
    if count > 0 {
        warnings.push(item("SYS_PAIR_BROKEN", format!("{} system pairs are not bidirectional", count), "systems"));
    }

    Ok(ValidationResult { errors, warnings })
}

fn count_query(doc: &SedDocument, sql: &str) -> Result<i64> {
    let rows = doc.query_raw(sql)?;
    if let Some(row) = rows.first() {
        Ok(row[0].1.parse().unwrap_or(0))
    } else {
        Ok(0)
    }
}

fn item(code: &str, message: String, table: &str) -> ValidationItem {
    ValidationItem {
        code: code.into(),
        message,
        table: Some(table.into()),
        element_id: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::generate_id;
    use crate::types::*;

    #[test]
    fn empty_doc_reports_meta_errors() {
        let doc = SedDocument::in_memory().unwrap();
        let result = validate(&doc).unwrap();
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.code == "META_MISSING"));
    }

    #[test]
    fn valid_doc_passes() {
        let doc = SedDocument::in_memory().unwrap();
        doc.set_meta("sed_version", "0.3").unwrap();
        doc.set_meta("project_name", "Test").unwrap();
        doc.set_meta("project_number", "T-001").unwrap();
        let result = validate(&doc).unwrap();
        assert!(result.is_valid());
    }

    #[test]
    fn skims_validates() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        drop(tmp);
        crate::examples::create_skims_americana(&path).unwrap();
        let doc = SedDocument::open(&path).unwrap();
        let result = validate(&doc).unwrap();
        assert!(result.is_valid(), "SKIMS should have no errors: {:?}", result.errors);
    }

    #[test]
    fn office_validates() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        drop(tmp);
        crate::examples_office::create_office_tower(&path).unwrap();
        let doc = SedDocument::open(&path).unwrap();
        let result = validate(&doc).unwrap();
        assert!(result.is_valid(), "Office tower should have no errors: {:?}", result.errors);
    }

    #[test]
    fn self_ref_segment_caught() {
        let doc = SedDocument::in_memory().unwrap();
        doc.set_meta("sed_version", "0.3").unwrap();
        doc.set_meta("project_name", "T").unwrap();
        doc.set_meta("project_number", "T").unwrap();

        let sys_id = generate_id();
        doc.add_system(&System {
            id: sys_id.clone(), tag: "S1".into(), name: "Test".into(),
            system_type: "supply".into(), medium: "air".into(),
            source_id: None, paired_system_id: None,
        }).unwrap();

        let node_id = generate_id();
        doc.add_node(&Node {
            id: node_id.clone(), system_id: sys_id.clone(),
            node_type: "junction".into(), placement_id: None,
            fitting_type: None, size_description: None,
            level: None, x: None, y: None,
        }).unwrap();

        doc.add_segment(&Segment {
            id: generate_id(), system_id: sys_id,
            from_node_id: node_id.clone(), to_node_id: node_id,
            shape: "round".into(), width_m: None, height_m: None,
            diameter_m: Some(0.2), length_m: Some(1.0),
            material: "galvanized".into(), gauge: None,
            pressure_class: None, construction: None, exposure: None,
            flow_design: None, flow_balanced: None,
            status: "new".into(), scope: "in_contract".into(),
        }).unwrap();

        let result = validate(&doc).unwrap();
        assert!(result.errors.iter().any(|e| e.code == "GRAPH_SELF_REF"));
    }
}
