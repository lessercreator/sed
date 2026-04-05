use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use sed_sdk::SedDocument;
use std::collections::BTreeMap;

mod export;
mod html_export;

#[derive(Parser)]
#[command(name = "sedtool", about = "Structured Engineering Document — CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Open a .sed file in the browser (generates interactive HTML viewer)
    Open {
        /// Path to .sed file
        file: String,
    },
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
    /// Export a .sed file to PDF (single level)
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
    /// Export all plan sheets + equipment schedule to a single PDF
    ExportPdfAll {
        /// Path to .sed file
        file: String,
        /// Output PDF path
        #[arg(short, long, default_value = "output.pdf")]
        output: String,
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
    /// Run design checks (duct sizing, air balance, connectivity)
    Check {
        /// Path to .sed file
        file: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Get AI-powered design suggestions
    Suggest {
        /// Path to .sed file
        file: String,
    },
    /// Generate a markdown project summary
    ReportMd {
        /// Path to .sed file
        file: String,
        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Export interactive HTML plan (opens in any browser)
    View {
        /// Path to .sed file
        file: String,
        /// Output HTML path
        #[arg(short, long, default_value = "plan.html")]
        output: String,
        /// Level
        #[arg(short, long, default_value = "Level 1")]
        level: String,
    },
    /// Add a redline note to a .sed file
    Markup {
        /// Path to .sed file
        file: String,
        /// Note text
        text: String,
        /// Level
        #[arg(short, long, default_value = "Level 1")]
        level: String,
        /// Position X (meters)
        #[arg(long, default_value = "0")]
        x: f64,
        /// Position Y (meters)
        #[arg(long, default_value = "0")]
        y: f64,
        /// Author name
        #[arg(short, long, default_value = "Contractor")]
        author: String,
    },
    /// Export all levels as interactive HTML (opens in any browser)
    ViewAll {
        /// Path to .sed file
        file: String,
        /// Output HTML path
        #[arg(short, long, default_value = "plan-all.html")]
        output: String,
    },
    /// Create a blank project with the default HVAC equipment catalog
    Catalog {
        /// Output path (default: catalog.sed)
        #[arg(default_value = "catalog.sed")]
        output: String,
        /// Project name
        #[arg(short, long, default_value = "New Project")]
        name: String,
        /// Project number
        #[arg(short = 'N', long, default_value = "000")]
        number: String,
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
        Commands::Open { file } => {
            let out = file.replace(".sed", ".html");
            let doc = SedDocument::open(&file)?;
            let levels: Vec<String> = doc.query_raw("SELECT DISTINCT level FROM spaces WHERE x IS NOT NULL ORDER BY level")?
                .into_iter().filter_map(|r| if r[0].1 != "NULL" { Some(r[0].1.clone()) } else { None }).collect();
            let level = levels.first().map(|s| s.as_str()).unwrap_or("Level 1");
            html_export::export_html(&file, &out, level)?;
            let _ = open::that(&out);
            Ok(())
        }
        Commands::Info { file } => cmd_info(&file),
        Commands::Query { file, sql } => cmd_query(&file, &sql),
        Commands::Report { file, name } => cmd_report(&file, &name),
        Commands::Validate { file } => cmd_validate(&file),
        Commands::Example { output } => cmd_example(&output),
        Commands::Office { output } => cmd_office(&output),
        Commands::ExportPdf { file, output, level } => export::export_pdf(&file, &output, &level),
        Commands::ExportPdfAll { file, output } => export::export_pdf_all(&file, &output),
        Commands::Diff { old, new, json } => cmd_diff(&old, &new, json),
        Commands::Stats { file } => cmd_stats(&file),
        Commands::ExportSchedule { file, output, schedule_type } => {
            let filter = match schedule_type {
                ScheduleType::All => "all",
                ScheduleType::Equipment => "equipment",
                ScheduleType::AirDevices => "air_devices",
            };
            export::export_schedule(&file, &output, filter)
        }
        Commands::ImportCsv { csv, output, name, number } => cmd_import_csv(&csv, &output, &name, &number),
        Commands::Ask { file, question } => cmd_ask(&file, &question),
        Commands::Check { file, json } => cmd_check(&file, json),
        Commands::Suggest { file } => cmd_suggest(&file),
        Commands::ReportMd { file, output } => {
            let doc = SedDocument::open(&file)?;
            let md = sed_sdk::report::project_summary(&doc)?;
            if let Some(path) = output {
                std::fs::write(&path, &md)?;
                println!("Written: {}", path);
            } else {
                print!("{}", md);
            }
            Ok(())
        }
        Commands::Markup { file, text, level, x, y, author } => {
            let doc = SedDocument::open(&file)?;
            let id = sed_sdk::markup::add_text_note(&doc, &level, x, y, &text, &author)?;
            println!("Added markup {} at ({}, {}) on {}", id, x, y, level);
            Ok(())
        }
        Commands::View { file, output, level } => {
            html_export::export_html(&file, &output, &level)?;
            let _ = open::that(&output);
            Ok(())
        }
        Commands::ViewAll { file, output } => {
            html_export::export_html_all(&file, &output)?;
            let _ = open::that(&output);
            Ok(())
        }
        Commands::Catalog { output, name, number } => cmd_catalog(&output, &name, &number),
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

fn cmd_check(file: &str, json: bool) -> Result<()> {
    let doc = SedDocument::open(file)?;
    let issues = sed_sdk::design_check::check_design(&doc)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&issues)?);
        return Ok(());
    }

    if issues.is_empty() {
        println!("PASS -- no design issues found");
        return Ok(());
    }

    let errors = issues.iter().filter(|i| i.severity == sed_sdk::design_check::Severity::Error).count();
    let warnings = issues.iter().filter(|i| i.severity == sed_sdk::design_check::Severity::Warning).count();
    let infos = issues.iter().filter(|i| i.severity == sed_sdk::design_check::Severity::Info).count();

    for issue in &issues {
        println!("{}", issue);
    }

    println!("\n{} error(s), {} warning(s), {} info(s)", errors, warnings, infos);

    if errors > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn cmd_suggest(file: &str) -> Result<()> {
    let doc = SedDocument::open(file)?;
    let suggestions = sed_sdk::suggest::suggest(&doc)?;
    if suggestions.is_empty() {
        println!("No suggestions — design looks good.");
    } else {
        println!("{} suggestion(s):\n", suggestions.len());
        for s in &suggestions {
            println!("{}", s);
        }
    }
    Ok(())
}

fn cmd_catalog(output: &str, name: &str, number: &str) -> Result<()> {
    println!("Creating blank project with default catalog: {}", output);
    let doc = SedDocument::create(output)?;
    doc.set_meta("sed_version", "0.3")?;
    doc.set_meta("project_name", name)?;
    doc.set_meta("project_number", number)?;

    let count = sed_sdk::catalog::populate_default_catalog(&doc)?;
    println!("Populated {} product types.", count);

    let info = doc.info()?;
    print!("\n{}", info);
    Ok(())
}
