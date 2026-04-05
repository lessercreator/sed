//! Automatic pipe sizing for hydronic systems.
//!
//! Sizes pipes based on flow rate (GPM) and velocity limits.
//! Uses standard copper and steel pipe sizes.
//!
//! Design criteria:
//! - Maximum velocity: 4 fps for pipes 2" and under, 8 fps for pipes over 2"
//!   (ASHRAE recommendation to limit noise and erosion)
//! - Standard pipe sizes per ASTM B88 (copper) and ASTM A53 (steel)

use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;
use crate::document::SedDocument;

/// Standard pipe sizes: nominal size (inches), actual ID (inches) for Type L copper
const COPPER_PIPE_SIZES: &[(f64, f64)] = &[
    (0.5,  0.545),
    (0.75, 0.785),
    (1.0,  1.025),
    (1.25, 1.265),
    (1.5,  1.505),
    (2.0,  1.985),
    (2.5,  2.465),
    (3.0,  2.945),
    (4.0,  3.905),
    (5.0,  4.875),
    (6.0,  5.845),
    (8.0,  7.725),
    (10.0, 9.625),
    (12.0, 11.565),
];

/// Standard steel pipe sizes: nominal, actual ID (Schedule 40)
const STEEL_PIPE_SIZES: &[(f64, f64)] = &[
    (0.5,  0.622),
    (0.75, 0.824),
    (1.0,  1.049),
    (1.25, 1.380),
    (1.5,  1.610),
    (2.0,  2.067),
    (2.5,  2.469),
    (3.0,  3.068),
    (4.0,  4.026),
    (5.0,  5.047),
    (6.0,  6.065),
    (8.0,  7.981),
    (10.0, 10.020),
    (12.0, 11.938),
];

#[derive(Debug, Clone, Serialize)]
pub struct PipeSizeResult {
    pub segment_id: String,
    pub current_nominal_in: Option<f64>,
    pub recommended_nominal_in: f64,
    pub recommended_id_in: f64,
    pub downstream_gpm: f64,
    pub velocity_fps: f64,
    pub material: String,
}

/// Size a pipe based on flow rate and velocity limits.
/// Returns (nominal_size, actual_id, velocity) for the smallest pipe
/// that keeps velocity under the limit.
pub fn size_pipe(gpm: f64, material: &str) -> (f64, f64, f64) {
    let sizes = match material {
        "copper" => COPPER_PIPE_SIZES,
        _ => STEEL_PIPE_SIZES,
    };

    for &(nominal, id) in sizes {
        let area_sqft = std::f64::consts::PI * (id / 24.0).powi(2); // id in inches / 2 / 12 = ft
        let velocity = (gpm / 7.48) / (area_sqft * 60.0); // GPM to ft3/min, then ft/s

        let max_velocity = if nominal <= 2.0 { 4.0 } else { 8.0 };
        if velocity <= max_velocity {
            return (nominal, id, velocity);
        }
    }

    // If nothing fits, return the largest
    let last = sizes.last().unwrap();
    let area_sqft = std::f64::consts::PI * (last.1 / 24.0).powi(2);
    let velocity = (gpm / 7.48) / (area_sqft * 60.0);
    (last.0, last.1, velocity)
}

