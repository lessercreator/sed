//! Structured diff engine for SED documents.
//!
//! Compare two .sed files and produce a precise, queryable list of changes.
//! Unlike PDF revision clouds, this tells you exactly what changed:
//! "AHU-1 CFM changed from 25000 to 28000" instead of a clouded area on a sheet.

use anyhow::Result;
use serde::Serialize;
use crate::document::SedDocument;

#[derive(Debug, Clone, Serialize)]
pub struct DiffResult {
    pub summary: DiffSummary,
    pub changes: Vec<DiffChange>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiffSummary {
    pub added: usize,
    pub removed: usize,
    pub modified: usize,
    pub unchanged: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiffChange {
    pub change_type: ChangeType,
    pub table: String,
    pub id: String,
    pub label: String,          // human-readable identifier (tag, name, etc.)
    pub fields: Vec<FieldDiff>, // for modifications: what fields changed
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum ChangeType {
    Added,
    Removed,
    Modified,
}

#[derive(Debug, Clone, Serialize)]
pub struct FieldDiff {
    pub field: String,
    pub old_value: String,
    pub new_value: String,
}

impl std::fmt::Display for DiffResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Diff: {} added, {} removed, {} modified, {} unchanged",
            self.summary.added, self.summary.removed, self.summary.modified, self.summary.unchanged)?;
        writeln!(f)?;

        for change in &self.changes {
            let symbol = match change.change_type {
                ChangeType::Added => "+",
                ChangeType::Removed => "-",
                ChangeType::Modified => "~",
            };
            writeln!(f, "{} [{}] {}", symbol, change.table, change.label)?;
            for fd in &change.fields {
                writeln!(f, "    {}: {} -> {}", fd.field, fd.old_value, fd.new_value)?;
            }
        }
        Ok(())
    }
}

/// Compare two SED documents and return structured differences.
pub fn diff(old: &SedDocument, new: &SedDocument) -> Result<DiffResult> {
    let mut changes = Vec::new();

    // Diff each major table
    diff_table(old, new, "spaces", "tag", "name",
        &["tag", "name", "level", "space_type", "scope", "area_m2", "ceiling_ht_m"],
        &mut changes)?;

    diff_table(old, new, "product_types", "tag", "tag",
        &["tag", "domain", "category", "manufacturer", "model", "description", "mounting", "size_nominal"],
        &mut changes)?;

    diff_placements(old, new, &mut changes)?;

    diff_table(old, new, "systems", "tag", "tag",
        &["tag", "name", "system_type", "medium"],
        &mut changes)?;

    diff_table(old, new, "keyed_notes", "key", "key",
        &["key", "text"],
        &mut changes)?;

    diff_table(old, new, "submittals", "description", "description",
        &["description", "status", "submitted_by", "company", "date_submitted"],
        &mut changes)?;

    diff_table(old, new, "sheets", "number", "number",
        &["number", "title", "discipline"],
        &mut changes)?;

    diff_segments(old, new, &mut changes)?;

    // Count summary
    let added = changes.iter().filter(|c| c.change_type == ChangeType::Added).count();
    let removed = changes.iter().filter(|c| c.change_type == ChangeType::Removed).count();
    let modified = changes.iter().filter(|c| c.change_type == ChangeType::Modified).count();

    // Count unchanged (total elements in new minus changed)
    let total_new: i64 = ["spaces", "product_types", "placements", "systems", "keyed_notes", "submittals", "sheets"]
        .iter()
        .map(|t| new.count(t).unwrap_or(0))
        .sum();
    let unchanged = (total_new as usize).saturating_sub(added + modified);

    Ok(DiffResult {
        summary: DiffSummary { added, removed, modified, unchanged },
        changes,
    })
}

