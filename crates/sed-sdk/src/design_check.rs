//! Design checks for SED documents.
//!
//! Analyzes duct sizing, air balance, equipment connectivity, and graph
//! topology to produce structured warnings and errors for engineers.

use anyhow::Result;
use serde::Serialize;
use crate::autosize;
use crate::document::SedDocument;

/// Severity levels for design issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// A single design issue found during analysis.
#[derive(Debug, Clone, Serialize)]
pub struct DesignIssue {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub element_id: Option<String>,
    pub element_table: Option<String>,
}

/// Run all design checks against the document.
pub fn check_design(doc: &SedDocument) -> Result<Vec<DesignIssue>> {
    let mut issues = Vec::new();

    check_duct_sizing(doc, &mut issues)?;
    check_missing_exhaust(doc, &mut issues)?;
    check_missing_return_path(doc, &mut issues)?;
    check_cfm_imbalance(doc, &mut issues)?;
    check_unconnected_equipment(doc, &mut issues)?;
    check_dead_end_duct(doc, &mut issues)?;
    check_high_velocity(doc, &mut issues)?;

    Ok(issues)
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Error => write!(f, "ERROR"),
            Severity::Warning => write!(f, "WARN"),
            Severity::Info => write!(f, "INFO"),
        }
    }
}

impl std::fmt::Display for DesignIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [{}]: {}", self.severity, self.code, self.message)
    }
}

// =========================================================================
// CHECK: Undersized and oversized duct
// =========================================================================

fn check_duct_sizing(doc: &SedDocument, issues: &mut Vec<DesignIssue>) -> Result<()> {
    let systems = doc.query_raw(
        "SELECT id, tag FROM systems WHERE medium = 'air'"
    )?;

    for sys_row in &systems {
        let sys_id = &sys_row[0].1;
        let sys_tag = &sys_row[1].1;

        let results = autosize::autosize_duct_system(doc, sys_id)?;

        for r in &results {
            if let Some(current) = r.current_diameter_in {
                if current > 0.0 && r.recommended_diameter_in > 0.0 {
                    let ratio = current / r.recommended_diameter_in;

                    if ratio < 0.80 {
                        issues.push(DesignIssue {
                            severity: Severity::Warning,
                            code: "UNDERSIZED_DUCT".into(),
                            message: format!(
                                "System {}: segment has {:.0}\" duct but {:.0}\" recommended for {:.0} CFM ({:.0}% of required)",
                                sys_tag, current, r.recommended_diameter_in, r.downstream_cfm, ratio * 100.0
                            ),
                            element_id: Some(r.segment_id.clone()),
                            element_table: Some("segments".into()),
                        });
                    }

                    if ratio > 1.50 {
                        issues.push(DesignIssue {
                            severity: Severity::Info,
                            code: "OVERSIZED_DUCT".into(),
                            message: format!(
                                "System {}: segment has {:.0}\" duct but only {:.0}\" needed for {:.0} CFM ({:.0}% of required)",
                                sys_tag, current, r.recommended_diameter_in, r.downstream_cfm, ratio * 100.0
                            ),
                            element_id: Some(r.segment_id.clone()),
                            element_table: Some("segments".into()),
                        });
                    }
                }
            }
        }
    }

    Ok(())
}

// =========================================================================
// CHECK: Missing exhaust in restrooms
// =========================================================================

fn check_missing_exhaust(doc: &SedDocument, issues: &mut Vec<DesignIssue>) -> Result<()> {
    // Find restroom spaces that have no exhaust device placement.
    let rows = doc.query_raw(
        "SELECT s.id, s.tag, s.name FROM spaces s
         WHERE s.space_type = 'restroom'
         AND s.id NOT IN (
             SELECT p.space_id FROM placements p
             JOIN product_types pt ON p.product_type_id = pt.id
             WHERE pt.category LIKE '%exhaust%'
             AND p.space_id IS NOT NULL
         )"
    )?;

    for row in &rows {
        issues.push(DesignIssue {
            severity: Severity::Error,
            code: "MISSING_EXHAUST".into(),
            message: format!("Restroom {} ({}) has no exhaust device", row[1].1, row[2].1),
            element_id: Some(row[0].1.clone()),
            element_table: Some("spaces".into()),
        });
    }

    Ok(())
}

// =========================================================================
// CHECK: Missing return path (supply but no return or transfer grille)
// =========================================================================

