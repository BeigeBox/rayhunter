use clap::{Parser, Subcommand};
use tower_db::export::{BoundingBox, ExportOptions};
use tower_db::states;
use tower_db::{CellIdentity, TowerDb};

#[derive(Parser)]
#[command(
    name = "rayhunter-tower-db",
    version,
    about = "Cell tower database management"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new empty tower database
    Init {
        #[arg(long, default_value = "towers.db")]
        db_path: String,
    },
    /// Import a full OpenCellID CSV dump
    ImportFull {
        #[arg(long, default_value = "towers.db")]
        db_path: String,
        #[arg(long)]
        csv_path: String,
    },
    /// Import an OpenCellID differential CSV (.csv.gz)
    ImportDiff {
        #[arg(long, default_value = "towers.db")]
        db_path: String,
        #[arg(long)]
        csv_path: String,
    },
    /// Export a filtered database for a device
    Export {
        #[arg(long, default_value = "towers.db")]
        db_path: String,
        #[arg(long)]
        output: String,
        /// MCC codes to include (comma-separated)
        #[arg(long, value_delimiter = ',')]
        mcc: Vec<u16>,
        /// Radio types to include (e.g. LTE,NR). Defaults to all.
        #[arg(long, value_delimiter = ',')]
        radio: Vec<String>,
        /// US state codes to filter by geography (e.g. CA,NY)
        #[arg(long, value_delimiter = ',')]
        states: Vec<String>,
        /// Exclude R-tree geospatial index from export
        #[arg(long)]
        no_rtree: bool,
    },
    /// Export one database per US state (size analysis)
    ExportAllStates {
        #[arg(long, default_value = "towers.db")]
        db_path: String,
        /// Output directory
        #[arg(long)]
        output_dir: String,
        /// Radio types to include (e.g. LTE,NR). Defaults to all.
        #[arg(long, value_delimiter = ',')]
        radio: Vec<String>,
    },
    /// Show database statistics
    Stats {
        #[arg(long, default_value = "towers.db")]
        db_path: String,
    },
    /// Look up a specific cell tower
    Lookup {
        #[arg(long, default_value = "towers.db")]
        db_path: String,
        radio: String,
        mcc: u16,
        mnc: u16,
        tac: u32,
        cid: u64,
    },
    /// Find towers near a location
    #[command(allow_negative_numbers = true)]
    Nearby {
        #[arg(long, default_value = "towers.db")]
        db_path: String,
        lat: f64,
        lon: f64,
        /// Search radius in meters
        #[arg(long, default_value = "1000")]
        radius: u32,
    },
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn resolve_state_bbox(state_codes: &[String]) -> Result<Option<BoundingBox>, tower_db::Error> {
    if state_codes.is_empty() {
        return Ok(None);
    }

    let mut min_lat = f64::MAX;
    let mut max_lat = f64::MIN;
    let mut min_lon = f64::MAX;
    let mut max_lon = f64::MIN;

    for code in state_codes {
        let state = states::find_state(code)
            .ok_or_else(|| tower_db::Error::Export(format!("unknown state code: {code}")))?;
        min_lat = min_lat.min(state.min_lat);
        max_lat = max_lat.max(state.max_lat);
        min_lon = min_lon.min(state.min_lon);
        max_lon = max_lon.max(state.max_lon);
    }

    Ok(Some(BoundingBox {
        min_lat,
        max_lat,
        min_lon,
        max_lon,
    }))
}

