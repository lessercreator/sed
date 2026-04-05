//! Natural language query engine for SED documents.
//!
//! Maps English questions to SQL queries against the known schema.
//! No LLM required — uses pattern matching against the fixed SED schema.
//!
//! Examples:
//!   "total supply cfm on level 1" → SUM(cfm) WHERE category LIKE 'supply%' AND level = 'Level 1'
//!   "how many vavs" → COUNT(*) WHERE category = 'vav_box'
//!   "what equipment is on the roof" → equipment WHERE level = 'Roof'
//!   "show submittals" → SELECT * FROM submittals
//!   "rooms with no supply" → spaces NOT IN supply placements

use anyhow::Result;
use crate::document::SedDocument;

/// Result of a natural language query
pub struct NlqResult {
    pub sql: String,
    pub interpretation: String,
    pub rows: Vec<Vec<(String, String)>>,
}

impl std::fmt::Display for NlqResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Interpreted as: {}", self.interpretation)?;
        writeln!(f, "SQL: {}", self.sql)?;
        writeln!(f)?;

        if self.rows.is_empty() {
            writeln!(f, "(no results)")?;
            return Ok(());
        }

        // Print as table
        let headers: Vec<&str> = self.rows[0].iter().map(|(k, _)| k.as_str()).collect();
        let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
        for row in &self.rows {
            for (i, (_, val)) in row.iter().enumerate() {
                widths[i] = widths[i].max(val.len().min(40));
            }
        }

        for (i, h) in headers.iter().enumerate() {
            write!(f, "{:width$}  ", h, width = widths[i])?;
        }
        writeln!(f)?;
        for w in &widths {
            write!(f, "{:-<width$}  ", "", width = *w)?;
        }
        writeln!(f)?;
        for row in &self.rows {
            for (i, (_, val)) in row.iter().enumerate() {
                let display = if val.len() > 40 { &val[..37] } else { val };
                write!(f, "{:width$}  ", display, width = widths[i])?;
            }
            writeln!(f)?;
        }
        writeln!(f, "\n({} rows)", self.rows.len())?;
        Ok(())
    }
}

/// Process a natural language question against a SED document.
pub fn ask(doc: &SedDocument, question: &str) -> Result<NlqResult> {
    let q = question.to_lowercase().trim().to_string();
    let q = q.trim_end_matches('?').trim().to_string();

    // Try each pattern matcher in priority order
    if let Some(r) = try_total_cfm(&q, doc)? { return Ok(r); }
    if let Some(r) = try_count_devices(&q, doc)? { return Ok(r); }
    if let Some(r) = try_equipment_query(&q, doc)? { return Ok(r); }
    if let Some(r) = try_submittal_query(&q, doc)? { return Ok(r); }
    if let Some(r) = try_room_query(&q, doc)? { return Ok(r); }
    if let Some(r) = try_rooms_missing(&q, doc)? { return Ok(r); }
    if let Some(r) = try_system_query(&q, doc)? { return Ok(r); }
    if let Some(r) = try_device_list(&q, doc)? { return Ok(r); }
    if let Some(r) = try_level_summary(&q, doc)? { return Ok(r); }
    if let Some(r) = try_notes(&q, doc)? { return Ok(r); }

    // Fallback: try interpreting the whole thing as a category or tag search
    if let Some(r) = try_generic_search(&q, doc)? { return Ok(r); }

    // Last resort
    Ok(NlqResult {
        sql: String::new(),
        interpretation: format!("Could not understand: \"{}\"", question),
        rows: vec![],
    })
}

fn try_total_cfm(q: &str, doc: &SedDocument) -> Result<Option<NlqResult>> {
    if !q.contains("cfm") && !q.contains("airflow") { return Ok(None); }
    if !q.contains("total") && !q.contains("sum") && !q.contains("how much") { return Ok(None); }

    let level = extract_level(q);
    let category = if q.contains("supply") { "supply%" }
        else if q.contains("return") { "return%" }
        else if q.contains("exhaust") { "exhaust%" }
        else { "supply%" };

    let mut sql = format!(
        "SELECT s.level, SUM(p.cfm) as total_cfm, COUNT(*) as device_count
         FROM placements p
         JOIN product_types pt ON p.product_type_id = pt.id
         LEFT JOIN spaces s ON p.space_id = s.id
         WHERE pt.category LIKE '{}'", category
    );
    if let Some(ref lvl) = level {
        sql += &format!(" AND p.level = '{}'", lvl);
    }
    sql += " GROUP BY s.level ORDER BY s.level";

    let rows = doc.query_raw(&sql)?;
    let cat_name = category.trim_end_matches('%');
    let interp = match &level {
        Some(lvl) => format!("Total {} CFM on {}", cat_name, lvl),
        None => format!("Total {} CFM by level", cat_name),
    };

    Ok(Some(NlqResult { sql, interpretation: interp, rows }))
}