fn check_missing_return_path(doc: &SedDocument, issues: &mut Vec<DesignIssue>) -> Result<()> {
    // Spaces that have supply devices but no return grille or transfer grille.
    let rows = doc.query_raw(
        "SELECT s.id, s.tag, s.name FROM spaces s
         WHERE s.id IN (
             SELECT p.space_id FROM placements p
             JOIN product_types pt ON p.product_type_id = pt.id
             WHERE (pt.category LIKE 'supply%' OR pt.category LIKE 'ceiling_diffuser%')
             AND p.space_id IS NOT NULL
         )
         AND s.id NOT IN (
             SELECT p.space_id FROM placements p
             JOIN product_types pt ON p.product_type_id = pt.id
             WHERE (pt.category LIKE 'return%' OR pt.category LIKE 'transfer%')
             AND p.space_id IS NOT NULL
         )"
    )?;

    for row in &rows {
        issues.push(DesignIssue {
            severity: Severity::Warning,
            code: "MISSING_RETURN_PATH".into(),
            message: format!("Space {} ({}) has supply air but no return grille or transfer grille", row[1].1, row[2].1),
            element_id: Some(row[0].1.clone()),
            element_table: Some("spaces".into()),
        });
    }

    Ok(())
}

// =========================================================================
// CHECK: CFM imbalance — supply CFM vs design schedule
// =========================================================================

fn check_cfm_imbalance(doc: &SedDocument, issues: &mut Vec<DesignIssue>) -> Result<()> {
    // For rooms with both supply devices and schedule_data entries, check if they match.
    // If no schedule_data table exists or is empty, compare supply vs. return+exhaust per space.
    let rows = doc.query_raw(
        "SELECT s.id, s.tag, s.name,
                COALESCE((SELECT SUM(p.cfm) FROM placements p
                 JOIN product_types pt ON p.product_type_id = pt.id
                 WHERE p.space_id = s.id AND (pt.category LIKE 'supply%' OR pt.category LIKE 'ceiling_diffuser%')
                 AND p.cfm IS NOT NULL), 0) as supply_cfm,
                COALESCE((SELECT SUM(p.cfm) FROM placements p
                 JOIN product_types pt ON p.product_type_id = pt.id
                 WHERE p.space_id = s.id AND (pt.category LIKE 'return%' OR pt.category LIKE 'exhaust%')
                 AND p.cfm IS NOT NULL), 0) as extract_cfm
         FROM spaces s
         WHERE s.space_type NOT IN ('elevator', 'circulation', 'mechanical')
         AND s.scope != 'nic'"
    )?;

    for row in &rows {
        let supply: f64 = row[3].1.parse().unwrap_or(0.0);
        let extract: f64 = row[4].1.parse().unwrap_or(0.0);

        // Only flag if both are non-zero and differ significantly.
        if supply > 0.0 && extract > 0.0 {
            let diff_pct = ((supply - extract) / supply).abs();
            if diff_pct > 0.10 {
                let severity = if diff_pct > 0.25 { Severity::Warning } else { Severity::Info };
                issues.push(DesignIssue {
                    severity,
                    code: "CFM_IMBALANCE".into(),
                    message: format!(
                        "Space {} ({}): supply {:.0} CFM vs extract {:.0} CFM ({:.0}% difference)",
                        row[1].1, row[2].1, supply, extract, diff_pct * 100.0
                    ),
                    element_id: Some(row[0].1.clone()),
                    element_table: Some("spaces".into()),
                });
            }
        }
    }

    Ok(())
}

// =========================================================================
// CHECK: Unconnected equipment
// =========================================================================

fn check_unconnected_equipment(doc: &SedDocument, issues: &mut Vec<DesignIssue>) -> Result<()> {
    let rows = doc.query_raw(
        "SELECT p.id, COALESCE(p.instance_tag, pt.tag) as tag FROM placements p
         JOIN product_types pt ON p.product_type_id = pt.id
         WHERE pt.domain = 'equipment'
         AND p.id NOT IN (SELECT placement_id FROM placement_systems WHERE placement_id IS NOT NULL)
         AND p.id NOT IN (SELECT source_id FROM systems WHERE source_id IS NOT NULL)
         AND p.id NOT IN (SELECT placement_id FROM nodes WHERE placement_id IS NOT NULL)"
    )?;

    for row in &rows {
        issues.push(DesignIssue {
            severity: Severity::Warning,
            code: "UNCONNECTED_EQUIPMENT".into(),
            message: format!("Equipment '{}' is not linked to any system or graph node", row[1].1),
            element_id: Some(row[0].1.clone()),
            element_table: Some("placements".into()),
        });
    }

    Ok(())
}

// =========================================================================
// CHECK: Dead-end duct nodes
// =========================================================================

