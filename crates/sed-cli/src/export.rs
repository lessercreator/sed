use anyhow::Result;
use printpdf::*;
use sed_sdk::SedDocument;

pub fn export_pdf(file: &str, output: &str, level: &str) -> Result<()> {
    let doc = SedDocument::open(file)?;
    let info = doc.info()?;
    let rooms = sed_sdk::geometry::get_room_geometry(&doc, level)?;

    let placements = doc.query_params(
        "SELECT pt.tag, pt.category, pt.domain, p.x, p.y, p.cfm, p.instance_tag, p.status
         FROM placements p
         JOIN product_types pt ON p.product_type_id = pt.id
         WHERE p.level = ?1 AND p.x IS NOT NULL
         ORDER BY pt.tag",
        &[&level as &dyn rusqlite::types::ToSql],
    )?;

    let segments = doc.query_params(
        "SELECT n1.x, n1.y, n2.x, n2.y, seg.diameter_m, seg.width_m
         FROM segments seg
         JOIN nodes n1 ON seg.from_node_id = n1.id
         JOIN nodes n2 ON seg.to_node_id = n2.id
         WHERE n1.level = ?1 AND n1.x IS NOT NULL AND n2.x IS NOT NULL",
        &[&level as &dyn rusqlite::types::ToSql],
    )?;

    // ARCH D (24x36")
    let page_w = Mm(914.4);
    let page_h = Mm(609.6_f32);
    let pdf = PdfDocument::empty(&format!("{} — {}", info.project_name, level));
    let (page_idx, layer_idx) = pdf.add_page(page_w, page_h, level);
    let layer = pdf.get_page(page_idx).get_layer(layer_idx);

    // Coordinate transform
    let (mut x_min, mut y_min, mut x_max, mut y_max) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for room in &rooms {
        for v in &room.vertices {
            x_min = x_min.min(v.x); y_min = y_min.min(v.y);
            x_max = x_max.max(v.x); y_max = y_max.max(v.y);
        }
    }
    if x_min == f64::MAX { x_min = 0.0; y_min = 0.0; x_max = 20.0; y_max = 20.0; }

    let margin: f32 = 50.0;
    let avail_w: f32 = 914.4 - margin * 2.0;
    let avail_h: f32 = 609.6 - margin * 2.0;
    let model_w = (x_max - x_min) as f32;
    let model_h = (y_max - y_min) as f32;
    let scale = (avail_w / (model_w * 1000.0)).min(avail_h / (model_h * 1000.0)) * 1000.0;
    let x_min = x_min as f32;
    let y_min = y_min as f32;

    let tx = move |x: f64| -> Mm { Mm(margin + (x as f32 - x_min) * scale) };
    let ty = move |y: f64| -> Mm { Mm(margin + (y as f32 - y_min) * scale) };

    // Rooms
    for room in &rooms {
        if room.vertices.len() < 3 { continue; }
        let points: Vec<(Point, bool)> = room.vertices.iter().map(|v| (Point::new(tx(v.x), ty(v.y)), false)).collect();
        let outline_color = if room.scope == "nic" {
            Color::Greyscale(Greyscale::new(0.7, None))
        } else {
            Color::Rgb(Rgb::new(0.2, 0.4, 0.8, None))
        };
        layer.set_outline_color(outline_color);
        layer.set_outline_thickness(0.5);
        layer.add_line(Line { points, is_closed: true });

        let label_x = tx(room.vertices.iter().map(|v| v.x).sum::<f64>() / room.vertices.len() as f64);
        let label_y = ty(room.vertices.iter().map(|v| v.y).sum::<f64>() / room.vertices.len() as f64);
        let font = pdf.add_builtin_font(BuiltinFont::Helvetica).unwrap();
        layer.use_text(&format!("{} — {}", room.tag, room.name), 6.0, label_x, label_y, &font);
    }

    // Duct segments
    layer.set_outline_color(Color::Greyscale(Greyscale::new(0.4, None)));
    layer.set_outline_thickness(0.3);
    for seg in &segments {
        let x1: f64 = seg[0].1.parse().unwrap_or(0.0);
        let y1: f64 = seg[1].1.parse().unwrap_or(0.0);
        let x2: f64 = seg[2].1.parse().unwrap_or(0.0);
        let y2: f64 = seg[3].1.parse().unwrap_or(0.0);
        layer.add_line(Line {
            points: vec![(Point::new(tx(x1), ty(y1)), false), (Point::new(tx(x2), ty(y2)), false)],
            is_closed: false,
        });
    }

    // Placement markers
    let font = pdf.add_builtin_font(BuiltinFont::Helvetica).unwrap();
    for p in &placements {
        let x: f64 = p[3].1.parse().unwrap_or(0.0);
        let y: f64 = p[4].1.parse().unwrap_or(0.0);
        let tag = &p[0].1;
        let cfm = &p[5].1;
        let r: f32 = 1.5;
        let px = tx(x);
        let py = ty(y);

        let color = match p[2].1.as_str() {
            "equipment" => Color::Rgb(Rgb::new(0.8, 0.2, 0.8, None)),
            "accessory" => Color::Rgb(Rgb::new(0.8, 0.8, 0.0, None)),
            _ => {
                let cat = &p[1].1;
                if cat.contains("return") { Color::Rgb(Rgb::new(0.2, 0.7, 0.2, None)) }
                else if cat.contains("exhaust") { Color::Rgb(Rgb::new(0.8, 0.2, 0.2, None)) }
                else { Color::Rgb(Rgb::new(0.2, 0.5, 0.9, None)) }
            }
        };
        layer.set_fill_color(color);
        layer.add_line(Line {
            points: vec![
                (Point::new(px, Mm(py.0 + r)), false),
                (Point::new(Mm(px.0 + r), py), false),
                (Point::new(px, Mm(py.0 - r)), false),
                (Point::new(Mm(px.0 - r), py), false),
            ],
            is_closed: true,
        });

        layer.set_fill_color(Color::Greyscale(Greyscale::new(0.3, None)));
        let label = if cfm != "NULL" { format!("{} {}CFM", tag, cfm) } else { tag.clone() };
        layer.use_text(&label, 4.0, Mm(px.0 + 2.0_f32), py, &font);
    }

    // Title block
    let title_font = pdf.add_builtin_font(BuiltinFont::HelveticaBold).unwrap();
    layer.use_text(&info.project_name, 14.0, Mm(margin), Mm(609.6_f32 - 20.0), &title_font);
    layer.use_text(&format!("#{} — {}", info.project_number, level), 10.0, Mm(margin), Mm(609.6_f32 - 32.0), &font);
    layer.use_text("Generated from .sed file", 6.0, Mm(margin), Mm(609.6_f32 - 40.0), &font);

    pdf.save(&mut std::io::BufWriter::new(std::fs::File::create(output)?))?;
    println!("Exported: {}", output);
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
