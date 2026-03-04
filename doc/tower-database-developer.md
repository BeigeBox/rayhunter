# Tower Database Maintenance

This guide covers how to build and update the tower database used for unknown tower detection. The database is built from [OpenCellID](https://opencellid.org/) data using the `rayhunter-tower-db` CLI tool.

## Getting OpenCellID Data

1. Create an account at [opencellid.org](https://opencellid.org/)
2. Get your API token from your account page
3. Download the full cell tower dump:

```bash
wget -O cell_towers.csv.gz \
  "https://opencellid.org/ocid/downloads?token=YOUR_TOKEN&type=full&file=cell_towers.csv.gz"
gunzip cell_towers.csv.gz
```

OpenCellID also publishes daily differential files (`cell_towers_diff-*.csv.gz`) that contain only new and updated towers since the last full dump.

## Building the Database

### Initial Setup

```bash
cargo build --release -p rayhunter-tower-db

# Create a new database
rayhunter-tower-db init

# Import the full CSV dump (~5M rows, takes about a minute)
RUST_LOG=info rayhunter-tower-db import-full --csv-path cell_towers.csv
```

### Applying Differentials

Download and import daily diffs to keep the database current. Already-imported diffs are automatically skipped.

```bash
rayhunter-tower-db import-diff --csv-path cell_towers_diff-2026-03-05.csv.gz
rayhunter-tower-db import-diff --csv-path cell_towers_diff-2026-03-06.csv.gz
```

### Checking the Database

```bash
# Show row counts by radio type and import history
rayhunter-tower-db stats

# Look up a specific tower
rayhunter-tower-db lookup LTE 310 260 12345 67890

# Find towers near a location (radius in meters)
rayhunter-tower-db nearby 37.7749 -122.4194 --radius 5000
```

## Exporting State Databases

The full database is too large for mobile devices. Export per-state databases filtered to US MCCs and relevant radio types:

```bash
# Export a single state
rayhunter-tower-db export \
  --mcc 310,311,312,313,316 \
  --radio LTE,NR \
  --states OR \
  --output oregon.db

# Export all 50 states + DC (for size analysis or release prep)
rayhunter-tower-db export-all-states \
  --radio LTE,NR \
  --output-dir state-dbs/
```

### Export Options

| Flag | Description |
|------|-------------|
| `--mcc` | MCC codes to include (required). US carriers use 310, 311, 312, 313, 316. |
| `--radio` | Radio types: GSM, UMTS, LTE, NR, CDMA. Defaults to all. LTE and NR are most relevant for modern devices. |
| `--states` | US state codes for geographic filtering (e.g., `CA,OR,WA`). Uses bounding boxes. |
| `--no-rtree` | Exclude the R-tree geospatial index. Saves space when geospatial queries aren't needed on device. |

### Typical State Sizes (LTE+NR, US MCCs)

Most states produce databases between 0.2 and 5 MB. California is the largest at about 10 MB. Total across all states is roughly 100 MB.

## Publishing State Databases

State databases are hosted on a long-lived GitHub Release tagged `tower-db`, separate from Rayhunter code releases. This allows tower data to be updated on its own schedule (e.g., monthly) without cutting a new Rayhunter release.

The installer and web UI download state databases from this release using the GitHub Releases API.

### Update workflow

1. Download the latest full OpenCellID dump (or apply recent diffs to an existing database)
2. Export all state databases
3. Upload them to the `tower-db` release, replacing the previous assets

```bash
# Build the master database
rayhunter-tower-db init --db-path towers.db
RUST_LOG=info rayhunter-tower-db import-full --csv-path cell_towers.csv --db-path towers.db

# Export per-state databases
rayhunter-tower-db export-all-states --radio LTE,NR --output-dir state-dbs/

# Upload to the tower-db release (replaces existing assets)
for db in state-dbs/*.db; do
  gh release upload tower-db "$db" --clobber
done
```

### Creating the release for the first time

```bash
gh release create tower-db --title "Tower Database" \
  --notes "Per-state cell tower databases built from OpenCellID. Updated independently of Rayhunter releases." \
  state-dbs/*.db
```

GitHub Releases have a 2 GB per-file limit and no bandwidth cap, so this is well within limits (total across all states is ~100 MB).

## Schema Reference

The database has three tables:

**`cell_towers`** stores tower data with a unique constraint on `(radio, mcc, mnc, tac, cid)`. The `first_seen` and `last_seen` fields track when a tower first appeared in and was last present in an OpenCellID dump. On upsert, `first_seen` is preserved and `last_seen` is updated.

**`cell_towers_geo`** is an R-tree virtual table indexing towers by longitude/latitude for spatial queries.

**`import_history`** records each import (filename, type, row count, timestamp) and prevents duplicate diff imports.