fn try_count_devices(q: &str, doc: &SedDocument) -> Result<Option<NlqResult>> {
    if !q.contains("how many") && !q.contains("count") && !q.contains("number of") { return Ok(None); }

    let category = extract_category(q);
    let level = extract_level(q);

    let mut sql = String::from(
        "SELECT pt.tag, pt.category, COUNT(*) as qty, SUM(p.cfm) as total_cfm
         FROM placements p
         JOIN product_types pt ON p.product_type_id = pt.id
         WHERE 1=1"
    );
    let mut interp = String::from("Count of ");

    if let Some(ref cat) = category {
        sql += &format!(" AND pt.category LIKE '%{}%'", cat);
        interp += cat;
    } else {
        interp += "all devices";
    }

    if let Some(ref lvl) = level {
        sql += &format!(" AND p.level = '{}'", lvl);
        interp += &format!(" on {}", lvl);
    }

    sql += " GROUP BY pt.id ORDER BY qty DESC";

    let rows = doc.query_raw(&sql)?;
    Ok(Some(NlqResult { sql, interpretation: interp, rows }))
}

fn try_equipment_query(q: &str, doc: &SedDocument) -> Result<Option<NlqResult>> {
    if !q.contains("equipment") && !q.contains("ahu") && !q.contains("rtu")
        && !q.contains("chiller") && !q.contains("boiler") && !q.contains("pump")
        && !q.contains("fan") && !q.contains("cooling tower") { return Ok(None); }

    if q.contains("how many") || q.contains("count") { return Ok(None); } // handled above

    let level = extract_level(q);
    let mut sql = String::from(
        "SELECT COALESCE(p.instance_tag, pt.tag) as tag, pt.category, pt.manufacturer, pt.model,
                p.status, p.level, p.cfm, p.notes
         FROM placements p
         JOIN product_types pt ON p.product_type_id = pt.id
         WHERE pt.domain = 'equipment'"
    );

    if let Some(ref lvl) = level {
        sql += &format!(" AND p.level = '{}'", lvl);
    }

    // Filter by specific equipment type
    if q.contains("ahu") { sql += " AND pt.category = 'ahu'"; }
    else if q.contains("rtu") { sql += " AND pt.category = 'rtu'"; }
    else if q.contains("chiller") { sql += " AND pt.category = 'chiller'"; }
    else if q.contains("boiler") { sql += " AND pt.category = 'boiler'"; }
    else if q.contains("pump") { sql += " AND pt.category = 'pump'"; }
    else if q.contains("fan") { sql += " AND pt.category LIKE '%fan%'"; }
    else if q.contains("cooling tower") { sql += " AND pt.category = 'cooling_tower'"; }

    sql += " ORDER BY tag";

    let rows = doc.query_raw(&sql)?;
    Ok(Some(NlqResult { sql, interpretation: format!("Equipment list"), rows }))
}

fn try_submittal_query(q: &str, doc: &SedDocument) -> Result<Option<NlqResult>> {
    if !q.contains("submittal") { return Ok(None); }

    let mut sql = String::from(
        "SELECT description, status, date_submitted, submitted_by, company FROM submittals"
    );

    if q.contains("pending") || q.contains("approval") {
        sql += " WHERE status = 'for_approval'";
    } else if q.contains("approved") {
        sql += " WHERE status = 'approved'";
    } else if q.contains("rejected") {
        sql += " WHERE status IN ('rejected', 'revise_resubmit')";
    }

    sql += " ORDER BY date_submitted";

    let rows = doc.query_raw(&sql)?;
    Ok(Some(NlqResult { sql, interpretation: "Submittal status".into(), rows }))
}

