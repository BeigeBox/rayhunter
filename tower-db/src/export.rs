use rusqlite::{params, Connection};

use crate::schema;

#[derive(Debug)]
pub struct ExportStats {
    pub rows_exported: usize,
    pub file_size_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub min_lat: f64,
    pub max_lat: f64,
    pub min_lon: f64,
    pub max_lon: f64,
}

pub struct ExportOptions<'a> {
    pub source_path: &'a str,
    pub output_path: &'a str,
    pub mcc_codes: &'a [u16],
    pub radio_types: &'a [String],
    pub bbox: Option<BoundingBox>,
    pub include_rtree: bool,
}

pub fn export_device_db(opts: &ExportOptions) -> Result<ExportStats, crate::Error> {
    if opts.mcc_codes.is_empty() {
        return Err(crate::Error::Export(
            "at least one MCC code required".into(),
        ));
    }

    let dest = Connection::open(opts.output_path)?;
    schema::create_schema(&dest)?;

    dest.execute("ATTACH DATABASE ?1 AS source", params![opts.source_path])?;

    let mcc_placeholders: String = opts
        .mcc_codes
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");

    let mut where_parts = vec![format!("mcc IN ({mcc_placeholders})")];

    if !opts.radio_types.is_empty() {
        let radio_placeholders: String = opts
            .radio_types
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        where_parts.push(format!("radio IN ({radio_placeholders})"));
    }

    if opts.bbox.is_some() {
        where_parts.push("lat >= ? AND lat <= ? AND lon >= ? AND lon <= ?".to_string());
    }

    let where_clause = where_parts.join(" AND ");

    let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = opts
        .mcc_codes
        .iter()
        .map(|m| Box::new(*m as i32) as Box<dyn rusqlite::types::ToSql>)
        .chain(
            opts.radio_types
                .iter()
                .map(|r| Box::new(r.clone()) as Box<dyn rusqlite::types::ToSql>),
        )
        .collect();

    if let Some(ref bbox) = opts.bbox {
        params_vec.push(Box::new(bbox.min_lat));
        params_vec.push(Box::new(bbox.max_lat));
        params_vec.push(Box::new(bbox.min_lon));
        params_vec.push(Box::new(bbox.max_lon));
    }

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| &**p).collect();

    let insert_sql = format!(
        "INSERT INTO main.cell_towers
         SELECT * FROM source.cell_towers WHERE {where_clause}"
    );

    let mut stmt = dest.prepare(&insert_sql)?;
    let rows = stmt.execute(param_refs.as_slice())?;
    drop(stmt);

    log::info!("exported {rows} towers");

    if opts.include_rtree {
        let rtree_sql = format!(
            "INSERT INTO main.cell_towers_geo
             SELECT g.id, g.min_lon, g.max_lon, g.min_lat, g.max_lat
             FROM source.cell_towers_geo g
             INNER JOIN source.cell_towers t ON g.id = t.id
             WHERE t.{where_clause}"
        );

        let mut stmt = dest.prepare(&rtree_sql)?;
        let rtree_rows = stmt.execute(param_refs.as_slice())?;
        drop(stmt);
        log::info!("exported {rtree_rows} R-tree entries");
    }

    dest.execute(
        "INSERT INTO main.import_history SELECT * FROM source.import_history",
        [],
    )?;

    dest.execute("DETACH DATABASE source", [])?;
    dest.execute_batch("VACUUM")?;

    let file_size_bytes = std::fs::metadata(opts.output_path)?.len();

    log::info!(
        "device database: {rows} rows, {:.1} MB",
        file_size_bytes as f64 / 1_048_576.0
    );

    Ok(ExportStats {
        rows_exported: rows,
        file_size_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::create_schema;
    use rusqlite::Connection;

    #[test]
    fn test_export_mcc_filter() {
        let dir = std::env::temp_dir().join("tower_db_export_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let source_path = dir.join("source.db");
        let output_path = dir.join("device.db");

        let mut source = Connection::open(&source_path).unwrap();
        create_schema(&source).unwrap();

        let csv = "radio,mcc,net,area,cell,unit,lon,lat,range,samples,changeable,created,updated,averageSignal
LTE,310,260,1234,56789,100,-122.4194,37.7749,500,10,1,1600000000,1700000000,0
LTE,262,2,435,37041,0,9.0114,53.098,25451,25,1,1288110641,1767530044,0
GSM,310,260,1234,99999,0,-122.4194,37.7749,500,10,1,1600000000,1700000000,0
";
        let tx = source.transaction().unwrap();
        crate::import::insert_rows_for_test(&tx, csv.as_bytes(), 1000).unwrap();
        tx.commit().unwrap();

        let stats = export_device_db(&ExportOptions {
            source_path: source_path.to_str().unwrap(),
            output_path: output_path.to_str().unwrap(),
            mcc_codes: &[310],
            radio_types: &["LTE".to_string()],
            bbox: None,
            include_rtree: true,
        })
        .unwrap();

        assert_eq!(stats.rows_exported, 1);

        let dest = Connection::open(&output_path).unwrap();
        let count: i64 = dest
            .query_row("SELECT COUNT(*) FROM cell_towers", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
