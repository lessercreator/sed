use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::Serialize;
use uuid::Uuid;

use crate::schema;
use crate::types::*;

pub fn generate_id() -> String {
    Uuid::new_v4().to_string()
}

pub struct SedDocument {
    pub(crate) conn: Connection,
}

impl SedDocument {
    /// Create a new .sed file at the given path.
    pub fn create(path: &str) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("Failed to create SED file: {}", path))?;
        schema::create_schema(&conn)?;
        Ok(SedDocument { conn })
    }

    /// Open an existing .sed file.
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open SED file: {}", path))?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        Ok(SedDocument { conn })
    }

    /// Create an in-memory SED document (for testing).
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        schema::create_schema(&conn)?;
        Ok(SedDocument { conn })
    }

    // =========================================================================
    // META
    // =========================================================================

    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
            rusqlite::params![key, value],
        )?;
        Ok(())
    }

    pub fn get_meta(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare("SELECT value FROM meta WHERE key = ?1")?;
        match stmt.query_row(rusqlite::params![key], |row| row.get(0)) {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    // =========================================================================
    // DIRECTORY
    // =========================================================================

    pub fn add_directory_entry(&self, entry: &DirectoryEntry) -> Result<()> {
        self.conn.execute(
            "INSERT INTO directory (id, role, company, contact, email, phone, address)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                entry.id, entry.role, entry.company, entry.contact,
                entry.email, entry.phone, entry.address
            ],
        )?;
        Ok(())
    }

    pub fn list_directory(&self) -> Result<Vec<DirectoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, role, company, contact, email, phone, address FROM directory ORDER BY role"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(DirectoryEntry {
                id: row.get(0)?, role: row.get(1)?, company: row.get(2)?,
                contact: row.get(3)?, email: row.get(4)?, phone: row.get(5)?, address: row.get(6)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    // =========================================================================
    // SPACES
    // =========================================================================

    pub fn add_space(&self, space: &Space) -> Result<()> {
        self.conn.execute(
            "INSERT INTO spaces (id, tag, name, level, space_type, area_m2, ceiling_ht_m, scope, parent_id, boundary_id, x, y)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            rusqlite::params![
                space.id, space.tag, space.name, space.level, space.space_type,
                space.area_m2, space.ceiling_ht_m, space.scope, space.parent_id,
                space.boundary_id, space.x, space.y
            ],
        )?;
        Ok(())
    }

    pub fn get_space(&self, id: &str) -> Result<Option<Space>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, tag, name, level, space_type, area_m2, ceiling_ht_m, scope, parent_id, boundary_id, x, y FROM spaces WHERE id = ?1"
        )?;
        match stmt.query_row(rusqlite::params![id], |row| {
            Ok(Space {
                id: row.get(0)?, tag: row.get(1)?, name: row.get(2)?, level: row.get(3)?,
                space_type: row.get(4)?, area_m2: row.get(5)?, ceiling_ht_m: row.get(6)?,
                scope: row.get(7)?, parent_id: row.get(8)?, boundary_id: row.get(9)?,
                x: row.get(10)?, y: row.get(11)?,
            })
        }) {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list_spaces(&self) -> Result<Vec<Space>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, tag, name, level, space_type, area_m2, ceiling_ht_m, scope, parent_id, boundary_id, x, y FROM spaces ORDER BY level, tag"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Space {
                id: row.get(0)?, tag: row.get(1)?, name: row.get(2)?, level: row.get(3)?,
                space_type: row.get(4)?, area_m2: row.get(5)?, ceiling_ht_m: row.get(6)?,
                scope: row.get(7)?, parent_id: row.get(8)?, boundary_id: row.get(9)?,
                x: row.get(10)?, y: row.get(11)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn update_space(&self, id: &str, field: &str, value: Option<&str>) -> Result<usize> {
        let allowed = ["tag", "name", "level", "space_type", "scope", "x", "y", "area_m2", "ceiling_ht_m"];
        if !allowed.contains(&field) {
            anyhow::bail!("Field '{}' not allowed for space update", field);
        }
        let sql = format!("UPDATE spaces SET {} = ?1 WHERE id = ?2", field);
        Ok(self.conn.execute(&sql, rusqlite::params![value, id])?)
    }

    pub fn delete_space(&self, id: &str) -> Result<usize> {
        Ok(self.conn.execute("DELETE FROM spaces WHERE id = ?1", rusqlite::params![id])?)
    }

    // =========================================================================
    // PRODUCT TYPES
    // =========================================================================

    pub fn add_product_type(&self, pt: &ProductType) -> Result<()> {
        self.conn.execute(
            "INSERT INTO product_types (id, tag, domain, category, manufacturer, model, description, mounting, finish, size_nominal, voltage, phase, hz, submittal_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            rusqlite::params![
                pt.id, pt.tag, pt.domain, pt.category, pt.manufacturer, pt.model,
                pt.description, pt.mounting, pt.finish, pt.size_nominal,
                pt.voltage, pt.phase, pt.hz, pt.submittal_id
            ],
        )?;
        Ok(())
    }

    pub fn list_product_types(&self) -> Result<Vec<ProductType>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, tag, domain, category, manufacturer, model, description, mounting, finish, size_nominal, voltage, phase, hz, submittal_id FROM product_types ORDER BY tag"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ProductType {
                id: row.get(0)?, tag: row.get(1)?, domain: row.get(2)?, category: row.get(3)?,
                manufacturer: row.get(4)?, model: row.get(5)?, description: row.get(6)?,
                mounting: row.get(7)?, finish: row.get(8)?, size_nominal: row.get(9)?,
                voltage: row.get(10)?, phase: row.get(11)?, hz: row.get(12)?, submittal_id: row.get(13)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn delete_product_type(&self, id: &str) -> Result<usize> {
        Ok(self.conn.execute("DELETE FROM product_types WHERE id = ?1", rusqlite::params![id])?)
    }

    // =========================================================================
    // PLACEMENTS
    // =========================================================================

    pub fn add_placement(&self, p: &Placement) -> Result<()> {
        self.conn.execute(
            "INSERT INTO placements (id, instance_tag, product_type_id, space_id, level, x, y, rotation, cfm, cfm_balanced, static_pressure_pa, status, scope, phase, weight_kg, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            rusqlite::params![
                p.id, p.instance_tag, p.product_type_id, p.space_id, p.level, p.x, p.y, p.rotation,
                p.cfm, p.cfm_balanced, p.static_pressure_pa, p.status, p.scope,
                p.phase, p.weight_kg, p.notes
            ],
        )?;
        Ok(())
    }

    pub fn list_placements(&self) -> Result<Vec<Placement>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, instance_tag, product_type_id, space_id, level, x, y, rotation, cfm, cfm_balanced, static_pressure_pa, status, scope, phase, weight_kg, notes FROM placements ORDER BY level"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Placement {
                id: row.get(0)?, instance_tag: row.get(1)?, product_type_id: row.get(2)?, space_id: row.get(3)?,
                level: row.get(4)?, x: row.get(5)?, y: row.get(6)?, rotation: row.get(7)?,
                cfm: row.get(8)?, cfm_balanced: row.get(9)?, static_pressure_pa: row.get(10)?,
                status: row.get(11)?, scope: row.get(12)?, phase: row.get(13)?,
                weight_kg: row.get(14)?, notes: row.get(15)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn update_placement(&self, id: &str, field: &str, value: Option<&str>) -> Result<usize> {
        let allowed = ["instance_tag", "cfm", "cfm_balanced", "status", "phase", "scope", "x", "y", "rotation", "space_id", "notes"];
        if !allowed.contains(&field) {
            anyhow::bail!("Field '{}' not allowed for placement update", field);
        }
        let sql = format!("UPDATE placements SET {} = ?1 WHERE id = ?2", field);
        Ok(self.conn.execute(&sql, rusqlite::params![value, id])?)
    }

    pub fn delete_placement(&self, id: &str) -> Result<usize> {
        Ok(self.conn.execute("DELETE FROM placements WHERE id = ?1", rusqlite::params![id])?)
    }

    // =========================================================================
    // SYSTEMS
    // =========================================================================

    pub fn add_system(&self, sys: &System) -> Result<()> {
        self.conn.execute(
            "INSERT INTO systems (id, tag, name, system_type, medium, source_id, paired_system_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![sys.id, sys.tag, sys.name, sys.system_type, sys.medium, sys.source_id, sys.paired_system_id],
        )?;
        Ok(())
    }

    pub fn list_systems(&self) -> Result<Vec<System>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, tag, name, system_type, medium, source_id, paired_system_id FROM systems ORDER BY tag"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(System {
                id: row.get(0)?, tag: row.get(1)?, name: row.get(2)?,
                system_type: row.get(3)?, medium: row.get(4)?, source_id: row.get(5)?,
                paired_system_id: row.get(6)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    // =========================================================================
    // PLACEMENT-SYSTEM MEMBERSHIP
    // =========================================================================

    pub fn add_placement_system(&self, ps: &PlacementSystem) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO placement_systems (placement_id, system_id, role)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![ps.placement_id, ps.system_id, ps.role],
        )?;
        Ok(())
    }

    pub fn list_placement_systems(&self, placement_id: &str) -> Result<Vec<PlacementSystem>> {
        let mut stmt = self.conn.prepare(
            "SELECT placement_id, system_id, role FROM placement_systems WHERE placement_id = ?1"
        )?;
        let rows = stmt.query_map(rusqlite::params![placement_id], |row| {
            Ok(PlacementSystem {
                placement_id: row.get(0)?, system_id: row.get(1)?, role: row.get(2)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    // =========================================================================
    // NODES
    // =========================================================================

    pub fn add_node(&self, node: &Node) -> Result<()> {
        self.conn.execute(
            "INSERT INTO nodes (id, system_id, node_type, placement_id, fitting_type, size_description, level, x, y)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                node.id, node.system_id, node.node_type, node.placement_id,
                node.fitting_type, node.size_description, node.level, node.x, node.y
            ],
        )?;
        Ok(())
    }

    // =========================================================================
    // SEGMENTS
    // =========================================================================

    pub fn add_segment(&self, seg: &Segment) -> Result<()> {
        self.conn.execute(
            "INSERT INTO segments (id, system_id, from_node_id, to_node_id, shape, width_m, height_m, diameter_m, length_m, material, gauge, pressure_class, construction, exposure, flow_design, flow_balanced, status, scope)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            rusqlite::params![
                seg.id, seg.system_id, seg.from_node_id, seg.to_node_id,
                seg.shape, seg.width_m, seg.height_m, seg.diameter_m, seg.length_m,
                seg.material, seg.gauge, seg.pressure_class, seg.construction,
                seg.exposure, seg.flow_design, seg.flow_balanced, seg.status, seg.scope
            ],
        )?;
        Ok(())
    }

    // =========================================================================
    // SHEETS
    // =========================================================================

    pub fn add_sheet(&self, sheet: &Sheet) -> Result<()> {
        self.conn.execute(
            "INSERT INTO sheets (id, number, title, discipline, sheet_size)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![sheet.id, sheet.number, sheet.title, sheet.discipline, sheet.sheet_size],
        )?;
        Ok(())
    }

    // =========================================================================
    // VIEWS
    // =========================================================================

    pub fn add_view(&self, v: &View) -> Result<()> {
        self.conn.execute(
            "INSERT INTO views (id, sheet_id, view_type, title, scale, level, vp_x, vp_y, vp_width, vp_height, model_x_min, model_y_min, model_x_max, model_y_max)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            rusqlite::params![
                v.id, v.sheet_id, v.view_type, v.title, v.scale, v.level,
                v.vp_x, v.vp_y, v.vp_width, v.vp_height,
                v.model_x_min, v.model_y_min, v.model_x_max, v.model_y_max
            ],
        )?;
        Ok(())
    }

    // =========================================================================
    // INSULATION
    // =========================================================================

    pub fn add_insulation(&self, ins: &Insulation) -> Result<()> {
        self.conn.execute(
            "INSERT INTO insulation (id, segment_id, type, manufacturer, product, thickness_m, r_value, facing, code_reference)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                ins.id, ins.segment_id, ins.insulation_type, ins.manufacturer,
                ins.product, ins.thickness_m, ins.r_value, ins.facing, ins.code_reference
            ],
        )?;
        Ok(())
    }

    // =========================================================================
    // GENERAL NOTES
    // =========================================================================

    pub fn add_general_note(&self, id: &str, discipline: Option<&str>, text: &str, sort_order: i32) -> Result<()> {
        self.conn.execute(
            "INSERT INTO general_notes (id, discipline, text, sort_order)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, discipline, text, sort_order],
        )?;
        Ok(())
    }

    // =========================================================================
    // KEYED NOTES
    // =========================================================================

    pub fn add_keyed_note(&self, note: &KeyedNote) -> Result<()> {
        self.conn.execute(
            "INSERT INTO keyed_notes (id, key, text, discipline, spec_section)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![note.id, note.key, note.text, note.discipline, note.spec_section],
        )?;
        Ok(())
    }

    pub fn list_keyed_notes(&self) -> Result<Vec<KeyedNote>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, key, text, discipline, spec_section FROM keyed_notes ORDER BY key"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(KeyedNote {
                id: row.get(0)?, key: row.get(1)?, text: row.get(2)?,
                discipline: row.get(3)?, spec_section: row.get(4)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    // =========================================================================
    // SUBMITTALS
    // =========================================================================

    pub fn add_submittal(&self, sub: &Submittal) -> Result<()> {
        self.conn.execute(
            "INSERT INTO submittals (id, number, description, submitted_by, company, date_submitted, status, spec_section)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                sub.id, sub.number, sub.description, sub.submitted_by,
                sub.company, sub.date_submitted, sub.status, sub.spec_section
            ],
        )?;
        Ok(())
    }

    pub fn list_submittals(&self) -> Result<Vec<Submittal>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, number, description, submitted_by, company, date_submitted, status, spec_section FROM submittals ORDER BY date_submitted"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Submittal {
                id: row.get(0)?, number: row.get(1)?, description: row.get(2)?,
                submitted_by: row.get(3)?, company: row.get(4)?,
                date_submitted: row.get(5)?, status: row.get(6)?, spec_section: row.get(7)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    // =========================================================================
    // REVISIONS
    // =========================================================================

    pub fn add_revision(&self, rev: &Revision) -> Result<()> {
        self.conn.execute(
            "INSERT INTO revisions (id, number, name, date, description, author)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![rev.id, rev.number, rev.name, rev.date, rev.description, rev.author],
        )?;
        Ok(())
    }

    // =========================================================================
    // QUERIES
    // =========================================================================

    /// Run a raw SQL SELECT query. Returns rows as key-value pairs.
    pub fn query_raw(&self, sql: &str) -> Result<Vec<Vec<(String, String)>>> {
        let mut stmt = self.conn.prepare(sql)?;
        let col_count = stmt.column_count();
        let col_names: Vec<String> = (0..col_count)
            .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
            .collect();

        let rows = stmt.query_map([], |row| {
            let mut values = Vec::new();
            for (i, name) in col_names.iter().enumerate() {
                let val: String = row.get::<_, rusqlite::types::Value>(i)
                    .map(|v| match v {
                        rusqlite::types::Value::Null => "NULL".to_string(),
                        rusqlite::types::Value::Integer(i) => i.to_string(),
                        rusqlite::types::Value::Real(f) => {
                            // Preserve full precision. Use compact representation
                            // (no trailing zeros) but never lose significant digits.
                            let s = format!("{}", f);
                            if s.contains('.') { s } else { format!("{}.0", s) }
                        }
                        rusqlite::types::Value::Text(s) => s,
                        rusqlite::types::Value::Blob(_) => "[BLOB]".to_string(),
                    })
                    .unwrap_or_else(|_| "ERROR".to_string());
                values.push((name.clone(), val));
            }
            Ok(values)
        })?;

        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Execute a parameterized write statement (INSERT, UPDATE, DELETE).
    /// Crate-internal only — external consumers should use typed methods.
    pub(crate) fn execute_raw(&self, sql: &str, params: &[&dyn rusqlite::types::ToSql]) -> Result<usize> {
        Ok(self.conn.execute(sql, params)?)
    }

    /// Count rows in a table. Table name is validated against known tables.
    pub fn count(&self, table: &str) -> Result<i64> {
        const ALLOWED: &[&str] = &[
            "meta", "directory", "geometry_polygons", "geometry_polylines",
            "spaces", "product_types", "placements", "systems", "placement_systems",
            "nodes", "segments", "insulation", "submittals", "attachments",
            "sheets", "views", "annotations", "general_notes", "keyed_notes",
            "keyed_note_refs", "revisions", "revision_changes", "schedule_data",
            "spatial_idx", "spatial_map",
        ];
        if !ALLOWED.contains(&table) {
            anyhow::bail!("Table '{}' not allowed in count()", table);
        }
        let sql = format!("SELECT COUNT(*) FROM {}", table);
        Ok(self.conn.query_row(&sql, [], |row| row.get(0))?)
    }

    // =========================================================================
    // INFO
    // =========================================================================

    pub fn info(&self) -> Result<DocumentInfo> {
        Ok(DocumentInfo {
            sed_version: self.get_meta("sed_version")?.unwrap_or_default(),
            project_name: self.get_meta("project_name")?.unwrap_or_default(),
            project_number: self.get_meta("project_number")?.unwrap_or_default(),
            spaces: self.count("spaces")?,
            product_types: self.count("product_types")?,
            placements: self.count("placements")?,
            systems: self.count("systems")?,
            nodes: self.count("nodes")?,
            segments: self.count("segments")?,
            sheets: self.count("sheets")?,
            submittals: self.count("submittals")?,
            keyed_notes: self.count("keyed_notes")?,
            revisions: self.count("revisions")?,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct DocumentInfo {
    pub sed_version: String,
    pub project_name: String,
    pub project_number: String,
    pub spaces: i64,
    pub product_types: i64,
    pub placements: i64,
    pub systems: i64,
    pub nodes: i64,
    pub segments: i64,
    pub sheets: i64,
    pub submittals: i64,
    pub keyed_notes: i64,
    pub revisions: i64,
}

impl std::fmt::Display for DocumentInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "SED v{}", self.sed_version)?;
        writeln!(f, "Project: {} ({})", self.project_name, self.project_number)?;
        writeln!(f)?;
        writeln!(f, "  Spaces:         {:>4}", self.spaces)?;
        writeln!(f, "  Product Types:  {:>4}", self.product_types)?;
        writeln!(f, "  Placements:     {:>4}", self.placements)?;
        writeln!(f, "  Systems:        {:>4}", self.systems)?;
        writeln!(f, "  Graph Nodes:    {:>4}", self.nodes)?;
        writeln!(f, "  Graph Segments: {:>4}", self.segments)?;
        writeln!(f, "  Sheets:         {:>4}", self.sheets)?;
        writeln!(f, "  Submittals:     {:>4}", self.submittals)?;
        writeln!(f, "  Keyed Notes:    {:>4}", self.keyed_notes)?;
        writeln!(f, "  Revisions:      {:>4}", self.revisions)?;
        Ok(())
    }
}
