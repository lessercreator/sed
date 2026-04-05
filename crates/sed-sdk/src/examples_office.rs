use anyhow::Result;
use crate::document::{SedDocument, generate_id};
use crate::types::*;

/// Builds a 10-story Class A office building with full mechanical systems.
/// This is a stress test of the SED schema against a realistic commercial building.
///
/// Central plant: 2x 500-ton chillers, 2x boilers, pumps, cooling towers.
/// Air systems: 4 AHUs serving floors 2-10 and lobby, VAV boxes with reheat.
/// Hydronic: chilled water and hot water piping loops.
/// Exhaust: restroom, garage, kitchen, stairwell pressurization.
///
/// Floors B1 through 3 are fully detailed. Floors 4-10 are abbreviated.
pub fn create_office_tower(path: &str) -> Result<()> {
    let doc = SedDocument::create(path)?;

    // =========================================================================
    // META
    // =========================================================================
    doc.set_meta("sed_version", "0.3")?;
    doc.set_meta("project_name", "One Commerce Plaza")?;
    doc.set_meta("project_number", "26-400")?;
    doc.set_meta("project_address", "100 Commerce Drive, Austin, TX 78701")?;
    doc.set_meta("units_display", "imperial")?;
    doc.set_meta("created_at", "2026-04-04T00:00:00Z")?;
    doc.set_meta("modified_at", "2026-04-04T00:00:00Z")?;
    doc.set_meta("building_type", "Class A Office")?;
    doc.set_meta("stories_above_grade", "10")?;
    doc.set_meta("stories_below_grade", "1")?;
    doc.set_meta("gross_area_sf", "250000")?;

    // =========================================================================
    // DIRECTORY
    // =========================================================================
    let entries = vec![
        ("owner", "Commerce Partners LLC", Some("David Chen"), Some("dchen@commercepartners.com"), Some("512-555-0100")),
        ("architect", "HKS Architects", Some("Maria Santos"), Some("msantos@hks.com"), Some("214-555-0200")),
        ("engineer_mep", "WSP USA", Some("James Mitchell"), Some("james.mitchell@wsp.com"), Some("512-555-0300")),
        ("engineer_struct", "Thornton Tomasetti", Some("Sarah Kim"), Some("skim@thorntontomasetti.com"), Some("512-555-0400")),
        ("contractor_mech", "TDIndustries", Some("Robert Flores"), Some("rflores@tdindustries.com"), Some("512-555-0500")),
        ("contractor_controls", "Siemens Building Technologies", Some("Mike Patterson"), Some("mike.patterson@siemens.com"), Some("512-555-0600")),
        ("commissioning", "Cx Associates", Some("Jennifer Walsh"), Some("jwalsh@cxassociates.com"), Some("802-555-0700")),
    ];
    for (role, company, contact, email, phone) in entries {
        doc.add_directory_entry(&DirectoryEntry {
            id: generate_id(), role: role.into(), company: company.into(),
            contact: contact.map(Into::into), email: email.map(Into::into),
            phone: phone.map(Into::into), address: None,
        })?;
    }

    // =========================================================================
    // SUBMITTALS
    // =========================================================================
    let sub_chiller = generate_id();
    let sub_boiler = generate_id();
    let sub_ahu = generate_id();
    let sub_vav = generate_id();
    let sub_pump = generate_id();
    let sub_ct = generate_id();
    let sub_ef = generate_id();
    let sub_diffuser = generate_id();
    let sub_piping = generate_id();
    let sub_insul = generate_id();

    let submittals_data = vec![
        (&sub_chiller, "M-001", "Centrifugal Chillers - Trane CenTraVac CVHF", "Trane", "233516"),
        (&sub_boiler, "M-002", "Condensing Boilers - Aerco BMK 3.0", "Aerco", "235223"),
        (&sub_ahu, "M-003", "Air Handling Units - Trane IntelliPak", "Trane", "237323"),
        (&sub_vav, "M-004", "VAV Terminal Units - Trane", "Trane", "233600"),
        (&sub_pump, "M-005", "Hydronic Pumps - Bell & Gossett", "Bell & Gossett", "232123"),
        (&sub_ct, "M-006", "Cooling Towers - BAC Series 3000", "BAC", "236500"),
        (&sub_ef, "M-007", "Exhaust Fans - Greenheck", "Greenheck", "233423"),
        (&sub_diffuser, "M-008", "Air Distribution Devices - Titus", "Titus", "233713"),
        (&sub_piping, "M-009", "Hydronic Piping - Victaulic", "Victaulic", "232113"),
        (&sub_insul, "M-010", "Pipe & Duct Insulation - Owens Corning", "Owens Corning", "230713"),
    ];
    for (id, number, desc, company, spec) in &submittals_data {
        doc.add_submittal(&Submittal {
            id: (*id).clone(), number: Some(number.to_string()),
            description: desc.to_string(),
            submitted_by: Some("Robert Flores".into()), company: Some(company.to_string()),
            date_submitted: Some("2026-03-15".into()), status: "for_approval".into(),
            spec_section: Some(spec.to_string()),
        })?;
    }

    // =========================================================================
    // SPACES
    // =========================================================================
    let mut space_ids: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    // Helper to add a space and track its ID
    let mut add_space = |tag: &str, name: &str, level: &str, stype: &str, area: Option<f64>, cht: Option<f64>| -> Result<String> {
        let id = generate_id();
        space_ids.insert(tag.to_string(), id.clone());
        doc.add_space(&Space {
            id: id.clone(), tag: tag.to_string(), name: name.to_string(), level: level.to_string(),
            space_type: Some(stype.to_string()), area_m2: area, ceiling_ht_m: cht,
            scope: "in_contract".into(), parent_id: None, boundary_id: None,
            x: None, y: None,
        })?;
        Ok(id)
    };

    // --- Basement (B1) ---
    add_space("B1-001", "Mechanical Room", "Basement", "mechanical", Some(465.0), Some(4.5))?;
    add_space("B1-002", "Electrical Room", "Basement", "electrical", Some(185.0), Some(4.5))?;
    add_space("B1-003", "Parking Garage", "Basement", "garage", Some(3250.0), Some(3.0))?;
    add_space("B1-004", "Fire Pump Room", "Basement", "mechanical", Some(28.0), Some(4.5))?;
    add_space("B1-005", "Elevator Machine Room", "Basement", "mechanical", Some(37.0), Some(4.5))?;
    add_space("B1-006", "Stairwell A", "Basement", "circulation", Some(14.0), Some(4.5))?;
    add_space("B1-007", "Stairwell B", "Basement", "circulation", Some(14.0), Some(4.5))?;
    add_space("B1-008", "Storage", "Basement", "storage", Some(46.5), Some(3.0))?;

    // --- Floor 1 (Lobby) ---
    add_space("L1-001", "Main Lobby", "Level 1", "lobby", Some(370.0), Some(6.0))?;
    add_space("L1-002", "Retail A", "Level 1", "retail", Some(140.0), Some(4.0))?;
    add_space("L1-003", "Retail B", "Level 1", "retail", Some(140.0), Some(4.0))?;
    add_space("L1-004", "Loading Dock", "Level 1", "service", Some(93.0), Some(4.5))?;
    add_space("L1-005", "Mailroom", "Level 1", "service", Some(28.0), Some(3.0))?;
    add_space("L1-006", "Security Office", "Level 1", "office", Some(23.0), Some(3.0))?;
    add_space("L1-007", "Elevator Lobby", "Level 1", "circulation", Some(37.0), Some(3.6))?;
    add_space("L1-008", "Restroom M", "Level 1", "restroom", Some(23.0), Some(3.0))?;
    add_space("L1-009", "Restroom F", "Level 1", "restroom", Some(28.0), Some(3.0))?;
    add_space("L1-010", "Stairwell A", "Level 1", "circulation", Some(14.0), Some(4.0))?;
    add_space("L1-011", "Stairwell B", "Level 1", "circulation", Some(14.0), Some(4.0))?;
    add_space("L1-012", "Electrical Closet", "Level 1", "electrical", Some(9.3), Some(3.0))?;
    add_space("L1-013", "IDF Closet", "Level 1", "telecom", Some(7.0), Some(3.0))?;

    // --- Floors 2-10 (typical office floors) ---
    // Floors 2-3 fully detailed, 4-10 abbreviated
    for floor in 2..=10 {
        let level = format!("Level {}", floor);
        let prefix = format!("L{}", floor);

        add_space(&format!("{}-001", prefix), "Open Office", &level, "office", Some(1115.0), Some(2.75))?;
        add_space(&format!("{}-002", prefix), "Perimeter Office North", &level, "office", Some(23.0), Some(2.75))?;
        add_space(&format!("{}-003", prefix), "Perimeter Office South", &level, "office", Some(23.0), Some(2.75))?;
        add_space(&format!("{}-004", prefix), "Perimeter Office East", &level, "office", Some(23.0), Some(2.75))?;
        add_space(&format!("{}-005", prefix), "Perimeter Office West", &level, "office", Some(23.0), Some(2.75))?;
        add_space(&format!("{}-006", prefix), "Conference Room A", &level, "conference", Some(37.0), Some(2.75))?;
        add_space(&format!("{}-007", prefix), "Conference Room B", &level, "conference", Some(37.0), Some(2.75))?;
        add_space(&format!("{}-008", prefix), "Restroom M", &level, "restroom", Some(23.0), Some(2.75))?;
        add_space(&format!("{}-009", prefix), "Restroom F", &level, "restroom", Some(28.0), Some(2.75))?;
        add_space(&format!("{}-010", prefix), "Elevator Lobby", &level, "circulation", Some(28.0), Some(2.75))?;
        add_space(&format!("{}-011", prefix), "Electrical Closet", &level, "electrical", Some(9.3), Some(2.75))?;
        add_space(&format!("{}-012", prefix), "IDF Closet", &level, "telecom", Some(7.0), Some(2.75))?;

        // Floor 2 has a tenant cafe
        if floor == 2 {
            add_space(&format!("{}-013", prefix), "Tenant Cafe", &level, "food_service", Some(93.0), Some(2.75))?;
            add_space(&format!("{}-014", prefix), "Cafe Kitchen", &level, "kitchen", Some(37.0), Some(2.75))?;
        }
    }

    // Roof
    add_space("RF-001", "Roof Mechanical Area", "Roof", "mechanical", Some(465.0), None)?;

    // =========================================================================
    // PRODUCT TYPES — Central Plant Equipment
    // =========================================================================

    // --- Chillers ---
    let pt_chiller = generate_id();
    doc.add_product_type(&ProductType {
        id: pt_chiller.clone(), tag: "CH".into(), domain: "equipment".into(), category: "chiller".into(),
        manufacturer: Some("Trane".into()), model: Some("CenTraVac CVHF 500".into()),
        description: Some("500-ton centrifugal chiller, R-134a, VFD, 0.55 kW/ton NPLV".into()),
        mounting: Some("floor".into()), finish: None, size_nominal: Some("500 ton".into()),
        voltage: Some(460.0), phase: Some(3), hz: Some(60.0), submittal_id: Some(sub_chiller.clone()),
    })?;

    // --- Boilers ---
    let pt_boiler = generate_id();
    doc.add_product_type(&ProductType {
        id: pt_boiler.clone(), tag: "B".into(), domain: "equipment".into(), category: "boiler".into(),
        manufacturer: Some("Aerco".into()), model: Some("BMK 3.0".into()),
        description: Some("3,000 MBH condensing boiler, 96% thermal efficiency, natural gas".into()),
        mounting: Some("floor".into()), finish: None, size_nominal: Some("3000 MBH".into()),
        voltage: Some(120.0), phase: Some(1), hz: Some(60.0), submittal_id: Some(sub_boiler.clone()),
    })?;

    // --- Cooling Towers ---
    let pt_ct = generate_id();
    doc.add_product_type(&ProductType {
        id: pt_ct.clone(), tag: "CT".into(), domain: "equipment".into(), category: "cooling_tower".into(),
        manufacturer: Some("BAC".into()), model: Some("Series 3000 VXI".into()),
        description: Some("500-ton induced draft counterflow cooling tower, VFD fan".into()),
        mounting: Some("floor".into()), finish: None, size_nominal: Some("500 ton".into()),
        voltage: Some(460.0), phase: Some(3), hz: Some(60.0), submittal_id: Some(sub_ct.clone()),
    })?;

    // --- Primary CHW Pumps (constant volume) ---
    let pt_chwp_pri = generate_id();
    doc.add_product_type(&ProductType {
        id: pt_chwp_pri.clone(), tag: "CHWP-PRI".into(), domain: "equipment".into(), category: "pump".into(),
        manufacturer: Some("Bell & Gossett".into()), model: Some("e-1510 10BC".into()),
        description: Some("Primary CHW pump, 1200 GPM at 50 ft HD, 25 HP, constant volume".into()),
        mounting: Some("floor".into()), finish: None, size_nominal: Some("10\" x 8\"".into()),
        voltage: Some(460.0), phase: Some(3), hz: Some(60.0), submittal_id: Some(sub_pump.clone()),
    })?;

    // --- Secondary CHW Pumps (variable speed) ---
    let pt_chwp_sec = generate_id();
    doc.add_product_type(&ProductType {
        id: pt_chwp_sec.clone(), tag: "CHWP-SEC".into(), domain: "equipment".into(), category: "pump".into(),
        manufacturer: Some("Bell & Gossett".into()), model: Some("e-1510 10BC".into()),
        description: Some("Secondary CHW pump, 1200 GPM at 80 ft HD, 40 HP, VFD".into()),
        mounting: Some("floor".into()), finish: None, size_nominal: Some("10\" x 8\"".into()),
        voltage: Some(460.0), phase: Some(3), hz: Some(60.0), submittal_id: Some(sub_pump.clone()),
    })?;

    // --- Condenser Water Pumps ---
    let pt_cwp = generate_id();
    doc.add_product_type(&ProductType {
        id: pt_cwp.clone(), tag: "CWP".into(), domain: "equipment".into(), category: "pump".into(),
        manufacturer: Some("Bell & Gossett".into()), model: Some("e-1510 12AD".into()),
        description: Some("Condenser water pump, 1500 GPM at 45 ft HD, 30 HP".into()),
        mounting: Some("floor".into()), finish: None, size_nominal: Some("12\" x 10\"".into()),
        voltage: Some(460.0), phase: Some(3), hz: Some(60.0), submittal_id: Some(sub_pump.clone()),
    })?;

    // --- Hot Water Pumps ---
    let pt_hwp = generate_id();
    doc.add_product_type(&ProductType {
        id: pt_hwp.clone(), tag: "HWP".into(), domain: "equipment".into(), category: "pump".into(),
        manufacturer: Some("Bell & Gossett".into()), model: Some("e-1510 6BD".into()),
        description: Some("Hot water pump, 400 GPM at 60 ft HD, 15 HP, VFD".into()),
        mounting: Some("floor".into()), finish: None, size_nominal: Some("6\" x 5\"".into()),
        voltage: Some(460.0), phase: Some(3), hz: Some(60.0), submittal_id: Some(sub_pump.clone()),
    })?;

    // --- AHUs ---
    let pt_ahu_large = generate_id();
    doc.add_product_type(&ProductType {
        id: pt_ahu_large.clone(), tag: "AHU-OFFICE".into(), domain: "equipment".into(), category: "ahu".into(),
        manufacturer: Some("Trane".into()), model: Some("IntelliPak SHC".into()),
        description: Some("Custom AHU, draw-through, CHW coil, HW preheat, MERV 13/MERV 8, VFD supply & return fans, energy recovery wheel".into()),
        mounting: Some("floor".into()), finish: None, size_nominal: None,
        voltage: Some(460.0), phase: Some(3), hz: Some(60.0), submittal_id: Some(sub_ahu.clone()),
    })?;

    let pt_ahu_lobby = generate_id();
    doc.add_product_type(&ProductType {
        id: pt_ahu_lobby.clone(), tag: "AHU-LOBBY".into(), domain: "equipment".into(), category: "ahu".into(),
        manufacturer: Some("Trane".into()), model: Some("IntelliPak SHC".into()),
        description: Some("Lobby AHU, draw-through, CHW coil, HW preheat, MERV 13/MERV 8, VFD supply fan".into()),
        mounting: Some("floor".into()), finish: None, size_nominal: None,
        voltage: Some(460.0), phase: Some(3), hz: Some(60.0), submittal_id: Some(sub_ahu.clone()),
    })?;

    // --- VAV Boxes ---
    let pt_vav_reheat = generate_id();
    doc.add_product_type(&ProductType {
        id: pt_vav_reheat.clone(), tag: "VAV-RH".into(), domain: "equipment".into(), category: "vav_box".into(),
        manufacturer: Some("Trane".into()), model: None, /* model depends on size */
        description: Some("Single duct VAV box with hot water reheat coil, DDC controller".into()),
        mounting: Some("suspended".into()), finish: None, size_nominal: None,
        voltage: Some(120.0), phase: Some(1), hz: Some(60.0), submittal_id: Some(sub_vav.clone()),
    })?;
    // SCHEMA_GAP: VAV boxes come in many inlet sizes (8", 10", 12", 14", 16"). The product_type
    // has no structured field for inlet_size or capacity_range. We encode it in size_nominal
    // but a single product_type covers all sizes in this example. In reality each inlet size
    // would be a separate product_type or the schema needs an inlet_size/capacity field.

    let pt_vav_cool = generate_id();
    doc.add_product_type(&ProductType {
        id: pt_vav_cool.clone(), tag: "VAV-CO".into(), domain: "equipment".into(), category: "vav_box".into(),
        manufacturer: Some("Trane".into()), model: None, /* cooling only */
        description: Some("Single duct VAV box, cooling only, DDC controller".into()),
        mounting: Some("suspended".into()), finish: None, size_nominal: None,
        voltage: Some(120.0), phase: Some(1), hz: Some(60.0), submittal_id: Some(sub_vav.clone()),
    })?;

    // --- Exhaust Fans ---
    let pt_ef_restroom = generate_id();
    doc.add_product_type(&ProductType {
        id: pt_ef_restroom.clone(), tag: "EF-REST".into(), domain: "equipment".into(), category: "exhaust_fan".into(),
        manufacturer: Some("Greenheck".into()), model: Some("SQ-130-VG".into()),
        description: Some("Centrifugal roof exhaust fan, 2000 CFM at 1.0\" wg, VFD".into()),
        mounting: Some("roof".into()), finish: None, size_nominal: None,
        voltage: Some(460.0), phase: Some(3), hz: Some(60.0), submittal_id: Some(sub_ef.clone()),
    })?;

    let pt_ef_garage = generate_id();
    doc.add_product_type(&ProductType {
        id: pt_ef_garage.clone(), tag: "EF-GAR".into(), domain: "equipment".into(), category: "exhaust_fan".into(),
        manufacturer: Some("Greenheck".into()), model: Some("SQ-200-VG".into()),
        description: Some("Garage exhaust fan, 10000 CFM at 1.5\" wg, VFD, CO interlock".into()),
        mounting: Some("roof".into()), finish: None, size_nominal: None,
        voltage: Some(460.0), phase: Some(3), hz: Some(60.0), submittal_id: Some(sub_ef.clone()),
    })?;

    let pt_ef_kitchen = generate_id();
    doc.add_product_type(&ProductType {
        id: pt_ef_kitchen.clone(), tag: "EF-KIT".into(), domain: "equipment".into(), category: "exhaust_fan".into(),
        manufacturer: Some("Greenheck".into()), model: Some("CUBE-180".into()),
        description: Some("Kitchen exhaust fan, 3000 CFM at 1.25\" wg, grease rated".into()),
        mounting: Some("roof".into()), finish: None, size_nominal: None,
        voltage: Some(460.0), phase: Some(3), hz: Some(60.0), submittal_id: Some(sub_ef.clone()),
    })?;

    // --- Stairwell Pressurization Fans ---
    let pt_spf = generate_id();
    doc.add_product_type(&ProductType {
        id: pt_spf.clone(), tag: "SPF".into(), domain: "equipment".into(), category: "pressurization_fan".into(),
        manufacturer: Some("Greenheck".into()), model: Some("BISW-18".into()),
        description: Some("Stairwell pressurization fan, 4500 CFM at 1.5\" wg".into()),
        mounting: Some("roof".into()), finish: None, size_nominal: None,
        voltage: Some(460.0), phase: Some(3), hz: Some(60.0), submittal_id: Some(sub_ef.clone()),
    })?;

    // --- Air Distribution Devices ---
    let pt_sd_ceiling = generate_id();
    doc.add_product_type(&ProductType {
        id: pt_sd_ceiling.clone(), tag: "SD-1".into(), domain: "air_device".into(), category: "supply_diffuser".into(),
        manufacturer: Some("Titus".into()), model: Some("TMS".into()),
        description: Some("Square cone ceiling diffuser, 24\"x24\" lay-in, insulated plenum".into()),
        mounting: Some("lay-in".into()), finish: Some("standard white".into()), size_nominal: Some("24\"x24\"".into()),
        voltage: None, phase: None, hz: None, submittal_id: Some(sub_diffuser.clone()),
    })?;

    let pt_sd_linear = generate_id();
    doc.add_product_type(&ProductType {
        id: pt_sd_linear.clone(), tag: "SD-2".into(), domain: "air_device".into(), category: "supply_diffuser".into(),
        manufacturer: Some("Titus".into()), model: Some("ML-40".into()),
        description: Some("Linear slot diffuser, 2-slot, 4' active length, perimeter heating mode".into()),
        mounting: Some("ceiling".into()), finish: Some("anodized aluminum".into()), size_nominal: None,
        voltage: None, phase: None, hz: None, submittal_id: Some(sub_diffuser.clone()),
    })?;

    let pt_rg = generate_id();
    doc.add_product_type(&ProductType {
        id: pt_rg.clone(), tag: "RG-1".into(), domain: "air_device".into(), category: "return_grille".into(),
        manufacturer: Some("Titus".into()), model: Some("350RL".into()),
        description: Some("Return air grille, 24\"x24\" lay-in, hinged".into()),
        mounting: Some("lay-in".into()), finish: Some("standard white".into()), size_nominal: Some("24\"x24\"".into()),
        voltage: None, phase: None, hz: None, submittal_id: Some(sub_diffuser.clone()),
    })?;

    let pt_eg = generate_id();
    doc.add_product_type(&ProductType {
        id: pt_eg.clone(), tag: "EG-1".into(), domain: "air_device".into(), category: "exhaust_grille".into(),
        manufacturer: Some("Titus".into()), model: Some("350RL".into()),
        description: Some("Exhaust grille, 12\"x12\", surface mount".into()),
        mounting: Some("surface".into()), finish: Some("standard white".into()), size_nominal: Some("12\"x12\"".into()),
        voltage: None, phase: None, hz: None, submittal_id: Some(sub_diffuser.clone()),
    })?;

    // =========================================================================
    // PLACEMENTS — Central Plant (Basement Mechanical Room)
    // =========================================================================
    let mech_room_id = space_ids.get("B1-001").cloned();

    // Chillers
    let ch1_id = generate_id();
    doc.add_placement(&Placement {
        id: ch1_id.clone(), instance_tag: Some("CH-1".into()),
        product_type_id: pt_chiller.clone(), space_id: mech_room_id.clone(),
        level: "Basement".into(), x: Some(10.0), y: Some(5.0), rotation: None,
        cfm: None, cfm_balanced: None, static_pressure_pa: None,
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: Some(6800.0),        notes: Some("Lead chiller. 500 ton, 0.55 kW/ton NPLV. Requires 3\" CHW connections, 4\" CW connections.".into()),
    })?;

    let ch2_id = generate_id();
    doc.add_placement(&Placement {
        id: ch2_id.clone(), instance_tag: Some("CH-2".into()),
        product_type_id: pt_chiller.clone(), space_id: mech_room_id.clone(),
        level: "Basement".into(), x: Some(10.0), y: Some(12.0), rotation: None,
        cfm: None, cfm_balanced: None, static_pressure_pa: None,
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: Some(6800.0),        notes: Some("Lag chiller. Identical to CH-1.".into()),
    })?;

    // Boilers
    let b1_id = generate_id();
    doc.add_placement(&Placement {
        id: b1_id.clone(), instance_tag: Some("B-1".into()),
        product_type_id: pt_boiler.clone(), space_id: mech_room_id.clone(),
        level: "Basement".into(), x: Some(25.0), y: Some(5.0), rotation: None,
        cfm: None, cfm_balanced: None, static_pressure_pa: None,
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: Some(1360.0),        notes: Some("Lead boiler. 3,000 MBH input, 96% efficiency. 2\" HW connections.".into()),
    })?;

    let b2_id = generate_id();
    doc.add_placement(&Placement {
        id: b2_id.clone(), instance_tag: Some("B-2".into()),
        product_type_id: pt_boiler.clone(), space_id: mech_room_id.clone(),
        level: "Basement".into(), x: Some(25.0), y: Some(12.0), rotation: None,
        cfm: None, cfm_balanced: None, static_pressure_pa: None,
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: Some(1360.0),        notes: Some("Lag boiler. Identical to B-1.".into()),
    })?;

    // Primary CHW Pumps
    let chwp1_id = generate_id();
    doc.add_placement(&Placement {
        id: chwp1_id.clone(), instance_tag: Some("CHWP-P1".into()),
        product_type_id: pt_chwp_pri.clone(), space_id: mech_room_id.clone(),
        level: "Basement".into(), x: Some(5.0), y: Some(5.0), rotation: None,
        cfm: None, cfm_balanced: None, static_pressure_pa: None,
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: Some(340.0),        notes: Some("Primary CHW pump 1, 1200 GPM, 25 HP, dedicated to CH-1.".into()),
    })?;

    let chwp2_id = generate_id();
    doc.add_placement(&Placement {
        id: chwp2_id.clone(), instance_tag: Some("CHWP-P2".into()),
        product_type_id: pt_chwp_pri.clone(), space_id: mech_room_id.clone(),
        level: "Basement".into(), x: Some(5.0), y: Some(12.0), rotation: None,
        cfm: None, cfm_balanced: None, static_pressure_pa: None,
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: Some(340.0),        notes: Some("Primary CHW pump 2, 1200 GPM, 25 HP, dedicated to CH-2.".into()),
    })?;

    // Secondary CHW Pumps
    let chwp_s1_id = generate_id();
    doc.add_placement(&Placement {
        id: chwp_s1_id.clone(), instance_tag: Some("CHWP-S1".into()),
        product_type_id: pt_chwp_sec.clone(), space_id: mech_room_id.clone(),
        level: "Basement".into(), x: Some(3.0), y: Some(5.0), rotation: None,
        cfm: None, cfm_balanced: None, static_pressure_pa: None,
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: Some(410.0),        notes: Some("Secondary CHW pump 1, 1200 GPM, 40 HP, VFD.".into()),
    })?;

    let chwp_s2_id = generate_id();
    doc.add_placement(&Placement {
        id: chwp_s2_id.clone(), instance_tag: Some("CHWP-S2".into()),
        product_type_id: pt_chwp_sec.clone(), space_id: mech_room_id.clone(),
        level: "Basement".into(), x: Some(3.0), y: Some(12.0), rotation: None,
        cfm: None, cfm_balanced: None, static_pressure_pa: None,
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: Some(410.0),        notes: Some("Secondary CHW pump 2, 1200 GPM, 40 HP, VFD.".into()),
    })?;

    // Condenser Water Pumps
    let cwp1_id = generate_id();
    doc.add_placement(&Placement {
        id: cwp1_id.clone(), instance_tag: Some("CWP-1".into()),
        product_type_id: pt_cwp.clone(), space_id: mech_room_id.clone(),
        level: "Basement".into(), x: Some(15.0), y: Some(5.0), rotation: None,
        cfm: None, cfm_balanced: None, static_pressure_pa: None,
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: Some(454.0),        notes: Some("Condenser water pump 1, 1500 GPM, 30 HP, dedicated to CH-1.".into()),
    })?;

    let cwp2_id = generate_id();
    doc.add_placement(&Placement {
        id: cwp2_id.clone(), instance_tag: Some("CWP-2".into()),
        product_type_id: pt_cwp.clone(), space_id: mech_room_id.clone(),
        level: "Basement".into(), x: Some(15.0), y: Some(12.0), rotation: None,
        cfm: None, cfm_balanced: None, static_pressure_pa: None,
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: Some(454.0),        notes: Some("Condenser water pump 2, 1500 GPM, 30 HP, dedicated to CH-2.".into()),
    })?;

    // Hot Water Pumps
    let hwp1_id = generate_id();
    doc.add_placement(&Placement {
        id: hwp1_id.clone(), instance_tag: Some("HWP-1".into()),
        product_type_id: pt_hwp.clone(), space_id: mech_room_id.clone(),
        level: "Basement".into(), x: Some(28.0), y: Some(5.0), rotation: None,
        cfm: None, cfm_balanced: None, static_pressure_pa: None,
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: Some(180.0),        notes: Some("Hot water pump 1, 400 GPM, 15 HP, VFD.".into()),
    })?;

    let hwp2_id = generate_id();
    doc.add_placement(&Placement {
        id: hwp2_id.clone(), instance_tag: Some("HWP-2".into()),
        product_type_id: pt_hwp.clone(), space_id: mech_room_id.clone(),
        level: "Basement".into(), x: Some(28.0), y: Some(12.0), rotation: None,
        cfm: None, cfm_balanced: None, static_pressure_pa: None,
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: Some(180.0),        notes: Some("Hot water pump 2, 400 GPM, 15 HP, VFD.".into()),
    })?;

    // Cooling Towers (Roof)
    let ct1_id = generate_id();
    doc.add_placement(&Placement {
        id: ct1_id.clone(), instance_tag: Some("CT-1".into()),
        product_type_id: pt_ct.clone(), space_id: space_ids.get("RF-001").cloned(),
        level: "Roof".into(), x: Some(10.0), y: Some(5.0), rotation: None,
        cfm: None, cfm_balanced: None, static_pressure_pa: None,
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: Some(5440.0),        notes: Some("Cooling tower 1, 500 ton, VFD fan. Basin heater for freeze protection.".into()),
    })?;

    let ct2_id = generate_id();
    doc.add_placement(&Placement {
        id: ct2_id.clone(), instance_tag: Some("CT-2".into()),
        product_type_id: pt_ct.clone(), space_id: space_ids.get("RF-001").cloned(),
        level: "Roof".into(), x: Some(20.0), y: Some(5.0), rotation: None,
        cfm: None, cfm_balanced: None, static_pressure_pa: None,
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: Some(5440.0),        notes: Some("Cooling tower 2, 500 ton, VFD fan.".into()),
    })?;

    // =========================================================================
    // PLACEMENTS — Air Handling Units (Basement Mechanical Room)
    // =========================================================================
    // AHU-1: 25,000 CFM, serves floors 2-4
    // AHU-2: 25,000 CFM, serves floors 5-7
    // AHU-3: 25,000 CFM, serves floors 8-10
    // AHU-4: 8,000 CFM, serves lobby (floor 1)

    let ahu1_id = generate_id();
    doc.add_placement(&Placement {
        id: ahu1_id.clone(), instance_tag: Some("AHU-1".into()),
        product_type_id: pt_ahu_large.clone(), space_id: mech_room_id.clone(),
        level: "Basement".into(), x: Some(35.0), y: Some(3.0), rotation: None,
        cfm: Some(25000.0), cfm_balanced: None, static_pressure_pa: Some(1000.0),
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: Some(3630.0),        notes: Some("AHU-1 serves floors 2-4. 25,000 CFM, 4\" wg ESP. CHW coil: 72 ton. HW preheat coil: 600 MBH.".into()),
    })?;

    let ahu2_id = generate_id();
    doc.add_placement(&Placement {
        id: ahu2_id.clone(), instance_tag: Some("AHU-2".into()),
        product_type_id: pt_ahu_large.clone(), space_id: mech_room_id.clone(),
        level: "Basement".into(), x: Some(35.0), y: Some(8.0), rotation: None,
        cfm: Some(25000.0), cfm_balanced: None, static_pressure_pa: Some(1120.0),
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: Some(3630.0),        notes: Some("AHU-2 serves floors 5-7. 25,000 CFM, 4.5\" wg ESP. Higher static for longer riser.".into()),
    })?;

    let ahu3_id = generate_id();
    doc.add_placement(&Placement {
        id: ahu3_id.clone(), instance_tag: Some("AHU-3".into()),
        product_type_id: pt_ahu_large.clone(), space_id: mech_room_id.clone(),
        level: "Basement".into(), x: Some(35.0), y: Some(13.0), rotation: None,
        cfm: Some(25000.0), cfm_balanced: None, static_pressure_pa: Some(1245.0),
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: Some(3630.0),        notes: Some("AHU-3 serves floors 8-10. 25,000 CFM, 5\" wg ESP. Highest static for top floors.".into()),
    })?;

    let ahu4_id = generate_id();
    doc.add_placement(&Placement {
        id: ahu4_id.clone(), instance_tag: Some("AHU-4".into()),
        product_type_id: pt_ahu_lobby.clone(), space_id: mech_room_id.clone(),
        level: "Basement".into(), x: Some(40.0), y: Some(3.0), rotation: None,
        cfm: Some(8000.0), cfm_balanced: None, static_pressure_pa: Some(625.0),
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: Some(1810.0),        notes: Some("AHU-4 serves lobby. 8,000 CFM, 2.5\" wg ESP. CHW coil: 25 ton. HW preheat: 200 MBH.".into()),
    })?;

    // =========================================================================
    // PLACEMENTS — Exhaust Fans (Roof)
    // =========================================================================
    let ef1_id = generate_id();
    doc.add_placement(&Placement {
        id: ef1_id.clone(), instance_tag: Some("EF-1".into()),
        product_type_id: pt_ef_restroom.clone(), space_id: space_ids.get("RF-001").cloned(),
        level: "Roof".into(), x: Some(30.0), y: Some(5.0), rotation: None,
        cfm: Some(2000.0), cfm_balanced: None, static_pressure_pa: Some(250.0),
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: Some(68.0),        notes: Some("Restroom exhaust fan, core A. Serves M/F restrooms floors 1-5.".into()),
    })?;

    let ef2_id = generate_id();
    doc.add_placement(&Placement {
        id: ef2_id.clone(), instance_tag: Some("EF-2".into()),
        product_type_id: pt_ef_restroom.clone(), space_id: space_ids.get("RF-001").cloned(),
        level: "Roof".into(), x: Some(32.0), y: Some(5.0), rotation: None,
        cfm: Some(2000.0), cfm_balanced: None, static_pressure_pa: Some(250.0),
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: Some(68.0),        notes: Some("Restroom exhaust fan, core B. Serves M/F restrooms floors 6-10.".into()),
    })?;

    let ef3_id = generate_id();
    doc.add_placement(&Placement {
        id: ef3_id.clone(), instance_tag: Some("EF-3".into()),
        product_type_id: pt_ef_garage.clone(), space_id: space_ids.get("RF-001").cloned(),
        level: "Roof".into(), x: Some(34.0), y: Some(5.0), rotation: None,
        cfm: Some(10000.0), cfm_balanced: None, static_pressure_pa: Some(375.0),
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: Some(181.0),        notes: Some("Garage exhaust fan. CO sensor interlock. Basement parking.".into()),
    })?;

    let ef4_id = generate_id();
    doc.add_placement(&Placement {
        id: ef4_id.clone(), instance_tag: Some("EF-4".into()),
        product_type_id: pt_ef_kitchen.clone(), space_id: space_ids.get("RF-001").cloned(),
        level: "Roof".into(), x: Some(36.0), y: Some(5.0), rotation: None,
        cfm: Some(3000.0), cfm_balanced: None, static_pressure_pa: Some(310.0),
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: Some(136.0),        notes: Some("Kitchen exhaust for floor 2 tenant cafe.".into()),
    })?;

    // Stairwell pressurization fans
    let spf1_id = generate_id();
    doc.add_placement(&Placement {
        id: spf1_id.clone(), instance_tag: Some("SPF-1".into()),
        product_type_id: pt_spf.clone(), space_id: space_ids.get("RF-001").cloned(),
        level: "Roof".into(), x: Some(38.0), y: Some(5.0), rotation: None,
        cfm: Some(4500.0), cfm_balanced: None, static_pressure_pa: Some(375.0),
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: Some(113.0),        notes: Some("Stairwell A pressurization fan. Fire/life safety.".into()),
    })?;

    let spf2_id = generate_id();
    doc.add_placement(&Placement {
        id: spf2_id.clone(), instance_tag: Some("SPF-2".into()),
        product_type_id: pt_spf.clone(), space_id: space_ids.get("RF-001").cloned(),
        level: "Roof".into(), x: Some(40.0), y: Some(5.0), rotation: None,
        cfm: Some(4500.0), cfm_balanced: None, static_pressure_pa: Some(375.0),
        status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
        weight_kg: Some(113.0),        notes: Some("Stairwell B pressurization fan. Fire/life safety.".into()),
    })?;

    // =========================================================================
    // PLACEMENTS — VAV Boxes, Floors 2-3 (fully detailed)
    // =========================================================================
    // Each floor: 8 perimeter VAV-RH + 4 interior VAV-CO = 12 per floor
    // Perimeter zones: N, S, E, W offices + 4 more open-office perimeter segments
    // Interior zones: 4 quadrants of open office

    // Typical perimeter VAV: 1200 CFM max, 480 CFM min (40%)
    // Typical interior VAV: 1600 CFM max, 640 CFM min (40%)
    // Typical conference room VAV-RH: 600 CFM max, 240 CFM min
    // Floor total: 8*1200 + 4*1600 = 16,000 CFM (with diversity ~0.85 gets to ~13,600)
    // But we have 3 floors per AHU at 25,000 CFM = ~8,333 CFM/floor design
    // Adjusted: perimeter 800 CFM, interior 1000 CFM, conference 500 CFM
    // 8*800 + 4*1000 = 10,400 CFM -- ~8,333 CFM with 0.80 diversity factor

    // SCHEMA_GAP: A VAV box with hot water reheat needs to belong to TWO systems: the air
    // supply system (from the AHU) AND the hot water system (for the reheat coil). The
    // current schema only has a single system_id per placement. We assign it to the air
    // system here since that's the primary relationship. The hot water connection must be
    // inferred from the piping graph (where a node references this placement_id).

    struct VavSpec {
        tag: &'static str,
        space_tag: &'static str,
        cfm: f64,
        reheat: bool,
        zone_desc: &'static str,
    }

    let floor_2_vavs = vec![
        VavSpec { tag: "VAV-2-01", space_tag: "L2-002", cfm: 800.0, reheat: true, zone_desc: "Perimeter North" },
        VavSpec { tag: "VAV-2-02", space_tag: "L2-003", cfm: 800.0, reheat: true, zone_desc: "Perimeter South" },
        VavSpec { tag: "VAV-2-03", space_tag: "L2-004", cfm: 800.0, reheat: true, zone_desc: "Perimeter East" },
        VavSpec { tag: "VAV-2-04", space_tag: "L2-005", cfm: 800.0, reheat: true, zone_desc: "Perimeter West" },
        VavSpec { tag: "VAV-2-05", space_tag: "L2-001", cfm: 800.0, reheat: true, zone_desc: "Perimeter NE open office" },
        VavSpec { tag: "VAV-2-06", space_tag: "L2-001", cfm: 800.0, reheat: true, zone_desc: "Perimeter NW open office" },
        VavSpec { tag: "VAV-2-07", space_tag: "L2-001", cfm: 800.0, reheat: true, zone_desc: "Perimeter SE open office" },
        VavSpec { tag: "VAV-2-08", space_tag: "L2-001", cfm: 800.0, reheat: true, zone_desc: "Perimeter SW open office" },
        VavSpec { tag: "VAV-2-09", space_tag: "L2-001", cfm: 1000.0, reheat: false, zone_desc: "Interior NE" },
        VavSpec { tag: "VAV-2-10", space_tag: "L2-001", cfm: 1000.0, reheat: false, zone_desc: "Interior NW" },
        VavSpec { tag: "VAV-2-11", space_tag: "L2-001", cfm: 1000.0, reheat: false, zone_desc: "Interior SE" },
        VavSpec { tag: "VAV-2-12", space_tag: "L2-001", cfm: 1000.0, reheat: false, zone_desc: "Interior SW" },
    ];

    let floor_3_vavs = vec![
        VavSpec { tag: "VAV-3-01", space_tag: "L3-002", cfm: 800.0, reheat: true, zone_desc: "Perimeter North" },
        VavSpec { tag: "VAV-3-02", space_tag: "L3-003", cfm: 800.0, reheat: true, zone_desc: "Perimeter South" },
        VavSpec { tag: "VAV-3-03", space_tag: "L3-004", cfm: 800.0, reheat: true, zone_desc: "Perimeter East" },
        VavSpec { tag: "VAV-3-04", space_tag: "L3-005", cfm: 800.0, reheat: true, zone_desc: "Perimeter West" },
        VavSpec { tag: "VAV-3-05", space_tag: "L3-001", cfm: 800.0, reheat: true, zone_desc: "Perimeter NE open office" },
        VavSpec { tag: "VAV-3-06", space_tag: "L3-001", cfm: 800.0, reheat: true, zone_desc: "Perimeter NW open office" },
        VavSpec { tag: "VAV-3-07", space_tag: "L3-001", cfm: 800.0, reheat: true, zone_desc: "Perimeter SE open office" },
        VavSpec { tag: "VAV-3-08", space_tag: "L3-001", cfm: 800.0, reheat: true, zone_desc: "Perimeter SW open office" },
        VavSpec { tag: "VAV-3-09", space_tag: "L3-001", cfm: 1000.0, reheat: false, zone_desc: "Interior NE" },
        VavSpec { tag: "VAV-3-10", space_tag: "L3-001", cfm: 1000.0, reheat: false, zone_desc: "Interior NW" },
        VavSpec { tag: "VAV-3-11", space_tag: "L3-001", cfm: 1000.0, reheat: false, zone_desc: "Interior SE" },
        VavSpec { tag: "VAV-3-12", space_tag: "L3-001", cfm: 1000.0, reheat: false, zone_desc: "Interior SW" },
    ];

    // Place VAV boxes for floors 2-3 with full detail
    let mut vav_placement_ids: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    for vav in floor_2_vavs.iter().chain(floor_3_vavs.iter()) {
        let floor_num: u32 = vav.tag.chars().nth(4).unwrap().to_digit(10).unwrap();
        let level = format!("Level {}", floor_num);
        let pt = if vav.reheat { &pt_vav_reheat } else { &pt_vav_cool };
        let pid = generate_id();
        vav_placement_ids.insert(vav.tag.to_string(), pid.clone());
        doc.add_placement(&Placement {
            id: pid, instance_tag: Some(vav.tag.to_string()),
            product_type_id: pt.clone(),
            space_id: space_ids.get(vav.space_tag).cloned(),
            level, x: None, y: None, rotation: None,
            cfm: Some(vav.cfm), cfm_balanced: None, static_pressure_pa: None,
            status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
            weight_kg: None,            notes: Some(format!("{}. Min CFM: {:.0}.", vav.zone_desc, vav.cfm * 0.4)),
        })?;
    }

    // Abbreviated VAV placements for floors 4-10
    for floor in 4..=10u32 {
        let level = format!("Level {}", floor);
        let prefix = format!("L{}", floor);
        // 8 perimeter reheat + 4 interior cooling-only
        for zone in 1..=12u32 {
            let tag = format!("VAV-{}-{:02}", floor, zone);
            let reheat = zone <= 8;
            let pt = if reheat { &pt_vav_reheat } else { &pt_vav_cool };
            let cfm = if reheat { 800.0 } else { 1000.0 };
            let space_tag = if zone <= 4 {
                format!("{}-{:03}", prefix, zone + 1) // perimeter offices
            } else {
                format!("{}-001", prefix) // open office
            };
            let pid = generate_id();
            vav_placement_ids.insert(tag.clone(), pid.clone());
            doc.add_placement(&Placement {
                id: pid, instance_tag: Some(tag),
                product_type_id: pt.clone(),
                space_id: space_ids.get(&space_tag).cloned(),
                level: level.clone(), x: None, y: None, rotation: None,
                cfm: Some(cfm), cfm_balanced: None, static_pressure_pa: None,
                status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
                weight_kg: None, notes: None,
            })?;
        }
    }

    // =========================================================================
    // PLACEMENTS — Supply Diffusers & Return Grilles (Floors 2-3 detailed)
    // =========================================================================
    // Each VAV feeds ~3-4 diffusers. We place a representative set.
    for floor in 2..=3u32 {
        let level = format!("Level {}", floor);
        let prefix = format!("L{}", floor);

        // Supply diffusers in open office (ceiling type)
        for i in 1..=20u32 {
            let space_tag = format!("{}-001", prefix);
            doc.add_placement(&Placement {
                id: generate_id(), instance_tag: None,
                product_type_id: pt_sd_ceiling.clone(),
                space_id: space_ids.get(&space_tag).cloned(),
                level: level.clone(), x: None, y: None, rotation: None,
                cfm: Some(250.0), cfm_balanced: None, static_pressure_pa: None,
                status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
                weight_kg: None, notes: None,
            })?;
            let _ = i; // suppress warning
        }

        // Linear slot diffusers at perimeter (2 per office)
        for office_num in 2..=5u32 {
            let space_tag = format!("{}-{:03}", prefix, office_num);
            for _ in 0..2 {
                doc.add_placement(&Placement {
                    id: generate_id(), instance_tag: None,
                    product_type_id: pt_sd_linear.clone(),
                    space_id: space_ids.get(&space_tag).cloned(),
                    level: level.clone(), x: None, y: None, rotation: None,
                    cfm: Some(200.0), cfm_balanced: None, static_pressure_pa: None,
                    status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
                    weight_kg: None, notes: None,
                })?;
            }
        }

        // Return grilles (return air plenum, so these are just ceiling transfer paths)
        for i in 1..=8u32 {
            let space_tag = format!("{}-001", prefix);
            doc.add_placement(&Placement {
                id: generate_id(), instance_tag: None,
                product_type_id: pt_rg.clone(),
                space_id: space_ids.get(&space_tag).cloned(),
                level: level.clone(), x: None, y: None, rotation: None,
                cfm: None, cfm_balanced: None, static_pressure_pa: None,
                status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
                weight_kg: None,                notes: Some("Return air via ceiling plenum.".into()),
            })?;
            let _ = i;
        }

        // Exhaust grilles in restrooms
        for rm in ["008", "009"] {
            let space_tag = format!("{}-{}", prefix, rm);
            doc.add_placement(&Placement {
                id: generate_id(), instance_tag: None,
                product_type_id: pt_eg.clone(),
                space_id: space_ids.get(&space_tag).cloned(),
                level: level.clone(), x: None, y: None, rotation: None,
                cfm: Some(200.0), cfm_balanced: None, static_pressure_pa: None,
                status: "new".into(), scope: "in_contract".into(), phase: "design".into(),
                weight_kg: None, notes: None,
            })?;
        }
    }

    // =========================================================================
    // SYSTEMS — Air Systems
    // =========================================================================
    let sys_ahu1_sa = generate_id();
    let sys_ahu1_ra = generate_id();
    let sys_ahu2_sa = generate_id();
    let sys_ahu2_ra = generate_id();
    let sys_ahu3_sa = generate_id();
    let sys_ahu3_ra = generate_id();
    let sys_ahu4_sa = generate_id();
    let sys_ahu4_ra = generate_id();
    let sys_ex_rest_a = generate_id();
    let sys_ex_rest_b = generate_id();
    let sys_ex_garage = generate_id();
    let sys_ex_kitchen = generate_id();
    let sys_spf_a = generate_id();
    let sys_spf_b = generate_id();

    let air_systems = vec![
        (&sys_ahu1_sa, "AHU-1-SA", "AHU-1 Supply Air (Floors 2-4)", "supply", &ahu1_id),
        (&sys_ahu1_ra, "AHU-1-RA", "AHU-1 Return Air (Floors 2-4)", "return", &ahu1_id),
        (&sys_ahu2_sa, "AHU-2-SA", "AHU-2 Supply Air (Floors 5-7)", "supply", &ahu2_id),
        (&sys_ahu2_ra, "AHU-2-RA", "AHU-2 Return Air (Floors 5-7)", "return", &ahu2_id),
        (&sys_ahu3_sa, "AHU-3-SA", "AHU-3 Supply Air (Floors 8-10)", "supply", &ahu3_id),
        (&sys_ahu3_ra, "AHU-3-RA", "AHU-3 Return Air (Floors 8-10)", "return", &ahu3_id),
        (&sys_ahu4_sa, "AHU-4-SA", "AHU-4 Supply Air (Lobby)", "supply", &ahu4_id),
        (&sys_ahu4_ra, "AHU-4-RA", "AHU-4 Return Air (Lobby)", "return", &ahu4_id),
        (&sys_ex_rest_a, "EX-REST-A", "Restroom Exhaust Core A", "exhaust", &ef1_id),
        (&sys_ex_rest_b, "EX-REST-B", "Restroom Exhaust Core B", "exhaust", &ef2_id),
        (&sys_ex_garage, "EX-GAR", "Garage Exhaust", "exhaust", &ef3_id),
        (&sys_ex_kitchen, "EX-KIT", "Kitchen Exhaust", "exhaust", &ef4_id),
        (&sys_spf_a, "SPF-A", "Stairwell A Pressurization", "pressurization", &spf1_id),
        (&sys_spf_b, "SPF-B", "Stairwell B Pressurization", "pressurization", &spf2_id),
    ];

    for (id, tag, name, sys_type, source) in &air_systems {
        doc.add_system(&System {
            id: (*id).clone(), tag: tag.to_string(), name: name.to_string(),
            system_type: sys_type.to_string(), medium: "air".into(),
            source_id: Some((*source).clone()),
            paired_system_id: None,
        })?;
    }

    // =========================================================================
    // SYSTEMS — Hydronic Systems
    // =========================================================================
    let sys_chws = generate_id();
    let sys_chwr = generate_id();
    let sys_hws = generate_id();
    let sys_hwr = generate_id();
    let sys_cws = generate_id();
    let sys_cwr = generate_id();

    // SCHEMA_GAP: Hydronic loops are bidirectional circuits (supply + return), but
    // the system model is unidirectional from a source. We model supply and return as
    // separate systems. The "source" of the return is semantically odd — it's really
    // the same loop. A loop_id or paired_system_id concept would be cleaner.

    // Insert all hydronic systems first without pairing (FK constraint)
    doc.add_system(&System {
        id: sys_chws.clone(), tag: "CHWS".into(), name: "Chilled Water Supply".into(),
        system_type: "supply".into(), medium: "chilled_water".into(),
        source_id: Some(ch1_id.clone()), paired_system_id: None,
    })?;
    doc.add_system(&System {
        id: sys_chwr.clone(), tag: "CHWR".into(), name: "Chilled Water Return".into(),
        system_type: "return".into(), medium: "chilled_water".into(),
        source_id: Some(ch1_id.clone()), paired_system_id: None,
    })?;
    doc.add_system(&System {
        id: sys_hws.clone(), tag: "HWS".into(), name: "Hot Water Supply".into(),
        system_type: "supply".into(), medium: "hot_water".into(),
        source_id: Some(b1_id.clone()), paired_system_id: None,
    })?;
    doc.add_system(&System {
        id: sys_hwr.clone(), tag: "HWR".into(), name: "Hot Water Return".into(),
        system_type: "return".into(), medium: "hot_water".into(),
        source_id: Some(b1_id.clone()), paired_system_id: None,
    })?;
    doc.add_system(&System {
        id: sys_cws.clone(), tag: "CWS".into(), name: "Condenser Water Supply".into(),
        system_type: "supply".into(), medium: "condenser_water".into(),
        source_id: Some(ct1_id.clone()), paired_system_id: None,
    })?;
    doc.add_system(&System {
        id: sys_cwr.clone(), tag: "CWR".into(), name: "Condenser Water Return".into(),
        system_type: "return".into(), medium: "condenser_water".into(),
        source_id: Some(ct1_id.clone()), paired_system_id: None,
    })?;
    // Now link the pairs
    doc.execute_raw("UPDATE systems SET paired_system_id = ?1 WHERE id = ?2", &[&sys_chwr as &dyn rusqlite::types::ToSql, &sys_chws])?;
    doc.execute_raw("UPDATE systems SET paired_system_id = ?1 WHERE id = ?2", &[&sys_chws as &dyn rusqlite::types::ToSql, &sys_chwr])?;
    doc.execute_raw("UPDATE systems SET paired_system_id = ?1 WHERE id = ?2", &[&sys_hwr as &dyn rusqlite::types::ToSql, &sys_hws])?;
    doc.execute_raw("UPDATE systems SET paired_system_id = ?1 WHERE id = ?2", &[&sys_hws as &dyn rusqlite::types::ToSql, &sys_hwr])?;
    doc.execute_raw("UPDATE systems SET paired_system_id = ?1 WHERE id = ?2", &[&sys_cwr as &dyn rusqlite::types::ToSql, &sys_cws])?;
    doc.execute_raw("UPDATE systems SET paired_system_id = ?1 WHERE id = ?2", &[&sys_cws as &dyn rusqlite::types::ToSql, &sys_cwr])?;

    // =========================================================================
    // DUCT GRAPH — AHU-1 Supply Air, Floor 2
    // Main riser from basement to floor 2, then horizontal trunk with VAV takeoffs.
    // Trunk: 36"x24" rectangular, runs N-S through the floor.
    // 12 VAV branches (8 reheat + 4 cooling-only)
    // =========================================================================

    // AHU-1 equipment connection node
    let ahu1_equip_node = generate_id();
    doc.add_node(&Node {
        id: ahu1_equip_node.clone(), system_id: sys_ahu1_sa.clone(),
        node_type: "equipment_conn".into(), placement_id: Some(ahu1_id.clone()),
        fitting_type: None, size_description: Some("36\"x24\" rectangular".into()),
        level: Some("Basement".into()), x: Some(35.0), y: Some(3.0),
    })?;

    // Riser node at floor 2
    let riser_f2_node = generate_id();
    doc.add_node(&Node {
        id: riser_f2_node.clone(), system_id: sys_ahu1_sa.clone(),
        node_type: "fitting".into(), placement_id: None,
        fitting_type: Some("riser_elbow".into()),
        size_description: Some("36\"x24\" to 36\"x24\"".into()),
        level: Some("Level 2".into()), x: Some(35.0), y: Some(0.0),
    })?;

    // Riser segment (vertical, basement to floor 2)
    doc.add_segment(&Segment {
        id: generate_id(), system_id: sys_ahu1_sa.clone(),
        from_node_id: ahu1_equip_node.clone(), to_node_id: riser_f2_node.clone(),
        shape: "rectangular".into(),
        width_m: Some(0.914), height_m: Some(0.610), diameter_m: None,
        length_m: Some(7.0), // basement to floor 2
        material: "galvanized".into(), gauge: Some(20),
        pressure_class: Some("4_in_wg".into()), construction: Some("tdc".into()),
        exposure: Some("concealed".into()),
        flow_design: Some(25000.0), flow_balanced: None,
        status: "new".into(), scope: "in_contract".into(),
    })?;

    // Horizontal trunk on floor 2, running N-S from y=0 to y=40
    // VAV branches tap off every ~3m
    let vav_tags_f2 = [
        "VAV-2-01", "VAV-2-02", "VAV-2-03", "VAV-2-04",
        "VAV-2-05", "VAV-2-06", "VAV-2-07", "VAV-2-08",
        "VAV-2-09", "VAV-2-10", "VAV-2-11", "VAV-2-12",
    ];
    let vav_cfms_f2 = [
        800.0, 800.0, 800.0, 800.0,
        800.0, 800.0, 800.0, 800.0,
        1000.0, 1000.0, 1000.0, 1000.0,
    ];
    let total_floor2_cfm: f64 = vav_cfms_f2.iter().sum();

    let mut prev_node = riser_f2_node;
    let mut duct_segment_ids: Vec<String> = Vec::new();

    for (i, (tag, cfm)) in vav_tags_f2.iter().zip(vav_cfms_f2.iter()).enumerate() {
        let tap_y = 3.0 + (i as f64) * 3.0;
        let flow_remaining: f64 = total_floor2_cfm - vav_cfms_f2[..i].iter().sum::<f64>();

        // Trunk size decreases as flow decreases
        let (trunk_w, trunk_h) = if flow_remaining > 8000.0 {
            (0.914, 0.610) // 36x24
        } else if flow_remaining > 5000.0 {
            (0.762, 0.508) // 30x20
        } else if flow_remaining > 3000.0 {
            (0.610, 0.406) // 24x16
        } else {
            (0.457, 0.356) // 18x14
        };

        // Tap node on the trunk
        let tap_node = generate_id();
        doc.add_node(&Node {
            id: tap_node.clone(), system_id: sys_ahu1_sa.clone(),
            node_type: "fitting".into(), placement_id: None,
            fitting_type: Some("tap_45".into()),
            size_description: Some(format!("{:.0}\"x{:.0}\" trunk", trunk_w / 0.0254, trunk_h / 0.0254)),
            level: Some("Level 2".into()), x: Some(35.0), y: Some(tap_y),
        })?;

        // Trunk segment from previous node to tap
        let prev_y = if i == 0 { 0.0 } else { 3.0 + ((i - 1) as f64) * 3.0 };
        let seg_len = tap_y - prev_y;
        let trunk_seg = generate_id();
        doc.add_segment(&Segment {
            id: trunk_seg.clone(), system_id: sys_ahu1_sa.clone(),
            from_node_id: prev_node.clone(), to_node_id: tap_node.clone(),
            shape: "rectangular".into(),
            width_m: Some(trunk_w), height_m: Some(trunk_h), diameter_m: None,
            length_m: Some(seg_len),
            material: "galvanized".into(), gauge: Some(if flow_remaining > 5000.0 { 20 } else { 22 }),
            pressure_class: Some("4_in_wg".into()), construction: Some("tdc".into()),
            exposure: Some("concealed".into()),
            flow_design: Some(flow_remaining), flow_balanced: None,
            status: "new".into(), scope: "in_contract".into(),
        })?;
        duct_segment_ids.push(trunk_seg);

        // VAV terminal node
        let vav_node = generate_id();
        let vav_pid = vav_placement_ids.get(*tag).cloned();
        doc.add_node(&Node {
            id: vav_node.clone(), system_id: sys_ahu1_sa.clone(),
            node_type: "terminal".into(), placement_id: vav_pid,
            fitting_type: None,
            size_description: Some(if *cfm > 900.0 { "14\" round".into() } else { "12\" round".into() }),
            level: Some("Level 2".into()), x: Some(32.0), y: Some(tap_y),
        })?;

        // Branch from tap to VAV
        let branch_dia = if *cfm > 900.0 { 0.356 } else { 0.305 }; // 14" or 12"
        let branch_seg = generate_id();
        doc.add_segment(&Segment {
            id: branch_seg.clone(), system_id: sys_ahu1_sa.clone(),
            from_node_id: tap_node.clone(), to_node_id: vav_node,
            shape: "round".into(),
            width_m: None, height_m: None, diameter_m: Some(branch_dia),
            length_m: Some(3.0),
            material: "galvanized".into(), gauge: Some(24),
            pressure_class: Some("4_in_wg".into()), construction: Some("spiral_lock".into()),
            exposure: Some("concealed".into()),
            flow_design: Some(*cfm), flow_balanced: None,
            status: "new".into(), scope: "in_contract".into(),
        })?;
        duct_segment_ids.push(branch_seg);

        prev_node = tap_node;
    }

    // Trunk end cap
    let end_cap_node = generate_id();
    doc.add_node(&Node {
        id: end_cap_node.clone(), system_id: sys_ahu1_sa.clone(),
        node_type: "cap".into(), placement_id: None,
        fitting_type: Some("end_cap".into()),
        size_description: Some("18\"x14\" rectangular".into()),
        level: Some("Level 2".into()), x: Some(35.0), y: Some(40.0),
    })?;
    let final_trunk_seg = generate_id();
    doc.add_segment(&Segment {
        id: final_trunk_seg.clone(), system_id: sys_ahu1_sa.clone(),
        from_node_id: prev_node, to_node_id: end_cap_node,
        shape: "rectangular".into(),
        width_m: Some(0.457), height_m: Some(0.356), diameter_m: None,
        length_m: Some(1.0),
        material: "galvanized".into(), gauge: Some(22),
        pressure_class: Some("4_in_wg".into()), construction: Some("tdc".into()),
        exposure: Some("concealed".into()),
        flow_design: Some(0.0), flow_balanced: None,
        status: "new".into(), scope: "in_contract".into(),
    })?;
    duct_segment_ids.push(final_trunk_seg);

    // =========================================================================
    // CHILLED WATER PIPING GRAPH
    // Chiller CH-1 evaporator out -> primary pump CHWP-P1 -> decoupler ->
    // secondary pump CHWP-S1 -> riser -> AHU-1 CHW coil -> AHU-2 CHW coil ->
    // AHU-3 CHW coil -> AHU-4 CHW coil -> return to chiller
    // =========================================================================

    // SCHEMA_GAP: Piping loops have a fundamental topology difference from duct trees.
    // A chilled water loop is a cycle (supply out -> coils -> return back to chiller).
    // The directed graph model (from_node -> to_node) can represent this, but the
    // "cap" / "end" concept doesn't apply. We close the loop by connecting the last
    // return node back to the chiller equipment_conn node. This works but feels
    // awkward since the graph is nominally a tree.

    // SCHEMA_GAP: The primary/secondary decoupler bypass is a common piping element
    // that doesn't map cleanly to a single node. It's a short pipe between the
    // primary and secondary loops. We model it as a fitting node.

    // Chiller 1 evaporator outlet
    let ch1_out = generate_id();
    doc.add_node(&Node {
        id: ch1_out.clone(), system_id: sys_chws.clone(),
        node_type: "equipment_conn".into(), placement_id: Some(ch1_id.clone()),
        fitting_type: None, size_description: Some("10\" pipe".into()),
        level: Some("Basement".into()), x: Some(10.0), y: Some(3.0),
    })?;

    // Primary pump 1
    let chwp1_node = generate_id();
    doc.add_node(&Node {
        id: chwp1_node.clone(), system_id: sys_chws.clone(),
        node_type: "equipment_conn".into(), placement_id: Some(chwp1_id.clone()),
        fitting_type: None, size_description: Some("10\" x 8\" pump".into()),
        level: Some("Basement".into()), x: Some(5.0), y: Some(3.0),
    })?;

    // Chiller to primary pump
    doc.add_segment(&Segment {
        id: generate_id(), system_id: sys_chws.clone(),
        from_node_id: ch1_out.clone(), to_node_id: chwp1_node.clone(),
        shape: "pipe".into(),
        width_m: None, height_m: None, diameter_m: Some(0.254), // 10"
        length_m: Some(5.0),
        material: "steel".into(), gauge: None,
        pressure_class: Some("150_psi".into()), construction: Some("welded".into()),
        exposure: Some("exposed".into()),
        flow_design: Some(1200.0), // GPM
        flow_balanced: None,
        status: "new".into(), scope: "in_contract".into(),
    })?;

    // Decoupler tee
    let decoupler_node = generate_id();
    doc.add_node(&Node {
        id: decoupler_node.clone(), system_id: sys_chws.clone(),
        node_type: "fitting".into(), placement_id: None,
        fitting_type: Some("decoupler_tee".into()),
        size_description: Some("10\" tee, primary/secondary decoupler".into()),
        level: Some("Basement".into()), x: Some(3.5), y: Some(3.0),
    })?;

    doc.add_segment(&Segment {
        id: generate_id(), system_id: sys_chws.clone(),
        from_node_id: chwp1_node.clone(), to_node_id: decoupler_node.clone(),
        shape: "pipe".into(),
        width_m: None, height_m: None, diameter_m: Some(0.254),
        length_m: Some(1.5),
        material: "steel".into(), gauge: None,
        pressure_class: Some("150_psi".into()), construction: Some("welded".into()),
        exposure: Some("exposed".into()),
        flow_design: Some(1200.0),
        flow_balanced: None,
        status: "new".into(), scope: "in_contract".into(),
    })?;

    // Secondary pump 1
    let chwps1_node = generate_id();
    doc.add_node(&Node {
        id: chwps1_node.clone(), system_id: sys_chws.clone(),
        node_type: "equipment_conn".into(), placement_id: Some(chwp_s1_id.clone()),
        fitting_type: None, size_description: Some("10\" x 8\" pump".into()),
        level: Some("Basement".into()), x: Some(2.0), y: Some(3.0),
    })?;

    doc.add_segment(&Segment {
        id: generate_id(), system_id: sys_chws.clone(),
        from_node_id: decoupler_node.clone(), to_node_id: chwps1_node.clone(),
        shape: "pipe".into(),
        width_m: None, height_m: None, diameter_m: Some(0.254),
        length_m: Some(1.5),
        material: "steel".into(), gauge: None,
        pressure_class: Some("150_psi".into()), construction: Some("welded".into()),
        exposure: Some("exposed".into()),
        flow_design: Some(1200.0),
        flow_balanced: None,
        status: "new".into(), scope: "in_contract".into(),
    })?;

    // CHW supply header
    let chw_header_node = generate_id();
    doc.add_node(&Node {
        id: chw_header_node.clone(), system_id: sys_chws.clone(),
        node_type: "fitting".into(), placement_id: None,
        fitting_type: Some("header".into()),
        size_description: Some("12\" supply header".into()),
        level: Some("Basement".into()), x: Some(2.0), y: Some(8.0),
    })?;

    doc.add_segment(&Segment {
        id: generate_id(), system_id: sys_chws.clone(),
        from_node_id: chwps1_node.clone(), to_node_id: chw_header_node.clone(),
        shape: "pipe".into(),
        width_m: None, height_m: None, diameter_m: Some(0.305), // 12"
        length_m: Some(5.0),
        material: "steel".into(), gauge: None,
        pressure_class: Some("150_psi".into()), construction: Some("welded".into()),
        exposure: Some("exposed".into()),
        flow_design: Some(2400.0), // total secondary flow
        flow_balanced: None,
        status: "new".into(), scope: "in_contract".into(),
    })?;

    // AHU coil connections from header
    let ahu_refs = [
        (&ahu1_id, "AHU-1", 600.0),  // 72 tons @ 2.4 GPM/ton
        (&ahu2_id, "AHU-2", 600.0),
        (&ahu3_id, "AHU-3", 600.0),
        (&ahu4_id, "AHU-4", 200.0),  // 25 tons
    ];

    for (ahu_id, ahu_tag, gpm) in &ahu_refs {
        let ahu_coil_node = generate_id();
        doc.add_node(&Node {
            id: ahu_coil_node.clone(), system_id: sys_chws.clone(),
            node_type: "equipment_conn".into(), placement_id: Some((*ahu_id).clone()),
            fitting_type: None,
            size_description: Some(format!("{} CHW coil connection", ahu_tag)),
            level: Some("Basement".into()), x: None, y: None,
        })?;

        let pipe_dia = if *gpm > 400.0 { 0.203 } else { 0.152 }; // 8" or 6"
        doc.add_segment(&Segment {
            id: generate_id(), system_id: sys_chws.clone(),
            from_node_id: chw_header_node.clone(), to_node_id: ahu_coil_node,
            shape: "pipe".into(),
            width_m: None, height_m: None, diameter_m: Some(pipe_dia),
            length_m: Some(8.0),
            material: "steel".into(), gauge: None,
            pressure_class: Some("150_psi".into()), construction: Some("grooved".into()),
            exposure: Some("exposed".into()),
            flow_design: Some(*gpm),
            flow_balanced: None,
            status: "new".into(), scope: "in_contract".into(),
        })?;
    }

    // =========================================================================
    // HOT WATER PIPING — simplified header to AHU preheat + VAV reheat risers
    // =========================================================================

    // Boiler 1 outlet
    let b1_out = generate_id();
    doc.add_node(&Node {
        id: b1_out.clone(), system_id: sys_hws.clone(),
        node_type: "equipment_conn".into(), placement_id: Some(b1_id.clone()),
        fitting_type: None, size_description: Some("6\" pipe".into()),
        level: Some("Basement".into()), x: Some(25.0), y: Some(3.0),
    })?;

    // HW pump 1
    let hwp1_node = generate_id();
    doc.add_node(&Node {
        id: hwp1_node.clone(), system_id: sys_hws.clone(),
        node_type: "equipment_conn".into(), placement_id: Some(hwp1_id.clone()),
        fitting_type: None, size_description: Some("6\" x 5\" pump".into()),
        level: Some("Basement".into()), x: Some(28.0), y: Some(3.0),
    })?;

    doc.add_segment(&Segment {
        id: generate_id(), system_id: sys_hws.clone(),
        from_node_id: b1_out.clone(), to_node_id: hwp1_node.clone(),
        shape: "pipe".into(),
        width_m: None, height_m: None, diameter_m: Some(0.152), // 6"
        length_m: Some(3.0),
        material: "steel".into(), gauge: None,
        pressure_class: Some("150_psi".into()), construction: Some("welded".into()),
        exposure: Some("exposed".into()),
        flow_design: Some(400.0),
        flow_balanced: None,
        status: "new".into(), scope: "in_contract".into(),
    })?;

    // HW supply header
    let hw_header_node = generate_id();
    doc.add_node(&Node {
        id: hw_header_node.clone(), system_id: sys_hws.clone(),
        node_type: "fitting".into(), placement_id: None,
        fitting_type: Some("header".into()),
        size_description: Some("8\" HW supply header".into()),
        level: Some("Basement".into()), x: Some(30.0), y: Some(8.0),
    })?;

    doc.add_segment(&Segment {
        id: generate_id(), system_id: sys_hws.clone(),
        from_node_id: hwp1_node.clone(), to_node_id: hw_header_node.clone(),
        shape: "pipe".into(),
        width_m: None, height_m: None, diameter_m: Some(0.203), // 8"
        length_m: Some(5.0),
        material: "steel".into(), gauge: None,
        pressure_class: Some("150_psi".into()), construction: Some("welded".into()),
        exposure: Some("exposed".into()),
        flow_design: Some(400.0),
        flow_balanced: None,
        status: "new".into(), scope: "in_contract".into(),
    })?;

    // HW riser nodes for each AHU zone (preheat + reheat coils share the riser)
    for (i, (ahu_id, ahu_tag, _gpm)) in ahu_refs.iter().enumerate() {
        // AHU preheat coil
        let preheat_node = generate_id();
        doc.add_node(&Node {
            id: preheat_node.clone(), system_id: sys_hws.clone(),
            node_type: "equipment_conn".into(), placement_id: Some((*ahu_id).clone()),
            fitting_type: None,
            size_description: Some(format!("{} HW preheat coil", ahu_tag)),
            level: Some("Basement".into()), x: None, y: None,
        })?;

        let preheat_gpm = if i < 3 { 60.0 } else { 20.0 }; // rough sizing
        doc.add_segment(&Segment {
            id: generate_id(), system_id: sys_hws.clone(),
            from_node_id: hw_header_node.clone(), to_node_id: preheat_node,
            shape: "pipe".into(),
            width_m: None, height_m: None, diameter_m: Some(0.076), // 3"
            length_m: Some(6.0),
            material: "steel".into(), gauge: None,
            pressure_class: Some("150_psi".into()), construction: Some("grooved".into()),
            exposure: Some("exposed".into()),
            flow_design: Some(preheat_gpm),
            flow_balanced: None,
            status: "new".into(), scope: "in_contract".into(),
        })?;
    }

    // HW riser to floors for VAV reheat — one riser node per zone group
    // SCHEMA_GAP: Individual VAV reheat coil piping connections would create hundreds
    // of nodes. In practice, the riser branches at each floor into a loop that feeds
    // the reheat coils. We model the riser as a single segment and note that the
    // per-floor distribution is not detailed in this graph. A "distribution_zone"
    // abstraction or a separate reheat piping system per floor would scale better.

    let reheat_riser_node = generate_id();
    doc.add_node(&Node {
        id: reheat_riser_node.clone(), system_id: sys_hws.clone(),
        node_type: "fitting".into(), placement_id: None,
        fitting_type: Some("riser_tee".into()),
        size_description: Some("4\" HW reheat riser, floors 2-10".into()),
        level: Some("Basement".into()), x: Some(33.0), y: Some(8.0),
    })?;

    doc.add_segment(&Segment {
        id: generate_id(), system_id: sys_hws.clone(),
        from_node_id: hw_header_node.clone(), to_node_id: reheat_riser_node,
        shape: "pipe".into(),
        width_m: None, height_m: None, diameter_m: Some(0.102), // 4"
        length_m: Some(3.0),
        material: "steel".into(), gauge: None,
        pressure_class: Some("150_psi".into()), construction: Some("grooved".into()),
        exposure: Some("exposed".into()),
        flow_design: Some(280.0), // total reheat flow
        flow_balanced: None,
        status: "new".into(), scope: "in_contract".into(),
    })?;

    // =========================================================================
    // INSULATION — CHW and HW piping
    // =========================================================================
    // Note: We would normally insulate every segment. For brevity, we add insulation
    // specs as a representative sample and note the pattern.

    // Duct insulation on AHU-1 supply duct segments
    for seg_id in &duct_segment_ids {
        doc.add_insulation(&Insulation {
            id: generate_id(),
            segment_id: Some(seg_id.clone()),
            insulation_type: "duct_wrap".into(),
            manufacturer: Some("Owens Corning".into()),
            product: Some("Fiberglas 75 Duct Wrap".into()),
            thickness_m: Some(0.038), // 1.5"
            r_value: Some(6.0),
            facing: Some("fsk".into()),
            code_reference: Some("ASHRAE 90.1 Section 6".into()),
        })?;
    }

    // =========================================================================
    // SHEETS
    // =========================================================================
    let sheets_data = vec![
        ("M-001", "Mechanical Cover Sheet, Abbreviations, Symbols"),
        ("M-002", "Mechanical General Notes & Schedules"),
        ("M-003", "Mechanical Central Plant Layout - Basement"),
        ("M-004", "Mechanical Central Plant Piping Diagram"),
        ("M-101", "Mechanical Floor Plan - Basement"),
        ("M-102", "Mechanical Floor Plan - Level 1"),
        ("M-103", "Mechanical Floor Plan - Level 2"),
        ("M-104", "Mechanical Floor Plan - Level 3"),
        ("M-105", "Mechanical Floor Plan - Level 4 (Typical 4-10)"),
        ("M-201", "Mechanical Roof Plan"),
        ("M-301", "Mechanical Sections & Details"),
        ("M-302", "Mechanical Piping Details"),
        ("M-401", "Mechanical Schedules - Equipment"),
        ("M-402", "Mechanical Schedules - VAV & Air Devices"),
        ("M-501", "HVAC Controls Diagram"),
        ("M-502", "HVAC Controls Sequences"),
        ("P-001", "Plumbing Cover Sheet"),
        ("P-101", "Plumbing Floor Plan - Basement"),
        ("P-102", "Plumbing Floor Plan - Level 1"),
    ];

    let mut sheet_ids: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for (number, title) in &sheets_data {
        let id = generate_id();
        sheet_ids.insert(number.to_string(), id.clone());
        doc.add_sheet(&Sheet {
            id, number: number.to_string(), title: title.to_string(),
            discipline: if number.starts_with('P') { "plumbing".into() } else { "mechanical".into() },
            sheet_size: Some("ARCH D".into()),
        })?;
    }

    // =========================================================================
    // VIEWS
    // =========================================================================
    let plan_scale = "1/8\" = 1'-0\"";

    // Central plant layout
    if let Some(sid) = sheet_ids.get("M-003") {
        doc.add_view(&View {
            id: generate_id(), sheet_id: sid.clone(),
            view_type: "plan".into(), title: Some("Central Plant Layout".into()),
            scale: Some("1/4\" = 1'-0\"".into()), level: Some("Basement".into()),
            vp_x: Some(1.0), vp_y: Some(1.0), vp_width: Some(32.0), vp_height: Some(20.0),
            model_x_min: Some(0.0), model_y_min: Some(0.0),
            model_x_max: Some(50.0), model_y_max: Some(20.0),
        })?;
    }

    // Piping diagram (schematic, no scale)
    if let Some(sid) = sheet_ids.get("M-004") {
        doc.add_view(&View {
            id: generate_id(), sheet_id: sid.clone(),
            view_type: "diagram".into(), title: Some("CHW/HW Piping Schematic".into()),
            scale: Some("NTS".into()), level: None,
            vp_x: None, vp_y: None, vp_width: None, vp_height: None,
            model_x_min: None, model_y_min: None, model_x_max: None, model_y_max: None,
        })?;
    }

    // Floor plans
    let floor_sheets = [
        ("M-101", "Basement"),
        ("M-102", "Level 1"),
        ("M-103", "Level 2"),
        ("M-104", "Level 3"),
        ("M-105", "Level 4"),
        ("M-201", "Roof"),
    ];
    for (sheet_num, level) in &floor_sheets {
        if let Some(sid) = sheet_ids.get(*sheet_num) {
            doc.add_view(&View {
                id: generate_id(), sheet_id: sid.clone(),
                view_type: "plan".into(),
                title: Some(format!("{} Mechanical Plan", level)),
                scale: Some(plan_scale.into()), level: Some(level.to_string()),
                vp_x: Some(1.0), vp_y: Some(1.0), vp_width: Some(32.0), vp_height: Some(20.0),
                model_x_min: Some(-5.0), model_y_min: Some(-5.0),
                model_x_max: Some(55.0), model_y_max: Some(45.0),
            })?;
        }
    }

    // Detail sheets
    if let Some(sid) = sheet_ids.get("M-301") {
        let details = [
            "Typical VAV Installation Detail",
            "Duct Riser Section",
            "AHU Connection Detail",
            "Fire/Smoke Damper Installation",
            "Seismic Bracing Detail",
            "Access Door Detail",
        ];
        for title in &details {
            doc.add_view(&View {
                id: generate_id(), sheet_id: sid.clone(),
                view_type: "detail".into(), title: Some(title.to_string()),
                scale: None, level: None,
                vp_x: None, vp_y: None, vp_width: None, vp_height: None,
                model_x_min: None, model_y_min: None, model_x_max: None, model_y_max: None,
            })?;
        }
    }

    // Schedule views
    if let Some(sid) = sheet_ids.get("M-401") {
        doc.add_view(&View {
            id: generate_id(), sheet_id: sid.clone(),
            view_type: "schedule".into(), title: Some("Equipment Schedule".into()),
            scale: None, level: None,
            vp_x: None, vp_y: None, vp_width: None, vp_height: None,
            model_x_min: None, model_y_min: None, model_x_max: None, model_y_max: None,
        })?;
    }
    if let Some(sid) = sheet_ids.get("M-402") {
        doc.add_view(&View {
            id: generate_id(), sheet_id: sid.clone(),
            view_type: "schedule".into(), title: Some("VAV & Air Device Schedule".into()),
            scale: None, level: None,
            vp_x: None, vp_y: None, vp_width: None, vp_height: None,
            model_x_min: None, model_y_min: None, model_x_max: None, model_y_max: None,
        })?;
    }

    // Controls diagrams
    if let Some(sid) = sheet_ids.get("M-501") {
        doc.add_view(&View {
            id: generate_id(), sheet_id: sid.clone(),
            view_type: "diagram".into(), title: Some("HVAC Controls Diagram".into()),
            scale: Some("NTS".into()), level: None,
            vp_x: None, vp_y: None, vp_width: None, vp_height: None,
            model_x_min: None, model_y_min: None, model_x_max: None, model_y_max: None,
        })?;
    }

    // =========================================================================
    // GENERAL NOTES
    // =========================================================================
    let general_notes = vec![
        "All work shall comply with the latest edition of the International Mechanical Code (IMC), ASHRAE Standards 55, 62.1, and 90.1, and all local amendments.",
        "Contractor shall visit the site and verify all existing conditions before bidding. No allowances for unforeseen conditions.",
        "All ductwork shall be constructed per SMACNA standards. Supply ductwork shall be sealed to SMACNA Seal Class A. Return ductwork to Seal Class B.",
        "All rectangular ductwork 24\" and larger shall be externally insulated with 1.5\" fiberglass duct wrap, R-6, FSK faced. Round spiral ductwork in unconditioned spaces shall be insulated similarly.",
        "Flexible duct connections shall not exceed 5'-0\" in length. Install with no more than 4\" sag per 10' run.",
        "All chilled water and hot water piping shall be insulated per ASHRAE 90.1 Table 6.8.3-1. CHW piping: 1\" fiberglass with vapor barrier. HW piping: 1.5\" fiberglass.",
        "Provide vibration isolation for all rotating equipment per ASHRAE Handbook recommendations. Spring isolators for equipment over 10 HP.",
        "All hydronic piping 2.5\" and larger shall be grooved mechanical joint (Victaulic or approved equal). Piping under 2.5\" shall be threaded or soldered copper.",
        "Test and balance all air and hydronic systems per AABC/NEBB standards. Provide certified TAB report.",
        "Provide seismic bracing for all ductwork, piping, and equipment per IBC Chapter 13 and ASCE 7. Refer to structural drawings for building importance factor.",
        "Commission all mechanical systems per ASHRAE Guideline 0 and owner's project requirements. Refer to commissioning specification for requirements.",
        "All fire and smoke dampers shall be UL listed. Access panels required at all damper locations. Dynamic rated where required by duct velocity.",
        "Maintain minimum 1\" clearance between ductwork and sprinkler piping, electrical conduit, and structural members.",
        "Provide balancing dampers at all duct branches. Manual volume dampers at supply outlets and return inlets.",
    ];
    for (i, text) in general_notes.iter().enumerate() {
        doc.add_general_note(&generate_id(), Some("mechanical"), text, (i + 1) as i32)?;
    }

    // =========================================================================
    // KEYED NOTES
    // =========================================================================
    let keyed_notes = vec![
        ("M1", "Provide 3/8\" flexible CHW connections to VAV reheat coil. Isolate with ball valves.", Some("230713")),
        ("M2", "Provide access door in ductwork at all fire/smoke dampers and upstream of all VAV boxes.", Some("233100")),
        ("M3", "Route condensate drain to nearest floor drain. Provide trap per manufacturer requirements.", Some("232113")),
        ("M4", "Provide combination fire/smoke damper at all rated wall and floor penetrations. UL Class 2, 3-hour rated.", Some("233400")),
        ("M5", "Provide duct-mounted smoke detector in supply and return ductwork over 2000 CFM per code. Interlock with AHU.", Some("230900")),
        ("M6", "Provide motorized isolation valve (2-way) at each AHU CHW coil. Fail closed.", Some("232113")),
        ("M7", "Provide 2-way modulating control valve at each AHU CHW coil. Sized for 3 PSI drop at design flow.", Some("230900")),
        ("M8", "Provide differential pressure sensor across each AHU CHW coil. Connect to BMS.", Some("230900")),
        ("M9", "Provide CO sensors in parking garage per IMC. Interlock with EF-3 garage exhaust fan. Alarm at 35 PPM, fan to high speed at 100 PPM.", Some("230900")),
        ("M10", "Provide stairwell differential pressure sensor at mid-height of each stairwell. Maintain 0.05\" to 0.10\" wg per IBC.", Some("230900")),
        ("M11", "Kitchen exhaust duct to be welded black steel, liquid tight, pitched back to hood. Provide cleanout at each change of direction.", Some("233100")),
        ("M12", "Provide seismic isolation valves on all risers at each floor per code.", Some("232113")),
        ("M13", "All exposed ductwork in lobby to be painted to match architect's finish schedule.", Some("233100")),
        ("M14", "Return air plenum: coordinate with architect for rated barriers above ceiling at all rated walls.", Some("233100")),
    ];
    for (key, text, spec) in &keyed_notes {
        doc.add_keyed_note(&KeyedNote {
            id: generate_id(), key: key.to_string(), text: text.to_string(),
            discipline: Some("mechanical".into()),
            spec_section: spec.map(|s| s.to_string()),
        })?;
    }

    // =========================================================================
    // REVISIONS
    // =========================================================================
    doc.add_revision(&Revision {
        id: generate_id(), number: 0, name: "SD Issue".into(),
        date: "2025-09-15".into(), description: Some("Schematic Design".into()),
        author: Some("WSP USA".into()),
    })?;
    doc.add_revision(&Revision {
        id: generate_id(), number: 1, name: "DD Issue".into(),
        date: "2025-12-01".into(), description: Some("Design Development".into()),
        author: Some("WSP USA".into()),
    })?;
    doc.add_revision(&Revision {
        id: generate_id(), number: 2, name: "CD Issue".into(),
        date: "2026-02-15".into(), description: Some("100% Construction Documents".into()),
        author: Some("WSP USA".into()),
    })?;
    doc.add_revision(&Revision {
        id: generate_id(), number: 3, name: "Addendum 1".into(),
        date: "2026-03-10".into(), description: Some("Bid clarifications, VAV schedule corrections".into()),
        author: Some("WSP USA".into()),
    })?;

    // =========================================================================
    // SCHEMA GAP SUMMARY (collected here for easy discovery)
    // =========================================================================
    // SCHEMA_GAP: (1) VAV reheat dual-system membership. A placement can only belong
    //   to one system_id, but a VAV with reheat participates in both an air system and
    //   a hot water system. Need either a junction table (placement_systems) or allow
    //   multiple system_ids.
    //
    // SCHEMA_GAP: (2) Hydronic loop topology. Supply and return are modeled as separate
    //   systems with no formal pairing. A loop_id or paired_system_id would let queries
    //   find the matching return for a supply system.
    //
    // SCHEMA_GAP: (3) Primary/secondary decoupler. This is a piping pattern (bypass
    //   bridge) that doesn't map cleanly to the node/segment tree model. We used a
    //   "decoupler_tee" fitting_type, but the actual topology is two overlapping loops
    //   sharing a common bypass segment.
    //
    // SCHEMA_GAP: (4) Per-floor reheat piping distribution. Modeling every VAV reheat
    //   coil connection individually creates a combinatorial explosion of nodes/segments.
    //   A "distribution_zone" or "sub-system" concept could abstract the per-floor
    //   piping branches without requiring hundreds of graph entities.
    //
    // SCHEMA_GAP: (5) VAV inlet sizing. product_type has no structured field for
    //   inlet_size or capacity_range. A single product_type is used for all VAV sizes,
    //   but in reality each inlet size (8", 10", 12", etc.) has different performance
    //   characteristics and may warrant separate product_types or a size parameter.
    //
    // SCHEMA_GAP: (6) Equipment performance data. Chillers have COP/IPLV, boilers have
    //   thermal efficiency, pumps have pump curves. None of these are representable in
    //   the current schema. A key-value "equipment_attributes" table would help.
    //
    // SCHEMA_GAP: (7) No concept of "schedule" data (equipment schedules with columns
    //   for design conditions, electrical data, weight, etc.). The View type supports
    //   schedule views, but there's no structured data model for the schedule content
    //   itself — it's just rendered as a view.
    //
    // SCHEMA_GAP: (8) flow_design on Segment uses f64 but the unit is ambiguous: CFM
    //   for air, GPM for water. The schema has no unit field. Either add a flow_unit
    //   field or inherit units from the system's medium.

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn create_test_doc() -> (SedDocument, String) {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        drop(tmp);
        create_office_tower(&path).expect("Should create office tower");
        let doc = SedDocument::open(&path).unwrap();
        (doc, path)
    }

    #[test]
    fn test_create_office_tower() {
        let (doc, _) = create_test_doc();
        let info = doc.info().unwrap();
        assert!(info.spaces > 100);
        assert!(info.placements > 100);
        assert!(info.systems > 10);
        assert!(info.nodes > 20);
        assert!(info.segments > 20);
    }

    #[test]
    fn test_all_vavs_have_instance_tags() {
        let (doc, _) = create_test_doc();
        let rows = doc.query_raw(
            "SELECT COUNT(*) FROM placements p JOIN product_types pt ON p.product_type_id = pt.id WHERE pt.category = 'vav_box' AND p.instance_tag IS NULL"
        ).unwrap();
        let untagged: i64 = rows[0][0].1.parse().unwrap();
        assert_eq!(untagged, 0, "All VAVs should have instance tags");
    }

    #[test]
    fn test_hydronic_pairs_linked() {
        let (doc, _) = create_test_doc();
        let rows = doc.query_raw(
            "SELECT s1.tag, s2.tag FROM systems s1 JOIN systems s2 ON s1.paired_system_id = s2.id WHERE s1.medium = 'chilled_water' AND s1.system_type = 'supply'"
        ).unwrap();
        assert!(!rows.is_empty(), "CHW supply should be paired with CHW return");
        assert_eq!(rows[0][0].1, "CHWS");
        assert_eq!(rows[0][1].1, "CHWR");
    }

    #[test]
    fn test_hydronic_pairs_bidirectional() {
        let (doc, _) = create_test_doc();
        // Every system with a pair should have its pair point back
        let rows = doc.query_raw(
            "SELECT s1.tag, s2.tag FROM systems s1 JOIN systems s2 ON s1.paired_system_id = s2.id WHERE s2.paired_system_id != s1.id"
        ).unwrap();
        assert!(rows.is_empty(), "Paired systems should point at each other: {:?}", rows);
    }

    #[test]
    fn test_duct_graph_traversal() {
        let (doc, _) = create_test_doc();
        // Find the AHU-1 supply system's equipment connection node
        let starts = doc.query_raw(
            "SELECT n.id FROM nodes n JOIN systems sys ON n.system_id = sys.id WHERE sys.tag = 'AHU-1-SA' AND n.node_type = 'equipment_conn'"
        ).unwrap();
        assert!(!starts.is_empty(), "AHU-1-SA should have an equipment_conn node");

        let start_id = &starts[0][0].1;
        let trace = doc.query_raw(&format!(
            "WITH RECURSIVE downstream AS (
                SELECT n.id, n.node_type, 0 as depth FROM nodes n WHERE n.id = '{}'
                UNION ALL
                SELECT n2.id, n2.node_type, d.depth + 1
                FROM downstream d
                JOIN segments seg ON seg.from_node_id = d.id
                JOIN nodes n2 ON n2.id = seg.to_node_id
                WHERE d.depth < 50
            )
            SELECT * FROM downstream", start_id
        )).unwrap();

        let terminals: Vec<_> = trace.iter().filter(|r| r[1].1 == "terminal").collect();
        assert!(terminals.len() >= 10, "AHU-1 trunk should reach at least 10 VAV terminals, got {}", terminals.len());
    }

    #[test]
    fn test_chw_piping_graph() {
        let (doc, _) = create_test_doc();
        // CHW supply system should have nodes and segments
        let rows = doc.query_raw(
            "SELECT COUNT(*) FROM nodes n JOIN systems sys ON n.system_id = sys.id WHERE sys.tag = 'CHWS'"
        ).unwrap();
        let node_count: i64 = rows[0][0].1.parse().unwrap();
        assert!(node_count > 5, "CHWS should have >5 nodes, got {}", node_count);

        let rows = doc.query_raw(
            "SELECT COUNT(*) FROM segments seg JOIN systems sys ON seg.system_id = sys.id WHERE sys.tag = 'CHWS'"
        ).unwrap();
        let seg_count: i64 = rows[0][0].1.parse().unwrap();
        assert!(seg_count > 3, "CHWS should have >3 segments, got {}", seg_count);
    }

    #[test]
    fn test_equipment_per_floor() {
        let (doc, _) = create_test_doc();
        // Each office floor (2-10) should have 12 VAVs
        for floor in 2..=10 {
            let rows = doc.query_raw(&format!(
                "SELECT COUNT(*) FROM placements p JOIN product_types pt ON p.product_type_id = pt.id WHERE pt.category = 'vav_box' AND p.level = 'Level {}'", floor
            )).unwrap();
            let count: i64 = rows[0][0].1.parse().unwrap();
            assert_eq!(count, 12, "Floor {} should have 12 VAVs, got {}", floor, count);
        }
    }

    #[test]
    fn test_total_system_cfm() {
        let (doc, _) = create_test_doc();
        // Total VAV CFM for floors 2-4 (served by AHU-1) should be around 25,000
        let rows = doc.query_raw(
            "SELECT SUM(p.cfm) FROM placements p JOIN product_types pt ON p.product_type_id = pt.id WHERE pt.category = 'vav_box' AND p.level IN ('Level 2', 'Level 3', 'Level 4')"
        ).unwrap();
        let total: f64 = rows[0][0].1.parse().unwrap();
        // 36 VAVs, 8x800 + 4x1000 = 10,400 per floor, 3 floors = 31,200
        // But AHU is rated 25,000 CFM — this is a diversity factor issue, which is realistic
        assert!(total > 20000.0, "Floors 2-4 total CFM should be >20000, got {}", total);
    }

    #[test]
    fn test_every_space_has_level() {
        let (doc, _) = create_test_doc();
        let rows = doc.query_raw(
            "SELECT COUNT(*) FROM spaces WHERE level IS NULL OR level = ''"
        ).unwrap();
        let count: i64 = rows[0][0].1.parse().unwrap();
        assert_eq!(count, 0, "Every space must have a level");
    }

    #[test]
    fn test_every_placement_has_status() {
        let (doc, _) = create_test_doc();
        let rows = doc.query_raw(
            "SELECT COUNT(*) FROM placements WHERE status IS NULL OR status = ''"
        ).unwrap();
        let count: i64 = rows[0][0].1.parse().unwrap();
        assert_eq!(count, 0, "Every placement must have a status");
    }
}