fn try_room_query(q: &str, doc: &SedDocument) -> Result<Option<NlqResult>> {
    if !q.contains("room") && !q.contains("space") { return Ok(None); }
    if q.contains("without") || q.contains("missing") || q.contains("no supply") || q.contains("no return") { return Ok(None); }

    let level = extract_level(q);
    let mut sql = String::from(
        "SELECT s.tag, s.name, s.level, s.space_type, s.scope FROM spaces s WHERE 1=1"
    );
    if let Some(ref lvl) = level {
        sql += &format!(" AND s.level = '{}'", lvl);
    }
    sql += " ORDER BY s.level, s.tag";

    let rows = doc.query_raw(&sql)?;
    Ok(Some(NlqResult { sql, interpretation: "Room list".into(), rows }))
}

fn try_rooms_missing(q: &str, doc: &SedDocument) -> Result<Option<NlqResult>> {
    let missing_supply = q.contains("no supply") || q.contains("without supply") || q.contains("missing supply");
    let missing_return = q.contains("no return") || q.contains("without return") || q.contains("missing return");
    let missing_exhaust = q.contains("no exhaust") || q.contains("without exhaust") || q.contains("missing exhaust");

    if !missing_supply && !missing_return && !missing_exhaust { return Ok(None); }

    let category = if missing_supply { "supply%" }
        else if missing_return { "return%" }
        else { "exhaust%" };
    let cat_name = if missing_supply { "supply" } else if missing_return { "return" } else { "exhaust" };

    let sql = format!(
        "SELECT s.tag, s.name, s.level FROM spaces s
         WHERE s.scope = 'in_contract'
         AND s.id NOT IN (
             SELECT p.space_id FROM placements p
             JOIN product_types pt ON p.product_type_id = pt.id
             WHERE pt.category LIKE '{}' AND p.space_id IS NOT NULL
         )
         ORDER BY s.level, s.tag", category
    );

    let rows = doc.query_raw(&sql)?;
    Ok(Some(NlqResult { sql, interpretation: format!("Rooms without {} devices", cat_name), rows }))
}

fn try_system_query(q: &str, doc: &SedDocument) -> Result<Option<NlqResult>> {
    if !q.contains("system") { return Ok(None); }

    let sql = "SELECT tag, name, system_type, medium,
               CASE WHEN paired_system_id IS NOT NULL THEN 'paired' ELSE 'unpaired' END as pairing
               FROM systems ORDER BY tag";

    let rows = doc.query_raw(sql)?;
    Ok(Some(NlqResult { sql: sql.into(), interpretation: "System list".into(), rows }))
}

fn try_device_list(q: &str, doc: &SedDocument) -> Result<Option<NlqResult>> {
    if !q.contains("list") && !q.contains("show") && !q.contains("all") { return Ok(None); }
    if q.contains("equipment") || q.contains("submittal") || q.contains("room") || q.contains("system") || q.contains("note") {
        return Ok(None);
    }

    let level = extract_level(q);
    let mut sql = String::from(
        "SELECT pt.tag, pt.category, COUNT(*) as qty, SUM(p.cfm) as total_cfm, pt.manufacturer, pt.model
         FROM placements p
         JOIN product_types pt ON p.product_type_id = pt.id
         WHERE p.status = 'new'"
    );
    if let Some(ref lvl) = level {
        sql += &format!(" AND p.level = '{}'", lvl);
    }
    sql += " GROUP BY pt.id ORDER BY qty DESC";

    let rows = doc.query_raw(&sql)?;
    Ok(Some(NlqResult { sql, interpretation: "Device summary".into(), rows }))
}

fn try_level_summary(q: &str, doc: &SedDocument) -> Result<Option<NlqResult>> {
    if !q.contains("summary") && !q.contains("overview") { return Ok(None); }

    let sql = "SELECT p.level,
               COUNT(CASE WHEN pt.domain = 'equipment' THEN 1 END) as equipment,
               COUNT(CASE WHEN pt.domain = 'air_device' THEN 1 END) as devices,
               COUNT(CASE WHEN pt.domain = 'accessory' THEN 1 END) as accessories,
               ROUND(SUM(CASE WHEN pt.category LIKE 'supply%' THEN p.cfm ELSE 0 END)) as supply_cfm
               FROM placements p
               JOIN product_types pt ON p.product_type_id = pt.id
               GROUP BY p.level ORDER BY p.level";

    let rows = doc.query_raw(sql)?;
    Ok(Some(NlqResult { sql: sql.into(), interpretation: "Level-by-level summary".into(), rows }))
}

