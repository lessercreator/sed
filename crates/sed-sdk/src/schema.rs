use rusqlite::Connection;
use anyhow::Result;

pub const SED_VERSION: &str = "0.3";
pub const SCHEMA_VERSION: i32 = 3;

pub fn create_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(&format!("PRAGMA user_version = {};", SCHEMA_VERSION))?;
    conn.execute_batch("PRAGMA journal_mode = WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    conn.execute_batch(
        "
        -- Project metadata
        CREATE TABLE IF NOT EXISTS meta (
            key     TEXT PRIMARY KEY,
            value   TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS directory (
            id          TEXT PRIMARY KEY,
            role        TEXT NOT NULL,
            company     TEXT NOT NULL,
            contact     TEXT,
            email       TEXT,
            phone       TEXT,
            address     TEXT
        );

        -- Geometry
        CREATE TABLE IF NOT EXISTS geometry_polygons (
            id              TEXT PRIMARY KEY,
            vertices        BLOB NOT NULL,
            vertex_count    INTEGER NOT NULL,
            level           TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS geometry_polylines (
            id              TEXT PRIMARY KEY,
            vertices        BLOB NOT NULL,
            vertex_count    INTEGER NOT NULL,
            level           TEXT NOT NULL,
            line_type       TEXT,
            weight          REAL,
            properties      TEXT
        );

        -- Spaces
        CREATE TABLE IF NOT EXISTS spaces (
            id              TEXT PRIMARY KEY,
            tag             TEXT NOT NULL,
            name            TEXT NOT NULL,
            level           TEXT NOT NULL,
            space_type      TEXT,
            area_m2         REAL,
            ceiling_ht_m    REAL,
            scope           TEXT NOT NULL DEFAULT 'in_contract',
            parent_id       TEXT REFERENCES spaces(id),
            boundary_id     TEXT REFERENCES geometry_polygons(id),
            x               REAL,
            y               REAL,
            properties      TEXT
        );

        -- Product types (catalog)
        CREATE TABLE IF NOT EXISTS product_types (
            id              TEXT PRIMARY KEY,
            tag             TEXT NOT NULL UNIQUE,
            domain          TEXT NOT NULL,
            category        TEXT NOT NULL,
            manufacturer    TEXT,
            model           TEXT,
            description     TEXT,
            mounting        TEXT,
            finish          TEXT,
            size_nominal    TEXT,
            voltage         REAL,
            phase           INTEGER,
            hz              REAL,
            submittal_id    TEXT REFERENCES submittals(id),
            properties      TEXT
        );

        -- Placements (instances)
        CREATE TABLE IF NOT EXISTS placements (
            id              TEXT PRIMARY KEY,
            instance_tag    TEXT,
            product_type_id TEXT NOT NULL REFERENCES product_types(id),
            space_id        TEXT REFERENCES spaces(id),
            level           TEXT NOT NULL,
            x               REAL,
            y               REAL,
            rotation        REAL DEFAULT 0,
            cfm             REAL,
            cfm_balanced    REAL,
            static_pressure_pa REAL,
            status          TEXT NOT NULL,
            scope           TEXT NOT NULL DEFAULT 'in_contract',
            phase           TEXT NOT NULL DEFAULT 'design',
            weight_kg       REAL,
            properties      TEXT,
            notes           TEXT
        );

        -- Systems
        CREATE TABLE IF NOT EXISTS systems (
            id              TEXT PRIMARY KEY,
            tag             TEXT NOT NULL UNIQUE,
            name            TEXT NOT NULL,
            system_type     TEXT NOT NULL,
            medium          TEXT NOT NULL DEFAULT 'air',
            source_id       TEXT REFERENCES placements(id),
            paired_system_id TEXT REFERENCES systems(id),  -- links supply to return in hydronic loops
            properties      TEXT
        );

        -- Junction: placements can belong to multiple systems
        -- A VAV with reheat belongs to both an air system (role='served_by')
        -- and a hot water system (role='reheat')
        CREATE TABLE IF NOT EXISTS placement_systems (
            placement_id    TEXT NOT NULL REFERENCES placements(id),
            system_id       TEXT NOT NULL REFERENCES systems(id),
            role            TEXT NOT NULL DEFAULT 'served_by',  -- 'served_by', 'source', 'reheat', 'coil'
            PRIMARY KEY (placement_id, system_id)
        );

        -- Graph: nodes
        CREATE TABLE IF NOT EXISTS nodes (
            id              TEXT PRIMARY KEY,
            system_id       TEXT NOT NULL REFERENCES systems(id),
            node_type       TEXT NOT NULL,
            placement_id    TEXT REFERENCES placements(id),
            fitting_type    TEXT,
            size_description TEXT,
            level           TEXT,
            x               REAL,
            y               REAL,
            properties      TEXT
        );

        -- Graph: segments
        CREATE TABLE IF NOT EXISTS segments (
            id              TEXT PRIMARY KEY,
            system_id       TEXT NOT NULL REFERENCES systems(id),
            from_node_id    TEXT NOT NULL REFERENCES nodes(id),
            to_node_id      TEXT NOT NULL REFERENCES nodes(id),
            shape           TEXT NOT NULL,
            width_m         REAL,
            height_m        REAL,
            diameter_m      REAL,
            length_m        REAL,
            material        TEXT NOT NULL DEFAULT 'galvanized',
            gauge           INTEGER,
            pressure_class  TEXT,
            construction    TEXT,
            exposure        TEXT,
            flow_design     REAL,           -- design flow rate in SI: m3/s for air, L/s for water
            flow_balanced   REAL,
            status          TEXT NOT NULL,
            scope           TEXT NOT NULL DEFAULT 'in_contract',
            properties      TEXT
        );

        -- Insulation
        CREATE TABLE IF NOT EXISTS insulation (
            id              TEXT PRIMARY KEY,
            segment_id      TEXT REFERENCES segments(id),
            type            TEXT NOT NULL,
            manufacturer    TEXT,
            product         TEXT,
            thickness_m     REAL,
            r_value         REAL,
            facing          TEXT,
            code_reference  TEXT
        );

        -- Submittals
        CREATE TABLE IF NOT EXISTS submittals (
            id              TEXT PRIMARY KEY,
            number          TEXT,
            description     TEXT NOT NULL,
            submitted_by    TEXT,
            company         TEXT,
            date_submitted  TEXT,
            status          TEXT NOT NULL,
            reviewed_by     TEXT,
            date_reviewed   TEXT,
            spec_section    TEXT,
            attachment_id   TEXT REFERENCES attachments(id),
            notes           TEXT
        );

        -- Attachments
        CREATE TABLE IF NOT EXISTS attachments (
            id          TEXT PRIMARY KEY,
            filename    TEXT NOT NULL,
            mime_type   TEXT NOT NULL,
            size_bytes  INTEGER,
            data        BLOB,
            description TEXT
        );

        -- Sheets
        CREATE TABLE IF NOT EXISTS sheets (
            id          TEXT PRIMARY KEY,
            number      TEXT NOT NULL UNIQUE,
            title       TEXT NOT NULL,
            discipline  TEXT NOT NULL,
            sheet_size  TEXT,
            properties  TEXT
        );

        -- Views
        CREATE TABLE IF NOT EXISTS views (
            id              TEXT PRIMARY KEY,
            sheet_id        TEXT NOT NULL REFERENCES sheets(id),
            view_type       TEXT NOT NULL,
            title           TEXT,
            scale           TEXT,
            level           TEXT,
            vp_x            REAL,
            vp_y            REAL,
            vp_width        REAL,
            vp_height       REAL,
            model_x_min     REAL,
            model_y_min     REAL,
            model_x_max     REAL,
            model_y_max     REAL,
            parent_view_id  TEXT REFERENCES views(id),
            callout_x       REAL,
            callout_y       REAL,
            properties      TEXT
        );

        -- Annotations
        CREATE TABLE IF NOT EXISTS annotations (
            id              TEXT PRIMARY KEY,
            view_id         TEXT NOT NULL REFERENCES views(id),
            anno_type       TEXT NOT NULL,
            ref_table       TEXT,
            ref_id          TEXT,
            x1              REAL,
            y1              REAL,
            x2              REAL,
            y2              REAL,
            geometry        BLOB,
            text            TEXT,
            text_height     REAL,
            text_rotation   REAL,
            revision_id     TEXT REFERENCES revisions(id),
            properties      TEXT
        );

        -- Notes
        CREATE TABLE IF NOT EXISTS general_notes (
            id          TEXT PRIMARY KEY,
            discipline  TEXT,
            text        TEXT NOT NULL,
            sort_order  INTEGER
        );

        CREATE TABLE IF NOT EXISTS keyed_notes (
            id          TEXT PRIMARY KEY,
            key         TEXT NOT NULL UNIQUE,
            text        TEXT NOT NULL,
            discipline  TEXT,
            spec_section TEXT
        );

        CREATE TABLE IF NOT EXISTS keyed_note_refs (
            note_id     TEXT REFERENCES keyed_notes(id),
            placement_id TEXT,
            view_id     TEXT REFERENCES views(id),
            x           REAL,
            y           REAL,
            PRIMARY KEY (note_id, view_id, x, y)
        );

        -- Revisions
        CREATE TABLE IF NOT EXISTS revisions (
            id          TEXT PRIMARY KEY,
            number      INTEGER NOT NULL,
            name        TEXT NOT NULL,
            date        TEXT NOT NULL,
            description TEXT,
            author      TEXT
        );

        CREATE TABLE IF NOT EXISTS revision_changes (
            id              TEXT PRIMARY KEY,
            revision_id     TEXT REFERENCES revisions(id),
            table_name      TEXT NOT NULL,
            element_id      TEXT NOT NULL,
            change_type     TEXT NOT NULL,
            field           TEXT,
            old_value       TEXT,
            new_value       TEXT
        );

        -- Schedule data
        CREATE TABLE IF NOT EXISTS schedule_data (
            id              TEXT PRIMARY KEY,
            space_id        TEXT REFERENCES spaces(id),
            equipment_id    TEXT REFERENCES placements(id),
            cfm_supply      REAL,
            cfm_return      REAL,
            cfm_exhaust     REAL,
            cfm_outside_air REAL,
            sensible_w      REAL,
            total_w         REAL,
            occupancy       INTEGER,
            oa_per_person   REAL,
            oa_per_area     REAL,
            properties      TEXT
        );

        -- Spatial index
        CREATE VIRTUAL TABLE IF NOT EXISTS spatial_idx USING rtree(
            id,
            x_min, x_max,
            y_min, y_max
        );
        "
    )?;

    Ok(())
}
