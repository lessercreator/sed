use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use sed_sdk::SedDocument;
use std::collections::BTreeMap;

#[derive(Parser)]
#[command(name = "sedtool", about = "Structured Engineering Document — CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show document summary
    Info {
        /// Path to .sed file
        file: String,
    },
    /// Run a SQL query against the document
    Query {
        /// Path to .sed file
        file: String,
        /// SQL query string
        sql: String,
    },
    /// Run a named pre-built query
    Report {
        /// Path to .sed file
        file: String,
        /// Report name: cfm, devices, submittals, equipment, ducts
        name: String,
    },
    /// Validate document structure
    Validate {
        /// Path to .sed file
        file: String,
    },
    /// Create the SKIMS Americana example file
    Example {
        /// Output path (default: skims-americana.sed)
        #[arg(default_value = "skims-americana.sed")]
        output: String,
    },
    /// Create the Office Tower example file (complex building stress test)
    Office {
        /// Output path (default: office-tower.sed)
        #[arg(default_value = "office-tower.sed")]
        output: String,
    },
    /// Export a .sed file to PDF
    ExportPdf {
        /// Path to .sed file
        file: String,
        /// Output PDF path
        #[arg(short, long, default_value = "output.pdf")]
        output: String,
        /// Level to export (default: Level 1)
        #[arg(short, long, default_value = "Level 1")]
        level: String,
    },
    /// Compare two .sed files and show differences
    Diff {
        /// Old (baseline) .sed file
        old: String,
        /// New (updated) .sed file
        new: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show comprehensive project statistics
    Stats {
        /// Path to .sed file
        file: String,
    },
    /// Export equipment schedule to CSV
    ExportSchedule {
        /// Path to .sed file
        file: String,
        /// Output CSV path
        #[arg(short, long, default_value = "schedule.csv")]
        output: String,
        /// Type filter: all, equipment, or air_devices
        #[arg(short = 't', long = "type", default_value = "all")]
        schedule_type: ScheduleType,
    },
    /// Import a CSV equipment schedule into a .sed file
    ImportCsv {
        /// CSV file to import
        csv: String,
        /// Output .sed file path
        #[arg(short, long, default_value = "imported.sed")]
        output: String,
        /// Project name
        #[arg(short, long, default_value = "Imported Project")]
        name: String,
        /// Project number
        #[arg(short = 'N', long, default_value = "IMP-001")]
        number: String,
    },
    /// Ask a question in plain English
    Ask {
        /// Path to .sed file
        file: String,
        /// Question in natural language
        question: String,
    },
}

#[derive(Clone, ValueEnum)]
enum ScheduleType {
    All,
    Equipment,
    AirDevices,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Info { file } => cmd_info(&file),
        Commands::Query { file, sql } => cmd_query(&file, &sql),
        Commands::Report { file, name } => cmd_report(&file, &name),
        Commands::Validate { file } => cmd_validate(&file),
        Commands::Example { output } => cmd_example(&output),
        Commands::Office { output } => cmd_office(&output),
        Commands::ExportPdf { file, output, level } => cmd_export_pdf(&file, &output, &level),
        Commands::Diff { old, new, json } => cmd_diff(&old, &new, json),
        Commands::Stats { file } => cmd_stats(&file),
        Commands::ExportSchedule { file, output, schedule_type } => cmd_export_schedule(&file, &output, &schedule_type),
        Commands::ImportCsv { csv, output, name, number } => cmd_import_csv(&csv, &output, &name, &number),
        Commands::Ask { file, question } => cmd_ask(&file, &question),
    }
}

fn cmd_info(file: &str) -> Result<()> {
    let doc = SedDocument::open(file)?;
    let info = doc.info()?;
    print!("{}", info);
    Ok(())
}

