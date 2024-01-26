use rusqlite::{Connection, Result};

const PATH: &str = "./db.db3";

#[derive(Debug)]
pub struct BlobRow {
    id: i32,
    digest: String,
    data: Option<Vec<u8>>,
}

pub fn init() -> Result<()> {
    let conn = Connection::open(PATH)?;

    // First check if the table exists
    let mut stmt =
        conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='blobs'")?;
    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        // Table exists, so we're done
        println!("Table exists");
        println!("Rows: {:?}", row);
        return Ok(());
    }

    conn.execute(
        "CREATE TABLE blobs (
            id       INTEGER PRIMARY KEY,
            digest   TEXT NOT NULL,
            data     BLOB
        )",
        (),
    )?;
    conn.execute("CREATE UNIQUE INDEX idx_blobs_digest ON blobs (digest)", ())?;

    Ok(())
}

pub fn connect() -> Result<Connection> {
    let conn = Connection::open(PATH)?;
    Ok(conn)
}

pub fn insert_blob(conn: &Connection, digest: &str, data: &[u8]) -> Result<usize> {
    let res = match conn.execute(
        "INSERT INTO blobs (digest, data) VALUES (?1, ?2)",
        (&digest, &data),
    ) {
        Ok(res) => res,
        Err(e) => {
            if e.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) {
                println!("Duplicate digest: {:?}", e);
                return Err(e);
            }
            println!("Error inserting blob: {:?}", e);
            return Err(e);
        }
    };

    Ok(res)
}

pub fn get_blob(conn: &Connection, digest: &str) -> Result<BlobRow> {
    let mut stmt = conn.prepare("SELECT id, digest, data FROM blobs WHERE digest = ?1")?;
    let mut rows = stmt.query([&digest])?;

    if let Some(row) = rows.next()? {
        let id: i32 = row.get(0)?;
        let digest: String = row.get(1)?;
        let data: Option<Vec<u8>> = row.get(2)?;

        let blob = BlobRow { id, digest, data };

        return Ok(blob);
    }
    Err(rusqlite::Error::QueryReturnedNoRows)
}
