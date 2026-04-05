use anyhow::Result;
use clap::{Parser, Subcommand};
use sed_sdk::SedDocument;

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
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // Check required meta keys
    for key in ["sed_version", "project_name", "project_number"] {
        if doc.get_meta(key)?.is_none() {
            errors.push(format!("Missing required meta key: {}", key));
        }
    }

    // Check for orphaned placements (product_type_id that doesn't exist)
    let orphaned: Vec<Vec<(String, String)>> = doc.query_raw(
        "SELECT p.id FROM placements p LEFT JOIN product_types pt ON p.product_type_id = pt.id WHERE pt.id IS NULL"
    )?;
    if !orphaned.is_empty() {
        errors.push(format!("{} placements reference non-existent product types", orphaned.len()));
    }

    // Check for placements without spaces
    let no_space: Vec<Vec<(String, String)>> = doc.query_raw(
        "SELECT COUNT(*) as n FROM placements WHERE space_id IS NULL AND status = 'new'"
    )?;
    if let Some(row) = no_space.first() {
        if let Some((_, count)) = row.first() {
            if count != "0" {
                warnings.push(format!("{} new placements have no assigned space", count));
            }
        }
    }

    // Check for disconnected graph nodes
    let disconnected: Vec<Vec<(String, String)>> = doc.query_raw(
        "SELECT COUNT(*) as n FROM nodes n
         WHERE n.id NOT IN (SELECT from_node_id FROM segments)
         AND n.id NOT IN (SELECT to_node_id FROM segments)
         AND (SELECT COUNT(*) FROM segments) > 0"
    )?;
    if let Some(row) = disconnected.first() {
        if let Some((_, count)) = row.first() {
            if count != "0" {
                warnings.push(format!("{} graph nodes are not connected to any segment", count));
            }
        }
    }

    if errors.is_empty() && warnings.is_empty() {
        println!("VALID — no errors, no warnings");
    } else {
        for e in &errors {
            println!("ERROR: {}", e);
        }
        for w in &warnings {
            println!("WARN:  {}", w);
        }
        println!("\n{} error(s), {} warning(s)", errors.len(), warnings.len());
    }

    Ok(())
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