fn cmd_query(file: &str, sql: &str) -> Result<()> {
    let doc = SedDocument::open(file)?;
    let rows = doc.query_raw(sql)?;

    if rows.is_empty() {
        println!("(no results)");
        return Ok(());
    }

    // Print header
    let headers: Vec<&str> = rows[0].iter().map(|(k, _)| k.as_str()).collect();
    let mut col_widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();

    // Calculate column widths
    for row in &rows {
        for (i, (_, val)) in row.iter().enumerate() {
            col_widths[i] = col_widths[i].max(val.len());
        }
    }

    // Print header
    for (i, h) in headers.iter().enumerate() {
        print!("{:width$}  ", h, width = col_widths[i]);
    }
    println!();
    for w in &col_widths {
        print!("{:-<width$}  ", "", width = *w);
    }
    println!();

    // Print rows
    for row in &rows {
        for (i, (_, val)) in row.iter().enumerate() {
            print!("{:width$}  ", val, width = col_widths[i]);
        }
        println!();
    }

    println!("\n({} rows)", rows.len());
    Ok(())
}

fn cmd_report(file: &str, name: &str) -> Result<()> {
    let sql = match name {
        "cfm" => sed_sdk::query::SUPPLY_CFM_BY_ROOM,
        "devices" => sed_sdk::query::ALL_PLACEMENTS_BY_TYPE,
        "submittals" => sed_sdk::query::SUBMITTAL_STATUS,
        "equipment" => sed_sdk::query::EQUIPMENT_LIST,
        "ducts" => sed_sdk::query::DUCT_SUMMARY_BY_SYSTEM,
        other => {
            eprintln!("Unknown report: {}. Available: cfm, devices, submittals, equipment, ducts", other);
            return Ok(());
        }
    };
    cmd_query(file, sql)
}

fn cmd_validate(file: &str) -> Result<()> {
    let doc = SedDocument::open(file)?;
    let result = sed_sdk::validate::validate(&doc)?;
    print!("{}", result);
    if result.is_valid() {
        std::process::exit(0);
    } else {
        std::process::exit(1);
    }
}

fn cmd_example(output: &str) -> Result<()> {
    println!("Creating SKIMS Americana example: {}", output);
    sed_sdk::examples::create_skims_americana(output)?;
    println!("Done.");

    let doc = SedDocument::open(output)?;
    let info = doc.info()?;
    print!("\n{}", info);
    Ok(())
}

fn cmd_office(output: &str) -> Result<()> {
    println!("Creating Office Tower example: {}", output);
    sed_sdk::examples_office::create_office_tower(output)?;
    println!("Done.");

    let doc = SedDocument::open(output)?;
    let info = doc.info()?;
    print!("\n{}", info);
    Ok(())
}

