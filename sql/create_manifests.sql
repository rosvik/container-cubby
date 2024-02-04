CREATE TABLE manifests (
  id        INTEGER PRIMARY KEY,
  name      TEXT NOT NULL,
  reference TEXT NOT NULL,
  data      BLOB
);
CREATE UNIQUE INDEX idx_manifests_name_reference ON manifests (name, reference);
