use anyhow::Result;
use printpdf::*;
use sed_sdk::SedDocument;

// Sheet constants (ARCH D: 24" x 36")
const SHEET_W: f32 = 914.4;  // 36" in mm
const SHEET_H: f32 = 609.6;  // 24" in mm
const BORDER: f32 = 12.7;    // 1/2" border
const TB_W: f32 = 190.0;     // title block width
const TB_H: f32 = 76.0;      // title block height

// Equipment schedule table column widths (mm)
const COL_TAG: f32 = 25.0;
const COL_CATEGORY: f32 = 35.0;
const COL_MFR: f32 = 35.0;
const COL_MODEL: f32 = 35.0;
const COL_CFM: f32 = 20.0;
const COL_STATUS: f32 = 20.0;
const COL_LEVEL: f32 = 25.0;
const COL_ROOM: f32 = 30.0;

const SCHEDULE_COLS: [(&str, f32); 8] = [
    ("Tag", COL_TAG),
    ("Category", COL_CATEGORY),
    ("Manufacturer", COL_MFR),
    ("Model", COL_MODEL),
    ("CFM", COL_CFM),
    ("Status", COL_STATUS),
    ("Level", COL_LEVEL),
    ("Room", COL_ROOM),
];

const ROWS_PER_SCHEDULE_PAGE: usize = 40;

pub fn export_pdf(file: &str, output: &str, level: &str) -> Result<()> {
    let doc = SedDocument::open(file)?;
    let info = doc.info()?;

    let pdf = PdfDocument::empty(&format!("{} — {}", info.project_name, level));
    let font = pdf.add_builtin_font(BuiltinFont::Helvetica).unwrap();
    let font_bold = pdf.add_builtin_font(BuiltinFont::HelveticaBold).unwrap();

    render_plan_page(&pdf, &doc, &info, level, &font, &font_bold)?;

    pdf.save(&mut std::io::BufWriter::new(std::fs::File::create(output)?))?;
    println!("Exported: {}", output);
    Ok(())
}

pub fn export_pdf_all(file: &str, output: &str) -> Result<()> {
    let doc = SedDocument::open(file)?;
    let info = doc.info()?;

    // Find all sheets that have a plan view, get the level from the view
    let plan_views = doc.query_raw(
        "SELECT DISTINCT v.level, s.number, s.title
         FROM views v
         JOIN sheets s ON v.sheet_id = s.id
         WHERE v.view_type = 'plan' AND v.level IS NOT NULL
         ORDER BY s.number"
    )?;

    if plan_views.is_empty() {
        anyhow::bail!("No plan views found in sheets table");
    }

    let pdf = PdfDocument::empty(&info.project_name);
    let font = pdf.add_builtin_font(BuiltinFont::Helvetica).unwrap();
    let font_bold = pdf.add_builtin_font(BuiltinFont::HelveticaBold).unwrap();

    // Render each plan level as a page
    for view in &plan_views {
        let level = &view[0].1;
        if level == "NULL" { continue; }
        render_plan_page(&pdf, &doc, &info, level, &font, &font_bold)?;
    }

    // Render equipment schedule page(s) at the end
    render_schedule_pages(&pdf, &doc, &font, &font_bold)?;

    pdf.save(&mut std::io::BufWriter::new(std::fs::File::create(output)?))?;
    println!("Exported: {} ({} plan pages + equipment schedule)", output, plan_views.len());
    Ok(())
}

