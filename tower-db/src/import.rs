use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, Transaction};
use serde::Deserialize;

use crate::schema;

#[derive(Debug, Deserialize)]
pub struct CsvRow {
    pub radio: String,
    pub mcc: u32,
    pub net: u32,
    pub area: u32,
    pub cell: u64,
    pub unit: i32,
    pub lon: f64,
    pub lat: f64,
    pub range: u32,
    pub samples: u32,
    pub changeable: u32,
    pub created: i64,
    pub updated: i64,
    #[serde(rename = "averageSignal")]
    pub average_signal: i32,
}

#[derive(Debug)]
pub struct ImportStats {
    pub rows_processed: usize,
}

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

const UPSERT_SQL: &str =
    "INSERT INTO cell_towers (radio, mcc, mnc, tac, cid, pci, lon, lat, range_m,
        samples, changeable, ocid_created, ocid_updated, average_signal,
        first_seen, last_seen)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15)
    ON CONFLICT(radio, mcc, mnc, tac, cid) DO UPDATE SET
        pci = excluded.pci,
        lon = excluded.lon,
        lat = excluded.lat,
        range_m = excluded.range_m,
        samples = excluded.samples,
        changeable = excluded.changeable,
        ocid_created = excluded.ocid_created,
        ocid_updated = excluded.ocid_updated,
        average_signal = excluded.average_signal,
        last_seen = excluded.last_seen";

const RTREE_UPSERT_SQL: &str =
    "INSERT OR REPLACE INTO cell_towers_geo (id, min_lon, max_lon, min_lat, max_lat)
    VALUES (
        (SELECT id FROM cell_towers WHERE radio = ?1 AND mcc = ?2 AND mnc = ?3 AND tac = ?4 AND cid = ?5),
        ?6, ?6, ?7, ?7
    )";

fn insert_rows<R: Read>(
    tx: &Transaction,
    reader: R,
    dump_timestamp: i64,
) -> Result<ImportStats, crate::Error> {
    let mut csv_reader = csv::Reader::from_reader(reader);
    let mut stmt = tx.prepare_cached(UPSERT_SQL)?;
    let mut rtree_stmt = tx.prepare_cached(RTREE_UPSERT_SQL)?;

    let mut count = 0usize;

    for result in csv_reader.deserialize::<CsvRow>() {
        let row = result?;
        stmt.execute(params![
            row.radio,
            row.mcc,
            row.net,
            row.area,
            row.cell,
            row.unit,
            row.lon,
            row.lat,
            row.range,
            row.samples,
            row.changeable,
            row.created,
            row.updated,
            row.average_signal,
            dump_timestamp,
        ])?;

        rtree_stmt.execute(params![
            row.radio, row.mcc, row.net, row.area, row.cell, row.lon, row.lat
        ])?;

        count += 1;
        if count.is_multiple_of(100_000) {
            log::info!("  {count} rows processed...");
        }
    }

    Ok(ImportStats {
        rows_processed: count,
    })
}

pub fn import_full(conn: &mut Connection, csv_path: &str) -> Result<ImportStats, crate::Error> {
    let filename = Path::new(csv_path)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| csv_path.to_string());

    log::info!("importing full dump: {filename}");
    let dump_timestamp = now_epoch();

    let tx = conn.transaction()?;

    schema::drop_secondary_indexes(&tx)?;
    log::info!("  secondary indexes dropped for bulk load");

    let file = File::open(csv_path)?;
    let stats = insert_rows(&tx, file, dump_timestamp)?;

    log::info!("  recreating secondary indexes...");
    schema::create_secondary_indexes(&tx)?;

    tx.execute(
        "INSERT OR REPLACE INTO import_history (filename, import_type, row_count, imported_at)
         VALUES (?1, 'full', ?2, ?3)",
        params![filename, stats.rows_processed, dump_timestamp],
    )?;

    tx.commit()?;
    log::info!("import complete: {} rows", stats.rows_processed);
    Ok(stats)
}

pub fn import_diff(conn: &mut Connection, csv_gz_path: &str) -> Result<ImportStats, crate::Error> {
    let filename = Path::new(csv_gz_path)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| csv_gz_path.to_string());

    let already_imported: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM import_history WHERE filename = ?1)",
        params![filename],
        |row| row.get(0),
    )?;
    if already_imported {
        log::info!("diff {filename} already imported, skipping");
        return Ok(ImportStats { rows_processed: 0 });
    }

    log::info!("importing diff: {filename}");
    let dump_timestamp = now_epoch();

    let file = File::open(csv_gz_path)?;
    let decoder = flate2::read::GzDecoder::new(file);

    let tx = conn.transaction()?;
    let stats = insert_rows(&tx, decoder, dump_timestamp)?;

    tx.execute(
        "INSERT INTO import_history (filename, import_type, row_count, imported_at)
         VALUES (?1, 'diff', ?2, ?3)",
        params![filename, stats.rows_processed, dump_timestamp],
    )?;

    tx.commit()?;
    log::info!("diff import complete: {} rows", stats.rows_processed);
    Ok(stats)
}

#[cfg(test)]
pub fn insert_rows_for_test<R: Read>(
    tx: &Transaction,
    reader: R,
    dump_timestamp: i64,
) -> Result<ImportStats, crate::Error> {
    insert_rows(tx, reader, dump_timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::create_schema;

    const TEST_CSV: &str = "radio,mcc,net,area,cell,unit,lon,lat,range,samples,changeable,created,updated,averageSignal
LTE,310,260,1234,56789,100,-122.4194,37.7749,500,10,1,1600000000,1700000000,0
GSM,262,2,435,37041,0,9.0114,53.098,25451,25,1,1288110641,1767530044,0
";

    #[test]
    fn test_import_csv() {
        let mut conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();

        let tx = conn.transaction().unwrap();
        let stats = insert_rows(&tx, TEST_CSV.as_bytes(), 1700000000).unwrap();
        tx.commit().unwrap();

        assert_eq!(stats.rows_processed, 2);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM cell_towers", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2);

        let geo_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM cell_towers_geo", [], |row| row.get(0))
            .unwrap();
        assert_eq!(geo_count, 2);
    }

    #[test]
    fn test_upsert_preserves_first_seen() {
        let mut conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();

        let csv1 = "radio,mcc,net,area,cell,unit,lon,lat,range,samples,changeable,created,updated,averageSignal
LTE,310,260,1234,56789,100,-122.4194,37.7749,500,10,1,1600000000,1700000000,0
";
        let tx = conn.transaction().unwrap();
        insert_rows(&tx, csv1.as_bytes(), 1000).unwrap();
        tx.commit().unwrap();

        let csv2 = "radio,mcc,net,area,cell,unit,lon,lat,range,samples,changeable,created,updated,averageSignal
LTE,310,260,1234,56789,100,-122.42,37.78,600,20,1,1600000000,1710000000,0
";
        let tx = conn.transaction().unwrap();
        insert_rows(&tx, csv2.as_bytes(), 2000).unwrap();
        tx.commit().unwrap();

        let (first_seen, last_seen): (i64, i64) = conn
            .query_row(
                "SELECT first_seen, last_seen FROM cell_towers WHERE cid = 56789",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(first_seen, 1000);
        assert_eq!(last_seen, 2000);
    }
}