fn cmd_export_pdf(file: &str, output: &str, level: &str) -> Result<()> {
    use printpdf::*;

    let doc = SedDocument::open(file)?;
    let info = doc.info()?;

    // Get room geometry
    let rooms = sed_sdk::geometry::get_room_geometry(&doc, level)?;

    // Get placements on this level
    let placements = doc.query_raw(&format!(
        "SELECT pt.tag, pt.category, pt.domain, p.x, p.y, p.cfm, p.instance_tag, p.status
         FROM placements p
         JOIN product_types pt ON p.product_type_id = pt.id
         WHERE p.level = '{}' AND p.x IS NOT NULL
         ORDER BY pt.tag", level.replace('\'', "''")
    ))?;

    // Get segments on this level
    let segments = doc.query_raw(&format!(
        "SELECT n1.x, n1.y, n2.x, n2.y, seg.diameter_m, seg.width_m
         FROM segments seg
         JOIN nodes n1 ON seg.from_node_id = n1.id
         JOIN nodes n2 ON seg.to_node_id = n2.id
         WHERE n1.level = '{}' AND n1.x IS NOT NULL AND n2.x IS NOT NULL",
        level.replace('\'', "''")
    ))?;

    // Page setup: ARCH D (24x36 inches)
    let page_w = Mm(914.4); // 36"
    let page_h = Mm(609.6_f32); // 24"
    let pdf = PdfDocument::empty(&format!("{} — {}", info.project_name, level));
    let (page_idx, layer_idx) = pdf.add_page(page_w, page_h, level);
    let layer = pdf.get_page(page_idx).get_layer(layer_idx);

    // Coordinate transform: model meters -> PDF mm
    // Find bounds
    let (mut x_min, mut y_min, mut x_max, mut y_max) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for room in &rooms {
        for v in &room.vertices {
            x_min = x_min.min(v.x);
            y_min = y_min.min(v.y);
            x_max = x_max.max(v.x);
            y_max = y_max.max(v.y);
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

    // Draw rooms
    for room in &rooms {
        if room.vertices.len() < 3 { continue; }
        let points: Vec<(Point, bool)> = room.vertices.iter().map(|v| {
            (Point::new(tx(v.x), ty(v.y)), false)
        }).collect();
        let line = Line {
            points,
            is_closed: true,
        };
        let outline_color = if room.scope == "nic" {
            Color::Greyscale(Greyscale::new(0.7, None))
        } else {
            Color::Rgb(Rgb::new(0.2, 0.4, 0.8, None))
        };
        layer.set_outline_color(outline_color);
        layer.set_outline_thickness(0.5);
        layer.add_line(line);

        // Room label
        if let (Some(_first), Some(_second)) = (room.vertices.first(), room.vertices.get(1)) {
            let label_x = tx(room.vertices.iter().map(|v| v.x).sum::<f64>() / room.vertices.len() as f64);
            let label_y = ty(room.vertices.iter().map(|v| v.y).sum::<f64>() / room.vertices.len() as f64);
            let font = pdf.add_builtin_font(BuiltinFont::Helvetica).unwrap();
            layer.use_text(&format!("{} — {}", room.tag, room.name), 6.0, label_x, label_y, &font);
        }
    }

    // Draw duct segments
    layer.set_outline_color(Color::Greyscale(Greyscale::new(0.4, None)));
    layer.set_outline_thickness(0.3);
    for seg in &segments {
        let x1: f64 = seg[0].1.parse().unwrap_or(0.0);
        let y1: f64 = seg[1].1.parse().unwrap_or(0.0);
        let x2: f64 = seg[2].1.parse().unwrap_or(0.0);
        let y2: f64 = seg[3].1.parse().unwrap_or(0.0);
        let line = Line {
            points: vec![
                (Point::new(tx(x1), ty(y1)), false),
                (Point::new(tx(x2), ty(y2)), false),
            ],
            is_closed: false,
        };
        layer.add_line(line);
    }

    // Draw placement markers
    let font = pdf.add_builtin_font(BuiltinFont::Helvetica).unwrap();
    for p in &placements {
        let x: f64 = p[3].1.parse().unwrap_or(0.0);
        let y: f64 = p[4].1.parse().unwrap_or(0.0);
        let tag = &p[0].1;
        let cfm = &p[5].1;

        // Draw a small circle (approximated as a diamond)
        let r = 1.5; // mm
        let px = tx(x);
        let py = ty(y);

        let domain = &p[2].1;
        let color = match domain.as_str() {
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
        let marker = Line {
            points: vec![
                (Point::new(px, Mm(py.0 + r as f32)), false),
                (Point::new(Mm(px.0 + r as f32), py), false),
                (Point::new(px, Mm(py.0 - r as f32)), false),
                (Point::new(Mm(px.0 - r as f32), py), false),
            ],
            is_closed: true,
        };
        layer.add_line(marker);

        // Tag label
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

fn cmd_diff(old_path: &str, new_path: &str, json: bool) -> Result<()> {
    let old = SedDocument::open(old_path)?;
    let new = SedDocument::open(new_path)?;
    let result = sed_sdk::diff::diff(&old, &new)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        print!("{}", result);
    }
    Ok(())
}

fn cmd_stats(file: &str) -> Result<()> {
    let doc = SedDocument::open(file)?;
    let info = doc.info()?;

    // Header
    println!("Project Statistics: {} (#{})", info.project_name, info.project_number);
    let header_len = format!("Project Statistics: {} (#{})", info.project_name, info.project_number).len();
    println!("{}", "\u{2550}".repeat(header_len));
    println!();

    // SYSTEMS — query systems with design CFM from source equipment or graph terminals
    let systems = doc.query_raw(
        "SELECT s.tag, s.name, s.system_type, s.medium,
                COALESCE(
                    (SELECT CAST(SUM(p.cfm) AS INTEGER) FROM nodes n
                     JOIN placements p ON n.placement_id = p.id
                     WHERE n.system_id = s.id AND n.node_type = 'terminal' AND p.cfm IS NOT NULL),
                    (SELECT CAST(src.cfm AS INTEGER) FROM placements src WHERE src.id = s.source_id)
                ) as design_cfm
         FROM systems s ORDER BY s.tag"
    )?;
    if !systems.is_empty() {
        println!("SYSTEMS");
        for row in &systems {
            let tag = &row[0].1;
            let name = &row[1].1;
            let cfm = &row[4].1;
            let cfm_str = if cfm != "NULL" {
                format!("    {} CFM design", cfm)
            } else {
                String::new()
            };
            println!("  {:<12}{}{}", tag, name, cfm_str);
        }
        println!();
    }

    // AIR BALANCE — per level, sum CFM by supply/return/exhaust categories
    let air_data = doc.query_raw(
        "SELECT p.level, pt.category,
                COALESCE(SUM(p.cfm), 0) as total_cfm
         FROM placements p
         JOIN product_types pt ON p.product_type_id = pt.id
         WHERE p.cfm IS NOT NULL AND pt.domain = 'air_device'
         GROUP BY p.level, pt.category
         ORDER BY p.level, pt.category"
    )?;
    if !air_data.is_empty() {
        // Group by level. Classify categories into supply/return/exhaust.
        let mut levels: BTreeMap<String, (f64, f64, f64)> = BTreeMap::new();
        for row in &air_data {
            let level = &row[0].1;
            let category = &row[1].1;
            let cfm: f64 = row[2].1.parse().unwrap_or(0.0);
            let entry = levels.entry(level.clone()).or_insert((0.0, 0.0, 0.0));
            if category.contains("supply") || category.contains("ceiling_diffuser") {
                entry.0 += cfm;
            } else if category.contains("return") {
                entry.1 += cfm;
            } else if category.contains("exhaust") {
                entry.2 += cfm;
            }
        }
        println!("AIR BALANCE");
        for (level, (supply, ret, exhaust)) in &levels {
            let net = supply - ret - exhaust;
            let sign = if net >= 0.0 { "+" } else { "" };
            println!("  {}: Supply {:.0} CFM | Return {:.0} CFM | Exhaust {:.0} CFM | Net {}{:.0}",
                     level, supply, ret, exhaust, sign, net);
        }
        println!();
    }

    // EQUIPMENT SUMMARY
    let equipment = doc.query_raw(
        "SELECT COUNT(*) as total,
                SUM(CASE WHEN p.status LIKE 'existing%' THEN 1 ELSE 0 END) as existing,
                SUM(CASE WHEN p.status = 'new' THEN 1 ELSE 0 END) as new_count
         FROM placements p
         JOIN product_types pt ON p.product_type_id = pt.id
         WHERE pt.domain = 'equipment'"
    )?;
    if !equipment.is_empty() {
        let total = &equipment[0][0].1;
        let existing = &equipment[0][1].1;
        let new_count = &equipment[0][2].1;
        if total != "0" {
            println!("EQUIPMENT SUMMARY");
            println!("  {} equipment items ({} existing, {} new)", total, existing, new_count);
            println!();
        }
    }

    // DEVICE COUNT BY CATEGORY
    let categories = doc.query_raw(
        "SELECT pt.category, COUNT(*) as cnt
         FROM placements p
         JOIN product_types pt ON p.product_type_id = pt.id
         GROUP BY pt.category
         ORDER BY cnt DESC"
    )?;
    if !categories.is_empty() {
        println!("DEVICE COUNT BY CATEGORY");
        // Find max category name length for alignment
        let max_len = categories.iter().map(|r| r[0].1.len()).max().unwrap_or(0);
        for row in &categories {
            println!("  {:width$}  {:>3}", row[0].1, row[1].1, width = max_len);
        }
        println!();
    }

    // SUBMITTAL STATUS
    let submittals = doc.query_raw(
        "SELECT COUNT(*) as total,
                SUM(CASE WHEN status = 'approved' THEN 1 ELSE 0 END) as approved,
                SUM(CASE WHEN status IN ('pending', 'submitted', 'for_approval') THEN 1 ELSE 0 END) as pending,
                SUM(CASE WHEN status = 'rejected' THEN 1 ELSE 0 END) as rejected
         FROM submittals"
    )?;
    if !submittals.is_empty() {
        let total = &submittals[0][0].1;
        if total != "0" {
            let approved = &submittals[0][1].1;
            let pending = &submittals[0][2].1;
            let rejected = &submittals[0][3].1;
            println!("SUBMITTAL STATUS");
            println!("  {} total: {} approved, {} pending, {} rejected", total, approved, pending, rejected);
            println!();
        }
    }

    // GRAPH COVERAGE
    let nodes_count = info.nodes;
    let segments_count = info.segments;
    if nodes_count > 0 || segments_count > 0 {
        let terminals = doc.query_raw(
            "SELECT COUNT(*) FROM nodes WHERE node_type = 'terminal'"
        )?;
        let terminal_count = if !terminals.is_empty() { terminals[0][0].1.clone() } else { "0".to_string() };
        println!("GRAPH COVERAGE");
        println!("  {} nodes, {} segments", nodes_count, segments_count);
        println!("  {} terminal connections", terminal_count);
        println!();
    }

    // COMPLETENESS
    let positioned = doc.query_raw(
        "SELECT COUNT(*) FROM placements WHERE x IS NOT NULL AND y IS NOT NULL"
    )?;
    let unpositioned = doc.query_raw(
        "SELECT COUNT(*) FROM placements WHERE x IS NULL OR y IS NULL"
    )?;
    let pos_count: i64 = if !positioned.is_empty() { positioned[0][0].1.parse().unwrap_or(0) } else { 0 };
    let unpos_count: i64 = if !unpositioned.is_empty() { unpositioned[0][0].1.parse().unwrap_or(0) } else { 0 };

    println!("COMPLETENESS");
    println!("  {} spaces defined", info.spaces);
    println!("  {} placements ({} positioned, {} unpositioned)", pos_count + unpos_count, pos_count, unpos_count);
    println!("  {} keyed notes", info.keyed_notes);

    Ok(())
}

fn cmd_export_schedule(file: &str, output: &str, schedule_type: &ScheduleType) -> Result<()> {
    let doc = SedDocument::open(file)?;

    let where_clause = match schedule_type {
        ScheduleType::Equipment => " WHERE pt.domain = 'equipment'",
        ScheduleType::AirDevices => " WHERE pt.domain = 'air_device'",
        ScheduleType::All => "",
    };

    let sql = format!(
        "SELECT COALESCE(p.instance_tag, pt.tag) as tag, p.instance_tag, pt.category,
                pt.manufacturer, pt.model, p.cfm, p.status, p.level,
                s.name as room, p.phase, p.notes
         FROM placements p
         JOIN product_types pt ON p.product_type_id = pt.id
         LEFT JOIN spaces s ON p.space_id = s.id
         {}
         ORDER BY p.level, pt.tag",
        where_clause
    );

    let rows = doc.query_raw(&sql)?;

    let out_file = std::fs::File::create(output)?;
    let mut wtr = csv::Writer::from_writer(out_file);

    // Write header
    wtr.write_record(&["Tag", "Instance Tag", "Category", "Manufacturer", "Model", "CFM", "Status", "Level", "Room", "Phase", "Notes"])?;

    // Write data rows
    for row in &rows {
        let record: Vec<&str> = row.iter().map(|(_, v)| {
            if v == "NULL" { "" } else { v.as_str() }
        }).collect();
        wtr.write_record(&record)?;
    }

    wtr.flush()?;
    println!("Exported {} rows to {}", rows.len(), output);
    Ok(())
}

fn cmd_import_csv(csv_path: &str, output: &str, name: &str, number: &str) -> Result<()> {
    println!("Importing: {} -> {}", csv_path, output);
    let mapping = sed_sdk::import::ColumnMapping::default();
    let result = sed_sdk::import::import_csv(csv_path, output, name, number, &mapping)?;
    print!("{}", result);

    let doc = SedDocument::open(output)?;
    let info = doc.info()?;
    print!("\n{}", info);
    Ok(())
}

fn cmd_ask(file: &str, question: &str) -> Result<()> {
    let doc = SedDocument::open(file)?;
    let result = sed_sdk::nlq::ask(&doc, question)?;
    print!("{}", result);
    Ok(())
}
