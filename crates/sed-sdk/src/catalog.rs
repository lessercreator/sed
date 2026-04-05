//! Built-in equipment catalog with real manufacturer data.
//!
//! Populates a SedDocument's product_types table with a standard HVAC
//! equipment catalog so that a blank project has equipment to place immediately.

use anyhow::Result;

use crate::document::{generate_id, SedDocument};
use crate::types::ProductType;

/// A catalog entry definition. Fields map directly to ProductType columns.
struct CatalogEntry {
    tag: &'static str,
    domain: &'static str,
    category: &'static str,
    manufacturer: &'static str,
    model: &'static str,
    description: &'static str,
    mounting: Option<&'static str>,
}

impl CatalogEntry {
    const fn new(
        tag: &'static str,
        domain: &'static str,
        category: &'static str,
        manufacturer: &'static str,
        model: &'static str,
        description: &'static str,
        mounting: Option<&'static str>,
    ) -> Self {
        Self { tag, domain, category, manufacturer, model, description, mounting }
    }
}

/// The full default catalog.
static CATALOG: &[CatalogEntry] = &[
    // =========================================================================
    // AIR DEVICES — Supply Diffusers
    // =========================================================================
    CatalogEntry::new(
        "SD", "air_device", "supply_diffuser",
        "Titus", "OMNI",
        "Square ceiling diffuser, 4-way throw, steel face with opposed-blade damper",
        Some("ceiling"),
    ),
    CatalogEntry::new(
        "RD", "air_device", "supply_diffuser",
        "Titus", "TMR",
        "Round ceiling diffuser, radial throw, aluminum cone with butterfly damper",
        Some("ceiling"),
    ),
    CatalogEntry::new(
        "LD", "air_device", "supply_diffuser",
        "Titus", "FL-10",
        "Linear slot diffuser, 1-slot, continuous length with plenum box",
        Some("ceiling"),
    ),
    CatalogEntry::new(
        "LD-2", "air_device", "supply_diffuser",
        "Titus", "FL-20",
        "Linear slot diffuser, 2-slot, continuous length with plenum box",
        Some("ceiling"),
    ),
    CatalogEntry::new(
        "PFD", "air_device", "supply_diffuser",
        "Titus", "PAS",
        "Perforated face ceiling diffuser, 24x24 lay-in, steel face with neck damper",
        Some("ceiling"),
    ),
    CatalogEntry::new(
        "SD-J", "air_device", "supply_diffuser",
        "Price", "JBD",
        "Jet nozzle diffuser, adjustable pattern, high-induction for large spaces",
        Some("ceiling"),
    ),

    // =========================================================================
    // AIR DEVICES — Supply Registers
    // =========================================================================
    CatalogEntry::new(
        "SR", "air_device", "supply_register",
        "Titus", "300FL",
        "Double deflection supply register, horizontal blades front, vertical rear",
        Some("sidewall"),
    ),
    CatalogEntry::new(
        "SR-S", "air_device", "supply_register",
        "Titus", "S300FL",
        "Stamped face double deflection supply register, steel",
        Some("sidewall"),
    ),
    CatalogEntry::new(
        "SR-D", "air_device", "supply_register",
        "Titus", "300D",
        "Duct-mounted supply register with opposed-blade damper",
        Some("duct"),
    ),

    // =========================================================================
    // AIR DEVICES — Return Grilles
    // =========================================================================
    CatalogEntry::new(
        "RG", "air_device", "return_grille",
        "Titus", "350FL",
        "Fixed-blade return grille, 45-degree blades, steel with flange frame",
        Some("ceiling"),
    ),
    CatalogEntry::new(
        "RG-E", "air_device", "return_grille",
        "Titus", "EGG",
        "Egg crate return grille, 1/2 x 1/2 x 1/2 aluminum core",
        Some("ceiling"),
    ),
    CatalogEntry::new(
        "RG-P", "air_device", "return_grille",
        "Price", "PFG",
        "Perforated face return grille, 24x24 lay-in, aluminum",
        Some("ceiling"),
    ),
    CatalogEntry::new(
        "RG-S", "air_device", "return_grille",
        "Titus", "350RL",
        "Fixed-blade return grille, steel, sidewall mount with filter rack",
        Some("sidewall"),
    ),

    // =========================================================================
    // AIR DEVICES — Exhaust Registers
    // =========================================================================
    CatalogEntry::new(
        "ER", "air_device", "exhaust_register",
        "Titus", "RCA-DS",
        "Exhaust register, adjustable pattern, steel with volume damper",
        Some("ceiling"),
    ),
    CatalogEntry::new(
        "ER-S", "air_device", "exhaust_register",
        "Titus", "300EX",
        "Sidewall exhaust register, fixed blade, steel",
        Some("sidewall"),
    ),

    // =========================================================================
    // AIR DEVICES — Transfer Grilles
    // =========================================================================
    CatalogEntry::new(
        "TG", "air_device", "transfer_grille",
        "Ruskin", "EME520DD",
        "Door transfer grille, double-deflection blades, steel frame",
        Some("door"),
    ),
    CatalogEntry::new(
        "TG-W", "air_device", "transfer_grille",
        "Ruskin", "EME520",
        "Wall transfer grille, fixed V-blade, sight-proof, steel frame",
        Some("wall"),
    ),

    // =========================================================================
    // AIR DEVICES — VAV Terminals
    // =========================================================================
    CatalogEntry::new(
        "VAV", "air_device", "vav_terminal",
        "Trane", "DERA",
        "Single-duct VAV terminal, pressure-independent, with hot water reheat coil",
        None,
    ),
    CatalogEntry::new(
        "VAV-C", "air_device", "vav_terminal",
        "Trane", "DERC",
        "Single-duct VAV terminal, cooling only, pressure-independent",
        None,
    ),
    CatalogEntry::new(
        "VAV-FP", "air_device", "vav_terminal",
        "Trane", "DERP",
        "Fan-powered VAV terminal, parallel, ECM motor with hot water reheat",
        None,
    ),

    // =========================================================================
    // EQUIPMENT — Rooftop Units
    // =========================================================================
    CatalogEntry::new(
        "RTU", "equipment", "rooftop_unit",
        "Trane", "IntelliPak",
        "Packaged rooftop unit, DX cooling, gas heating, variable speed supply fan",
        Some("roof"),
    ),
    CatalogEntry::new(
        "RTU-C", "equipment", "rooftop_unit",
        "Carrier", "48/50XC WeatherMaker",
        "Packaged rooftop unit, DX cooling, gas/electric heat, economizer",
        Some("roof"),
    ),
    CatalogEntry::new(
        "RTU-L", "equipment", "rooftop_unit",
        "Lennox", "Energence",
        "High-efficiency packaged rooftop unit, variable speed compressor, IEER 21+",
        Some("roof"),
    ),

    // =========================================================================
    // EQUIPMENT — Air Handling Units
    // =========================================================================
    CatalogEntry::new(
        "AHU", "equipment", "air_handling_unit",
        "Trane", "IntelliPak SHC",
        "Custom air handling unit, chilled water coil, VFD supply and return fans",
        Some("mechanical_room"),
    ),
    CatalogEntry::new(
        "AHU-C", "equipment", "air_handling_unit",
        "Carrier", "39M AeroAcoustic",
        "Central station air handling unit, draw-through, double-wall insulated casing",
        Some("mechanical_room"),
    ),

    // =========================================================================
    // EQUIPMENT — Split Systems
    // =========================================================================
    CatalogEntry::new(
        "SS", "equipment", "split_system",
        "Daikin", "DX20VC",
        "Split system condensing unit, variable speed inverter compressor, R-410A",
        Some("grade"),
    ),
    CatalogEntry::new(
        "SS-M", "equipment", "split_system",
        "Mitsubishi", "MXZ",
        "Multi-zone mini-split condensing unit, inverter driven, R-410A",
        Some("grade"),
    ),
    CatalogEntry::new(
        "FCU", "equipment", "fan_coil",
        "Daikin", "FTX",
        "Wall-mounted fan coil unit, multi-speed EC fan, with drain pump",
        Some("wall"),
    ),

    // =========================================================================
    // EQUIPMENT — Chillers
    // =========================================================================
    CatalogEntry::new(
        "CH", "equipment", "chiller",
        "Trane", "CenTraVac",
        "Centrifugal water-cooled chiller, variable speed drive, R-123 or R-514A",
        Some("mechanical_room"),
    ),
    CatalogEntry::new(
        "CH-Y", "equipment", "chiller",
        "Johnson Controls / York", "YK",
        "Centrifugal water-cooled chiller, single stage, R-134a",
        Some("mechanical_room"),
    ),

    // =========================================================================
    // EQUIPMENT — Boilers
    // =========================================================================
    CatalogEntry::new(
        "B", "equipment", "boiler",
        "Aerco", "BMK",
        "Condensing hot water boiler, modulating gas burner, 95%+ thermal efficiency",
        Some("mechanical_room"),
    ),
    CatalogEntry::new(
        "B-L", "equipment", "boiler",
        "Lochinvar", "CREST",
        "Condensing fire-tube boiler, 10:1 turndown, stainless steel heat exchanger",
        Some("mechanical_room"),
    ),

    // =========================================================================
    // EQUIPMENT — Pumps
    // =========================================================================
    CatalogEntry::new(
        "P", "equipment", "pump",
        "Bell & Gossett", "e-1510",
        "End-suction centrifugal pump, base-mounted, close-coupled, cast iron",
        Some("mechanical_room"),
    ),
    CatalogEntry::new(
        "P-G", "equipment", "pump",
        "Grundfos", "NB/NBE",
        "End-suction centrifugal pump, variable speed, integrated VFD",
        Some("mechanical_room"),
    ),
    CatalogEntry::new(
        "P-IL", "equipment", "pump",
        "Bell & Gossett", "e-80",
        "Inline centrifugal pump, maintenance-free seal, cast iron volute",
        Some("mechanical_room"),
    ),

    // =========================================================================
    // EQUIPMENT — Cooling Towers
    // =========================================================================
    CatalogEntry::new(
        "CT", "equipment", "cooling_tower",
        "BAC", "Series 3000",
        "Induced-draft crossflow cooling tower, FRP casing, low-sound fan",
        Some("roof"),
    ),

    // =========================================================================
    // EQUIPMENT — Exhaust Fans
    // =========================================================================
    CatalogEntry::new(
        "EF", "equipment", "exhaust_fan",
        "Greenheck", "SQ",
        "Square inline centrifugal exhaust fan, direct drive, aluminum wheel",
        Some("roof"),
    ),
    CatalogEntry::new(
        "EF-B", "equipment", "exhaust_fan",
        "Broan-NuTone", "L-Series",
        "Ceiling-mounted exhaust fan, low sone, for restroom/utility",
        Some("ceiling"),
    ),
    CatalogEntry::new(
        "EF-W", "equipment", "exhaust_fan",
        "Greenheck", "CSP",
        "Ceiling/cabinet exhaust fan, spun aluminum housing, kitchen/lab rated",
        Some("ceiling"),
    ),

    // =========================================================================
    // EQUIPMENT — Energy Recovery
    // =========================================================================
    CatalogEntry::new(
        "ERV", "equipment", "energy_recovery",
        "Carrier", "50XT",
        "Energy recovery ventilator, enthalpy wheel, DOAS-ready controls",
        Some("mechanical_room"),
    ),
    CatalogEntry::new(
        "ERV-R", "equipment", "energy_recovery",
        "RenewAire", "EV Premium",
        "Static-plate energy recovery ventilator, no moving parts, 80%+ effectiveness",
        Some("mechanical_room"),
    ),

    // =========================================================================
    // ACCESSORIES — Dampers
    // =========================================================================
    CatalogEntry::new(
        "FSD", "accessory", "fire_smoke_damper",
        "Pottorff", "FSD-352",
        "Combination fire/smoke damper, UL 555 & 555S, curtain type, with actuator",
        Some("duct"),
    ),
    CatalogEntry::new(
        "FD", "accessory", "fire_damper",
        "Pottorff", "FD-150",
        "Fire damper, curtain type, 1-1/2 hour rated, UL 555",
        Some("duct"),
    ),
    CatalogEntry::new(
        "MVD", "accessory", "manual_volume_damper",
        "Ruskin", "CD50",
        "Manual volume damper, opposed-blade, galvanized steel, locking quadrant",
        Some("duct"),
    ),
    CatalogEntry::new(
        "BDD", "accessory", "backdraft_damper",
        "Ruskin", "CBD6",
        "Backdraft damper, gravity-operated, aluminum blades, rubber seals",
        Some("duct"),
    ),
    CatalogEntry::new(
        "SMKD", "accessory", "smoke_detector",
        "System Sensor", "D4120",
        "Duct-mounted smoke detector, photoelectric, with relay base and housing",
        Some("duct"),
    ),
    CatalogEntry::new(
        "AD", "accessory", "access_door",
        "Ductmate", "Tabbed-Style",
        "Duct access door, hinged, insulated, 8x8 through 24x24, galvanized steel",
        Some("duct"),
    ),
];

