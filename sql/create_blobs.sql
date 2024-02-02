CREATE TABLE blobs (
  id        INTEGER PRIMARY KEY,
  name      TEXT NOT NULL,
  digest    TEXT NOT NULL,
  data      BLOB
);
CREATE UNIQUE INDEX idx_blobs_digest ON blobs (name, digest);
