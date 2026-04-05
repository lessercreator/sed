//! Geometry utilities for SED documents.
//!
//! Coordinates are in meters, origin at building datum (southwest corner of Level 1).
//! The SKIMS Americana store is approximately 53' wide x 58' deep (16.2m x 17.7m).

use anyhow::Result;
use crate::document::{SedDocument, generate_id};

pub struct RoomLayout {
    pub tag: &'static str,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

pub const LEVEL_1_ROOMS: &[RoomLayout] = &[
    RoomLayout { tag: "L1-01", x: 3.0,  y: 2.0,  w: 10.0, h: 13.0 },
    RoomLayout { tag: "L1-02", x: 13.5, y: 12.0, w: 2.5,  h: 2.5 },
    RoomLayout { tag: "L1-03", x: 13.5, y: 9.5,  w: 2.5,  h: 2.5 },
    RoomLayout { tag: "L1-04", x: 13.5, y: 7.5,  w: 2.5,  h: 2.0 },
    RoomLayout { tag: "L1-05", x: 13.5, y: 5.5,  w: 2.5,  h: 2.0 },
    RoomLayout { tag: "L1-06", x: 13.5, y: 3.5,  w: 2.5,  h: 2.0 },
    RoomLayout { tag: "L1-07", x: 13.5, y: 1.5,  w: 2.5,  h: 2.0 },
    RoomLayout { tag: "L1-08", x: 13.5, y: -0.5, w: 2.5,  h: 2.0 },
    RoomLayout { tag: "L1-09", x: 13.5, y: -2.5, w: 2.5,  h: 2.0 },
    RoomLayout { tag: "L1-10", x: 0.0,  y: 7.0,  w: 2.5,  h: 3.0 },
    RoomLayout { tag: "L1-11", x: 0.0,  y: 10.5, w: 2.5,  h: 3.0 },
    RoomLayout { tag: "L1-12", x: 0.0,  y: 14.0, w: 3.0,  h: 3.5 },
    RoomLayout { tag: "L1-13", x: 0.0,  y: 13.5, w: 13.0, h: 1.2 },
    RoomLayout { tag: "L1-14", x: 0.0,  y: 4.0,  w: 2.5,  h: 3.0 },
    RoomLayout { tag: "L1-16", x: -2.0, y: 0.0,  w: 1.5,  h: 17.7 },
    RoomLayout { tag: "L1-17", x: 16.5, y: 0.0,  w: 1.5,  h: 17.7 },
];

pub const LEVEL_2_ROOMS: &[RoomLayout] = &[
    RoomLayout { tag: "L2-01", x: 3.0,  y: 2.0,  w: 10.0, h: 5.0 },
    RoomLayout { tag: "L2-02", x: 3.0,  y: 7.5,  w: 10.0, h: 1.5 },
    RoomLayout { tag: "L2-03", x: 3.0,  y: 9.5,  w: 5.0,  h: 4.0 },
    RoomLayout { tag: "L2-04", x: 8.5,  y: 9.5,  w: 1.5,  h: 4.0 },
    RoomLayout { tag: "L2-05", x: 10.5, y: 9.5,  w: 3.0,  h: 4.0 },
    RoomLayout { tag: "L2-06", x: 10.5, y: 14.0, w: 2.0,  h: 2.0 },
    RoomLayout { tag: "L2-07", x: 12.5, y: 14.0, w: 2.0,  h: 2.0 },
    RoomLayout { tag: "L2-08", x: 3.0,  y: 14.0, w: 4.0,  h: 3.0 },
    RoomLayout { tag: "L2-09", x: 7.5,  y: 14.0, w: 3.0,  h: 3.0 },
    RoomLayout { tag: "L2-10", x: 0.5,  y: 9.0,  w: 2.0,  h: 2.5 },
    RoomLayout { tag: "L2-11", x: 0.0,  y: 7.0,  w: 2.5,  h: 2.0 },
    RoomLayout { tag: "L2-12", x: 7.5,  y: 13.0, w: 1.5,  h: 1.0 },
    RoomLayout { tag: "L2-00", x: 0.0,  y: 5.0,  w: 2.5,  h: 2.0 },
];

/// Store room boundaries as polygons in the database AND assign coordinates to spaces/placements.
pub fn populate_skims_geometry(doc: &SedDocument) -> Result<()> {
    let all_rooms: Vec<&RoomLayout> = LEVEL_1_ROOMS.iter().chain(LEVEL_2_ROOMS.iter()).collect();

    for room in &all_rooms {
        let level = if room.tag.starts_with("L1") { "Level 1" } else { "Level 2" };

        // Create polygon for room boundary (4 vertices, rectangular)
        let vertices = pack_rect_vertices(room.x, room.y, room.w, room.h);
        let poly_id = generate_id();
        doc.execute_raw(
            "INSERT INTO geometry_polygons (id, vertices, vertex_count, level) VALUES (?1, ?2, 4, ?3)",
            rusqlite::params![poly_id, vertices, level],
        )?;

        // Find space by tag, update coordinates and boundary link
        let rows = doc.query_raw(&format!(
            "SELECT id FROM spaces WHERE tag = '{}'", room.tag
        ))?;
        if let Some(row) = rows.first() {
            let id = &row[0].1;
            let cx = room.x + room.w / 2.0;
            let cy = room.y + room.h / 2.0;
            doc.update_space(id, "x", Some(&cx.to_string()))?;
            doc.update_space(id, "y", Some(&cy.to_string()))?;
            doc.execute_raw(
                "UPDATE spaces SET boundary_id = ?1 WHERE id = ?2",
                rusqlite::params![poly_id, id],
            )?;
        }
    }

    // Distribute placements within their rooms
    let placements = doc.query_raw(
        "SELECT p.id, s.tag as space_tag FROM placements p LEFT JOIN spaces s ON p.space_id = s.id"
    )?;

    let mut by_room: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for row in &placements {
        let p_id = row[0].1.clone();
        let space_tag = row[1].1.clone();
        if space_tag != "NULL" {
            by_room.entry(space_tag).or_default().push(p_id);
        }
    }

    for (tag, placement_ids) in &by_room {
        if let Some(room) = all_rooms.iter().find(|r| r.tag == *tag) {
            let margin = 0.3;
            let inner_x = room.x + margin;
            let inner_y = room.y + margin;
            let inner_w = room.w - margin * 2.0;
            let inner_h = room.h - margin * 2.0;

            let count = placement_ids.len();
            let cols = ((count as f64).sqrt().ceil()) as usize;
            let rows_count = (count + cols - 1) / cols;

            for (i, p_id) in placement_ids.iter().enumerate() {
                let col = i % cols;
                let row_idx = i / cols;
                let px = inner_x + (col as f64 + 0.5) * inner_w / cols as f64;
                let py = inner_y + (row_idx as f64 + 0.5) * inner_h / rows_count as f64;
                doc.update_placement(p_id, "x", Some(&px.to_string()))?;
                doc.update_placement(p_id, "y", Some(&py.to_string()))?;
            }
        }
    }

    // Populate spatial index for all positioned elements
    populate_spatial_index(doc)?;

    Ok(())
}

/// Register all positioned elements in the R-tree spatial index.
pub fn populate_spatial_index(doc: &SedDocument) -> Result<()> {
    // Clear existing index
    doc.execute_raw("DELETE FROM spatial_idx", rusqlite::params![])?;

    let mut idx: i64 = 1;

    // Spaces (use boundary rectangles)
    let all_rooms: Vec<&RoomLayout> = LEVEL_1_ROOMS.iter().chain(LEVEL_2_ROOMS.iter()).collect();
    for room in &all_rooms {
        doc.execute_raw(
            "INSERT INTO spatial_idx (id, x_min, x_max, y_min, y_max) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![idx, room.x, room.x + room.w, room.y, room.y + room.h],
        )?;
        idx += 1;
    }

    // Placements with coordinates
    let placed = doc.query_raw(
        "SELECT ROWID, x, y FROM placements WHERE x IS NOT NULL AND y IS NOT NULL"
    )?;
    for row in &placed {
        let x: f64 = row[1].1.parse().unwrap_or(0.0);
        let y: f64 = row[2].1.parse().unwrap_or(0.0);
        let r = 0.15; // radius for point elements
        doc.execute_raw(
            "INSERT INTO spatial_idx (id, x_min, x_max, y_min, y_max) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![idx, x - r, x + r, y - r, y + r],
        )?;
        idx += 1;
    }

    // Segments (use endpoint bounding box)
    let segs = doc.query_raw(
        "SELECT seg.id, n1.x, n1.y, n2.x, n2.y
         FROM segments seg
         JOIN nodes n1 ON seg.from_node_id = n1.id
         JOIN nodes n2 ON seg.to_node_id = n2.id
         WHERE n1.x IS NOT NULL AND n2.x IS NOT NULL"
    )?;
    for row in &segs {
        let x1: f64 = row[1].1.parse().unwrap_or(0.0);
        let y1: f64 = row[2].1.parse().unwrap_or(0.0);
        let x2: f64 = row[3].1.parse().unwrap_or(0.0);
        let y2: f64 = row[4].1.parse().unwrap_or(0.0);
        let margin = 0.1;
        doc.execute_raw(
            "INSERT INTO spatial_idx (id, x_min, x_max, y_min, y_max) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![idx, x1.min(x2) - margin, x1.max(x2) + margin, y1.min(y2) - margin, y1.max(y2) + margin],
        )?;
        idx += 1;
    }

    Ok(())
}

/// Pack 4 rectangle vertices into a BLOB (8 f64 values = 64 bytes).
fn pack_rect_vertices(x: f64, y: f64, w: f64, h: f64) -> Vec<u8> {
    let points = [
        x, y,         // bottom-left
        x + w, y,     // bottom-right
        x + w, y + h, // top-right
        x, y + h,     // top-left
    ];
    let mut bytes = Vec::with_capacity(64);
    for p in &points {
        bytes.extend_from_slice(&p.to_le_bytes());
    }
    bytes
}

/// Unpack rectangle vertices from a BLOB.
pub fn unpack_vertices(blob: &[u8]) -> Vec<(f64, f64)> {
    let mut points = Vec::new();
    let mut i = 0;
    while i + 16 <= blob.len() {
        let x = f64::from_le_bytes(blob[i..i+8].try_into().unwrap());
        let y = f64::from_le_bytes(blob[i+8..i+16].try_into().unwrap());
        points.push((x, y));
        i += 16;
    }
    points
}

/// Get the bounding rectangle for all rooms on a given level.
pub fn level_bounds(doc: &SedDocument, level: &str) -> Result<(f64, f64, f64, f64)> {
    let rows = doc.query_raw(&format!(
        "SELECT MIN(x), MIN(y), MAX(x), MAX(y) FROM spaces WHERE level = '{}' AND x IS NOT NULL", level
    ))?;
    if let Some(row) = rows.first() {
        let x_min: f64 = row[0].1.parse().unwrap_or(0.0);
        let y_min: f64 = row[1].1.parse().unwrap_or(0.0);
        let x_max: f64 = row[2].1.parse().unwrap_or(20.0);
        let y_max: f64 = row[3].1.parse().unwrap_or(20.0);
        Ok((x_min, y_min, x_max, y_max))
    } else {
        Ok((0.0, 0.0, 20.0, 20.0))
    }
}

/// Get room layout by tag (for renderer).
pub fn get_room_layout(tag: &str) -> Option<(f64, f64, f64, f64)> {
    let all: Vec<&RoomLayout> = LEVEL_1_ROOMS.iter().chain(LEVEL_2_ROOMS.iter()).collect();
    all.iter().find(|r| r.tag == tag).map(|r| (r.x, r.y, r.w, r.h))
}
