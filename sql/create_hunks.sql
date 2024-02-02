CREATE TABLE hunks (
  id        INTEGER PRIMARY KEY,
  name      TEXT NOT NULL,
  reference TEXT NOT NULL,
  last_byte INTEGER,
  data      BLOB
);
CREATE UNIQUE INDEX idx_hunks_digest ON blobs (name, reference);