fn check_dead_end_duct(doc: &SedDocument, issues: &mut Vec<DesignIssue>) -> Result<()> {
    // A dead-end node has only one segment connection and is not a terminal,
    // equipment_conn, or cap.
    let rows = doc.query_raw(
        "SELECT n.id, n.node_type, n.system_id,
                (SELECT COUNT(*) FROM segments seg
                 WHERE seg.from_node_id = n.id OR seg.to_node_id = n.id) as seg_count
         FROM nodes n
         WHERE n.node_type NOT IN ('terminal', 'equipment_conn', 'cap')"
    )?;

    for row in &rows {
        let seg_count: i64 = row[3].1.parse().unwrap_or(0);
        if seg_count == 1 {
            // Look up system tag for a useful message.
            let sys_rows = doc.query_raw(&format!(
                "SELECT tag FROM systems WHERE id = '{}'", row[2].1
            )).unwrap_or_default();
            let sys_tag = sys_rows.first()
                .map(|r| r[0].1.as_str())
                .unwrap_or("unknown");

            issues.push(DesignIssue {
                severity: Severity::Warning,
                code: "DEAD_END_DUCT".into(),
                message: format!(
                    "Node {} in system {} is a dead-end (only 1 segment connection, type '{}')",
                    row[0].1, sys_tag, row[1].1
                ),
                element_id: Some(row[0].1.clone()),
                element_table: Some("nodes".into()),
            });
        }
    }

    Ok(())
}

// =========================================================================
// CHECK: High velocity duct segments
// =========================================================================

