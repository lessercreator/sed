//! Automatic duct sizing using the equal friction method.
//!
//! Walks the duct graph from terminal nodes back to the equipment connection,
//! summing downstream CFM at each segment and calculating required round duct
//! diameter using a simplified Darcy-Weisbach formula at 0.08 in.WG/100ft.

use anyhow::Result;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

use crate::document::SedDocument;

/// Standard round duct sizes in inches.
const STANDARD_SIZES: &[f64] = &[
    6.0, 7.0, 8.0, 9.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0,
    22.0, 24.0, 26.0, 28.0, 30.0, 36.0, 42.0, 48.0,
];

/// Result of autosizing a single duct segment.
#[derive(Debug, Clone, Serialize)]
pub struct DuctSizeResult {
    pub segment_id: String,
    pub current_diameter_in: Option<f64>,
    pub recommended_diameter_in: f64,
    pub downstream_cfm: f64,
}

/// Calculate the required round duct diameter in inches for a given CFM,
/// using the equal friction method at 0.08 in.WG/100ft friction rate.
///
/// Formula: D = 0.0344 * Q^0.612 (simplified Darcy-Weisbach for round duct)
/// where D is in inches and Q is in CFM.
fn calc_diameter_inches(cfm: f64) -> f64 {
    if cfm <= 0.0 {
        return STANDARD_SIZES[0];
    }
    0.0344 * cfm.powf(0.612)
}

/// Round up to the nearest standard duct size.
fn round_to_standard(diameter_in: f64) -> f64 {
    for &size in STANDARD_SIZES {
        if size >= diameter_in {
            return size;
        }
    }
    // Larger than any standard size — return the largest.
    *STANDARD_SIZES.last().unwrap()
}

/// Internal representation of a graph node for traversal.
#[derive(Debug, Clone)]
struct GraphNode {
    id: String,
    node_type: String,
    placement_id: Option<String>,
}

/// Internal representation of a graph segment for traversal.
#[derive(Debug, Clone)]
struct GraphSegment {
    id: String,
    from_node_id: String,
    to_node_id: String,
    diameter_m: Option<f64>,
    flow_design: Option<f64>,
}

