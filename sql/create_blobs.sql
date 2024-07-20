CREATE TABLE blobs (
  id        SERIAL PRIMARY KEY,
  name      VARCHAR(1024) NOT NULL,
  digest    CHAR(32) NOT NULL,
  data      BYTEA,
  UNIQUE(name, digest)
)
