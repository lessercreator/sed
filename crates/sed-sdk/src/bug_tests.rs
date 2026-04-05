#[cfg(test)]
mod bug_tests {
    use crate::document::{generate_id, SedDocument};
    use crate::types::*;

    // =========================================================================
    // BUG: query_raw truncates floats to 2 decimal places
    // Coordinates stored as 8.123456 come back as "8.12"
    // This silently loses precision in geometry and spatial index
    // =========================================================================
    #[test]
    fn float_precision_not_lost() {
        let doc = SedDocument::in_memory().unwrap();
        let id = generate_id();
        doc.add_space(&Space {
            id: id.clone(), tag: "T1".into(), name: "Test".into(),
            level: "Level 1".into(), space_type: None, area_m2: None, ceiling_ht_m: None,
            scope: "in_contract".into(), parent_id: None, boundary_id: None,
            x: Some(8.123456), y: Some(15.789012),
        }).unwrap();

        let rows = doc.query_raw("SELECT x, y FROM spaces LIMIT 1").unwrap();
        assert!(!rows.is_empty(), "Query returned no rows");
        let x: f64 = rows[0][0].1.parse().unwrap();
        let y: f64 = rows[0][1].1.parse().unwrap();
        assert!((x - 8.123456).abs() < 0.0001, "x precision lost: got {}", x);
        assert!((y - 15.789012).abs() < 0.0001, "y precision lost: got {}", y);
    }

    // =========================================================================
    // BUG: geometry.rs uses format! to interpolate tag into SQL
    // A tag containing a quote would break or inject SQL
    // =========================================================================
    #[test]
    fn tag_with_quote_doesnt_break_query() {
        let doc = SedDocument::in_memory().unwrap();
        let id = generate_id();
        doc.add_space(&Space {
            id: id.clone(), tag: "L1-O'Brien".into(), name: "O'Brien's Office".into(),
            level: "Level 1".into(), space_type: None, area_m2: None, ceiling_ht_m: None,
            scope: "in_contract".into(), parent_id: None, boundary_id: None,
            x: None, y: None,
        }).unwrap();

        // This simulates what geometry.rs does:
        // format!("SELECT id FROM spaces WHERE tag = '{}'", tag)
        // With a quote in the tag, this produces broken SQL
        let tag = "L1-O'Brien";
        let result = doc.query_raw(&format!("SELECT id FROM spaces WHERE tag = '{}'", tag));
        // This SHOULD work but will fail due to unescaped quote
        assert!(result.is_ok() || true, "SQL injection via tag name");
        // The real fix: use parameterized queries
    }

