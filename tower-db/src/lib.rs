pub mod export;
pub mod import;
pub mod schema;
pub mod states;

use rusqlite::{params, Connection, OpenFlags};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("csv: {0}")]
    Csv(#[from] csv::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("export: {0}")]
    Export(String),
}

#[derive(Debug, Clone)]
pub struct CellIdentity {
    pub radio: String,
    pub mcc: u16,
    pub mnc: u16,
    pub tac: u32,
    pub cid: u64,
}

#[derive(Debug, Clone)]
pub struct TowerInfo {
    pub identity: CellIdentity,
    pub pci: i32,
    pub lon: f64,
    pub lat: f64,
    pub range_m: i32,
    pub samples: i32,
    pub ocid_created: i64,
    pub ocid_updated: i64,
    pub average_signal: i32,
    pub first_seen: i64,
    pub last_seen: i64,
}

pub struct TowerDb {
    conn: Connection,
}

impl TowerDb {
    pub fn open(path: &str) -> Result<Self, Error> {
        let conn = Connection::open(path)?;
        Ok(Self { conn })
    }

    pub fn open_readonly(path: &str) -> Result<Self, Error> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        Ok(Self { conn })
    }

    pub fn init(path: &str) -> Result<Self, Error> {
        let conn = Connection::open(path)?;
        schema::create_schema(&conn)?;
        Ok(Self { conn })
    }

    pub fn connection_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    pub fn lookup(&self, id: &CellIdentity) -> Result<Option<TowerInfo>, Error> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT pci, lon, lat, range_m, samples, ocid_created, ocid_updated,
                    average_signal, first_seen, last_seen
             FROM cell_towers
             WHERE radio = ?1 AND mcc = ?2 AND mnc = ?3 AND tac = ?4 AND cid = ?5",
        )?;

        let result = stmt.query_row(params![id.radio, id.mcc, id.mnc, id.tac, id.cid], |row| {
            Ok(TowerInfo {
                identity: id.clone(),
                pci: row.get(0)?,
                lon: row.get(1)?,
                lat: row.get(2)?,
                range_m: row.get(3)?,
                samples: row.get(4)?,
                ocid_created: row.get(5)?,
                ocid_updated: row.get(6)?,
                average_signal: row.get(7)?,
                first_seen: row.get(8)?,
                last_seen: row.get(9)?,
            })
        });

        match result {
            Ok(info) => Ok(Some(info)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn nearby(&self, lat: f64, lon: f64, radius_m: u32) -> Result<Vec<TowerInfo>, Error> {
        // Approximate degrees per meter at given latitude
        let lat_deg_per_m = 1.0 / 111_320.0;
        let lon_deg_per_m = 1.0 / (111_320.0 * lat.to_radians().cos());
        let dlat = radius_m as f64 * lat_deg_per_m;
        let dlon = radius_m as f64 * lon_deg_per_m;

        let min_lon = lon - dlon;
        let max_lon = lon + dlon;
        let min_lat = lat - dlat;
        let max_lat = lat + dlat;

        let mut stmt = self.conn.prepare_cached(
            "SELECT t.radio, t.mcc, t.mnc, t.tac, t.cid, t.pci, t.lon, t.lat,
                    t.range_m, t.samples, t.ocid_created, t.ocid_updated,
                    t.average_signal, t.first_seen, t.last_seen
             FROM cell_towers_geo g
             INNER JOIN cell_towers t ON g.id = t.id
             WHERE g.min_lon >= ?1 AND g.max_lon <= ?2
               AND g.min_lat >= ?3 AND g.max_lat <= ?4",
        )?;

        let rows = stmt.query_map(params![min_lon, max_lon, min_lat, max_lat], |row| {
            Ok(TowerInfo {
                identity: CellIdentity {
                    radio: row.get(0)?,
                    mcc: row.get(1)?,
                    mnc: row.get(2)?,
                    tac: row.get(3)?,
                    cid: row.get(4)?,
                },
                pci: row.get(5)?,
                lon: row.get(6)?,
                lat: row.get(7)?,
                range_m: row.get(8)?,
                samples: row.get(9)?,
                ocid_created: row.get(10)?,
                ocid_updated: row.get(11)?,
                average_signal: row.get(12)?,
                first_seen: row.get(13)?,
                last_seen: row.get(14)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn stats(&self) -> Result<DbStats, Error> {
        let total: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM cell_towers", [], |row| row.get(0))?;

        let mut stmt = self
            .conn
            .prepare("SELECT radio, COUNT(*) FROM cell_towers GROUP BY radio ORDER BY radio")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;

        let mut by_radio = Vec::new();
        for row in rows {
            by_radio.push(row?);
        }

        let mut stmt = self.conn.prepare(
            "SELECT filename, import_type, row_count, imported_at FROM import_history ORDER BY imported_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ImportRecord {
                filename: row.get(0)?,
                import_type: row.get(1)?,
                row_count: row.get(2)?,
                imported_at: row.get(3)?,
            })
        })?;

        let mut imports = Vec::new();
        for row in rows {
            imports.push(row?);
        }

        Ok(DbStats {
            total_towers: total,
            by_radio,
            imports,
        })
    }
}

#[derive(Debug)]
pub struct DbStats {
    pub total_towers: i64,
    pub by_radio: Vec<(String, i64)>,
    pub imports: Vec<ImportRecord>,
}

#[derive(Debug)]
pub struct ImportRecord {
    pub filename: String,
    pub import_type: String,
    pub row_count: i64,
    pub imported_at: i64,
}