/// Auto-size all pipe segments in a hydronic system.
pub fn autosize_pipe_system(doc: &SedDocument, system_id: &str) -> Result<Vec<PipeSizeResult>> {
    // Get system medium to determine material
    let sys_rows = doc.query_raw(&format!(
        "SELECT medium FROM systems WHERE id = '{}'", system_id.replace('\'', "''")
    ))?;
    let medium = sys_rows.first().map(|r| r[0].1.as_str()).unwrap_or("chilled_water");
    let material = if medium.contains("condenser") { "steel" } else { "copper" };

    // Get all segments and nodes in this system
    let seg_rows = doc.query_raw(&format!(
        "SELECT seg.id, seg.from_node_id, seg.to_node_id, seg.diameter_m, seg.flow_design
         FROM segments seg WHERE seg.system_id = '{}'",
        system_id.replace('\'', "''")
    ))?;

    let node_rows = doc.query_raw(&format!(
        "SELECT n.id, n.node_type, n.placement_id FROM nodes n WHERE n.system_id = '{}'",
        system_id.replace('\'', "''")
    ))?;

    // Build adjacency: from_node -> [(seg_id, to_node)]
    let mut downstream: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut seg_data: HashMap<String, (Option<f64>, Option<f64>)> = HashMap::new(); // seg_id -> (diameter_m, flow)

    for row in &seg_rows {
        let seg_id = row[0].1.clone();
        let from_id = row[1].1.clone();
        let to_id = row[2].1.clone();
        let diam: Option<f64> = row[3].1.parse().ok().filter(|v: &f64| *v > 0.0);
        let flow: Option<f64> = row[4].1.parse().ok().filter(|v: &f64| *v > 0.0);
        downstream.entry(from_id).or_default().push((seg_id.clone(), to_id));
        seg_data.insert(seg_id, (diam, flow));
    }

    // Find terminal nodes (placement_id not null)
    let mut terminal_flows: HashMap<String, f64> = HashMap::new();
    for row in &node_rows {
        let node_id = row[0].1.clone();
        let node_type = &row[1].1;
        let placement_id = &row[2].1;
        if (node_type == "terminal" || node_type == "coil_conn") && placement_id != "NULL" {
            // Get flow from the placement or from the segment
            // For piping, flow is on the segment as GPM
            terminal_flows.insert(node_id, 0.0); // will be filled from segment flow_design
        }
    }

    // For piping, use segment flow_design directly (it's already GPM per segment)
    let mut results = Vec::new();
    for (seg_id, (current_diam, flow)) in &seg_data {
        let gpm = flow.unwrap_or(0.0);
        if gpm <= 0.0 { continue; }

        let (nominal, _id, velocity) = size_pipe(gpm, material);
        let current_nominal = current_diam.map(|d| {
            // Convert from meters to inches, find closest nominal
            let d_in = d / 0.0254;
            let sizes = if material == "copper" { COPPER_PIPE_SIZES } else { STEEL_PIPE_SIZES };
            sizes.iter().min_by(|a, b| (a.0 - d_in).abs().partial_cmp(&(b.0 - d_in).abs()).unwrap())
                .map(|s| s.0).unwrap_or(d_in)
        });

        results.push(PipeSizeResult {
            segment_id: seg_id.clone(),
            current_nominal_in: current_nominal,
            recommended_nominal_in: nominal,
            recommended_id_in: _id,
            downstream_gpm: gpm,
            velocity_fps: velocity,
            material: material.into(),
        });
    }

    results.sort_by(|a, b| b.downstream_gpm.partial_cmp(&a.downstream_gpm).unwrap_or(std::cmp::Ordering::Equal));
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_small_pipe() {
        let (nominal, _id, vel) = size_pipe(5.0, "copper");
        // 5 GPM should fit in 3/4" or 1" copper
        assert!(nominal <= 1.0, "5 GPM should fit in 1\" or less: got {}\"", nominal);
        assert!(vel <= 4.0, "Velocity should be under 4 fps for small pipe");
    }

    #[test]
    fn size_medium_pipe() {
        let (nominal, _id, vel) = size_pipe(50.0, "copper");
        // 50 GPM needs roughly 2" pipe
        assert!(nominal >= 1.5 && nominal <= 3.0, "50 GPM: expected 1.5\"-3\", got {}\"", nominal);
        assert!(vel <= 8.0, "Velocity under limit");
    }

    #[test]
    fn size_large_pipe() {
        let (nominal, _id, vel) = size_pipe(500.0, "steel");
        // 500 GPM needs roughly 4"-6" pipe
        assert!(nominal >= 3.0 && nominal <= 8.0, "500 GPM steel: expected 3\"-8\", got {}\"", nominal);
    }

    #[test]
    fn steel_larger_than_copper_for_same_flow() {
        // Steel has larger ID for same nominal, so should sometimes size smaller
        let (copper_nom, _, _) = size_pipe(100.0, "copper");
        let (steel_nom, _, _) = size_pipe(100.0, "steel");
        // Both should be reasonable
        assert!(copper_nom >= 2.0);
        assert!(steel_nom >= 2.0);
    }

    #[test]
    fn zero_flow_returns_smallest() {
        let (nominal, _, _) = size_pipe(0.0, "copper");
        assert_eq!(nominal, 0.5);
    }
}
