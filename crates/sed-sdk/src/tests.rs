#[cfg(test)]
mod tests {
    use crate::document::{generate_id, SedDocument};
    use crate::types::*;
    use tempfile::NamedTempFile;

    // =========================================================================
    // SCHEMA
    // =========================================================================

    #[test]
    fn create_in_memory() {
        let doc = SedDocument::in_memory().unwrap();
        let info = doc.info().unwrap();
        assert_eq!(info.spaces, 0);
        assert_eq!(info.placements, 0);
    }

    #[test]
    fn schema_version() {
        let doc = SedDocument::in_memory().unwrap();
        doc.set_meta("sed_version", "0.3").unwrap();
        assert_eq!(doc.get_meta("sed_version").unwrap(), Some("0.3".into()));
    }

    // =========================================================================
    // META
    // =========================================================================

    #[test]
    fn meta_round_trip() {
        let doc = SedDocument::in_memory().unwrap();
        doc.set_meta("project_name", "Test Project").unwrap();
        doc.set_meta("project_number", "TP-001").unwrap();
        assert_eq!(doc.get_meta("project_name").unwrap(), Some("Test Project".into()));
        assert_eq!(doc.get_meta("project_number").unwrap(), Some("TP-001".into()));
        assert_eq!(doc.get_meta("nonexistent").unwrap(), None);
    }

    #[test]
    fn meta_overwrite() {
        let doc = SedDocument::in_memory().unwrap();
        doc.set_meta("key", "value1").unwrap();
        doc.set_meta("key", "value2").unwrap();
        assert_eq!(doc.get_meta("key").unwrap(), Some("value2".into()));
    }

    // =========================================================================
    // DIRECTORY
    // =========================================================================