fn render_plan_page(
    pdf: &PdfDocumentReference,
    doc: &SedDocument,
    info: &sed_sdk::document::DocumentInfo,
    level: &str,
    font: &IndirectFontRef,
    font_bold: &IndirectFontRef,
) -> Result<()> {
    let rooms = sed_sdk::geometry::get_room_geometry(doc, level)?;

    let placements = doc.query_params(
        "SELECT pt.tag, pt.category, pt.domain, p.x, p.y, p.cfm, p.instance_tag, p.status
         FROM placements p JOIN product_types pt ON p.product_type_id = pt.id
         WHERE p.level = ?1 AND p.x IS NOT NULL ORDER BY pt.tag",
        &[&level as &dyn rusqlite::types::ToSql],
    )?;

    let segments = doc.query_params(
        "SELECT n1.x as x1, n1.y as y1, n2.x as x2, n2.y as y2, seg.diameter_m, seg.width_m
         FROM segments seg
         JOIN nodes n1 ON seg.from_node_id = n1.id
         JOIN nodes n2 ON seg.to_node_id = n2.id
         WHERE n1.level = ?1 AND n1.x IS NOT NULL AND n2.x IS NOT NULL",
        &[&level as &dyn rusqlite::types::ToSql],
    )?;

    // Get sheet number
    let sheet_rows = doc.query_params(
        "SELECT number, title FROM sheets WHERE title LIKE ?1 LIMIT 1",
        &[&format!("%{}%", level) as &dyn rusqlite::types::ToSql],
    ).unwrap_or_default();
    let sheet_num = sheet_rows.first().map(|r| r[0].1.as_str()).unwrap_or("M-100");
    let sheet_title = sheet_rows.first().map(|r| r[1].1.as_str()).unwrap_or(level);

    let (page_idx, layer_idx) = pdf.add_page(Mm(SHEET_W), Mm(SHEET_H), sheet_title);
    let layer = pdf.get_page(page_idx).get_layer(layer_idx);

    // =========================================================================
    // SHEET BORDER
    // =========================================================================
    layer.set_outline_color(Color::Greyscale(Greyscale::new(0.0, None)));
    layer.set_outline_thickness(0.75);
    layer.add_line(Line {
        points: vec![
            (Point::new(Mm(BORDER), Mm(BORDER)), false),
            (Point::new(Mm(SHEET_W - BORDER), Mm(BORDER)), false),
            (Point::new(Mm(SHEET_W - BORDER), Mm(SHEET_H - BORDER)), false),
            (Point::new(Mm(BORDER), Mm(SHEET_H - BORDER)), false),
        ],
        is_closed: true,
    });

    // Inner border (drawing area)
    layer.set_outline_thickness(0.25);
    layer.add_line(Line {
        points: vec![
            (Point::new(Mm(BORDER + 2.0), Mm(BORDER + 2.0)), false),
            (Point::new(Mm(SHEET_W - BORDER - 2.0), Mm(BORDER + 2.0)), false),
            (Point::new(Mm(SHEET_W - BORDER - 2.0), Mm(SHEET_H - BORDER - 2.0)), false),
            (Point::new(Mm(BORDER + 2.0), Mm(SHEET_H - BORDER - 2.0)), false),
        ],
        is_closed: true,
    });

    // =========================================================================
    // TITLE BLOCK (bottom right)
    // =========================================================================
    let tb_x = SHEET_W - BORDER - TB_W;
    let tb_y = BORDER + 2.0;

    // Title block border
    layer.set_outline_thickness(0.5);
    layer.add_line(Line {
        points: vec![
            (Point::new(Mm(tb_x), Mm(tb_y)), false),
            (Point::new(Mm(tb_x + TB_W - 2.0), Mm(tb_y)), false),
            (Point::new(Mm(tb_x + TB_W - 2.0), Mm(tb_y + TB_H)), false),
            (Point::new(Mm(tb_x), Mm(tb_y + TB_H)), false),
        ],
        is_closed: true,
    });

    // Horizontal dividers
    let rows_y = [tb_y + 15.0, tb_y + 30.0, tb_y + 45.0, tb_y + 60.0];
    layer.set_outline_thickness(0.15);
    for &ry in &rows_y {
        layer.add_line(Line {
            points: vec![
                (Point::new(Mm(tb_x), Mm(ry)), false),
                (Point::new(Mm(tb_x + TB_W - 2.0), Mm(ry)), false),
            ],
            is_closed: false,
        });
    }

    // Vertical divider (labels | values)
    let label_w = 45.0;
    layer.add_line(Line {
        points: vec![
            (Point::new(Mm(tb_x + label_w), Mm(tb_y)), false),
            (Point::new(Mm(tb_x + label_w), Mm(tb_y + TB_H)), false),
        ],
        is_closed: false,
    });

    // Title block content
    let small = 5.0_f32;
    let medium = 7.0_f32;
    let large = 10.0_f32;

    // Row 1: Project name
    layer.set_fill_color(Color::Greyscale(Greyscale::new(0.5, None)));
    layer.use_text("PROJECT", small, Mm(tb_x + 3.0), Mm(tb_y + 3.0), &font);
    layer.set_fill_color(Color::Greyscale(Greyscale::new(0.0, None)));
    layer.use_text(&info.project_name, large, Mm(tb_x + label_w + 3.0), Mm(tb_y + 4.0), &font_bold);

    // Row 2: Project number
    layer.set_fill_color(Color::Greyscale(Greyscale::new(0.5, None)));
    layer.use_text("PROJECT NO.", small, Mm(tb_x + 3.0), Mm(rows_y[0] + 3.0), &font);
    layer.set_fill_color(Color::Greyscale(Greyscale::new(0.0, None)));
    layer.use_text(&info.project_number, medium, Mm(tb_x + label_w + 3.0), Mm(rows_y[0] + 4.0), &font);

    // Row 3: Sheet title
    layer.set_fill_color(Color::Greyscale(Greyscale::new(0.5, None)));
    layer.use_text("SHEET TITLE", small, Mm(tb_x + 3.0), Mm(rows_y[1] + 3.0), &font);
    layer.set_fill_color(Color::Greyscale(Greyscale::new(0.0, None)));
    layer.use_text(sheet_title, medium, Mm(tb_x + label_w + 3.0), Mm(rows_y[1] + 4.0), &font_bold);

    // Row 4: Sheet number
    layer.set_fill_color(Color::Greyscale(Greyscale::new(0.5, None)));
    layer.use_text("SHEET NO.", small, Mm(tb_x + 3.0), Mm(rows_y[2] + 3.0), &font);
    layer.set_fill_color(Color::Greyscale(Greyscale::new(0.0, None)));
    layer.use_text(sheet_num, large, Mm(tb_x + label_w + 3.0), Mm(rows_y[2] + 3.0), &font_bold);

    // Row 5: Generated by
    layer.set_fill_color(Color::Greyscale(Greyscale::new(0.5, None)));
    layer.use_text("GENERATED", small, Mm(tb_x + 3.0), Mm(rows_y[3] + 3.0), &font);
    layer.set_fill_color(Color::Greyscale(Greyscale::new(0.0, None)));
    layer.use_text("From .sed file — Structured Engineering Document", small, Mm(tb_x + label_w + 3.0), Mm(rows_y[3] + 3.0), &font);

    // =========================================================================
    // COORDINATE TRANSFORM — fit model into drawing area
    // =========================================================================
    let draw_left = BORDER + 15.0;
    let draw_bottom = BORDER + TB_H + 15.0;
    let draw_right = SHEET_W - BORDER - 15.0;
    let draw_top = SHEET_H - BORDER - 15.0;
    let draw_w = draw_right - draw_left;
    let draw_h = draw_top - draw_bottom;

    let (mut x_min, mut y_min, mut x_max, mut y_max) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for room in &rooms {
        for v in &room.vertices {
            x_min = x_min.min(v.x); y_min = y_min.min(v.y);
            x_max = x_max.max(v.x); y_max = y_max.max(v.y);
        }
    }
    for p in &placements {
        let px: f64 = p[3].1.parse().unwrap_or(0.0);
        let py: f64 = p[4].1.parse().unwrap_or(0.0);
        x_min = x_min.min(px); y_min = y_min.min(py);
        x_max = x_max.max(px); y_max = y_max.max(py);
    }
    if x_min == f64::MAX { x_min = 0.0; y_min = 0.0; x_max = 20.0; y_max = 20.0; }

    // Add padding
    let pad = 1.0; // 1m padding
    x_min -= pad; y_min -= pad; x_max += pad; y_max += pad;

    let model_w = (x_max - x_min) as f32;
    let model_h = (y_max - y_min) as f32;
    let scale = (draw_w / (model_w * 1000.0)).min(draw_h / (model_h * 1000.0)) * 1000.0;
    let x_off = x_min as f32;
    let y_off = y_min as f32;

    let tx = move |x: f64| -> Mm { Mm(draw_left + (x as f32 - x_off) * scale) };
    let ty = move |y: f64| -> Mm { Mm(draw_bottom + (y as f32 - y_off) * scale) };

    // =========================================================================
    // SCALE BAR
    // =========================================================================
    let scale_m: f32 = 1.0 / scale; // meters per mm
    let bar_len_m: f32 = (5.0 * scale_m).ceil();
    let bar_len_mm: f32 = bar_len_m / scale_m;
    let bar_x = draw_left;
    let bar_y = BORDER + TB_H + 5.0;

    layer.set_outline_color(Color::Greyscale(Greyscale::new(0.0, None)));
    layer.set_outline_thickness(0.5);
    layer.add_line(Line {
        points: vec![
            (Point::new(Mm(bar_x), Mm(bar_y)), false),
            (Point::new(Mm(bar_x + bar_len_mm as f32), Mm(bar_y)), false),
        ],
        is_closed: false,
    });
    // Tick marks
    for i in 0..=(bar_len_m as i32) {
        let tick_x = bar_x + (i as f32 / bar_len_m as f32) * bar_len_mm as f32;
        layer.add_line(Line {
            points: vec![
                (Point::new(Mm(tick_x), Mm(bar_y - 1.5)), false),
                (Point::new(Mm(tick_x), Mm(bar_y + 1.5)), false),
            ],
            is_closed: false,
        });
    }
    layer.set_fill_color(Color::Greyscale(Greyscale::new(0.3, None)));
    layer.use_text(&format!("0"), small, Mm(bar_x - 1.0), Mm(bar_y + 3.0), &font);
    layer.use_text(&format!("{}m", bar_len_m as i32), small, Mm(bar_x + bar_len_mm - 3.0), Mm(bar_y + 3.0), &font);

    // =========================================================================
    // NORTH ARROW (simple)
    // =========================================================================
    let na_x = draw_right - 15.0;
    let na_y = draw_top - 15.0;
    layer.set_fill_color(Color::Greyscale(Greyscale::new(0.0, None)));
    layer.add_line(Line {
        points: vec![
            (Point::new(Mm(na_x), Mm(na_y + 10.0)), false),
            (Point::new(Mm(na_x + 3.0), Mm(na_y)), false),
            (Point::new(Mm(na_x), Mm(na_y + 3.0)), false),
            (Point::new(Mm(na_x - 3.0), Mm(na_y)), false),
        ],
        is_closed: true,
    });
    layer.use_text("N", medium, Mm(na_x - 2.0), Mm(na_y + 12.0), &font_bold);

    // =========================================================================
    // DRAW ROOMS
    // =========================================================================
    for room in &rooms {
        if room.vertices.len() < 3 { continue; }
        let pts: Vec<(Point, bool)> = room.vertices.iter().map(|v| (Point::new(tx(v.x), ty(v.y)), false)).collect();
        let color = if room.scope == "nic" {
            Color::Greyscale(Greyscale::new(0.7, None))
        } else {
            Color::Greyscale(Greyscale::new(0.0, None))
        };
        layer.set_outline_color(color);
        layer.set_outline_thickness(if room.scope == "nic" { 0.15 } else { 0.35 });
        layer.add_line(Line { points: pts, is_closed: true });

        // Room label
        let cx = room.vertices.iter().map(|v| v.x).sum::<f64>() / room.vertices.len() as f64;
        let cy = room.vertices.iter().map(|v| v.y).sum::<f64>() / room.vertices.len() as f64;
        layer.set_fill_color(Color::Greyscale(Greyscale::new(0.3, None)));
        layer.use_text(&room.tag, small, tx(cx), ty(cy), &font_bold);
        layer.use_text(&room.name, 4.0, tx(cx), Mm(ty(cy).0 - 4.0), &font);
    }

    // =========================================================================
    // DRAW DUCT SEGMENTS
    // =========================================================================
    layer.set_outline_color(Color::Greyscale(Greyscale::new(0.4, None)));
    for seg in &segments {
        let sx1: f64 = seg[0].1.parse().unwrap_or(0.0);
        let sy1: f64 = seg[1].1.parse().unwrap_or(0.0);
        let sx2: f64 = seg[2].1.parse().unwrap_or(0.0);
        let sy2: f64 = seg[3].1.parse().unwrap_or(0.0);
        let diam: f32 = seg[4].1.parse().unwrap_or(0.2);
        layer.set_outline_thickness((diam * scale * 0.8).max(0.2));
        layer.add_line(Line {
            points: vec![(Point::new(tx(sx1), ty(sy1)), false), (Point::new(tx(sx2), ty(sy2)), false)],
            is_closed: false,
        });
    }

    // =========================================================================
    // DRAW PLACEMENTS
    // =========================================================================
    for p in &placements {
        let px: f64 = p[3].1.parse().unwrap_or(0.0);
        let py: f64 = p[4].1.parse().unwrap_or(0.0);
        let tag = &p[0].1;
        let cfm = &p[5].1;
        let domain = &p[2].1;
        let cat = &p[1].1;
        let r: f32 = if domain == "equipment" { 2.5 } else { 1.5 };
        let ptx = tx(px);
        let pty = ty(py);

        let color = match domain.as_str() {
            "equipment" => Color::Greyscale(Greyscale::new(0.0, None)),
            "accessory" => Color::Greyscale(Greyscale::new(0.3, None)),
            _ => {
                if cat.contains("return") || cat.contains("exhaust") {
                    Color::Greyscale(Greyscale::new(0.2, None))
                } else {
                    Color::Greyscale(Greyscale::new(0.0, None))
                }
            }
        };

        layer.set_outline_color(color);
        layer.set_outline_thickness(0.3);

        if domain == "equipment" {
            // Equipment: diamond
            layer.add_line(Line {
                points: vec![
                    (Point::new(ptx, Mm(pty.0 + r)), false),
                    (Point::new(Mm(ptx.0 + r), pty), false),
                    (Point::new(ptx, Mm(pty.0 - r)), false),
                    (Point::new(Mm(ptx.0 - r), pty), false),
                ],
                is_closed: true,
            });
        } else {
            // Devices: small circle (octagon approximation)
            let n = 8;
            let pts: Vec<(Point, bool)> = (0..n).map(|i| {
                let angle = std::f32::consts::PI * 2.0 * i as f32 / n as f32;
                (Point::new(Mm(ptx.0 + r * angle.cos()), Mm(pty.0 + r * angle.sin())), false)
            }).collect();
            layer.add_line(Line { points: pts, is_closed: true });
        }

        // Tag + CFM label
        layer.set_fill_color(Color::Greyscale(Greyscale::new(0.2, None)));
        let label = if cfm != "NULL" {
            format!("{}\n{} CFM", tag, cfm)
        } else {
            tag.clone()
        };
        layer.use_text(&label, 3.5, Mm(ptx.0 + r + 1.0), Mm(pty.0 + 0.5), &font);
    }

    println!("  Rendered plan page: {} ({})", sheet_title, sheet_num);
    Ok(())
}

