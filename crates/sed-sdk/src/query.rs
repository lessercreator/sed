/// Pre-built queries for common operations on SED documents.
/// These are the queries AI and tools will use most often.

pub const SUPPLY_CFM_BY_ROOM: &str = "
    SELECT s.level, s.tag, s.name, SUM(p.cfm) as total_supply_cfm, COUNT(*) as device_count
    FROM placements p
    JOIN product_types pt ON p.product_type_id = pt.id
    JOIN spaces s ON p.space_id = s.id
    WHERE pt.category LIKE 'supply%'
    GROUP BY s.id
    ORDER BY s.level, s.tag
";

pub const ALL_PLACEMENTS_BY_TYPE: &str = "
    SELECT pt.tag, pt.category, pt.manufacturer, pt.model, COUNT(*) as qty,
           SUM(p.cfm) as total_cfm
    FROM placements p
    JOIN product_types pt ON p.product_type_id = pt.id
    WHERE p.status = 'new'
    GROUP BY pt.id
    ORDER BY qty DESC
";

pub const SUBMITTAL_STATUS: &str = "
    SELECT s.description, s.status, s.date_submitted, s.submitted_by, s.company
    FROM submittals s
    ORDER BY s.date_submitted
";

pub const TRACE_DOWNSTREAM: &str = "
    WITH RECURSIVE downstream AS (
        SELECT n.id, n.node_type, n.fitting_type, n.size_description, 0 as depth
        FROM nodes n
        WHERE n.id = ?1
        UNION ALL
        SELECT n2.id, n2.node_type, n2.fitting_type, n2.size_description, d.depth + 1
        FROM downstream d
        JOIN segments seg ON seg.from_node_id = d.id
        JOIN nodes n2 ON n2.id = seg.to_node_id
        WHERE d.depth < 100
    )
    SELECT * FROM downstream
";

pub const ROOMS_WITH_EXHAUST_NO_SUPPLY: &str = "
    SELECT s.name, s.tag, s.level FROM spaces s
    WHERE s.id IN (
        SELECT p.space_id FROM placements p
        JOIN product_types pt ON p.product_type_id = pt.id
        WHERE pt.category = 'exhaust_register'
    ) AND s.id NOT IN (
        SELECT p.space_id FROM placements p
        JOIN product_types pt ON p.product_type_id = pt.id
        WHERE pt.category LIKE 'supply%'
    )
";

pub const EQUIPMENT_LIST: &str = "
    SELECT COALESCE(p.instance_tag, pt.tag) as tag, pt.category, pt.manufacturer, pt.model,
           p.status, p.level, p.cfm, p.notes
    FROM placements p
    JOIN product_types pt ON p.product_type_id = pt.id
    WHERE pt.domain = 'equipment'
    ORDER BY tag
";

pub const ELEMENTS_IN_REGION: &str = "
    SELECT sm.source_table, sm.source_id
    FROM spatial_idx si
    JOIN spatial_map sm ON sm.spatial_id = si.id
    WHERE si.x_min <= ?1 AND si.x_max >= ?2
      AND si.y_min <= ?3 AND si.y_max >= ?4
";

pub const DUCT_SUMMARY_BY_SYSTEM: &str = "
    SELECT sys.tag, sys.name,
           COUNT(*) as segment_count,
           ROUND(SUM(seg.length_m * 3.28084), 1) as total_length_ft,
           seg.shape, seg.material
    FROM segments seg
    JOIN systems sys ON seg.system_id = sys.id
    GROUP BY sys.id, seg.shape, seg.material
    ORDER BY sys.tag
";