    #[test]
    fn directory_round_trip() {
        let doc = SedDocument::in_memory().unwrap();
        let id = generate_id();
        doc.add_directory_entry(&DirectoryEntry {
            id: id.clone(),
            role: "architect".into(),
            company: "RDC".into(),
            contact: Some("James Botha".into()),
            email: Some("james@rdc.com".into()),
            phone: None,
            address: None,
        }).unwrap();

        let entries = doc.list_directory().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].company, "RDC");
        assert_eq!(entries[0].contact, Some("James Botha".into()));
    }

    // =========================================================================
    // SPACES
    // =========================================================================

    #[test]
    fn space_round_trip() {
        let doc = SedDocument::in_memory().unwrap();
        let id = generate_id();
        doc.add_space(&Space {
            id: id.clone(), tag: "L1-01".into(), name: "Sales Area".into(),
            level: "Level 1".into(), space_type: Some("retail".into()),
            area_m2: Some(400.0), ceiling_ht_m: Some(4.3),
            scope: "in_contract".into(), parent_id: None, boundary_id: None,
            x: Some(10.0), y: Some(20.0),
        }).unwrap();

        let space = doc.get_space(&id).unwrap().unwrap();
        assert_eq!(space.tag, "L1-01");
        assert_eq!(space.name, "Sales Area");
        assert_eq!(space.area_m2, Some(400.0));
        assert_eq!(space.x, Some(10.0));
    }

    #[test]
    fn space_list_and_count() {
        let doc = SedDocument::in_memory().unwrap();
        for i in 0..5 {
            doc.add_space(&Space {
                id: generate_id(), tag: format!("L1-{:02}", i), name: format!("Room {}", i),
                level: "Level 1".into(), space_type: None, area_m2: None, ceiling_ht_m: None,
                scope: "in_contract".into(), parent_id: None, boundary_id: None,
                x: None, y: None,
            }).unwrap();
        }
        assert_eq!(doc.count("spaces").unwrap(), 5);
        assert_eq!(doc.list_spaces().unwrap().len(), 5);
    }

    #[test]
    fn space_update() {
        let doc = SedDocument::in_memory().unwrap();
        let id = generate_id();
        doc.add_space(&Space {
            id: id.clone(), tag: "L1-01".into(), name: "Old Name".into(),
            level: "Level 1".into(), space_type: None, area_m2: None, ceiling_ht_m: None,
            scope: "in_contract".into(), parent_id: None, boundary_id: None,
            x: None, y: None,
        }).unwrap();

        doc.update_space(&id, "name", Some("New Name")).unwrap();
        let space = doc.get_space(&id).unwrap().unwrap();
        assert_eq!(space.name, "New Name");
    }

    #[test]
    fn space_update_rejects_bad_field() {
        let doc = SedDocument::in_memory().unwrap();
        let id = generate_id();
        doc.add_space(&Space {
            id: id.clone(), tag: "L1-01".into(), name: "Room".into(),
            level: "Level 1".into(), space_type: None, area_m2: None, ceiling_ht_m: None,
            scope: "in_contract".into(), parent_id: None, boundary_id: None,
            x: None, y: None,
        }).unwrap();

        let result = doc.update_space(&id, "id", Some("hacked"));
        assert!(result.is_err());
        let result = doc.update_space(&id, "DROP TABLE spaces; --", Some("x"));
        assert!(result.is_err());
    }

    #[test]
    fn space_delete() {
        let doc = SedDocument::in_memory().unwrap();
        let id = generate_id();
        doc.add_space(&Space {
            id: id.clone(), tag: "L1-01".into(), name: "Room".into(),
            level: "Level 1".into(), space_type: None, area_m2: None, ceiling_ht_m: None,
            scope: "in_contract".into(), parent_id: None, boundary_id: None,
            x: None, y: None,
        }).unwrap();

        assert_eq!(doc.count("spaces").unwrap(), 1);
        doc.delete_space(&id).unwrap();
        assert_eq!(doc.count("spaces").unwrap(), 0);
    }

    // =========================================================================
    // PRODUCT TYPES
    // =========================================================================

    #[test]
    fn product_type_round_trip() {
        let doc = SedDocument::in_memory().unwrap();
        doc.add_product_type(&ProductType {
            id: generate_id(), tag: "LD-1".into(), domain: "air_device".into(),
            category: "supply_diffuser".into(), manufacturer: Some("Titus".into()),
            model: Some("FL-10".into()), description: Some("Linear slot".into()),
            mounting: Some("mud-in".into()), finish: None, size_nominal: None,
            voltage: None, phase: None, hz: None, submittal_id: None,
        }).unwrap();

        let types = doc.list_product_types().unwrap();
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].tag, "LD-1");
        assert_eq!(types[0].manufacturer, Some("Titus".into()));
    }

    #[test]
    fn product_type_unique_tag() {
        let doc = SedDocument::in_memory().unwrap();
        doc.add_product_type(&ProductType {
            id: generate_id(), tag: "LD-1".into(), domain: "air_device".into(),
            category: "supply_diffuser".into(), manufacturer: None, model: None,
            description: None, mounting: None, finish: None, size_nominal: None,
            voltage: None, phase: None, hz: None, submittal_id: None,
        }).unwrap();

        let result = doc.add_product_type(&ProductType {
            id: generate_id(), tag: "LD-1".into(), domain: "air_device".into(),
            category: "supply_diffuser".into(), manufacturer: None, model: None,
            description: None, mounting: None, finish: None, size_nominal: None,
            voltage: None, phase: None, hz: None, submittal_id: None,
        });
        assert!(result.is_err()); // duplicate tag
    }

    // =========================================================================
    // PLACEMENTS
    // =========================================================================

    #[test]
    fn placement_round_trip() {
        let doc = SedDocument::in_memory().unwrap();
        let pt_id = generate_id();
        doc.add_product_type(&ProductType {
            id: pt_id.clone(), tag: "LD-1".into(), domain: "air_device".into(),
            category: "supply_diffuser".into(), manufacturer: None, model: None,
            description: None, mounting: None, finish: None, size_nominal: None,
            voltage: None, phase: None, hz: None, submittal_id: None,
        }).unwrap();

        let p_id = generate_id();
        doc.add_placement(&Placement {
            id: p_id.clone(), instance_tag: None, product_type_id: pt_id, space_id: None,
            level: "Level 1".into(), x: Some(5.0), y: Some(10.0), rotation: None,
            cfm: Some(185.0), cfm_balanced: None, static_pressure_pa: None,
            status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
            weight_kg: None, notes: None,
        }).unwrap();

        let placements = doc.list_placements().unwrap();
        assert_eq!(placements.len(), 1);
        assert_eq!(placements[0].cfm, Some(185.0));
        assert_eq!(placements[0].x, Some(5.0));
    }

    #[test]
    fn placement_update() {
        let doc = SedDocument::in_memory().unwrap();
        let pt_id = generate_id();
        doc.add_product_type(&ProductType {
            id: pt_id.clone(), tag: "SR-1".into(), domain: "air_device".into(),
            category: "supply_register".into(), manufacturer: None, model: None,
            description: None, mounting: None, finish: None, size_nominal: None,
            voltage: None, phase: None, hz: None, submittal_id: None,
        }).unwrap();

        let p_id = generate_id();
        doc.add_placement(&Placement {
            id: p_id.clone(), instance_tag: None, product_type_id: pt_id, space_id: None,
            level: "Level 1".into(), x: None, y: None, rotation: None,
            cfm: Some(100.0), cfm_balanced: None, static_pressure_pa: None,
            status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
            weight_kg: None, notes: None,
        }).unwrap();

        doc.update_placement(&p_id, "cfm", Some("125")).unwrap();
        doc.update_placement(&p_id, "phase", Some("submitted")).unwrap();

        let placements = doc.list_placements().unwrap();
        assert_eq!(placements[0].phase, "submitted");
    }

    #[test]
    fn placement_update_rejects_bad_field() {
        let doc = SedDocument::in_memory().unwrap();
        let result = doc.update_placement("some-id", "product_type_id", Some("hacked"));
        assert!(result.is_err());
    }

    // =========================================================================
    // SYSTEMS
    // =========================================================================

    #[test]
    fn system_round_trip() {
        let doc = SedDocument::in_memory().unwrap();
        doc.add_system(&System {
            id: generate_id(), tag: "RTU-1-SA".into(), name: "RTU-1 Supply".into(),
            system_type: "supply".into(), medium: "air".into(), source_id: None, paired_system_id: None,
        }).unwrap();

        let systems = doc.list_systems().unwrap();
        assert_eq!(systems.len(), 1);
        assert_eq!(systems[0].tag, "RTU-1-SA");
    }

    // =========================================================================
    // KEYED NOTES
    // =========================================================================

    #[test]
    fn keyed_notes_round_trip() {
        let doc = SedDocument::in_memory().unwrap();
        doc.add_keyed_note(&KeyedNote {
            id: generate_id(), key: "H1".into(),
            text: "Refer to arch RCP for alignments.".into(),
            discipline: Some("mechanical".into()), spec_section: None,
        }).unwrap();

        let notes = doc.list_keyed_notes().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].key, "H1");
    }

    // =========================================================================
    // RAW QUERIES
    // =========================================================================

    #[test]
    fn raw_query() {
        let doc = SedDocument::in_memory().unwrap();
        doc.set_meta("sed_version", "0.3").unwrap();
        let rows = doc.query_raw("SELECT key, value FROM meta").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0].1, "sed_version");
        assert_eq!(rows[0][1].1, "0.3");
    }

    // =========================================================================
    // SKIMS EXAMPLE ROUND-TRIP
    // =========================================================================

    #[test]
    fn skims_example_creates_and_queries() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        drop(tmp); // close so create_skims_americana can open it
        crate::examples::create_skims_americana(&path).unwrap();
        let doc = SedDocument::open(&path).unwrap();

        let info = doc.info().unwrap();
        assert_eq!(info.project_name, "SKIMS Americana at Brand");
        assert_eq!(info.project_number, "25-161");
        assert_eq!(info.spaces, 29);
        assert_eq!(info.product_types, 15);
        assert!(info.placements > 50);
        assert_eq!(info.systems, 4);
        assert_eq!(info.sheets, 6);
        assert_eq!(info.submittals, 4);
        assert_eq!(info.keyed_notes, 11);
        assert_eq!(info.revisions, 2);

        // Query: total supply CFM
        let rows = doc.query_raw(
            "SELECT SUM(p.cfm) as total FROM placements p JOIN product_types pt ON p.product_type_id = pt.id WHERE pt.category LIKE 'supply%'"
        ).unwrap();
        let total_cfm: f64 = rows[0][0].1.parse().unwrap();
        assert!(total_cfm > 5000.0); // should be around 6000+ CFM

        // Query: room count per level
        let rows = doc.query_raw(
            "SELECT level, COUNT(*) as n FROM spaces GROUP BY level ORDER BY level"
        ).unwrap();
        assert!(rows.len() >= 2); // at least Level 1 and Level 2

        // Query: submittals all for_approval
        let rows = doc.query_raw(
            "SELECT COUNT(*) as n FROM submittals WHERE status = 'for_approval'"
        ).unwrap();
        assert_eq!(rows[0][0].1, "4");

        // tempfile auto-cleans
    }

    #[test]
    fn skims_supply_cfm_by_room() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        drop(tmp);
        crate::examples::create_skims_americana(&path).unwrap();
        let doc = SedDocument::open(&path).unwrap();

        let rows = doc.query_raw(crate::query::SUPPLY_CFM_BY_ROOM).unwrap();
        // Sales Area should be the highest
        assert!(!rows.is_empty());
        let sales_cfm: f64 = rows[0][3].1.parse().unwrap(); // total_supply_cfm column
        assert!(sales_cfm > 2500.0);

        // tempfile auto-cleans
    }

    // =========================================================================
    // INFO DISPLAY
    // =========================================================================

    #[test]
    fn info_display() {
        let doc = SedDocument::in_memory().unwrap();
        doc.set_meta("sed_version", "0.3").unwrap();
        doc.set_meta("project_name", "Test").unwrap();
        doc.set_meta("project_number", "T-001").unwrap();
        let info = doc.info().unwrap();
        let display = format!("{}", info);
        assert!(display.contains("SED v0.3"));
        assert!(display.contains("Test"));
    }
}
