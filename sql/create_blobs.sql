CREATE TABLE blobs (
  id       INTEGER PRIMARY KEY,
  digest   TEXT NOT NULL,
  data     BLOB
)