/// Render equipment schedule table pages into the PDF document.
fn render_schedule_pages(
    pdf: &PdfDocumentReference,
    doc: &SedDocument,
    font: &IndirectFontRef,
    font_bold: &IndirectFontRef,
) -> Result<()> {
    let rows = doc.query_raw(
        "SELECT COALESCE(p.instance_tag, pt.tag) as tag, pt.category, pt.manufacturer,
                pt.model, p.cfm, p.status, p.level, s.name as room
         FROM placements p
         JOIN product_types pt ON p.product_type_id = pt.id
         LEFT JOIN spaces s ON p.space_id = s.id
         ORDER BY p.level, pt.tag"
    )?;

    if rows.is_empty() {
        return Ok(());
    }

    // Paginate
    let chunks: Vec<&[Vec<(String, String)>]> = rows.chunks(ROWS_PER_SCHEDULE_PAGE).collect();

    for (page_num, chunk) in chunks.iter().enumerate() {
        let page_label = if chunks.len() == 1 {
            "EQUIPMENT SCHEDULE".to_string()
        } else {
            format!("EQUIPMENT SCHEDULE (Page {} of {})", page_num + 1, chunks.len())
        };

        let (page_idx, layer_idx) = pdf.add_page(Mm(SHEET_W), Mm(SHEET_H), &page_label);
        let layer = pdf.get_page(page_idx).get_layer(layer_idx);

        // Sheet border
        layer.set_outline_color(Color::Greyscale(Greyscale::new(0.0, None)));
        layer.set_outline_thickness(0.75);
        layer.add_line(Line {
            points: vec![
                (Point::new(Mm(BORDER), Mm(BORDER)), false),
                (Point::new(Mm(SHEET_W - BORDER), Mm(BORDER)), false),
                (Point::new(Mm(SHEET_W - BORDER), Mm(SHEET_H - BORDER)), false),
                (Point::new(Mm(BORDER), Mm(SHEET_H - BORDER)), false),
            ],
            is_closed: true,
        });

        // Title
        let title_y = SHEET_H - BORDER - 15.0;
        layer.set_fill_color(Color::Greyscale(Greyscale::new(0.0, None)));
        layer.use_text(&page_label, 14.0, Mm(BORDER + 10.0), Mm(title_y), font_bold);

        // Table layout
        let table_x = BORDER + 10.0;
        let table_top = title_y - 10.0;
        let row_height: f32 = 6.0;
        let header_height: f32 = 8.0;
        let total_width: f32 = SCHEDULE_COLS.iter().map(|(_, w)| w).sum();

        // Header background
        layer.set_fill_color(Color::Greyscale(Greyscale::new(0.85, None)));
        layer.set_outline_thickness(0.0);
        layer.add_polygon(Polygon {
            rings: vec![vec![
                (Point::new(Mm(table_x), Mm(table_top - header_height)), false),
                (Point::new(Mm(table_x + total_width), Mm(table_top - header_height)), false),
                (Point::new(Mm(table_x + total_width), Mm(table_top)), false),
                (Point::new(Mm(table_x), Mm(table_top)), false),
            ]],
            mode: path::PaintMode::Fill,
            winding_order: path::WindingOrder::NonZero,
        });

        // Header text
        layer.set_fill_color(Color::Greyscale(Greyscale::new(0.0, None)));
        let mut col_x = table_x;
        for (name, width) in &SCHEDULE_COLS {
            layer.use_text(*name, 8.0, Mm(col_x + 1.0), Mm(table_top - 6.0), font_bold);
            col_x += width;
        }

        // Header bottom line
        layer.set_outline_color(Color::Greyscale(Greyscale::new(0.0, None)));
        layer.set_outline_thickness(0.5);
        layer.add_line(Line {
            points: vec![
                (Point::new(Mm(table_x), Mm(table_top - header_height)), false),
                (Point::new(Mm(table_x + total_width), Mm(table_top - header_height)), false),
            ],
            is_closed: false,
        });

        // Data rows
        let data_top = table_top - header_height;

        for (row_idx, row) in chunk.iter().enumerate() {
            let row_y = data_top - (row_idx as f32 + 1.0) * row_height;

            // Alternate row background
            if row_idx % 2 == 0 {
                layer.set_fill_color(Color::Greyscale(Greyscale::new(0.95, None)));
                layer.set_outline_thickness(0.0);
                layer.add_polygon(Polygon {
                    rings: vec![vec![
                        (Point::new(Mm(table_x), Mm(row_y)), false),
                        (Point::new(Mm(table_x + total_width), Mm(row_y)), false),
                        (Point::new(Mm(table_x + total_width), Mm(row_y + row_height)), false),
                        (Point::new(Mm(table_x), Mm(row_y + row_height)), false),
                    ]],
                    mode: path::PaintMode::Fill,
                    winding_order: path::WindingOrder::NonZero,
                });
            }

            // Row text
            layer.set_fill_color(Color::Greyscale(Greyscale::new(0.1, None)));
            let mut col_x = table_x;
            for (col_idx, (_, width)) in SCHEDULE_COLS.iter().enumerate() {
                let val = if col_idx < row.len() {
                    let v = &row[col_idx].1;
                    if v == "NULL" { "" } else { v.as_str() }
                } else {
                    ""
                };
                // Truncate if too long for column
                let max_chars = (*width as usize * 2 / 3).max(4);
                let display = if val.len() > max_chars {
                    &val[..max_chars]
                } else {
                    val
                };
                layer.use_text(display, 7.0, Mm(col_x + 1.0), Mm(row_y + 1.5), font);
                col_x += width;
            }
        }

        // Table outer border
        let table_bottom = data_top - (chunk.len() as f32) * row_height;
        layer.set_outline_color(Color::Greyscale(Greyscale::new(0.3, None)));
        layer.set_outline_thickness(0.3);
        layer.add_line(Line {
            points: vec![
                (Point::new(Mm(table_x), Mm(table_bottom)), false),
                (Point::new(Mm(table_x + total_width), Mm(table_bottom)), false),
                (Point::new(Mm(table_x + total_width), Mm(table_top)), false),
                (Point::new(Mm(table_x), Mm(table_top)), false),
            ],
            is_closed: true,
        });

        // Vertical column dividers
        layer.set_outline_thickness(0.15);
        let mut col_x = table_x;
        for (_, width) in &SCHEDULE_COLS {
            col_x += width;
            if col_x < table_x + total_width {
                layer.add_line(Line {
                    points: vec![
                        (Point::new(Mm(col_x), Mm(table_bottom)), false),
                        (Point::new(Mm(col_x), Mm(table_top)), false),
                    ],
                    is_closed: false,
                });
            }
        }

        // Row count footer
        layer.set_fill_color(Color::Greyscale(Greyscale::new(0.4, None)));
        layer.use_text(
            &format!("{} items total", rows.len()),
            6.0,
            Mm(table_x),
            Mm(table_bottom - 5.0),
            font,
        );
    }

    Ok(())
}