/// Generic table diff using a label column for identification.
fn diff_table(
    old: &SedDocument, new: &SedDocument,
    table: &str, id_col: &str, label_col: &str,
    compare_cols: &[&str],
    changes: &mut Vec<DiffChange>,
) -> Result<()> {
    let cols_str = compare_cols.join(", ");
    let old_sql = format!("SELECT id, {} FROM {}", cols_str, table);
    let new_sql = format!("SELECT id, {} FROM {}", cols_str, table);

    let old_rows = old.query_raw(&old_sql)?;
    let new_rows = new.query_raw(&new_sql)?;

    // Build maps keyed by the id_col value
    let id_idx = compare_cols.iter().position(|c| *c == id_col).unwrap_or(0);
    let label_idx = compare_cols.iter().position(|c| *c == label_col).unwrap_or(0);

    let old_map: std::collections::HashMap<String, Vec<(String, String)>> = old_rows.into_iter()
        .map(|row| {
            let key = row[id_idx + 1].1.clone(); // +1 because id column is first
            (key, row)
        }).collect();

    let new_map: std::collections::HashMap<String, Vec<(String, String)>> = new_rows.into_iter()
        .map(|row| {
            let key = row[id_idx + 1].1.clone();
            (key, row)
        }).collect();

    // Find added
    for (key, row) in &new_map {
        if !old_map.contains_key(key) {
            changes.push(DiffChange {
                change_type: ChangeType::Added,
                table: table.into(),
                id: row[0].1.clone(),
                label: row[label_idx + 1].1.clone(),
                fields: vec![],
            });
        }
    }

    // Find removed
    for (key, row) in &old_map {
        if !new_map.contains_key(key) {
            changes.push(DiffChange {
                change_type: ChangeType::Removed,
                table: table.into(),
                id: row[0].1.clone(),
                label: row[label_idx + 1].1.clone(),
                fields: vec![],
            });
        }
    }

    // Find modified
    for (key, old_row) in &old_map {
        if let Some(new_row) = new_map.get(key) {
            let mut field_diffs = Vec::new();
            for (i, col) in compare_cols.iter().enumerate() {
                let old_val = &old_row[i + 1].1;
                let new_val = &new_row[i + 1].1;
                if old_val != new_val {
                    field_diffs.push(FieldDiff {
                        field: col.to_string(),
                        old_value: old_val.clone(),
                        new_value: new_val.clone(),
                    });
                }
            }
            if !field_diffs.is_empty() {
                changes.push(DiffChange {
                    change_type: ChangeType::Modified,
                    table: table.into(),
                    id: old_row[0].1.clone(),
                    label: old_row[label_idx + 1].1.clone(),
                    fields: field_diffs,
                });
            }
        }
    }

    Ok(())
}

/// Diff placements using instance_tag or product_type tag + space for identity.
fn diff_placements(old: &SedDocument, new: &SedDocument, changes: &mut Vec<DiffChange>) -> Result<()> {
    let sql = "SELECT p.id, COALESCE(p.instance_tag, pt.tag || '-' || COALESCE(s.tag, 'unassigned')) as label,
                      pt.tag, p.cfm, p.status, p.phase, p.scope, p.level,
                      s.tag as space_tag, p.instance_tag
               FROM placements p
               JOIN product_types pt ON p.product_type_id = pt.id
               LEFT JOIN spaces s ON p.space_id = s.id
               ORDER BY label";

    let old_rows = old.query_raw(sql)?;
    let new_rows = new.query_raw(sql)?;

    let old_map: std::collections::HashMap<String, Vec<(String, String)>> = old_rows.into_iter()
        .map(|row| (row[1].1.clone(), row)).collect();
    let new_map: std::collections::HashMap<String, Vec<(String, String)>> = new_rows.into_iter()
        .map(|row| (row[1].1.clone(), row)).collect();

    for (label, row) in &new_map {
        if !old_map.contains_key(label) {
            changes.push(DiffChange {
                change_type: ChangeType::Added, table: "placements".into(),
                id: row[0].1.clone(), label: label.clone(), fields: vec![],
            });
        }
    }

    for (label, row) in &old_map {
        if !new_map.contains_key(label) {
            changes.push(DiffChange {
                change_type: ChangeType::Removed, table: "placements".into(),
                id: row[0].1.clone(), label: label.clone(), fields: vec![],
            });
        }
    }

    let compare_fields = ["cfm", "status", "phase", "scope", "level"];
    let compare_indices = [3, 4, 5, 6, 7]; // indices in the SELECT

    for (label, old_row) in &old_map {
        if let Some(new_row) = new_map.get(label) {
            let mut field_diffs = Vec::new();
            for (fi, &col_idx) in compare_indices.iter().enumerate() {
                let old_val = &old_row[col_idx].1;
                let new_val = &new_row[col_idx].1;
                if old_val != new_val {
                    field_diffs.push(FieldDiff {
                        field: compare_fields[fi].to_string(),
                        old_value: old_val.clone(),
                        new_value: new_val.clone(),
                    });
                }
            }
            if !field_diffs.is_empty() {
                changes.push(DiffChange {
                    change_type: ChangeType::Modified, table: "placements".into(),
                    id: old_row[0].1.clone(), label: label.clone(), fields: field_diffs,
                });
            }
        }
    }

    Ok(())
}

