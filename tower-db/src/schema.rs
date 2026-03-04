use rusqlite::Connection;

pub fn create_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS cell_towers (
            id              INTEGER PRIMARY KEY,
            radio           TEXT    NOT NULL,
            mcc             INTEGER NOT NULL,
            mnc             INTEGER NOT NULL,
            tac             INTEGER NOT NULL,
            cid             INTEGER NOT NULL,
            pci             INTEGER NOT NULL DEFAULT 0,
            lon             REAL    NOT NULL,
            lat             REAL    NOT NULL,
            range_m         INTEGER NOT NULL DEFAULT 0,
            samples         INTEGER NOT NULL DEFAULT 0,
            changeable      INTEGER NOT NULL DEFAULT 1,
            ocid_created    INTEGER NOT NULL DEFAULT 0,
            ocid_updated    INTEGER NOT NULL DEFAULT 0,
            average_signal  INTEGER NOT NULL DEFAULT 0,
            first_seen      INTEGER NOT NULL,
            last_seen       INTEGER NOT NULL
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_cell_identity
            ON cell_towers (radio, mcc, mnc, tac, cid);

        CREATE INDEX IF NOT EXISTS idx_mcc
            ON cell_towers (mcc);

        CREATE VIRTUAL TABLE IF NOT EXISTS cell_towers_geo USING rtree(
            id,
            min_lon, max_lon,
            min_lat, max_lat
        );

        CREATE TABLE IF NOT EXISTS import_history (
            id          INTEGER PRIMARY KEY,
            filename    TEXT    NOT NULL UNIQUE,
            import_type TEXT    NOT NULL,
            row_count   INTEGER NOT NULL DEFAULT 0,
            imported_at INTEGER NOT NULL
        );
        ",
    )
}

pub fn drop_secondary_indexes(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("DROP INDEX IF EXISTS idx_mcc;")
}

pub fn create_secondary_indexes(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("CREATE INDEX IF NOT EXISTS idx_mcc ON cell_towers (mcc);")
}