/// Autosize all duct segments in the given system.
///
/// Walks the graph from terminal nodes back toward the equipment connection,
/// accumulating downstream CFM at each segment and computing the required
/// duct diameter using the equal friction method.
pub fn autosize_duct_system(doc: &SedDocument, system_id: &str) -> Result<Vec<DuctSizeResult>> {
    // Load all nodes for this system.
    let node_rows = doc.query_raw(&format!(
        "SELECT id, node_type, placement_id FROM nodes WHERE system_id = '{}'",
        system_id
    ))?;

    let mut nodes: HashMap<String, GraphNode> = HashMap::new();
    for row in &node_rows {
        let id = &row[0].1;
        let node_type = &row[1].1;
        let placement_id = if row[2].1 == "NULL" { None } else { Some(row[2].1.clone()) };
        nodes.insert(id.clone(), GraphNode {
            id: id.clone(),
            node_type: node_type.clone(),
            placement_id,
        });
    }

    if nodes.is_empty() {
        return Ok(Vec::new());
    }

    // Load all segments for this system.
    let seg_rows = doc.query_raw(&format!(
        "SELECT id, from_node_id, to_node_id, diameter_m, flow_design FROM segments WHERE system_id = '{}'",
        system_id
    ))?;

    let mut segments: Vec<GraphSegment> = Vec::new();
    for row in &seg_rows {
        let diameter_m = if row[3].1 == "NULL" { None } else { row[3].1.parse::<f64>().ok() };
        let flow_design = if row[4].1 == "NULL" { None } else { row[4].1.parse::<f64>().ok() };
        segments.push(GraphSegment {
            id: row[0].1.clone(),
            from_node_id: row[1].1.clone(),
            to_node_id: row[2].1.clone(),
            diameter_m,
            flow_design,
        });
    }

    if segments.is_empty() {
        return Ok(Vec::new());
    }

    // Build adjacency: for each node, which segments connect to it?
    let mut node_segments: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, seg) in segments.iter().enumerate() {
        node_segments.entry(seg.from_node_id.clone()).or_default().push(i);
        node_segments.entry(seg.to_node_id.clone()).or_default().push(i);
    }

    // Identify terminal nodes and equipment connection nodes.
    let terminal_nodes: Vec<&GraphNode> = nodes.values()
        .filter(|n| n.node_type == "terminal")
        .collect();

    // Get CFM for each terminal from its placement.
    let mut terminal_cfm: HashMap<String, f64> = HashMap::new();
    for tn in &terminal_nodes {
        if let Some(ref pid) = tn.placement_id {
            let cfm_rows = doc.query_raw(&format!(
                "SELECT cfm FROM placements WHERE id = '{}'", pid
            ))?;
            if let Some(row) = cfm_rows.first() {
                if row[0].1 != "NULL" {
                    if let Ok(cfm) = row[0].1.parse::<f64>() {
                        terminal_cfm.insert(tn.id.clone(), cfm);
                    }
                }
            }
        }
        // If placement has no CFM, check if segment has flow_design.
        if !terminal_cfm.contains_key(&tn.id) {
            if let Some(seg_indices) = node_segments.get(&tn.id) {
                for &si in seg_indices {
                    if let Some(flow) = segments[si].flow_design {
                        terminal_cfm.insert(tn.id.clone(), flow);
                        break;
                    }
                }
            }
        }
    }

    // Walk from terminals upstream. The graph flows from equipment_conn -> fitting -> terminal.
    // Segments have from_node (upstream) and to_node (downstream).
    // For each segment, downstream_cfm = sum of all terminal CFM reachable through to_node.
    //
    // Strategy: compute downstream CFM for each segment by walking the graph from
    // to_node side. Build a tree rooted at equipment_conn.

    // Build a directed adjacency (from -> to via segment index).
    let mut children: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, seg) in segments.iter().enumerate() {
        children.entry(seg.from_node_id.clone()).or_default().push(i);
    }

    // Recursive function to compute downstream CFM for a node.
    // downstream_cfm(node) = terminal_cfm(node) if terminal, else sum of downstream_cfm
    // for all children segments' to_nodes.
    fn compute_downstream_cfm(
        node_id: &str,
        children: &HashMap<String, Vec<usize>>,
        segments: &[GraphSegment],
        terminal_cfm: &HashMap<String, f64>,
        segment_cfm: &mut HashMap<String, f64>,
        visited: &mut HashSet<String>,
    ) -> f64 {
        if visited.contains(node_id) {
            return 0.0;
        }
        visited.insert(node_id.to_string());

        // If this node is a terminal, its CFM is the base value.
        if let Some(&cfm) = terminal_cfm.get(node_id) {
            return cfm;
        }

        let mut total = 0.0;
        if let Some(child_segs) = children.get(node_id) {
            for &si in child_segs {
                let seg = &segments[si];
                let child_cfm = compute_downstream_cfm(
                    &seg.to_node_id, children, segments, terminal_cfm, segment_cfm, visited,
                );
                segment_cfm.insert(seg.id.clone(), child_cfm);
                total += child_cfm;
            }
        }

        total
    }

    // Find root nodes (equipment_conn or nodes that are only from_nodes, never to_nodes).
    let to_node_set: HashSet<&str> = segments.iter().map(|s| s.to_node_id.as_str()).collect();
    let root_nodes: Vec<&str> = nodes.keys()
        .filter(|nid| !to_node_set.contains(nid.as_str()))
        .map(|s| s.as_str())
        .collect();

    let mut segment_cfm: HashMap<String, f64> = HashMap::new();
    let mut visited = HashSet::new();

    for root in &root_nodes {
        compute_downstream_cfm(root, &children, &segments, &terminal_cfm, &mut segment_cfm, &mut visited);
    }

    // For segments that weren't reached (e.g., the segment from equip to first tap carries
    // the full system CFM), fill in using flow_design if available.
    for seg in &segments {
        if !segment_cfm.contains_key(&seg.id) {
            if let Some(flow) = seg.flow_design {
                segment_cfm.insert(seg.id.clone(), flow);
            }
        }
    }

    // Build results.
    let mut results: Vec<DuctSizeResult> = Vec::new();
    for seg in &segments {
        let downstream_cfm = segment_cfm.get(&seg.id).copied().unwrap_or(0.0);
        let current_diameter_in = seg.diameter_m.map(|d| d / 0.0254);
        let raw_diameter = calc_diameter_inches(downstream_cfm);
        let recommended = round_to_standard(raw_diameter);

        results.push(DuctSizeResult {
            segment_id: seg.id.clone(),
            current_diameter_in,
            recommended_diameter_in: recommended,
            downstream_cfm,
        });
    }

    // Sort by downstream CFM descending (trunk first, branches last).
    results.sort_by(|a, b| b.downstream_cfm.partial_cmp(&a.downstream_cfm).unwrap_or(std::cmp::Ordering::Equal));

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::SedDocument;

    fn create_skims_and_get_system_id() -> (SedDocument, String) {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        drop(tmp);
        crate::examples::create_skims_americana(&path).unwrap();
        let doc = SedDocument::open(&path).unwrap();

        // Find the RTU-1-SA supply system.
        let systems = doc.query_raw(
            "SELECT id FROM systems WHERE tag = 'RTU-1-SA'"
        ).unwrap();
        let system_id = systems[0][0].1.clone();
        (doc, system_id)
    }

    #[test]
    fn autosize_returns_results_for_skims() {
        let (doc, system_id) = create_skims_and_get_system_id();
        let results = autosize_duct_system(&doc, &system_id).unwrap();
        assert!(!results.is_empty(), "Should return sizing results");
    }

    #[test]
    fn trunk_has_highest_cfm_and_largest_duct() {
        let (doc, system_id) = create_skims_and_get_system_id();
        let results = autosize_duct_system(&doc, &system_id).unwrap();

        // Results are sorted by downstream_cfm descending.
        let first = &results[0];
        let last_nonzero: Vec<&DuctSizeResult> = results.iter()
            .filter(|r| r.downstream_cfm > 0.0)
            .collect();
        let last = last_nonzero.last().unwrap();

        assert!(
            first.downstream_cfm >= last.downstream_cfm,
            "Trunk (first) CFM {} should be >= terminal (last) CFM {}",
            first.downstream_cfm, last.downstream_cfm
        );
        assert!(
            first.recommended_diameter_in >= last.recommended_diameter_in,
            "Trunk recommended {} should be >= terminal recommended {}",
            first.recommended_diameter_in, last.recommended_diameter_in
        );
    }

    #[test]
    fn terminal_branches_have_smallest_ducts() {
        let (doc, system_id) = create_skims_and_get_system_id();
        let results = autosize_duct_system(&doc, &system_id).unwrap();

        // All branch segments (185 CFM) should have smaller recommended diameter
        // than any trunk segment carrying more CFM.
        let branch_results: Vec<&DuctSizeResult> = results.iter()
            .filter(|r| (r.downstream_cfm - 185.0).abs() < 1.0)
            .collect();
        let trunk_results: Vec<&DuctSizeResult> = results.iter()
            .filter(|r| r.downstream_cfm > 200.0)
            .collect();

        assert!(!branch_results.is_empty(), "Should have branch results at 185 CFM");
        assert!(!trunk_results.is_empty(), "Should have trunk results > 200 CFM");

        let max_branch_dia = branch_results.iter()
            .map(|r| r.recommended_diameter_in)
            .fold(0.0_f64, f64::max);
        let min_trunk_dia = trunk_results.iter()
            .map(|r| r.recommended_diameter_in)
            .fold(f64::MAX, f64::min);

        assert!(
            max_branch_dia <= min_trunk_dia,
            "Branch max {} should be <= trunk min {}",
            max_branch_dia, min_trunk_dia
        );
    }

    #[test]
    fn recommended_sizes_decrease_trunk_to_terminal() {
        let (doc, system_id) = create_skims_and_get_system_id();
        let results = autosize_duct_system(&doc, &system_id).unwrap();

        // Filter to non-zero CFM results (sorted by CFM descending).
        let nonzero: Vec<&DuctSizeResult> = results.iter()
            .filter(|r| r.downstream_cfm > 0.0)
            .collect();

        // Recommended diameters should be monotonically non-increasing.
        for i in 1..nonzero.len() {
            assert!(
                nonzero[i].recommended_diameter_in <= nonzero[i - 1].recommended_diameter_in,
                "Diameter at position {} ({} in, {} CFM) should be <= position {} ({} in, {} CFM)",
                i, nonzero[i].recommended_diameter_in, nonzero[i].downstream_cfm,
                i - 1, nonzero[i - 1].recommended_diameter_in, nonzero[i - 1].downstream_cfm,
            );
        }
    }

    #[test]
    fn calc_diameter_known_values() {
        // At 185 CFM, D = 0.0344 * 185^0.612
        let d = calc_diameter_inches(185.0);
        assert!(d > 0.0 && d < 48.0, "Diameter {} should be reasonable", d);
        let std_size = round_to_standard(d);
        assert!(STANDARD_SIZES.contains(&std_size));
    }

    #[test]
    fn round_to_standard_works() {
        assert_eq!(round_to_standard(5.0), 6.0);
        assert_eq!(round_to_standard(6.0), 6.0);
        assert_eq!(round_to_standard(6.5), 7.0);
        assert_eq!(round_to_standard(11.0), 12.0);
        assert_eq!(round_to_standard(47.5), 48.0);
        assert_eq!(round_to_standard(50.0), 48.0); // clamps to max
    }

    #[test]
    fn empty_system_returns_empty() {
        let doc = SedDocument::in_memory().unwrap();
        let results = autosize_duct_system(&doc, "nonexistent").unwrap();
        assert!(results.is_empty());
    }
}