/// Diff segments by system tag + from/to node positions.
fn diff_segments(old: &SedDocument, new: &SedDocument, changes: &mut Vec<DiffChange>) -> Result<()> {
    let sql = "SELECT seg.id, sys.tag || ':' || ROUND(n1.x,1) || ',' || ROUND(n1.y,1) || '->' || ROUND(n2.x,1) || ',' || ROUND(n2.y,1) as label,
                      seg.shape, seg.diameter_m, seg.width_m, seg.flow_design, seg.status
               FROM segments seg
               JOIN systems sys ON seg.system_id = sys.id
               JOIN nodes n1 ON seg.from_node_id = n1.id
               JOIN nodes n2 ON seg.to_node_id = n2.id
               WHERE n1.x IS NOT NULL";

    let old_rows = old.query_raw(sql).unwrap_or_default();
    let new_rows = new.query_raw(sql).unwrap_or_default();

    let old_map: std::collections::HashMap<String, Vec<(String, String)>> = old_rows.into_iter()
        .map(|row| (row[1].1.clone(), row)).collect();
    let new_map: std::collections::HashMap<String, Vec<(String, String)>> = new_rows.into_iter()
        .map(|row| (row[1].1.clone(), row)).collect();

    for (label, row) in &new_map {
        if !old_map.contains_key(label) {
            changes.push(DiffChange {
                change_type: ChangeType::Added, table: "segments".into(),
                id: row[0].1.clone(), label: label.clone(), fields: vec![],
            });
        }
    }
    for (label, row) in &old_map {
        if !new_map.contains_key(label) {
            changes.push(DiffChange {
                change_type: ChangeType::Removed, table: "segments".into(),
                id: row[0].1.clone(), label: label.clone(), fields: vec![],
            });
        }
    }

    let fields = ["shape", "diameter_m", "width_m", "flow_design", "status"];
    for (label, old_row) in &old_map {
        if let Some(new_row) = new_map.get(label) {
            let mut diffs = Vec::new();
            for (i, field) in fields.iter().enumerate() {
                let ov = &old_row[i + 2].1;
                let nv = &new_row[i + 2].1;
                if ov != nv {
                    diffs.push(FieldDiff { field: field.to_string(), old_value: ov.clone(), new_value: nv.clone() });
                }
            }
            if !diffs.is_empty() {
                changes.push(DiffChange {
                    change_type: ChangeType::Modified, table: "segments".into(),
                    id: old_row[0].1.clone(), label: label.clone(), fields: diffs,
                });
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::generate_id;
    use crate::types::*;

    fn make_base_doc() -> SedDocument {
        let doc = SedDocument::in_memory().unwrap();
        doc.set_meta("sed_version", "0.3").unwrap();
        doc.set_meta("project_name", "Test").unwrap();
        doc.set_meta("project_number", "T-001").unwrap();

        let pt_id = generate_id();
        doc.add_product_type(&ProductType {
            id: pt_id.clone(), tag: "LD-1".into(), domain: "air_device".into(),
            category: "supply_diffuser".into(), manufacturer: Some("Titus".into()),
            model: Some("FL-10".into()), description: None, mounting: None,
            finish: None, size_nominal: None, voltage: None, phase: None,
            hz: None, submittal_id: None,
        }).unwrap();

        let space_id = generate_id();
        doc.add_space(&Space {
            id: space_id.clone(), tag: "L1-01".into(), name: "Room A".into(),
            level: "Level 1".into(), space_type: Some("office".into()),
            area_m2: None, ceiling_ht_m: None, scope: "in_contract".into(),
            parent_id: None, boundary_id: None, x: None, y: None,
        }).unwrap();

        doc.add_placement(&Placement {
            id: generate_id(), instance_tag: Some("LD-1-01".into()),
            product_type_id: pt_id, space_id: Some(space_id),
            level: "Level 1".into(), x: None, y: None, rotation: None,
            cfm: Some(185.0), cfm_balanced: None, static_pressure_pa: None,
            status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
            weight_kg: None, notes: None,
        }).unwrap();

        doc
    }

    #[test]
    fn identical_docs_no_changes() {
        let doc = make_base_doc();
        let result = diff(&doc, &doc).unwrap();
        assert_eq!(result.summary.added, 0);
        assert_eq!(result.summary.removed, 0);
        assert_eq!(result.summary.modified, 0);
    }

    #[test]
    fn detect_added_space() {
        let old = make_base_doc();
        let new = make_base_doc();
        new.add_space(&Space {
            id: generate_id(), tag: "L1-02".into(), name: "Room B".into(),
            level: "Level 1".into(), space_type: Some("office".into()),
            area_m2: None, ceiling_ht_m: None, scope: "in_contract".into(),
            parent_id: None, boundary_id: None, x: None, y: None,
        }).unwrap();

        let result = diff(&old, &new).unwrap();
        assert!(result.summary.added > 0);
        assert!(result.changes.iter().any(|c|
            c.change_type == ChangeType::Added && c.table == "spaces" && c.label == "Room B"
        ));
    }

    #[test]
    fn detect_modified_cfm() {
        let old = make_base_doc();
        let new = make_base_doc();
        // Modify CFM on the placement
        let placements = new.list_placements().unwrap();
        new.update_placement(&placements[0].id, "cfm", Some("200")).unwrap();

        let result = diff(&old, &new).unwrap();
        assert!(result.summary.modified > 0);
        let cfm_change = result.changes.iter().find(|c|
            c.table == "placements" && c.change_type == ChangeType::Modified
        );
        assert!(cfm_change.is_some());
        let fd = &cfm_change.unwrap().fields;
        assert!(fd.iter().any(|f| f.field == "cfm"));
    }

    #[test]
    fn detect_removed_element() {
        let old = make_base_doc();
        let new = make_base_doc();
        // Remove the placement first (FK constraint), then the space
        let placements = new.list_placements().unwrap();
        for p in &placements { new.delete_placement(&p.id).unwrap(); }
        let spaces = new.list_spaces().unwrap();
        new.delete_space(&spaces[0].id).unwrap();

        let result = diff(&old, &new).unwrap();
        assert!(result.summary.removed > 0);
    }

    #[test]
    fn skims_self_diff_empty() {
        let tmp1 = tempfile::NamedTempFile::new().unwrap();
        let p1 = tmp1.path().to_str().unwrap().to_string();
        drop(tmp1);
        crate::examples::create_skims_americana(&p1).unwrap();
        let doc = SedDocument::open(&p1).unwrap();

        let result = diff(&doc, &doc).unwrap();
        assert_eq!(result.changes.len(), 0, "Self-diff should produce no changes");
    }
}