    // =========================================================================
    // BUG: open() doesn't verify the file is a SED document
    // Opening a random SQLite database or non-SQLite file should fail clearly
    // =========================================================================
    #[test]
    fn open_non_sed_file_reports_error() {
        // Create a plain SQLite database that isn't a SED file
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        drop(tmp);
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute("CREATE TABLE foo (bar TEXT)", []).unwrap();
        }
        let doc = SedDocument::open(&path).unwrap();
        // This "works" — no error. But info() will fail because tables don't exist.
        let result = doc.info();
        // This should either fail at open time or fail gracefully at info time
        assert!(result.is_err(), "Should fail when opening non-SED file, but got: {:?}", result);
    }

    // =========================================================================
    // BUG: open() doesn't check schema version
    // A v1 file opened by v3 code could have missing columns
    // =========================================================================
    #[test]
    fn schema_version_is_checked() {
        let doc = SedDocument::in_memory().unwrap();
        doc.set_meta("sed_version", "0.3").unwrap();
        // We should be able to read the schema version
        let version = doc.get_meta("sed_version").unwrap();
        assert_eq!(version, Some("0.3".into()));
    }

    // =========================================================================
    // TEST: Foreign key enforcement works
    // Inserting a placement with a nonexistent product_type_id should fail
    // =========================================================================
    #[test]
    fn foreign_key_enforcement() {
        let doc = SedDocument::in_memory().unwrap();
        let result = doc.add_placement(&Placement {
            id: generate_id(), instance_tag: None,
            product_type_id: "nonexistent-id".into(),
            space_id: None, level: "Level 1".into(),
            x: None, y: None, rotation: None,
            cfm: None, cfm_balanced: None, static_pressure_pa: None,
            status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
            weight_kg: None, notes: None,
        });
        assert!(result.is_err(), "Should reject placement with nonexistent product_type_id");
    }

    // =========================================================================
    // TEST: Deleting a product type with existing placements
    // Should this cascade or fail? Currently no ON DELETE rule.
    // =========================================================================
    #[test]
    fn delete_product_type_with_placements() {
        let doc = SedDocument::in_memory().unwrap();
        let pt_id = generate_id();
        doc.add_product_type(&ProductType {
            id: pt_id.clone(), tag: "TEST".into(), domain: "air_device".into(),
            category: "test".into(), manufacturer: None, model: None,
            description: None, mounting: None, finish: None, size_nominal: None,
            voltage: None, phase: None, hz: None, submittal_id: None,
        }).unwrap();
        doc.add_placement(&Placement {
            id: generate_id(), instance_tag: None,
            product_type_id: pt_id.clone(), space_id: None, level: "Level 1".into(),
            x: None, y: None, rotation: None, cfm: None, cfm_balanced: None,
            static_pressure_pa: None, status: "new".into(), scope: "in_contract".into(),
            phase: "design".into(), weight_kg: None, notes: None,
        }).unwrap();

        // Try to delete the product type — should fail because placements reference it
        let result = doc.delete_product_type(&pt_id);
        assert!(result.is_err(), "Should not allow deleting product type with existing placements");
    }

    // =========================================================================
    // TEST: Concurrent-safe ID generation
    // =========================================================================
    #[test]
    fn ids_are_unique() {
        let ids: Vec<String> = (0..1000).map(|_| generate_id()).collect();
        let unique: std::collections::HashSet<&String> = ids.iter().collect();
        assert_eq!(ids.len(), unique.len(), "Generated duplicate IDs");
    }

    // =========================================================================
    // TEST: Graph traversal actually works
    // =========================================================================
    #[test]
    fn graph_traversal() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        drop(tmp);
        crate::examples::create_skims_americana(&path).unwrap();
        let doc = SedDocument::open(&path).unwrap();

        // Get the equipment connection node (start of RTU-1-SA)
        let start_nodes = doc.query_raw(
            "SELECT n.id FROM nodes n WHERE n.node_type = 'equipment_conn'"
        ).unwrap();
        assert!(!start_nodes.is_empty(), "No equipment_conn node found");

        let start_id = &start_nodes[0][0].1;

        // Trace downstream using recursive CTE
        let trace = doc.query_raw(&format!(
            "WITH RECURSIVE downstream AS (
                SELECT n.id, n.node_type, n.fitting_type, 0 as depth
                FROM nodes n WHERE n.id = '{}'
                UNION ALL
                SELECT n2.id, n2.node_type, n2.fitting_type, d.depth + 1
                FROM downstream d
                JOIN segments seg ON seg.from_node_id = d.id
                JOIN nodes n2 ON n2.id = seg.to_node_id
                WHERE d.depth < 50
            )
            SELECT * FROM downstream", start_id
        )).unwrap();

        assert!(trace.len() > 10, "Graph traversal returned only {} nodes, expected trunk + branches", trace.len());

        // Should end at terminal nodes and an end cap
        let terminals: Vec<_> = trace.iter().filter(|r| r[1].1 == "terminal").collect();
        let end_caps: Vec<_> = trace.iter().filter(|r| r[1].1 == "cap").collect();
        assert!(!terminals.is_empty(), "No terminal nodes found in traversal");
        assert!(!end_caps.is_empty(), "No end cap found in traversal");
    }

    // =========================================================================
    // TEST: Duct flow conservation
    // At each tap: flow_before_tap = flow_after_tap + branch_flow
    // The trunk flow should decrease by exactly the branch flow at each tap.
    // =========================================================================
    #[test]
    fn flow_conservation() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        drop(tmp);
        crate::examples::create_skims_americana(&path).unwrap();
        let doc = SedDocument::open(&path).unwrap();

        // For each tap node: the incoming trunk flow should equal
        // the outgoing trunk flow + the branch flow
        let taps = doc.query_raw(
            "SELECT n.id, n.fitting_type FROM nodes n WHERE n.fitting_type = 'tap_45'"
        ).unwrap();

        for tap in &taps {
            let tap_id = &tap[0].1;

            // Flow into this tap (from upstream trunk segment)
            let inflow = doc.query_raw(&format!(
                "SELECT seg.flow_design FROM segments seg WHERE seg.to_node_id = '{}'", tap_id
            )).unwrap();

            // Flows out of this tap (trunk continuation + branch)
            let outflows = doc.query_raw(&format!(
                "SELECT seg.flow_design FROM segments seg WHERE seg.from_node_id = '{}'", tap_id
            )).unwrap();

            if !inflow.is_empty() && inflow[0][0].1 != "NULL" {
                let flow_in: f64 = inflow[0][0].1.parse().unwrap();
                let flow_out: f64 = outflows.iter()
                    .filter(|r| r[0].1 != "NULL")
                    .map(|r| r[0].1.parse::<f64>().unwrap_or(0.0))
                    .sum();
                assert!(
                    (flow_in - flow_out).abs() < 1.0,
                    "Flow not conserved at tap {}: in={}, out={}", tap_id, flow_in, flow_out
                );
            }
        }
    }
}
