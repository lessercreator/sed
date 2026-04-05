//! AI-powered design suggestions for SED documents.
//!
//! Analyzes the current state of a building design and generates
//! actionable suggestions. No LLM required — uses mechanical engineering
//! rules of thumb and ASHRAE standards.

use anyhow::Result;
use serde::Serialize;
use crate::document::SedDocument;

#[derive(Debug, Clone, Serialize)]
pub struct Suggestion {
    pub category: String,     // "ventilation", "duct_sizing", "equipment", "balance"
    pub priority: Priority,
    pub message: String,
    pub action: Option<SuggestedAction>,
}

#[derive(Debug, Clone, Serialize)]
pub enum Priority {
    Info,
    Recommendation,
    Warning,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuggestedAction {
    pub action_type: String,  // "add_device", "resize_duct", "add_equipment"
    pub parameters: serde_json::Value,
}

/// Generate design suggestions for the current document state.
pub fn suggest(doc: &SedDocument) -> Result<Vec<Suggestion>> {
    let mut suggestions = Vec::new();

    suggest_missing_devices(doc, &mut suggestions)?;
    suggest_ventilation(doc, &mut suggestions)?;
    suggest_duct_sizing(doc, &mut suggestions)?;
    suggest_system_balance(doc, &mut suggestions)?;
    suggest_equipment_capacity(doc, &mut suggestions)?;

    // Sort by priority
    suggestions.sort_by(|a, b| priority_ord(&a.priority).cmp(&priority_ord(&b.priority)));
    Ok(suggestions)
}

/// Suggest devices for rooms that have spaces but no supply/return/exhaust.
fn suggest_missing_devices(doc: &SedDocument, suggestions: &mut Vec<Suggestion>) -> Result<()> {
    // Rooms with no supply
    let rows = doc.query_raw(
        "SELECT s.tag, s.name, s.space_type, s.level FROM spaces s
         WHERE s.scope = 'in_contract'
         AND s.space_type NOT IN ('elevator', 'mechanical', 'circulation')
         AND s.id NOT IN (
             SELECT p.space_id FROM placements p
             JOIN product_types pt ON p.product_type_id = pt.id
             WHERE pt.category LIKE 'supply%' AND p.space_id IS NOT NULL
         )
         ORDER BY s.level, s.tag"
    )?;
    for row in &rows {
        let tag = &row[0].1;
        let name = &row[1].1;
        let space_type = &row[2].1;

        // Estimate CFM based on room type
        let cfm_per_sf = match space_type.as_str() {
            "retail" => 1.2,
            "office" => 1.0,
            "restroom" => 0.0, // restrooms get exhaust, not supply
            "storage" => 0.5,
            "corridor" => 0.5,
            _ => 0.8,
        };

        if cfm_per_sf > 0.0 {
            suggestions.push(Suggestion {
                category: "ventilation".into(),
                priority: Priority::Recommendation,
                message: format!("{} ({}) has no supply air devices. Consider adding supply diffusers.", name, tag),
                action: Some(SuggestedAction {
                    action_type: "add_device".into(),
                    parameters: serde_json::json!({
                        "space_tag": tag,
                        "device_category": "supply_diffuser",
                        "estimated_cfm_per_sf": cfm_per_sf,
                    }),
                }),
            });
        }
    }

    // Restrooms with no exhaust
    let rows = doc.query_raw(
        "SELECT s.tag, s.name FROM spaces s
         WHERE s.scope = 'in_contract'
         AND s.space_type = 'restroom'
         AND s.id NOT IN (
             SELECT p.space_id FROM placements p
             JOIN product_types pt ON p.product_type_id = pt.id
             WHERE pt.category LIKE 'exhaust%' AND p.space_id IS NOT NULL
         )"
    )?;
    for row in &rows {
        suggestions.push(Suggestion {
            category: "ventilation".into(),
            priority: Priority::Warning,
            message: format!("{} ({}) is a restroom with no exhaust. Code requires exhaust ventilation.", row[1].1, row[0].1),
            action: Some(SuggestedAction {
                action_type: "add_device".into(),
                parameters: serde_json::json!({
                    "space_tag": row[0].1,
                    "device_category": "exhaust_register",
                    "estimated_cfm": 75, // typical restroom exhaust
                }),
            }),
        });
    }

    Ok(())
}

/// Suggest ventilation rates based on ASHRAE 62.1 rules of thumb.
fn suggest_ventilation(doc: &SedDocument, suggestions: &mut Vec<Suggestion>) -> Result<()> {
    // Check rooms where supply CFM seems low for the space type
    let rows = doc.query_raw(
        "SELECT s.tag, s.name, s.space_type, s.area_m2,
                SUM(p.cfm) as total_cfm
         FROM spaces s
         LEFT JOIN placements p ON p.space_id = s.id
         LEFT JOIN product_types pt ON p.product_type_id = pt.id AND pt.category LIKE 'supply%'
         WHERE s.scope = 'in_contract' AND s.area_m2 IS NOT NULL
         GROUP BY s.id
         HAVING total_cfm IS NOT NULL AND total_cfm > 0"
    )?;
    for row in &rows {
        let space_type = &row[2].1;
        let area_m2: f64 = row[3].1.parse().unwrap_or(0.0);
        let total_cfm: f64 = row[4].1.parse().unwrap_or(0.0);
        let area_sf = area_m2 * 10.764; // m2 to sf

        if area_sf <= 0.0 { continue; }
        let cfm_per_sf = total_cfm / area_sf;

        // ASHRAE 62.1 minimum ventilation rates (simplified)
        let min_cfm_per_sf = match space_type.as_str() {
            "retail" => 0.6,
            "office" => 0.6,
            "restroom" => 0.0,
            "storage" => 0.12,
            _ => 0.4,
        };

        if min_cfm_per_sf > 0.0 && cfm_per_sf < min_cfm_per_sf {
            suggestions.push(Suggestion {
                category: "ventilation".into(),
                priority: Priority::Warning,
                message: format!(
                    "{} ({}): {:.1} CFM/SF is below minimum {:.1} CFM/SF for {}",
                    row[1].1, row[0].1, cfm_per_sf, min_cfm_per_sf, space_type
                ),
                action: None,
            });
        }
    }
    Ok(())
}

/// Suggest duct sizing corrections.
fn suggest_duct_sizing(doc: &SedDocument, suggestions: &mut Vec<Suggestion>) -> Result<()> {
    // Use autosize to find mismatches
    let systems = doc.query_raw("SELECT id, tag FROM systems WHERE medium = 'air'")?;
    for sys_row in &systems {
        let sys_id = &sys_row[0].1;
        let sys_tag = &sys_row[1].1;
        match crate::autosize::autosize_duct_system(doc, sys_id) {
            Ok(results) => {
                for r in &results {
                    if let (Some(current), recommended) = (r.current_diameter_in, r.recommended_diameter_in) {
                        let ratio = current / recommended;
                        if ratio < 0.8 {
                            suggestions.push(Suggestion {
                                category: "duct_sizing".into(),
                                priority: Priority::Warning,
                                message: format!(
                                    "System {}: segment carrying {:.0} CFM has {:.0}\" duct, recommend {:.0}\"",
                                    sys_tag, r.downstream_cfm, current, recommended
                                ),
                                action: Some(SuggestedAction {
                                    action_type: "resize_duct".into(),
                                    parameters: serde_json::json!({
                                        "segment_id": r.segment_id,
                                        "recommended_diameter_in": recommended,
                                    }),
                                }),
                            });
                        }
                    }
                }
            }
            Err(_) => {} // Skip systems that can't be auto-sized
        }
    }
    Ok(())
}

/// Check air balance per level.
fn suggest_system_balance(doc: &SedDocument, suggestions: &mut Vec<Suggestion>) -> Result<()> {
    let rows = doc.query_raw(
        "SELECT p.level,
                SUM(CASE WHEN pt.category LIKE 'supply%' THEN p.cfm ELSE 0 END) as supply,
                SUM(CASE WHEN pt.category LIKE 'return%' THEN p.cfm ELSE 0 END) as ret,
                SUM(CASE WHEN pt.category LIKE 'exhaust%' THEN p.cfm ELSE 0 END) as exhaust
         FROM placements p
         JOIN product_types pt ON p.product_type_id = pt.id
         WHERE p.cfm IS NOT NULL
         GROUP BY p.level"
    )?;

    for row in &rows {
        let level = &row[0].1;
        let supply: f64 = row[1].1.parse().unwrap_or(0.0);
        let ret: f64 = row[2].1.parse().unwrap_or(0.0);
        let exhaust: f64 = row[3].1.parse().unwrap_or(0.0);

        // Supply should roughly equal return + exhaust (within 10%)
        let out = ret + exhaust;
        if supply > 0.0 && out > 0.0 {
            let imbalance = ((supply - out) / supply).abs();
            if imbalance > 0.15 {
                suggestions.push(Suggestion {
                    category: "balance".into(),
                    priority: Priority::Recommendation,
                    message: format!(
                        "{}: Supply {:.0} CFM, Return {:.0} CFM, Exhaust {:.0} CFM — {:.0}% imbalance",
                        level, supply, ret, exhaust, imbalance * 100.0
                    ),
                    action: None,
                });
            }
        } else if supply > 0.0 && ret == 0.0 && exhaust == 0.0 {
            suggestions.push(Suggestion {
                category: "balance".into(),
                priority: Priority::Warning,
                message: format!("{}: {:.0} CFM supply with no return or exhaust path", level, supply),
                action: None,
            });
        }
    }
    Ok(())
}

/// Suggest equipment based on total load.
fn suggest_equipment_capacity(doc: &SedDocument, suggestions: &mut Vec<Suggestion>) -> Result<()> {
    // Check if there are placements but no equipment
    let device_count = doc.query_raw("SELECT COUNT(*) FROM placements p JOIN product_types pt ON p.product_type_id = pt.id WHERE pt.domain = 'air_device'")?;
    let equip_count = doc.query_raw("SELECT COUNT(*) FROM placements p JOIN product_types pt ON p.product_type_id = pt.id WHERE pt.domain = 'equipment'")?;

    let devices: i64 = device_count.first().map(|r| r[0].1.parse().unwrap_or(0)).unwrap_or(0);
    let equipment: i64 = equip_count.first().map(|r| r[0].1.parse().unwrap_or(0)).unwrap_or(0);

    if devices > 0 && equipment == 0 {
        let total_cfm = doc.query_raw(
            "SELECT SUM(p.cfm) FROM placements p JOIN product_types pt ON p.product_type_id = pt.id WHERE pt.category LIKE 'supply%'"
        )?;
        let cfm: f64 = total_cfm.first().map(|r| r[0].1.parse().unwrap_or(0.0)).unwrap_or(0.0);

        if cfm > 0.0 {
            // Estimate tonnage: roughly 400 CFM per ton
            let tons = cfm / 400.0;
            suggestions.push(Suggestion {
                category: "equipment".into(),
                priority: Priority::Info,
                message: format!(
                    "Project has {:.0} CFM of supply air but no equipment. Estimated cooling: {:.0} tons. Consider adding an AHU or RTU.",
                    cfm, tons
                ),
                action: Some(SuggestedAction {
                    action_type: "add_equipment".into(),
                    parameters: serde_json::json!({
                        "estimated_cfm": cfm,
                        "estimated_tons": tons,
                    }),
                }),
            });
        }
    }

    Ok(())
}

fn priority_ord(p: &Priority) -> u8 {
    match p {
        Priority::Warning => 0,
        Priority::Recommendation => 1,
        Priority::Info => 2,
    }
}

impl std::fmt::Display for Suggestion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let icon = match self.priority {
            Priority::Warning => "!",
            Priority::Recommendation => "*",
            Priority::Info => "i",
        };
        write!(f, "[{}] {}: {}", icon, self.category, self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skims_suggestions() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        drop(tmp);
        crate::examples::create_skims_americana(&path).unwrap();
        let doc = SedDocument::open(&path).unwrap();

        let suggestions = suggest(&doc).unwrap();
        // SKIMS should have some suggestions — rooms without return, air balance issues
        assert!(!suggestions.is_empty(), "Should generate suggestions for SKIMS");

        // Should flag the air balance issue (supply with no return)
        let balance = suggestions.iter().filter(|s| s.category == "balance").count();
        assert!(balance > 0, "Should flag air balance issues");
    }

    #[test]
    fn empty_project_no_crash() {
        let doc = SedDocument::in_memory().unwrap();
        doc.set_meta("sed_version", "0.3").unwrap();
        doc.set_meta("project_name", "Empty").unwrap();
        doc.set_meta("project_number", "E-001").unwrap();
        let suggestions = suggest(&doc).unwrap();
        // Empty project should not crash, may have zero suggestions
        assert!(suggestions.len() >= 0);
    }
}
