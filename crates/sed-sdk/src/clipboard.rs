//! Copy/paste support for SED elements.
//!
//! Copies a placement (or set of placements) and creates duplicates
//! at an offset position. Preserves product type, CFM, status, and
//! other properties. Generates new UUIDs.

use anyhow::Result;
use crate::document::{SedDocument, generate_id};
use crate::types::*;

/// A clipboard entry — enough info to recreate a placement.
#[derive(Debug, Clone)]
pub struct ClipboardEntry {
    pub product_type_id: String,
    pub cfm: Option<f64>,
    pub status: String,
    pub scope: String,
    pub phase: String,
    pub instance_tag: Option<String>,
    pub notes: Option<String>,
    pub rel_x: f64, // relative to copy origin
    pub rel_y: f64,
}

/// Copy placements by ID. Returns clipboard entries relative to their centroid.
pub fn copy_placements(doc: &SedDocument, ids: &[String]) -> Result<Vec<ClipboardEntry>> {
    let mut entries = Vec::new();
    let mut xs = Vec::new();
    let mut ys = Vec::new();

    let placements = doc.list_placements()?;
    let selected: Vec<&Placement> = placements.iter().filter(|p| ids.contains(&p.id)).collect();

    for p in &selected {
        let x = p.x.unwrap_or(0.0);
        let y = p.y.unwrap_or(0.0);
        xs.push(x);
        ys.push(y);
    }

    if xs.is_empty() { return Ok(entries); }

    let cx = xs.iter().sum::<f64>() / xs.len() as f64;
    let cy = ys.iter().sum::<f64>() / ys.len() as f64;

    for p in &selected {
        entries.push(ClipboardEntry {
            product_type_id: p.product_type_id.clone(),
            cfm: p.cfm,
            status: p.status.clone(),
            scope: p.scope.clone(),
            phase: p.phase.clone(),
            instance_tag: None, // don't duplicate instance tags
            notes: p.notes.clone(),
            rel_x: p.x.unwrap_or(0.0) - cx,
            rel_y: p.y.unwrap_or(0.0) - cy,
        });
    }

    Ok(entries)
}

/// Paste clipboard entries at a target position on a given level.
/// Returns the IDs of the newly created placements.
pub fn paste_placements(
    doc: &SedDocument,
    entries: &[ClipboardEntry],
    target_x: f64,
    target_y: f64,
    level: &str,
) -> Result<Vec<String>> {
    let mut ids = Vec::new();
    for entry in entries {
        let id = generate_id();
        doc.add_placement(&Placement {
            id: id.clone(),
            instance_tag: entry.instance_tag.clone(),
            product_type_id: entry.product_type_id.clone(),
            space_id: None,
            level: level.into(),
            x: Some(target_x + entry.rel_x),
            y: Some(target_y + entry.rel_y),
            rotation: None,
            cfm: entry.cfm,
            cfm_balanced: None,
            static_pressure_pa: None,
            status: entry.status.clone(),
            scope: entry.scope.clone(),
            phase: entry.phase.clone(),
            weight_kg: None,
            notes: entry.notes.clone(),
        })?;
        ids.push(id);
    }
    Ok(ids)
}

