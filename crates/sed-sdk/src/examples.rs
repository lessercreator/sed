use anyhow::Result;
use crate::document::{SedDocument, generate_id};
use crate::types::*;

/// Populates a .sed file with real data from the SKIMS Americana at Brand project.
/// Project 25-161, 233 South Brand Blvd, Glendale, CA.
/// Source: KLH Engineers mechanical plans, Air-Tec takeoffs, submittals.
pub fn create_skims_americana(path: &str) -> Result<()> {
    let doc = SedDocument::create(path)?;

    // =========================================================================
    // META
    // =========================================================================
    doc.set_meta("sed_version", "0.3")?;
    doc.set_meta("project_name", "SKIMS Americana at Brand")?;
    doc.set_meta("project_number", "25-161")?;
    doc.set_meta("project_address", "233 South Brand Blvd, Glendale, CA 92106")?;
    doc.set_meta("units_display", "imperial")?;
    doc.set_meta("created_at", "2026-04-04T00:00:00Z")?;
    doc.set_meta("modified_at", "2026-04-04T00:00:00Z")?;

    // =========================================================================
    // DIRECTORY
    // =========================================================================
    let entries = vec![
        ("owner", "Caruso", Some("Ken Greenberg"), Some("kgreenberg@caruso.com"), Some("323-900-8137")),
        ("tenant", "SKIMS Retail, LLC", Some("Robbie Zweig"), Some("rz@skims.com"), Some("646-530-0153")),
        ("architect", "RDC", Some("James Botha"), Some("james.botha@rdcollaborative.com"), Some("562-901-1582")),
        ("engineer_mep", "KLH Engineers", Some("Jordan Laycock"), Some("jlaycock@klhengrs.com"), Some("859-547-0242")),
        ("engineer_struct", "RMJ & Associates", Some("Jayson Haines"), Some("jhaines@rmjse.com"), Some("510-991-0977")),
        ("contractor_mech", "Air-Tec", None, None, Some("310-549-1698")),
    ];
    for (role, company, contact, email, phone) in entries {
        doc.add_directory_entry(&DirectoryEntry {
            id: generate_id(), role: role.into(), company: company.into(),
            contact: contact.map(Into::into), email: email.map(Into::into),
            phone: phone.map(Into::into), address: None,
        })?;
    }

    // =========================================================================
    // SPACES — every room from the SKIMS plans
    // =========================================================================
    let spaces_data = vec![
        ("L1-01", "Sales Area",       "Level 1", "retail",     "in_contract"),
        ("L1-02", "Fit Room 1",       "Level 1", "retail",     "in_contract"),
        ("L1-03", "Fit Room 2 ADA",   "Level 1", "retail",     "in_contract"),
        ("L1-04", "Fit Room 3",       "Level 1", "retail",     "in_contract"),
        ("L1-05", "Fit Room 4",       "Level 1", "retail",     "in_contract"),
        ("L1-06", "Fit Room 5",       "Level 1", "retail",     "in_contract"),
        ("L1-07", "Fit Room 6",       "Level 1", "retail",     "in_contract"),
        ("L1-08", "Fit Room 7",       "Level 1", "retail",     "in_contract"),
        ("L1-09", "Fit Room 8",       "Level 1", "retail",     "in_contract"),
        ("L1-10", "Elevator Shaft",   "Level 1", "elevator",   "nic"),
        ("L1-11", "Go-Backs",         "Level 1", "storage",    "in_contract"),
        ("L1-12", "BOH Storage",      "Level 1", "storage",    "in_contract"),
        ("L1-13", "Corridor 1",       "Level 1", "corridor",   "in_contract"),
        ("L1-14", "Stairs",           "Level 1", "circulation","nic"),
        ("L1-16", "Mall Service Circulation", "Level 1", "corridor", "nic"),
        ("L1-17", "Mall Service Corridor",    "Level 1", "corridor", "nic"),
        ("L2-00", "Elevator Shaft",   "Level 2", "elevator",   "nic"),
        ("L2-01", "Unused",           "Level 2", "storage",    "in_contract"),
        ("L2-02", "Corridor 2",       "Level 2", "corridor",   "in_contract"),
        ("L2-03", "BOH Storage",      "Level 2", "storage",    "in_contract"),
        ("L2-04", "Corridor 3",       "Level 2", "corridor",   "in_contract"),
        ("L2-05", "Break Area",       "Level 2", "office",     "in_contract"),
        ("L2-06", "Restroom 1",       "Level 2", "restroom",   "in_contract"),
        ("L2-07", "Restroom 2",       "Level 2", "restroom",   "in_contract"),
        ("L2-08", "Managers Office",  "Level 2", "office",     "in_contract"),
        ("L2-09", "BOH Storage",      "Level 2", "storage",    "in_contract"),
        ("L2-10", "Riser Shaft",      "Level 2", "mechanical", "in_contract"),
        ("L2-11", "Elevator Machine Room", "Level 2", "mechanical", "nic"),
        ("L2-12", "Mop Closet",       "Level 2", "storage",    "in_contract"),
    ];

    let mut space_ids: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for (tag, name, level, stype, scope) in &spaces_data {
        let id = generate_id();
        space_ids.insert(tag.to_string(), id.clone());
        doc.add_space(&Space {
            id, tag: tag.to_string(), name: name.to_string(), level: level.to_string(),
            space_type: Some(stype.to_string()), area_m2: None, ceiling_ht_m: None,
            scope: scope.to_string(), parent_id: None, boundary_id: None,
            x: None, y: None,
        })?;
    }

    // =========================================================================
    // SUBMITTALS
    // =========================================================================
    let sub_fsd_id = generate_id();
    let sub_ad_id = generate_id();
    let sub_ef_id = generate_id();
    let sub_insul_id = generate_id();

    doc.add_submittal(&Submittal {
        id: sub_fsd_id.clone(), number: Some("1".into()),
        description: "Fire Smoke Dampers — Pottorff FSD-352".into(),
        submitted_by: Some("Dasha Perkins".into()), company: Some("Toro-Aire Inc".into()),
        date_submitted: Some("2026-03-18".into()), status: "for_approval".into(),
        spec_section: Some("233400".into()),
    })?;
    doc.add_submittal(&Submittal {
        id: sub_ad_id.clone(), number: Some("2".into()),
        description: "Air Distribution Devices — Titus".into(),
        submitted_by: Some("Dasha Perkins".into()), company: Some("So-Cal Air Distribution".into()),
        date_submitted: Some("2026-03-18".into()), status: "for_approval".into(),
        spec_section: Some("233713".into()),
    })?;
    doc.add_submittal(&Submittal {
        id: sub_ef_id.clone(), number: Some("3".into()),
        description: "Exhaust Fan — Broan L-400L".into(),
        submitted_by: Some("Dasha Perkins".into()), company: Some("So-Cal Air Distribution".into()),
        date_submitted: Some("2026-03-18".into()), status: "for_approval".into(),
        spec_section: Some("233423".into()),
    })?;
    doc.add_submittal(&Submittal {
        id: sub_insul_id.clone(), number: Some("4".into()),
        description: "Duct Insulation — CertainTeed SoftTouch".into(),
        submitted_by: Some("Mark Schaefer".into()), company: Some("So-Cal Insulation Inc".into()),
        date_submitted: Some("2026-03-18".into()), status: "for_approval".into(),
        spec_section: Some("230713".into()),
    })?;

    // =========================================================================
    // PRODUCT TYPES
    // =========================================================================
    let pt_exrtu = generate_id();
    let pt_ef1 = generate_id();
    let pt_ld1 = generate_id();
    let pt_ld2 = generate_id();
    let pt_sr1 = generate_id();
    let pt_sr2 = generate_id();
    let pt_cd1 = generate_id();
    let pt_cd2 = generate_id();
    let pt_er1 = generate_id();
    let pt_rg1 = generate_id();
    let pt_rg2 = generate_id();
    let pt_tg1 = generate_id();
    let pt_tg2 = generate_id();
    let pt_tg3 = generate_id();
    let pt_fsd1 = generate_id();

    let product_types = vec![
        ProductType { id: pt_exrtu.clone(), tag: "EXRTU".into(), domain: "equipment".into(), category: "rtu".into(),
            manufacturer: None, model: None, description: Some("Existing rooftop unit".into()),
            mounting: None, finish: None, size_nominal: None, voltage: None, phase: None, hz: None, submittal_id: None },
        ProductType { id: pt_ef1.clone(), tag: "EF-1".into(), domain: "equipment".into(), category: "exhaust_fan".into(),
            manufacturer: Some("Broan-NuTone".into()), model: Some("L-400L".into()),
            description: Some("Losone Select in-line ventilator with backdraft damper".into()),
            mounting: Some("suspended".into()), finish: None, size_nominal: None,
            voltage: Some(120.0), phase: Some(1), hz: Some(60.0), submittal_id: Some(sub_ef_id.clone()) },
        ProductType { id: pt_ld1.clone(), tag: "LD-1".into(), domain: "air_device".into(), category: "supply_diffuser".into(),
            manufacturer: Some("Titus".into()), model: Some("FL-10".into()),
            description: Some("Aluminum architectural linear slot, 1\" single slot, insulated plenum, Border 22 mud-in".into()),
            mounting: Some("mud-in".into()), finish: Some("standard white".into()), size_nominal: None,
            voltage: None, phase: None, hz: None, submittal_id: Some(sub_ad_id.clone()) },
        ProductType { id: pt_ld2.clone(), tag: "LD-2".into(), domain: "air_device".into(), category: "supply_diffuser".into(),
            manufacturer: Some("Titus".into()), model: Some("FL-10".into()),
            description: Some("Aluminum architectural linear slot, Border 14 offset".into()),
            mounting: Some("offset".into()), finish: Some("standard white".into()), size_nominal: None,
            voltage: None, phase: None, hz: None, submittal_id: Some(sub_ad_id.clone()) },
        ProductType { id: pt_sr1.clone(), tag: "SR-1".into(), domain: "air_device".into(), category: "supply_register".into(),
            manufacturer: Some("Titus".into()), model: Some("S300FL".into()),
            description: Some("Double deflection direct spiral duct mounted supply register with ASD air scoop".into()),
            mounting: Some("duct_mounted".into()), finish: Some("standard white".into()), size_nominal: None,
            voltage: None, phase: None, hz: None, submittal_id: Some(sub_ad_id.clone()) },
        ProductType { id: pt_sr2.clone(), tag: "SR-2".into(), domain: "air_device".into(), category: "supply_register".into(),
            manufacturer: Some("Titus".into()), model: Some("S300FL".into()),
            description: Some("Double deflection direct spiral duct mounted supply register with ASD air scoop".into()),
            mounting: Some("duct_mounted".into()), finish: Some("standard white".into()), size_nominal: None,
            voltage: None, phase: None, hz: None, submittal_id: Some(sub_ad_id.clone()) },
        ProductType { id: pt_cd1.clone(), tag: "CD-1".into(), domain: "air_device".into(), category: "ceiling_diffuser".into(),
            manufacturer: Some("Titus".into()), model: Some("OMNI".into()),
            description: Some("Steel square plaque supply diffuser, Border 3, lay-in".into()),
            mounting: Some("lay-in".into()), finish: Some("standard white".into()), size_nominal: None,
            voltage: None, phase: None, hz: None, submittal_id: Some(sub_ad_id.clone()) },
        ProductType { id: pt_cd2.clone(), tag: "CD-2".into(), domain: "air_device".into(), category: "ceiling_diffuser".into(),
            manufacturer: Some("Titus".into()), model: Some("OMNI".into()),
            description: Some("Steel square plaque supply diffuser, 12\"x12\" module, Border 3, TRM frame for hard lid".into()),
            mounting: Some("surface".into()), finish: Some("standard white".into()), size_nominal: Some("10\"".into()),
            voltage: None, phase: None, hz: None, submittal_id: Some(sub_ad_id.clone()) },
        ProductType { id: pt_er1.clone(), tag: "ER-1".into(), domain: "air_device".into(), category: "exhaust_register".into(),
            manufacturer: Some("Titus".into()), model: Some("350FL".into()),
            description: Some("Aluminum louvered return grille, 3/4\" blade spacing, 35deg deflection, surface mount".into()),
            mounting: Some("surface".into()), finish: Some("standard white".into()), size_nominal: None,
            voltage: None, phase: None, hz: None, submittal_id: Some(sub_ad_id.clone()) },
        ProductType { id: pt_rg1.clone(), tag: "RG-1".into(), domain: "air_device".into(), category: "return_grille".into(),
            manufacturer: Some("Titus".into()), model: Some("350FL".into()),
            description: Some("Aluminum louvered return grille, lay-in with round neck adapter".into()),
            mounting: Some("lay-in".into()), finish: Some("standard white".into()), size_nominal: Some("22\"x22\" neck".into()),
            voltage: None, phase: None, hz: None, submittal_id: Some(sub_ad_id.clone()) },
        ProductType { id: pt_rg2.clone(), tag: "RG-2".into(), domain: "air_device".into(), category: "return_grille".into(),
            manufacturer: Some("Titus".into()), model: Some("350FL".into()),
            description: Some("Aluminum louvered return grille, surface mount".into()),
            mounting: Some("surface".into()), finish: Some("standard white".into()), size_nominal: None,
            voltage: None, phase: None, hz: None, submittal_id: Some(sub_ad_id.clone()) },
        ProductType { id: pt_tg1.clone(), tag: "TG-1".into(), domain: "air_device".into(), category: "transfer_grille".into(),
            manufacturer: Some("Titus".into()), model: Some("350FL".into()),
            description: Some("Transfer grille, surface mount".into()),
            mounting: Some("surface".into()), finish: Some("standard white".into()), size_nominal: None,
            voltage: None, phase: None, hz: None, submittal_id: Some(sub_ad_id.clone()) },
        ProductType { id: pt_tg2.clone(), tag: "TG-2".into(), domain: "air_device".into(), category: "transfer_grille".into(),
            manufacturer: Some("Titus".into()), model: Some("350FL".into()),
            description: Some("Transfer grille, surface mount".into()),
            mounting: Some("surface".into()), finish: Some("standard white".into()), size_nominal: None,
            voltage: None, phase: None, hz: None, submittal_id: Some(sub_ad_id.clone()) },
        ProductType { id: pt_tg3.clone(), tag: "TG-3".into(), domain: "air_device".into(), category: "transfer_grille".into(),
            manufacturer: Some("Titus".into()), model: Some("350FL".into()),
            description: Some("Transfer grille, surface mount".into()),
            mounting: Some("surface".into()), finish: Some("standard white".into()), size_nominal: None,
            voltage: None, phase: None, hz: None, submittal_id: Some(sub_ad_id.clone()) },
        ProductType { id: pt_fsd1.clone(), tag: "FSD-1".into(), domain: "accessory".into(), category: "fire_smoke_damper".into(),
            manufacturer: Some("Pottorff".into()), model: Some("FSD-352".into()),
            description: Some("Combination fire smoke damper, 3 hour, UL class 2, airfoil blade, FSLF120 actuator".into()),
            mounting: None, finish: None, size_nominal: Some("10\"x10\"".into()),
            voltage: Some(120.0), phase: None, hz: None, submittal_id: Some(sub_fsd_id.clone()) },
    ];

    for pt in &product_types {
        doc.add_product_type(pt)?;
    }

    // =========================================================================
    // PLACEMENTS — every physical instance
    // =========================================================================

    // Equipment
    let exrtu1_id = generate_id();
    let exrtu2_id = generate_id();
    let ef1_id = generate_id();
    let fsd1_id = generate_id();

    doc.add_placement(&Placement {
        id: exrtu1_id.clone(), instance_tag: Some("EXRTU-1".into()),
        product_type_id: pt_exrtu.clone(), space_id: None,
        level: "Roof".into(), x: None, y: None, rotation: None,
        cfm: None, cfm_balanced: None, static_pressure_pa: None,
        status: "existing_remain".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: Some(738.0),
        notes: Some("Existing RTU to remain. Balance to scheduled airflow. Clean and verify proper operation; clean coils, recharge refrigerant, replace belt/drive/motor as required, replace filters. Provide reconditioning report.".into()),
    })?;
    doc.add_placement(&Placement {
        id: exrtu2_id.clone(), instance_tag: Some("EXRTU-2".into()),
        product_type_id: pt_exrtu.clone(), space_id: None,
        level: "Roof".into(), x: None, y: None, rotation: None,
        cfm: None, cfm_balanced: None, static_pressure_pa: None,
        status: "existing_remain".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: Some(738.0),
        notes: Some("Existing RTU to remain. Same scope as EXRTU-1.".into()),
    })?;
    doc.add_placement(&Placement {
        id: ef1_id.clone(), instance_tag: Some("EF-1".into()),
        product_type_id: pt_ef1.clone(),
        space_id: space_ids.get("L2-09").cloned(), level: "Level 2".into(),
        x: None, y: None, rotation: None,
        cfm: Some(210.0), cfm_balanced: None, static_pressure_pa: Some(125.0), // 0.5 inWG
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: None,
        notes: Some("Balance to 130 CFM. Extend new exhaust ductwork to existing exhaust main.".into()),
    })?;

    // FSD
    doc.add_placement(&Placement {
        id: fsd1_id.clone(), instance_tag: Some("FSD-1".into()),
        product_type_id: pt_fsd1.clone(),
        space_id: None, level: "Level 1".into(),
        x: None, y: None, rotation: None,
        cfm: None, cfm_balanced: None, static_pressure_pa: None,
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: None, notes: None,
    })?;

    // Level 1 — Sales Area supply diffusers (LD-1)
    // From plans: approximately 16 LD-1 placements at 180-185 CFM each
    // We track their IDs for duct graph terminal connections
    let ld1_cfms = vec![
        185.0, 180.0, 185.0, 185.0, 180.0, 180.0, 185.0, 180.0,
        185.0, 180.0, 180.0, 185.0, 185.0, 180.0, 185.0, 180.0,
    ];
    let mut ld1_placement_ids: Vec<String> = Vec::new();
    for cfm in &ld1_cfms {
        let pid = generate_id();
        ld1_placement_ids.push(pid.clone());
        doc.add_placement(&Placement {
            id: pid, instance_tag: None,
            product_type_id: pt_ld1.clone(),
            space_id: space_ids.get("L1-01").cloned(), level: "Level 1".into(),
            x: None, y: None, rotation: None,
            cfm: Some(*cfm), cfm_balanced: None, static_pressure_pa: None,
            status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
            weight_kg: None, notes: None,
        })?;
    }

    // Level 1 — Fit rooms (LD-2)
    let fit_rooms = vec![
        ("L1-02", 185.0), ("L1-03", 95.0),
        ("L1-04", 85.0), ("L1-05", 85.0), ("L1-06", 85.0),
        ("L1-07", 85.0), ("L1-08", 85.0), ("L1-09", 95.0),
    ];
    for (room_tag, cfm) in &fit_rooms {
        doc.add_placement(&Placement {
            id: generate_id(), instance_tag: None,
            product_type_id: pt_ld2.clone(),
            space_id: space_ids.get(*room_tag).cloned(), level: "Level 1".into(),
            x: None, y: None, rotation: None,
            cfm: Some(*cfm), cfm_balanced: None, static_pressure_pa: None,
            status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
            weight_kg: None, notes: None,
        })?;
    }

    // Level 1 — LD-2 in corridor/go-backs
    doc.add_placement(&Placement {
        id: generate_id(), instance_tag: None,
        product_type_id: pt_ld2.clone(),
        space_id: space_ids.get("L1-11").cloned(), level: "Level 1".into(),
        x: None, y: None, rotation: None,
        cfm: Some(150.0), cfm_balanced: None, static_pressure_pa: None,
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: None, notes: None,
    })?;

    // Level 1 — CD-2
    doc.add_placement(&Placement {
        id: generate_id(), instance_tag: None,
        product_type_id: pt_cd2.clone(),
        space_id: space_ids.get("L1-01").cloned(), level: "Level 1".into(),
        x: None, y: None, rotation: None,
        cfm: Some(190.0), cfm_balanced: None, static_pressure_pa: None,
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: None, notes: None,
    })?;

    // Level 1 — SR-1 (BOH/corridor)
    for cfm in [95.0, 90.0, 90.0] {
        doc.add_placement(&Placement {
            id: generate_id(), instance_tag: None,
            product_type_id: pt_sr1.clone(),
            space_id: space_ids.get("L1-12").cloned(), level: "Level 1".into(),
            x: None, y: None, rotation: None,
            cfm: Some(cfm), cfm_balanced: None, static_pressure_pa: None,
            status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
            weight_kg: None, notes: None,
        })?;
    }

    // Level 1 — SR-2
    doc.add_placement(&Placement {
        id: generate_id(), instance_tag: None,
        product_type_id: pt_sr2.clone(),
        space_id: space_ids.get("L1-12").cloned(), level: "Level 1".into(),
        x: None, y: None, rotation: None,
        cfm: Some(65.0), cfm_balanced: None, static_pressure_pa: None,
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: None, notes: None,
    })?;

    // Level 1 — TG-1 (transfer grilles)
    for _ in 0..2 {
        doc.add_placement(&Placement {
            id: generate_id(), instance_tag: None,
            product_type_id: pt_tg1.clone(),
            space_id: space_ids.get("L1-13").cloned(), level: "Level 1".into(),
            x: None, y: None, rotation: None,
            cfm: None, cfm_balanced: None, static_pressure_pa: None,
            status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
            weight_kg: None, notes: None,
        })?;
    }

    // Level 1 — RG-2
    doc.add_placement(&Placement {
        id: generate_id(), instance_tag: None,
        product_type_id: pt_rg2.clone(),
        space_id: space_ids.get("L1-09").cloned(), level: "Level 1".into(),
        x: None, y: None, rotation: None,
        cfm: None, cfm_balanced: None, static_pressure_pa: None,
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: None, notes: None,
    })?;

    // Level 2 — Managers Office
    doc.add_placement(&Placement {
        id: generate_id(), instance_tag: None,
        product_type_id: pt_cd1.clone(),
        space_id: space_ids.get("L2-08").cloned(), level: "Level 2".into(),
        x: None, y: None, rotation: None,
        cfm: Some(100.0), cfm_balanced: None, static_pressure_pa: None,
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: None, notes: None,
    })?;

    // Level 2 — RG-1 (managers office return)
    doc.add_placement(&Placement {
        id: generate_id(), instance_tag: None,
        product_type_id: pt_rg1.clone(),
        space_id: space_ids.get("L2-08").cloned(), level: "Level 2".into(),
        x: None, y: None, rotation: None,
        cfm: None, cfm_balanced: None, static_pressure_pa: None,
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: None, notes: None,
    })?;

    // Level 2 — Restroom exhaust (ER-1)
    for (room, cfm) in [("L2-06", 80.0), ("L2-07", 80.0)] {
        doc.add_placement(&Placement {
            id: generate_id(), instance_tag: None,
            product_type_id: pt_er1.clone(),
            space_id: space_ids.get(room).cloned(), level: "Level 2".into(),
            x: None, y: None, rotation: None,
            cfm: Some(cfm), cfm_balanced: None, static_pressure_pa: None,
            status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
            weight_kg: None, notes: None,
        })?;
    }

    // Level 2 — CD-2 (restrooms)
    for (room, cfm) in [("L2-06", 40.0), ("L2-07", 40.0)] {
        doc.add_placement(&Placement {
            id: generate_id(), instance_tag: None,
            product_type_id: pt_cd2.clone(),
            space_id: space_ids.get(room).cloned(), level: "Level 2".into(),
            x: None, y: None, rotation: None,
            cfm: Some(cfm), cfm_balanced: None, static_pressure_pa: None,
            status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
            weight_kg: None, notes: None,
        })?;
    }

    // Level 2 — SR-1 supply registers
    let l2_sr1 = vec![
        ("L2-05", 110.0), ("L2-05", 115.0), ("L2-05", 110.0),
        ("L2-02", 110.0), ("L2-02", 135.0), ("L2-02", 135.0), ("L2-02", 135.0),
        ("L2-03", 135.0),
        ("L2-08", 125.0), ("L2-08", 115.0),
        ("L2-01", 135.0),
    ];
    for (room, cfm) in &l2_sr1 {
        doc.add_placement(&Placement {
            id: generate_id(), instance_tag: None,
            product_type_id: pt_sr1.clone(),
            space_id: space_ids.get(*room).cloned(), level: "Level 2".into(),
            x: None, y: None, rotation: None,
            cfm: Some(*cfm), cfm_balanced: None, static_pressure_pa: None,
            status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
            weight_kg: None, notes: None,
        })?;
    }

    // Level 2 — SR-2
    for (room, cfm) in [("L2-06", 50.0), ("L2-07", 50.0), ("L2-04", 55.0)] {
        doc.add_placement(&Placement {
            id: generate_id(), instance_tag: None,
            product_type_id: pt_sr2.clone(),
            space_id: space_ids.get(room).cloned(), level: "Level 2".into(),
            x: None, y: None, rotation: None,
            cfm: Some(cfm), cfm_balanced: None, static_pressure_pa: None,
            status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
            weight_kg: None, notes: None,
        })?;
    }

    // Level 2 — Transfer grilles
    for _ in 0..2 {
        doc.add_placement(&Placement {
            id: generate_id(), instance_tag: None,
            product_type_id: pt_tg2.clone(),
            space_id: space_ids.get("L2-01").cloned(), level: "Level 2".into(),
            x: None, y: None, rotation: None,
            cfm: None, cfm_balanced: None, static_pressure_pa: None,
            status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
            weight_kg: None, notes: None,
        })?;
    }
    doc.add_placement(&Placement {
        id: generate_id(), instance_tag: None,
        product_type_id: pt_tg3.clone(),
        space_id: space_ids.get("L2-09").cloned(), level: "Level 2".into(),
        x: None, y: None, rotation: None,
        cfm: None, cfm_balanced: None, static_pressure_pa: None,
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: None, notes: None,
    })?;

    // Level 2 — ER-1 (mop closet)
    doc.add_placement(&Placement {
        id: generate_id(), instance_tag: None,
        product_type_id: pt_er1.clone(),
        space_id: space_ids.get("L2-12").cloned(), level: "Level 2".into(),
        x: None, y: None, rotation: None,
        cfm: Some(50.0), cfm_balanced: None, static_pressure_pa: None,
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: None, notes: None,
    })?;

    // =========================================================================
    // SYSTEMS
    // =========================================================================
    let sys_rtu1_sa = generate_id();
    let sys_rtu1_ra = generate_id();
    let sys_rtu2_sa = generate_id();
    let sys_ex1 = generate_id();

    doc.add_system(&System { id: sys_rtu1_sa.clone(), tag: "RTU-1-SA".into(), name: "EXRTU-1 Supply Air".into(), system_type: "supply".into(), medium: "air".into(), source_id: Some(exrtu1_id.clone()), paired_system_id: None })?;
    doc.add_system(&System { id: sys_rtu1_ra.clone(), tag: "RTU-1-RA".into(), name: "EXRTU-1 Return Air".into(), system_type: "return".into(), medium: "air".into(), source_id: Some(exrtu1_id.clone()), paired_system_id: None })?;
    doc.add_system(&System { id: sys_rtu2_sa.clone(), tag: "RTU-2-SA".into(), name: "EXRTU-2 Supply Air".into(), system_type: "supply".into(), medium: "air".into(), source_id: Some(exrtu2_id.clone()), paired_system_id: None })?;
    doc.add_system(&System { id: sys_ex1.clone(), tag: "EX-1".into(), name: "Exhaust System 1".into(), system_type: "exhaust".into(), medium: "air".into(), source_id: Some(ef1_id.clone()), paired_system_id: None })?;
    // Link supply/return pairs after both exist
    doc.execute_raw("UPDATE systems SET paired_system_id = ?1 WHERE id = ?2", &[&sys_rtu1_ra as &dyn rusqlite::types::ToSql, &sys_rtu1_sa])?;
    doc.execute_raw("UPDATE systems SET paired_system_id = ?1 WHERE id = ?2", &[&sys_rtu1_sa as &dyn rusqlite::types::ToSql, &sys_rtu1_ra])?;

    // =========================================================================
    // DUCT GRAPH — RTU-1 Supply Air, Level 1
    // Main trunk runs east-west along the corridor at y=13.0 from x=3.0 to x=13.0.
    // 8 branch taps serve LD-1 diffusers in the sales area.
    // Trunk decreases in diameter after each branch takeoff.
    // =========================================================================

    // Branch tap data: (x_position, trunk_diameter_inches, ld1_index)
    // Total supply = 8 branches x 185 CFM = 1480 CFM entering this trunk run
    // (The full system has 16 diffusers; this models the east trunk run)
    let branch_cfm = 185.0;
    let branch_taps: Vec<(f64, f64, usize)> = vec![
        (3.5,  28.0, 0),
        (4.75, 26.0, 1),
        (6.0,  24.0, 2),
        (7.25, 22.0, 3),
        (8.5,  20.0, 4),
        (9.75, 18.0, 5),
        (11.0, 16.0, 6),
        (12.25, 14.0, 7),
    ];
    let total_flow = branch_taps.len() as f64 * branch_cfm;

    // Create the equipment connection node (EXRTU-1 supply outlet)
    let equip_node_id = generate_id();
    doc.add_node(&Node {
        id: equip_node_id.clone(), system_id: sys_rtu1_sa.clone(),
        node_type: "equipment_conn".into(), placement_id: Some(exrtu1_id.clone()),
        fitting_type: None, size_description: Some("28\" round".into()),
        level: Some("Level 1".into()), x: Some(3.0), y: Some(13.0),
    })?;

    let mut prev_node_id = equip_node_id;
    let mut segment_ids: Vec<String> = Vec::new();

    for (i, (tap_x, trunk_dia, ld1_idx)) in branch_taps.iter().enumerate() {
        let trunk_dia_m = trunk_dia * 0.0254;
        let branch_dia_m = 8.0 * 0.0254;
        let flow_at_tap = total_flow - (i as f64 * branch_cfm); // flow entering this tap

        // Create tap node on the trunk
        let tap_node_id = generate_id();
        doc.add_node(&Node {
            id: tap_node_id.clone(), system_id: sys_rtu1_sa.clone(),
            node_type: "fitting".into(), placement_id: None,
            fitting_type: Some("tap_45".into()),
            size_description: Some(format!("{:.0}\" trunk / 8\" branch", trunk_dia)),
            level: Some("Level 1".into()), x: Some(*tap_x), y: Some(13.0),
        })?;

        // Trunk segment from previous node to this tap
        let gauge = if *trunk_dia >= 20.0 { 24 } else { 26 };
        let seg_len = tap_x - if i == 0 { 3.0 } else { branch_taps[i - 1].0 };
        let trunk_seg_id = generate_id();
        doc.add_segment(&Segment {
            id: trunk_seg_id.clone(), system_id: sys_rtu1_sa.clone(),
            from_node_id: prev_node_id.clone(), to_node_id: tap_node_id.clone(),
            shape: "round".into(), width_m: None, height_m: None,
            diameter_m: Some(trunk_dia_m), length_m: Some(seg_len),
            material: "galvanized".into(), gauge: Some(gauge),
            pressure_class: Some("2_in_wg".into()), construction: Some("spiral_lock".into()),
            exposure: Some("concealed".into()),
            flow_design: Some(flow_at_tap), flow_balanced: None,
            status: "new".into(), scope: "in_contract".into(),
        })?;
        segment_ids.push(trunk_seg_id);

        // Terminal node for the LD-1 diffuser at this branch
        let terminal_node_id = generate_id();
        let terminal_placement = if *ld1_idx < ld1_placement_ids.len() {
            Some(ld1_placement_ids[*ld1_idx].clone())
        } else {
            None
        };
        doc.add_node(&Node {
            id: terminal_node_id.clone(), system_id: sys_rtu1_sa.clone(),
            node_type: "terminal".into(), placement_id: terminal_placement,
            fitting_type: None, size_description: Some("8\" round".into()),
            level: Some("Level 1".into()), x: Some(*tap_x), y: Some(11.5),
        })?;

        // Branch segment from tap to terminal
        let branch_seg_id = generate_id();
        doc.add_segment(&Segment {
            id: branch_seg_id.clone(), system_id: sys_rtu1_sa.clone(),
            from_node_id: tap_node_id.clone(), to_node_id: terminal_node_id,
            shape: "round".into(), width_m: None, height_m: None,
            diameter_m: Some(branch_dia_m), length_m: Some(1.5),
            material: "galvanized".into(), gauge: Some(26),
            pressure_class: Some("2_in_wg".into()), construction: Some("spiral_lock".into()),
            exposure: Some("concealed".into()),
            flow_design: Some(185.0), flow_balanced: None,
            status: "new".into(), scope: "in_contract".into(),
        })?;
        segment_ids.push(branch_seg_id);

        prev_node_id = tap_node_id;
    }

    // Final trunk end node (dead end or cap)
    let end_node_id = generate_id();
    doc.add_node(&Node {
        id: end_node_id.clone(), system_id: sys_rtu1_sa.clone(),
        node_type: "cap".into(), placement_id: None,
        fitting_type: Some("end_cap".into()), size_description: Some("14\" round".into()),
        level: Some("Level 1".into()), x: Some(13.0), y: Some(13.0),
    })?;
    let final_seg_id = generate_id();
    doc.add_segment(&Segment {
        id: final_seg_id.clone(), system_id: sys_rtu1_sa.clone(),
        from_node_id: prev_node_id, to_node_id: end_node_id,
        shape: "round".into(), width_m: None, height_m: None,
        diameter_m: Some(14.0 * 0.0254), length_m: Some(0.75),
        material: "galvanized".into(), gauge: Some(26),
        pressure_class: Some("2_in_wg".into()), construction: Some("spiral_lock".into()),
        exposure: Some("concealed".into()),
        flow_design: Some(total_flow - (branch_taps.len() as f64 * branch_cfm)), flow_balanced: None,
        status: "new".into(), scope: "in_contract".into(),
    })?;
    segment_ids.push(final_seg_id);

    // =========================================================================
    // INSULATION — duct wrap on all new concealed supply segments
    // =========================================================================
    for seg_id in &segment_ids {
        doc.add_insulation(&Insulation {
            id: generate_id(),
            segment_id: Some(seg_id.clone()),
            insulation_type: "duct_wrap".into(),
            manufacturer: Some("CertainTeed".into()),
            product: Some("SoftTouch".into()),
            thickness_m: Some(0.038),
            r_value: Some(4.2),
            facing: Some("fsk".into()),
            code_reference: Some("CA Title 24".into()),
        })?;
    }

    // =========================================================================
    // SHEETS
    // =========================================================================
    let sheet_m001 = generate_id();
    let sheet_m101 = generate_id();
    let sheet_m102 = generate_id();
    let sheet_m103 = generate_id();
    let sheet_m104 = generate_id();
    let sheet_m105 = generate_id();

    let sheets_data = vec![
        (&sheet_m001, "M-001", "Mechanical Cover Sheet", "mechanical"),
        (&sheet_m101, "M-101", "Mechanical Ductwork Plan - Level 1", "mechanical"),
        (&sheet_m102, "M-102", "Mechanical Ductwork Plan - Level 2", "mechanical"),
        (&sheet_m103, "M-103", "Mechanical Ductwork Plan - Roof", "mechanical"),
        (&sheet_m104, "M-104", "Mechanical Details", "mechanical"),
        (&sheet_m105, "M-105", "Mechanical Schedules", "mechanical"),
    ];
    for (id, number, title, discipline) in &sheets_data {
        doc.add_sheet(&Sheet {
            id: (*id).clone(), number: number.to_string(), title: title.to_string(),
            discipline: discipline.to_string(), sheet_size: Some("ARCH D".into()),
        })?;
    }

    // =========================================================================
    // VIEWS
    // =========================================================================
    let plan_scale = "1/4\" = 1'-0\"";

    // M-001: title block
    doc.add_view(&View {
        id: generate_id(), sheet_id: sheet_m001.clone(),
        view_type: "title_block".into(), title: Some("Cover Sheet".into()),
        scale: None, level: None,
        vp_x: None, vp_y: None, vp_width: None, vp_height: None,
        model_x_min: None, model_y_min: None, model_x_max: None, model_y_max: None,
    })?;

    // M-101: Level 1 plan
    doc.add_view(&View {
        id: generate_id(), sheet_id: sheet_m101.clone(),
        view_type: "plan".into(), title: Some("Level 1 Mechanical Plan".into()),
        scale: Some(plan_scale.into()), level: Some("Level 1".into()),
        vp_x: Some(1.0), vp_y: Some(1.0), vp_width: Some(32.0), vp_height: Some(20.0),
        model_x_min: Some(-3.0), model_y_min: Some(-4.0),
        model_x_max: Some(18.0), model_y_max: Some(19.0),
    })?;

    // M-102: Level 2 plan
    doc.add_view(&View {
        id: generate_id(), sheet_id: sheet_m102.clone(),
        view_type: "plan".into(), title: Some("Level 2 Mechanical Plan".into()),
        scale: Some(plan_scale.into()), level: Some("Level 2".into()),
        vp_x: Some(1.0), vp_y: Some(1.0), vp_width: Some(32.0), vp_height: Some(20.0),
        model_x_min: Some(-3.0), model_y_min: Some(-4.0),
        model_x_max: Some(18.0), model_y_max: Some(19.0),
    })?;

    // M-103: Roof plan
    doc.add_view(&View {
        id: generate_id(), sheet_id: sheet_m103.clone(),
        view_type: "plan".into(), title: Some("Roof Mechanical Plan".into()),
        scale: Some(plan_scale.into()), level: Some("Roof".into()),
        vp_x: Some(1.0), vp_y: Some(1.0), vp_width: Some(32.0), vp_height: Some(20.0),
        model_x_min: Some(-3.0), model_y_min: Some(-4.0),
        model_x_max: Some(18.0), model_y_max: Some(19.0),
    })?;

    // M-104: details
    let details = vec![
        "Diffuser Installation Detail",
        "Inline Fan Installation Detail",
        "Linear Diffuser Mounting Detail",
        "Manual Damper Detail",
    ];
    for title in &details {
        doc.add_view(&View {
            id: generate_id(), sheet_id: sheet_m104.clone(),
            view_type: "detail".into(), title: Some(title.to_string()),
            scale: None, level: None,
            vp_x: None, vp_y: None, vp_width: None, vp_height: None,
            model_x_min: None, model_y_min: None, model_x_max: None, model_y_max: None,
        })?;
    }

    // M-105: schedule
    doc.add_view(&View {
        id: generate_id(), sheet_id: sheet_m105.clone(),
        view_type: "schedule".into(), title: Some("Equipment & Air Device Schedule".into()),
        scale: None, level: None,
        vp_x: None, vp_y: None, vp_width: None, vp_height: None,
        model_x_min: None, model_y_min: None, model_x_max: None, model_y_max: None,
    })?;

    // =========================================================================
    // GENERAL NOTES
    // =========================================================================
    let general_notes = vec![
        "All rectangular return air and supply air ductwork shall be lined with acoustical liner the first 15 feet.",
        "Flex duct shall be limited to 5'-0\" in length. No duct board allowed.",
        "All exposed ducts to be painted to match ceiling.",
        "HVAC contractor is required to visit the job site to become familiar with existing conditions.",
        "Coordinate all ductwork prior to fabrication. Install ductwork as high as possible.",
    ];
    for (i, text) in general_notes.iter().enumerate() {
        doc.add_general_note(&generate_id(), Some("mechanical"), text, (i + 1) as i32)?;
    }

    // =========================================================================
    // KEYED NOTES
    // =========================================================================
    let notes = vec![
        ("H1", "Refer to arch RCP for blank-off alignments with light fixtures & architectural elements."),
        ("H2", "Provide cable operated damper for MVD serving diffuser in inaccessible ceiling."),
        ("H3", "Provide birdscreen for return duct."),
        ("H4", "Install transfer grille as high as possible above ceiling."),
        ("H5", "Duct mounted smoke detector. Mechanical contractor shall install smoke detector in the supply air duct. Mechanical contractor shall provide wiring to fan interlock. E.C. shall provide wiring for connection to remote annunciator."),
        ("H6", "Provide new programmable thermostat in managers office with remote sensor in sales floor. Thermostats/sensors shall be same manufacturer as HVAC unit. Coordinate exact location with SKIMS project manager prior to installation. Sensors to be button style and SS finish."),
        ("H7", "Existing 24\"x42\" opening facing up on top of duct."),
        ("H8", "1\" door undercut."),
        ("H9", "Provide new inline exhaust fan. Balance to the scheduled airflow. Extend new exhaust ductwork to existing exhaust main. Field verify exact location prior to bid."),
        ("H10", "Existing rooftop unit to remain. Balance to the scheduled airflow. Clean and verify proper operation; clean cooling, heating coils, recharge refrigerant, replace belt, drive, and motor as required, replace filters. Check compressor and fans, replace/repair as required. Provide owner with reconditioning report prior to turnover. Field verify exact location and orientation prior to bid."),
        ("H11", "Existing exhaust roof penetrations and caps to remain. Field verify exact location prior to bid."),
    ];
    for (key, text) in &notes {
        doc.add_keyed_note(&KeyedNote {
            id: generate_id(), key: key.to_string(), text: text.to_string(),
            discipline: Some("mechanical".into()), spec_section: None,
        })?;
    }

    // =========================================================================
    // REVISIONS
    // =========================================================================
    doc.add_revision(&Revision {
        id: generate_id(), number: 1, name: "CD Issue".into(),
        date: "2025-12-04".into(), description: None, author: Some("KLH Engineers".into()),
    })?;
    doc.add_revision(&Revision {
        id: generate_id(), number: 2, name: "Bid Set".into(),
        date: "2026-01-20".into(), description: None, author: Some("KLH Engineers".into()),
    })?;

    // =========================================================================
    // GEOMETRY — assign real coordinates to spaces and placements
    // =========================================================================
    crate::geometry::populate_skims_geometry(&doc)?;

    Ok(())
}
