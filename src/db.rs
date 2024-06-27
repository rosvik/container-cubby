use rusqlite::{Connection, Error, ErrorCode, Result};

use crate::digestor;

const DATABASE_PATH: &str = "./db.sqlite3";

#[derive(Debug)]
pub struct BlobRow {
  pub id: u32,
  pub name: String,
  pub digest: String,
  pub data: Option<Vec<u8>>,
}

pub struct HunkRow {
  pub id: u32,
  pub name: String,
  pub reference: String,
  // The index of the last byte of the stored hunk. None if no data is stored.
  pub last_byte: Option<usize>,
  pub data: Option<Vec<u8>>,
}

pub struct ManifestRow {
  pub id: u32,
  pub name: String,
  pub reference: String,
  pub data: Option<Vec<u8>>,
}

pub fn init() -> Result<()> {
  let conn = Connection::open(DATABASE_PATH)?;
  let mut stmt = conn.prepare(include_str!("../sql/find_existing_table.sql"))?;
  let is_blobs_initialized = stmt.query(["blobs"])?.next()?.is_some();
  if !is_blobs_initialized {
    conn.execute_batch(include_str!("../sql/create_blobs.sql"))?;
  }
  let is_hunks_initialized = stmt.query(["hunks"])?.next()?.is_some();
  if !is_hunks_initialized {
    conn.execute_batch(include_str!("../sql/create_hunks.sql"))?;
  }
  let is_manifests_initialized = stmt.query(["manifests"])?.next()?.is_some();
  if !is_manifests_initialized {
    conn.execute_batch(include_str!("../sql/create_manifests.sql"))?;
  }
  Ok(())
}

pub fn connect() -> Result<Connection> {
  let conn = Connection::open(DATABASE_PATH)?;
  Ok(conn)
}

pub fn insert_blob(conn: &Connection, name: &str, digest: &str, data: &[u8]) -> Result<usize> {
  let stmt = include_str!("../sql/insert_blob.sql");
  conn.execute(stmt, (&name, &digest, &data))
}

pub fn verify_and_insert_blob(
  conn: &Connection,
  name: &str,
  digest: &str,
  data: &[u8],
) -> Result<(), Error> {
  // Query digest MUST match the blob's digest.
  let blob_digest = digestor::get_sha256_digest(&data.to_vec());
  if blob_digest != digest {
    println!("Digest mismatch: digest_hash_string {}, digest {}", blob_digest, digest);
    return Err(Error::InvalidQuery);
  }
  match insert_blob(conn, name, digest, data) {
    Ok(_) => Ok(()),
    Err(e) => {
      if e.sqlite_error_code() != Some(ErrorCode::ConstraintViolation) {
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
  let mut rows = stmt.query([&name, &digest])?;

  if let Some(row) = rows.next()? {
    let blob = BlobRow {
      id: row.get(0)?,
      digest: row.get(1)?,
      name: row.get(2)?,
      data: row.get(3)?,
    };

    return Ok(blob);
  }
  Err(Error::QueryReturnedNoRows)
}

pub fn delete_blob(conn: &Connection, name: &str, reference: &str) -> Result<usize> {
  let stmt = include_str!("../sql/delete_blob.sql");
  conn.execute(stmt, (&name, &reference))
}

pub fn insert_empty_hunk(conn: &Connection, name: &str, reference: &str) -> Result<usize> {
  let stmt = include_str!("../sql/insert_hunk.sql");
  conn.execute(stmt, (&name, &reference))
}

pub fn get_hunk(conn: &Connection, name: &str, reference: &str) -> Result<HunkRow> {
  let mut stmt = conn.prepare(include_str!("../sql/get_hunk.sql"))?;
  let mut rows = stmt.query([&name, &reference])?;

  if let Some(row) = rows.next()? {
    let hunk = HunkRow {
      id: row.get(0)?,
      name: row.get(1)?,
      reference: row.get(2)?,
      last_byte: row.get(3)?,
      data: row.get(4)?,
    };

    return Ok(hunk);
  }
  Err(Error::QueryReturnedNoRows)
}

pub fn append_hunk(conn: &Connection, name: &str, reference: &str, data: Vec<u8>) -> Result<usize> {
  let hunk = get_hunk(conn, name, reference)?;

  // Combine stored hunk data with new data
  let mut new_data: Vec<u8> = hunk.data.unwrap_or(Vec::new());
  new_data.extend(data);

  let stmt = include_str!("../sql/update_hunk.sql");
  conn.execute(stmt, (&reference, &new_data.len() - 1, &new_data))
}

pub fn delete_hunk(conn: &Connection, name: &str, reference: &str) -> Result<usize> {
  let stmt = include_str!("../sql/delete_hunk.sql");
  conn.execute(stmt, (&name, &reference))
}

pub fn commit_hunk(
  conn: &Connection,
  name: &str,
  reference: &str,
  digest: &str,
) -> Result<(), Error> {
  // Get stored hunk data
  let hunk = get_hunk(conn, name, reference)?;

  // Verify and insert as a blob
  verify_and_insert_blob(conn, name, digest, &hunk.data.unwrap())?;

  // Delete the hunk
  delete_hunk(conn, name, reference)?;
  Ok(())
}

pub fn insert_manifest(
  conn: &Connection,
  name: &str,
  reference: &str,
  data: Vec<u8>,
) -> Result<usize> {
  let stmt = include_str!("../sql/insert_manifest.sql");
  conn.execute(stmt, (&name, &reference, data))
}

pub fn get_manifest(conn: &Connection, name: &str, reference: &str) -> Result<ManifestRow> {
  let mut stmt = conn.prepare(include_str!("../sql/get_manifest.sql"))?;
  let mut rows = stmt.query([&name, &reference])?;

  if let Some(row) = rows.next()? {
    let manifest = ManifestRow {
      id: row.get(0)?,
      name: row.get(1)?,
      reference: row.get(2)?,
      data: row.get(3)?,
    };

    return Ok(manifest);
  }
  Err(Error::QueryReturnedNoRows)
}

pub fn delete_manifest(conn: &Connection, name: &str, reference: &str) -> Result<usize> {
  let stmt = include_str!("../sql/delete_manifest.sql");
  conn.execute(stmt, (&name, &reference))
}

pub fn get_tags(conn: &Connection, name: &str) -> Result<Vec<String>> {
  let mut stmt = conn.prepare(include_str!("../sql/get_tags.sql"))?;
  let mut rows = stmt.query([&name])?;

  let mut tags = Vec::new();
  while let Some(row) = rows.next()? {
    tags.push(row.get(0)?);
  }

  println!("tags: {:?}", tags);

  Ok(tags)
}