/// Duplicate a single placement at an offset. Convenience wrapper.
pub fn duplicate_placement(doc: &SedDocument, placement_id: &str, offset_x: f64, offset_y: f64) -> Result<String> {
    let entries = copy_placements(doc, &[placement_id.into()])?;
    if entries.is_empty() {
        anyhow::bail!("Placement not found: {}", placement_id);
    }
    let placements = doc.list_placements()?;
    let original = placements.iter().find(|p| p.id == placement_id)
        .ok_or_else(|| anyhow::anyhow!("Placement not found"))?;
    let x = original.x.unwrap_or(0.0) + offset_x;
    let y = original.y.unwrap_or(0.0) + offset_y;
    let level = &original.level;
    let ids = paste_placements(doc, &entries, x, y, level)?;
    Ok(ids.into_iter().next().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_paste_round_trip() {
        let doc = SedDocument::in_memory().unwrap();
        doc.set_meta("sed_version", "0.3").unwrap();
        doc.set_meta("project_name", "Test").unwrap();
        doc.set_meta("project_number", "T").unwrap();

        let pt_id = generate_id();
        doc.add_product_type(&ProductType {
            id: pt_id.clone(), tag: "LD-1".into(), domain: "air_device".into(),
            category: "supply_diffuser".into(), manufacturer: None, model: None,
            description: None, mounting: None, finish: None, size_nominal: None,
            voltage: None, phase: None, hz: None, submittal_id: None,
        }).unwrap();

        let p1 = generate_id();
        doc.add_placement(&Placement {
            id: p1.clone(), instance_tag: None, product_type_id: pt_id.clone(),
            space_id: None, level: "Level 1".into(),
            x: Some(5.0), y: Some(10.0), rotation: None,
            cfm: Some(185.0), cfm_balanced: None, static_pressure_pa: None,
            status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
            weight_kg: None, notes: None,
        }).unwrap();

        let p2 = generate_id();
        doc.add_placement(&Placement {
            id: p2.clone(), instance_tag: None, product_type_id: pt_id.clone(),
            space_id: None, level: "Level 1".into(),
            x: Some(7.0), y: Some(10.0), rotation: None,
            cfm: Some(180.0), cfm_balanced: None, static_pressure_pa: None,
            status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
            weight_kg: None, notes: None,
        }).unwrap();

        // Copy both
        let entries = copy_placements(&doc, &[p1.clone(), p2.clone()]).unwrap();
        assert_eq!(entries.len(), 2);

        // Paste at offset
        let new_ids = paste_placements(&doc, &entries, 5.0, 15.0, "Level 2").unwrap();
        assert_eq!(new_ids.len(), 2);

        // Verify new placements exist on Level 2
        let all = doc.list_placements().unwrap();
        assert_eq!(all.len(), 4); // 2 original + 2 pasted
        let pasted: Vec<_> = all.iter().filter(|p| p.level == "Level 2").collect();
        assert_eq!(pasted.len(), 2);
        assert_eq!(pasted[0].cfm.or(pasted[1].cfm), Some(185.0)); // at least one has 185
    }

    #[test]
    fn duplicate_single() {
        let doc = SedDocument::in_memory().unwrap();
        doc.set_meta("sed_version", "0.3").unwrap();
        doc.set_meta("project_name", "T").unwrap();
        doc.set_meta("project_number", "T").unwrap();

        let pt_id = generate_id();
        doc.add_product_type(&ProductType {
            id: pt_id.clone(), tag: "SR-1".into(), domain: "air_device".into(),
            category: "supply_register".into(), manufacturer: None, model: None,
            description: None, mounting: None, finish: None, size_nominal: None,
            voltage: None, phase: None, hz: None, submittal_id: None,
        }).unwrap();

        let p_id = generate_id();
        doc.add_placement(&Placement {
            id: p_id.clone(), instance_tag: None, product_type_id: pt_id,
            space_id: None, level: "Level 1".into(),
            x: Some(3.0), y: Some(8.0), rotation: None,
            cfm: Some(110.0), cfm_balanced: None, static_pressure_pa: None,
            status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
            weight_kg: None, notes: None,
        }).unwrap();

        let dup_id = duplicate_placement(&doc, &p_id, 1.0, 0.0).unwrap();
        let all = doc.list_placements().unwrap();
        assert_eq!(all.len(), 2);

        let dup = all.iter().find(|p| p.id == dup_id).unwrap();
        assert_eq!(dup.x, Some(4.0)); // 3.0 + 1.0 offset
        assert_eq!(dup.y, Some(8.0));
        assert_eq!(dup.cfm, Some(110.0));
    }
}