fn check_high_velocity(doc: &SedDocument, issues: &mut Vec<DesignIssue>) -> Result<()> {
    // Velocity = CFM / Area (ft^2). For round duct: Area = pi * (D/2)^2 / 144
    // where D is in inches and we convert to ft^2.
    // Flag if velocity > 1500 FPM.
    let rows = doc.query_raw(
        "SELECT seg.id, seg.system_id, seg.diameter_m, seg.flow_design
         FROM segments seg
         WHERE seg.diameter_m IS NOT NULL AND seg.flow_design IS NOT NULL
         AND seg.diameter_m > 0"
    )?;

    for row in &rows {
        let diameter_m: f64 = row[2].1.parse().unwrap_or(0.0);
        let flow_cfm: f64 = row[3].1.parse().unwrap_or(0.0);

        if diameter_m <= 0.0 || flow_cfm <= 0.0 {
            continue;
        }

        let diameter_in = diameter_m / 0.0254;
        let area_ft2 = std::f64::consts::PI * (diameter_in / 2.0).powi(2) / 144.0;
        let velocity_fpm = flow_cfm / area_ft2;

        if velocity_fpm > 1500.0 {
            let sys_rows = doc.query_raw(&format!(
                "SELECT tag FROM systems WHERE id = '{}'", row[1].1
            )).unwrap_or_default();
            let sys_tag = sys_rows.first()
                .map(|r| r[0].1.as_str())
                .unwrap_or("unknown");

            let severity = if velocity_fpm > 2000.0 { Severity::Error } else { Severity::Warning };
            issues.push(DesignIssue {
                severity,
                code: "HIGH_VELOCITY".into(),
                message: format!(
                    "System {}: segment with {:.0}\" duct at {:.0} CFM = {:.0} FPM (limit 1500 FPM)",
                    sys_tag, diameter_in, flow_cfm, velocity_fpm
                ),
                element_id: Some(row[0].1.clone()),
                element_table: Some("segments".into()),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::SedDocument;

    fn open_skims() -> SedDocument {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        drop(tmp);
        crate::examples::create_skims_americana(&path).unwrap();
        SedDocument::open(&path).unwrap()
    }

    #[test]
    fn check_design_runs_on_skims() {
        let doc = open_skims();
        let issues = check_design(&doc).unwrap();
        // Should complete without error and return some issues.
        // The SKIMS project is real-world data so we expect some info/warnings.
        for issue in &issues {
            println!("{}", issue);
        }
    }

    #[test]
    fn check_design_empty_doc() {
        let doc = SedDocument::in_memory().unwrap();
        let issues = check_design(&doc).unwrap();
        assert!(issues.is_empty(), "Empty doc should have no design issues");
    }

    #[test]
    fn undersized_duct_detected() {
        // Create a doc with a deliberately undersized duct.
        let doc = SedDocument::in_memory().unwrap();
        crate::schema::create_schema(&doc.conn).ok();

        let sys_id = crate::document::generate_id();
        doc.add_system(&crate::types::System {
            id: sys_id.clone(), tag: "TEST-SA".into(), name: "Test Supply".into(),
            system_type: "supply".into(), medium: "air".into(),
            source_id: None, paired_system_id: None,
        }).unwrap();

        let equip_node = crate::document::generate_id();
        doc.add_node(&crate::types::Node {
            id: equip_node.clone(), system_id: sys_id.clone(),
            node_type: "equipment_conn".into(), placement_id: None,
            fitting_type: None, size_description: None,
            level: None, x: None, y: None,
        }).unwrap();

        // Create a placement with very high CFM so 6" duct is clearly undersized.
        // At 10000 CFM, D = 0.0344 * 10000^0.612 ~ 9.65" -> rounds to 10".
        // 6" / 10" = 60% < 80% threshold.
        let placement_id = crate::document::generate_id();
        let pt_id = crate::document::generate_id();
        doc.add_product_type(&crate::types::ProductType {
            id: pt_id.clone(), tag: "SD-1".into(), domain: "air_device".into(),
            category: "supply_diffuser".into(), manufacturer: None, model: None,
            description: None, mounting: None, finish: None, size_nominal: None,
            voltage: None, phase: None, hz: None, submittal_id: None,
        }).unwrap();
        doc.add_placement(&crate::types::Placement {
            id: placement_id.clone(), instance_tag: None,
            product_type_id: pt_id, space_id: None, level: "Level 1".into(),
            x: None, y: None, rotation: None,
            cfm: Some(10000.0), cfm_balanced: None, static_pressure_pa: None,
            status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
            weight_kg: None, notes: None,
        }).unwrap();

        let terminal_node = crate::document::generate_id();
        doc.add_node(&crate::types::Node {
            id: terminal_node.clone(), system_id: sys_id.clone(),
            node_type: "terminal".into(), placement_id: Some(placement_id),
            fitting_type: None, size_description: None,
            level: None, x: None, y: None,
        }).unwrap();

        // Create a deliberately undersized segment: 6" duct for 10000 CFM.
        doc.add_segment(&crate::types::Segment {
            id: crate::document::generate_id(), system_id: sys_id,
            from_node_id: equip_node, to_node_id: terminal_node,
            shape: "round".into(), width_m: None, height_m: None,
            diameter_m: Some(6.0 * 0.0254), length_m: Some(3.0),
            material: "galvanized".into(), gauge: None,
            pressure_class: None, construction: None, exposure: None,
            flow_design: Some(10000.0), flow_balanced: None,
            status: "new".into(), scope: "in_contract".into(),
        }).unwrap();

        let issues = check_design(&doc).unwrap();
        let undersized: Vec<&DesignIssue> = issues.iter()
            .filter(|i| i.code == "UNDERSIZED_DUCT")
            .collect();
        assert!(!undersized.is_empty(), "Should detect undersized duct: {:?}", issues);
    }

    #[test]
    fn high_velocity_detected() {
        let doc = SedDocument::in_memory().unwrap();
        crate::schema::create_schema(&doc.conn).ok();

        let sys_id = crate::document::generate_id();
        doc.add_system(&crate::types::System {
            id: sys_id.clone(), tag: "HV-SA".into(), name: "High Vel".into(),
            system_type: "supply".into(), medium: "air".into(),
            source_id: None, paired_system_id: None,
        }).unwrap();

        let n1 = crate::document::generate_id();
        let n2 = crate::document::generate_id();
        doc.add_node(&crate::types::Node {
            id: n1.clone(), system_id: sys_id.clone(),
            node_type: "equipment_conn".into(), placement_id: None,
            fitting_type: None, size_description: None, level: None, x: None, y: None,
        }).unwrap();
        doc.add_node(&crate::types::Node {
            id: n2.clone(), system_id: sys_id.clone(),
            node_type: "terminal".into(), placement_id: None,
            fitting_type: None, size_description: None, level: None, x: None, y: None,
        }).unwrap();

        // 6" duct with 500 CFM -> area = pi*(3)^2/144 = 0.196 ft^2
        // velocity = 500/0.196 = 2549 FPM (> 1500)
        doc.add_segment(&crate::types::Segment {
            id: crate::document::generate_id(), system_id: sys_id,
            from_node_id: n1, to_node_id: n2,
            shape: "round".into(), width_m: None, height_m: None,
            diameter_m: Some(6.0 * 0.0254), length_m: Some(3.0),
            material: "galvanized".into(), gauge: None,
            pressure_class: None, construction: None, exposure: None,
            flow_design: Some(500.0), flow_balanced: None,
            status: "new".into(), scope: "in_contract".into(),
        }).unwrap();

        let issues = check_design(&doc).unwrap();
        let hv: Vec<&DesignIssue> = issues.iter()
            .filter(|i| i.code == "HIGH_VELOCITY")
            .collect();
        assert!(!hv.is_empty(), "Should detect high velocity: {:?}", issues);
    }
}