fn try_notes(q: &str, doc: &SedDocument) -> Result<Option<NlqResult>> {
    if !q.contains("note") { return Ok(None); }

    let sql = "SELECT key, text FROM keyed_notes ORDER BY key";
    let rows = doc.query_raw(sql)?;
    Ok(Some(NlqResult { sql: sql.into(), interpretation: "Keyed notes".into(), rows }))
}

fn try_generic_search(q: &str, doc: &SedDocument) -> Result<Option<NlqResult>> {
    // Try to find a product type tag or category that matches any word in the query
    let words: Vec<&str> = q.split_whitespace().collect();
    for word in &words {
        let upper = word.to_uppercase();
        let sql = format!(
            "SELECT pt.tag, pt.category, pt.manufacturer, pt.model, COUNT(*) as qty
             FROM placements p JOIN product_types pt ON p.product_type_id = pt.id
             WHERE UPPER(pt.tag) LIKE '%{}%' OR pt.category LIKE '%{}%'
             GROUP BY pt.id ORDER BY qty DESC",
            upper, word
        );
        let rows = doc.query_raw(&sql)?;
        if !rows.is_empty() {
            return Ok(Some(NlqResult {
                sql,
                interpretation: format!("Search for '{}'", word),
                rows,
            }));
        }
    }
    Ok(None)
}

// ============================================================================
// HELPERS
// ============================================================================

fn extract_level(q: &str) -> Option<String> {
    // Match "level 1", "floor 2", "level 10", "basement", "roof"
    if q.contains("basement") { return Some("Basement".into()); }
    if q.contains("roof") { return Some("Roof".into()); }

    for pattern in ["level ", "floor "] {
        if let Some(pos) = q.find(pattern) {
            let rest = &q[pos + pattern.len()..];
            let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !num.is_empty() {
                return Some(format!("Level {}", num));
            }
        }
    }
    None
}

fn extract_category(q: &str) -> Option<String> {
    if q.contains("diffuser") { return Some("diffuser".into()); }
    if q.contains("register") { return Some("register".into()); }
    if q.contains("grille") { return Some("grille".into()); }
    if q.contains("vav") { return Some("vav".into()); }
    if q.contains("damper") { return Some("damper".into()); }
    if q.contains("fan") { return Some("fan".into()); }
    if q.contains("ahu") { return Some("ahu".into()); }
    if q.contains("rtu") { return Some("rtu".into()); }
    if q.contains("supply") { return Some("supply".into()); }
    if q.contains("return") { return Some("return".into()); }
    if q.contains("exhaust") { return Some("exhaust".into()); }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_skims() -> SedDocument {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        drop(tmp);
        crate::examples::create_skims_americana(&path).unwrap();
        SedDocument::open(&path).unwrap()
    }

    #[test]
    fn ask_total_cfm() {
        let doc = load_skims();
        let r = ask(&doc, "total supply CFM on level 1").unwrap();
        assert!(!r.rows.is_empty());
        assert!(r.interpretation.contains("supply"));
    }

    #[test]
    fn ask_how_many_diffusers() {
        let doc = load_skims();
        let r = ask(&doc, "how many diffusers").unwrap();
        assert!(!r.rows.is_empty());
    }

    #[test]
    fn ask_equipment() {
        let doc = load_skims();
        let r = ask(&doc, "what equipment is on the roof").unwrap();
        assert!(!r.rows.is_empty());
    }

    #[test]
    fn ask_submittals() {
        let doc = load_skims();
        let r = ask(&doc, "show pending submittals").unwrap();
        assert!(!r.rows.is_empty());
    }

    #[test]
    fn ask_rooms_without_supply() {
        let doc = load_skims();
        let r = ask(&doc, "rooms with no supply").unwrap();
        assert!(!r.rows.is_empty()); // Mop closet, NIC spaces, etc.
    }

    #[test]
    fn ask_summary() {
        let doc = load_skims();
        let r = ask(&doc, "project summary").unwrap();
        assert!(!r.rows.is_empty());
    }

    #[test]
    fn ask_notes() {
        let doc = load_skims();
        let r = ask(&doc, "show notes").unwrap();
        assert_eq!(r.rows.len(), 11);
    }

    #[test]
    fn ask_nonsense_doesnt_crash() {
        let doc = load_skims();
        let r = ask(&doc, "what is the meaning of life").unwrap();
        assert!(r.rows.is_empty() || !r.interpretation.is_empty());
    }
}