pub fn export_schedule(file: &str, output: &str, type_filter: &str) -> Result<()> {
    let doc = SedDocument::open(file)?;

    let mut sql = String::from(
        "SELECT COALESCE(p.instance_tag, pt.tag) as tag, p.instance_tag, pt.category,
                pt.manufacturer, pt.model, p.cfm, p.status, p.level,
                s.name as room, p.phase, p.notes
         FROM placements p
         JOIN product_types pt ON p.product_type_id = pt.id
         LEFT JOIN spaces s ON p.space_id = s.id"
    );

    match type_filter {
        "equipment" => sql += " WHERE pt.domain = 'equipment'",
        "air_devices" => sql += " WHERE pt.domain = 'air_device'",
        _ => {}
    }
    sql += " ORDER BY p.level, pt.tag";

    let rows = doc.query_raw(&sql)?;

    let mut wtr = csv::Writer::from_path(output)?;
    wtr.write_record(["Tag", "Instance Tag", "Category", "Manufacturer", "Model", "CFM", "Status", "Level", "Room", "Phase", "Notes"])?;

    for row in &rows {
        let vals: Vec<&str> = row.iter().map(|(_, v)| if v == "NULL" { "" } else { v.as_str() }).collect();
        wtr.write_record(&vals)?;
    }
    wtr.flush()?;
    println!("Exported: {} ({} rows)", output, rows.len());
    Ok(())
}