/// Populate the product_types table with the default HVAC equipment catalog.
///
/// Inserts all catalog entries into the document. Existing product types are
/// not affected — the function only appends. Returns the number of product
/// types inserted.
pub fn populate_default_catalog(doc: &SedDocument) -> Result<usize> {
    let mut count = 0;
    for entry in CATALOG {
        let pt = ProductType {
            id: generate_id(),
            tag: entry.tag.to_string(),
            domain: entry.domain.to_string(),
            category: entry.category.to_string(),
            manufacturer: Some(entry.manufacturer.to_string()),
            model: Some(entry.model.to_string()),
            description: Some(entry.description.to_string()),
            mounting: entry.mounting.map(|s| s.to_string()),
            finish: None,
            size_nominal: None,
            voltage: None,
            phase: None,
            hz: None,
            submittal_id: None,
        };
        doc.add_product_type(&pt)?;
        count += 1;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_at_least_30_types() {
        assert!(
            CATALOG.len() >= 30,
            "Catalog should have at least 30 entries, found {}",
            CATALOG.len()
        );
    }

    #[test]
    fn populate_default_catalog_inserts_all() {
        let doc = SedDocument::in_memory().unwrap();
        let count = populate_default_catalog(&doc).unwrap();
        assert!(count >= 30, "Expected at least 30 product types, got {}", count);

        let actual = doc.count("product_types").unwrap();
        assert_eq!(actual as usize, count);
    }

    #[test]
    fn catalog_covers_all_domains() {
        let domains: Vec<&str> = CATALOG.iter().map(|e| e.domain).collect();
        assert!(domains.contains(&"air_device"), "Missing air_device domain");
        assert!(domains.contains(&"equipment"), "Missing equipment domain");
        assert!(domains.contains(&"accessory"), "Missing accessory domain");
    }

    #[test]
    fn catalog_entries_have_required_fields() {
        for entry in CATALOG {
            assert!(!entry.tag.is_empty(), "Entry has empty tag");
            assert!(!entry.domain.is_empty(), "Entry has empty domain");
            assert!(!entry.category.is_empty(), "Entry has empty category");
            assert!(!entry.manufacturer.is_empty(), "Entry has empty manufacturer");
            assert!(!entry.model.is_empty(), "Entry has empty model");
            assert!(!entry.description.is_empty(), "Entry has empty description");
        }
    }

    #[test]
    fn catalog_tags_are_unique() {
        let mut tags: Vec<&str> = CATALOG.iter().map(|e| e.tag).collect();
        tags.sort();
        let before = tags.len();
        tags.dedup();
        assert_eq!(before, tags.len(), "Duplicate tags found in catalog");
    }

    #[test]
    fn roundtrip_catalog_product_types() {
        let doc = SedDocument::in_memory().unwrap();
        populate_default_catalog(&doc).unwrap();
        let types = doc.list_product_types().unwrap();

        // Verify a known entry
        let omni = types.iter().find(|t| t.tag == "SD").unwrap();
        assert_eq!(omni.manufacturer.as_deref(), Some("Titus"));
        assert_eq!(omni.model.as_deref(), Some("OMNI"));
        assert_eq!(omni.domain, "air_device");
    }
}
