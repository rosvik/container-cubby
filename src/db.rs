use rusqlite::{Connection, Error, Result};

use crate::digestor;

const DATABASE_PATH: &str = "./db.sqlite3";

#[derive(Debug)]
pub struct BlobRow {
  pub id: i32,
  pub digest: String,
  pub name: String,
  pub data: Option<Vec<u8>>,
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

pub fn insert_and_verify_blob(
  conn: &Connection,
  digest: &str,
  name: &str,
  data: &[u8],
) -> Result<(), Error> {
  // Query digest MUST match the blob's digest.
  let blob_digest = digestor::get_sha256_digest(&data.to_vec());
  if blob_digest != digest {
    println!("Digest mismatch: digest_hash_string {}, digest {}", blob_digest, digest);
    return Err(Error::InvalidQuery);
  }
  match insert_blob(&conn, &digest, &name, &data) {
    Ok(res) => Ok(()),
    Err(e) => {
      if e.sqlite_error_code() != Some(rusqlite::ErrorCode::ConstraintViolation) {
        return Err(e);
      }
      // We have already stored this blob. Until the spec tells us what to do in
      // this case, we treat it as a success and continue the normal flow.
      println!("Warning: Duplicate blob, name='{}' digest='{}'", name, digest);
      Ok(())
    }
  }
}

pub fn get_blob(conn: &Connection, name: &str, digest: &str) -> Result<BlobRow> {
  let mut stmt = conn.prepare(include_str!("../sql/get_blob.sql"))?;
  let mut rows = stmt.query([&digest, &name])?;

  if let Some(row) = rows.next()? {
    let blob = BlobRow {
      id: row.get(0)?,
      digest: row.get(1)?,
      name: row.get(2)?,
      data: row.get(3)?,
    };

    return Ok(blob);
  }
  Err(rusqlite::Error::QueryReturnedNoRows)
}
