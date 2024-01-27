CREATE TABLE blobs (
  id       INTEGER PRIMARY KEY,
  digest   TEXT NOT NULL,
  name     TEXT NOT NULL,
  data     BLOB
);
CREATE UNIQUE INDEX idx_blobs_digest ON blobs (digest, name);
