//! Generates formatted reports from SED documents.

use anyhow::Result;
use crate::document::SedDocument;

/// Generate a complete project summary as markdown text.
pub fn project_summary(doc: &SedDocument) -> Result<String> {
    let info = doc.info()?;
    let mut out = String::new();

    out += &format!("# {} ({})\n\n", info.project_name, info.project_number);

    // Levels and spaces
    let levels = doc.query_raw("SELECT level, COUNT(*) as rooms FROM spaces WHERE scope = 'in_contract' GROUP BY level ORDER BY level")?;
    out += "## Spaces\n\n";
    out += "| Level | Rooms |\n|---|---|\n";
    for row in &levels {
        out += &format!("| {} | {} |\n", row[0].1, row[1].1);
    }
    out += "\n";

    // Equipment
    let equip = doc.query_raw(
        "SELECT COALESCE(p.instance_tag, pt.tag) as tag, pt.category, pt.manufacturer, pt.model, p.cfm, p.status, p.level
         FROM placements p JOIN product_types pt ON p.product_type_id = pt.id
         WHERE pt.domain = 'equipment' ORDER BY tag"
    )?;
    if !equip.is_empty() {
        out += "## Equipment\n\n";
        out += "| Tag | Category | Manufacturer | Model | CFM | Status | Level |\n|---|---|---|---|---|---|---|\n";
        for row in &equip {
            let vals: Vec<&str> = row.iter().map(|(_, v)| if v == "NULL" { "-" } else { v.as_str() }).collect();
            out += &format!("| {} |\n", vals.join(" | "));
        }
        out += "\n";
    }

    // Air device summary
    let devices = doc.query_raw(
        "SELECT pt.tag, pt.category, pt.manufacturer, pt.model, COUNT(*) as qty, SUM(p.cfm) as total_cfm
         FROM placements p JOIN product_types pt ON p.product_type_id = pt.id
         WHERE pt.domain = 'air_device' GROUP BY pt.id ORDER BY qty DESC"
    )?;
    if !devices.is_empty() {
        out += "## Air Devices\n\n";
        out += "| Type | Category | Manufacturer | Model | Qty | Total CFM |\n|---|---|---|---|---|---|\n";
        for row in &devices {
            let vals: Vec<&str> = row.iter().map(|(_, v)| if v == "NULL" { "-" } else { v.as_str() }).collect();
            out += &format!("| {} |\n", vals.join(" | "));
        }
        out += "\n";
    }

    // CFM by room
    let cfm = doc.query_raw(
        "SELECT s.level, s.tag, s.name, SUM(p.cfm) as cfm, COUNT(*) as devices
         FROM placements p JOIN product_types pt ON p.product_type_id = pt.id
         JOIN spaces s ON p.space_id = s.id
         WHERE pt.category LIKE 'supply%'
         GROUP BY s.id ORDER BY s.level, s.tag"
    )?;
    if !cfm.is_empty() {
        out += "## Supply CFM by Room\n\n";
        out += "| Level | Room | Name | CFM | Devices |\n|---|---|---|---|---|\n";
        for row in &cfm {
            out += &format!("| {} | {} | {} | {} | {} |\n", row[0].1, row[1].1, row[2].1, row[3].1, row[4].1);
        }
        out += "\n";
    }

    // Systems
    let systems = doc.query_raw("SELECT tag, name, system_type, medium FROM systems ORDER BY tag")?;
    if !systems.is_empty() {
        out += "## Systems\n\n";
        out += "| Tag | Name | Type | Medium |\n|---|---|---|---|\n";
        for row in &systems {
            out += &format!("| {} | {} | {} | {} |\n", row[0].1, row[1].1, row[2].1, row[3].1);
        }
        out += "\n";
    }

    // Submittals
    let subs = doc.query_raw("SELECT description, status, date_submitted, submitted_by, company FROM submittals ORDER BY date_submitted")?;
    if !subs.is_empty() {
        out += "## Submittals\n\n";
        out += "| Description | Status | Date | Submitted By | Company |\n|---|---|---|---|---|\n";
        for row in &subs {
            let vals: Vec<&str> = row.iter().map(|(_, v)| if v == "NULL" { "-" } else { v.as_str() }).collect();
            out += &format!("| {} |\n", vals.join(" | "));
        }
        out += "\n";
    }

    out += &format!("---\n*Generated from SED v{} — Structured Engineering Document*\n", info.sed_version);
    Ok(out)
}
