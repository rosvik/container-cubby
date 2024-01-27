use rusqlite::{Connection, Result};

const DATABASE_PATH: &str = "./db.sqlite3";

#[derive(Debug)]
pub struct BlobRow {
  _id: i32,
  _digest: String,
  _name: String,
  _data: Option<Vec<u8>>,
}

pub fn init() -> Result<()> {
  let conn = Connection::open(DATABASE_PATH)?;
  let mut stmt = conn.prepare(include_str!("../sql/find_existing_table.sql"))?;
  let is_initialized = stmt.query(["blobs"])?.next()?.is_some();
  if !is_initialized {
    conn.execute_batch(include_str!("../sql/create_blobs.sql"))?;
  }
  Ok(())
}

pub fn connect() -> Result<Connection> {
  let conn = Connection::open(DATABASE_PATH)?;
  Ok(conn)
}

pub fn insert_blob(conn: &Connection, digest: &str, name: &str, data: &[u8]) -> Result<usize> {
  let res = match conn.execute(include_str!("../sql/insert_blob.sql"), (&digest, &name, &data)) {
    Ok(res) => res,
    Err(e) => {
      if e.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) {
        return Err(e);
      }
      return Err(e);
    }
  };

  Ok(res)
}

pub fn get_blob(conn: &Connection, name: &str, digest: &str) -> Result<BlobRow> {
  let mut stmt = conn.prepare(include_str!("../sql/get_blob.sql"))?;
  let mut rows = stmt.query([&digest, &name])?;

  if let Some(row) = rows.next()? {
    let blob = BlobRow {
      _id: row.get(0)?,
      _digest: row.get(1)?,
      _name: row.get(2)?,
      _data: row.get(3)?,
    };

    return Ok(blob);
  }
  Err(rusqlite::Error::QueryReturnedNoRows)
}