fn run(cli: Cli) -> Result<(), tower_db::Error> {
    match cli.command {
        Commands::Init { db_path } => {
            TowerDb::init(&db_path)?;
            println!("created database: {db_path}");
        }
        Commands::ImportFull { db_path, csv_path } => {
            let mut db = TowerDb::open(&db_path)?;
            let stats = tower_db::import::import_full(db.connection_mut(), &csv_path)?;
            println!("imported {} rows", stats.rows_processed);
        }
        Commands::ImportDiff { db_path, csv_path } => {
            let mut db = TowerDb::open(&db_path)?;
            let stats = tower_db::import::import_diff(db.connection_mut(), &csv_path)?;
            println!("processed {} rows", stats.rows_processed);
        }
        Commands::Export {
            db_path,
            output,
            mcc,
            radio,
            states,
            no_rtree,
        } => {
            let bbox = resolve_state_bbox(&states)?;
            let stats = tower_db::export::export_device_db(&ExportOptions {
                source_path: &db_path,
                output_path: &output,
                mcc_codes: &mcc,
                radio_types: &radio,
                bbox,
                include_rtree: !no_rtree,
            })?;
            println!(
                "exported {} towers ({:.1} MB)",
                stats.rows_exported,
                stats.file_size_bytes as f64 / 1_048_576.0
            );
        }
        Commands::ExportAllStates {
            db_path,
            output_dir,
            radio,
        } => {
            std::fs::create_dir_all(&output_dir)?;
            let us_mccs: Vec<u16> = vec![310, 311, 312, 313, 316];

            let mut results: Vec<(String, String, usize, u64)> = Vec::new();

            for state in states::us_states() {
                let output_path = format!("{}/{}.db", output_dir, state.code.to_lowercase());
                let _ = std::fs::remove_file(&output_path);

                let bbox = BoundingBox {
                    min_lat: state.min_lat,
                    max_lat: state.max_lat,
                    min_lon: state.min_lon,
                    max_lon: state.max_lon,
                };

                let stats = tower_db::export::export_device_db(&ExportOptions {
                    source_path: &db_path,
                    output_path: &output_path,
                    mcc_codes: &us_mccs,
                    radio_types: &radio,
                    bbox: Some(bbox),
                    include_rtree: false,
                })?;

                results.push((
                    state.code.to_string(),
                    state.name.to_string(),
                    stats.rows_exported,
                    stats.file_size_bytes,
                ));
            }

            results.sort_by(|a, b| b.3.cmp(&a.3));

            println!(
                "{:<4} {:<22} {:>8} {:>10}",
                "Code", "State", "Towers", "Size"
            );
            println!("{}", "-".repeat(48));
            let mut total_towers = 0;
            for (code, name, towers, size) in &results {
                println!(
                    "{:<4} {:<22} {:>8} {:>7.1} MB",
                    code,
                    name,
                    towers,
                    *size as f64 / 1_048_576.0
                );
                total_towers += towers;
            }
            println!("{}", "-".repeat(48));
            println!(
                "total: {} towers across {} states",
                total_towers,
                results.len()
            );
        }
        Commands::Stats { db_path } => {
            let db = TowerDb::open_readonly(&db_path)?;
            let stats = db.stats()?;
            println!("total towers: {}", stats.total_towers);
            println!();
            for (radio, count) in &stats.by_radio {
                println!("  {radio}: {count}");
            }
            if !stats.imports.is_empty() {
                println!();
                println!("import history:");
                for imp in &stats.imports {
                    println!(
                        "  {} ({}) - {} rows at {}",
                        imp.filename, imp.import_type, imp.row_count, imp.imported_at
                    );
                }
            }
        }
        Commands::Lookup {
            db_path,
            radio,
            mcc,
            mnc,
            tac,
            cid,
        } => {
            let db = TowerDb::open_readonly(&db_path)?;
            let id = CellIdentity {
                radio,
                mcc,
                mnc,
                tac,
                cid,
            };
            match db.lookup(&id)? {
                Some(tower) => {
                    println!(
                        "found: {}/{}/{}/{}/{}",
                        tower.identity.radio,
                        tower.identity.mcc,
                        tower.identity.mnc,
                        tower.identity.tac,
                        tower.identity.cid
                    );
                    println!("  location: ({}, {})", tower.lat, tower.lon);
                    println!("  range: {} m", tower.range_m);
                    println!("  samples: {}", tower.samples);
                    println!("  PCI: {}", tower.pci);
                    println!("  signal: {}", tower.average_signal);
                    println!("  first seen by us: {}", tower.first_seen);
                    println!("  last seen by us: {}", tower.last_seen);
                    println!("  OCID created: {}", tower.ocid_created);
                    println!("  OCID updated: {}", tower.ocid_updated);
                }
                None => {
                    println!("not found");
                }
            }
        }
        Commands::Nearby {
            db_path,
            lat,
            lon,
            radius,
        } => {
            let db = TowerDb::open_readonly(&db_path)?;
            let towers = db.nearby(lat, lon, radius)?;
            println!(
                "found {} towers within {} m of ({}, {}):",
                towers.len(),
                radius,
                lat,
                lon
            );
            for t in &towers {
                println!(
                    "  {}/{}/{}/{}/{} at ({}, {}) range={}m samples={}",
                    t.identity.radio,
                    t.identity.mcc,
                    t.identity.mnc,
                    t.identity.tac,
                    t.identity.cid,
                    t.lat,
                    t.lon,
                    t.range_m,
                    t.samples
                );
            }
        }
    }

    Ok(())
}
