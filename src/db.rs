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
  let mut stmt = conn.prepare(include_str!("../sql/find_existing_table.sql"))?;
  let is_initialized = stmt.query(["blobs"])?.next()?.is_some();
  if !is_initialized {
    conn.execute(include_str!("../sql/create_blobs.sql"), ())?;
    conn.execute(include_str!("../sql/create_blobs_index.sql"), ())?;
  }
  Ok(())
}

pub fn connect() -> Result<Connection> {
  let conn = Connection::open(PATH)?;
  Ok(conn)
}

pub fn insert_blob(conn: &Connection, digest: &str, data: &[u8]) -> Result<usize> {
  let res = match conn.execute(include_str!("../sql/insert_blob.sql"), (&digest, &data)) {
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

pub fn get_blob(conn: &Connection, digest: &str) -> Result<BlobRow> {
  let mut stmt = conn.prepare(include_str!("../sql/get_blob.sql"))?;
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
