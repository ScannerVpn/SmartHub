-- SmartHub licensing schema (Cloudflare D1 / SQLite)
-- Run: npx wrangler d1 execute smarthub-licenses --file=./schema.sql

-- One row per (license key hash, device instance).
-- Raw license keys are NEVER stored — only their SHA-256 hash.
CREATE TABLE IF NOT EXISTS licenses (
  id                 INTEGER PRIMARY KEY AUTOINCREMENT,
  license_key_hash   TEXT NOT NULL,
  license_key_id     INTEGER,              -- Lemon Squeezy internal id
  instance_id        TEXT,                 -- device activation id
  status             TEXT NOT NULL DEFAULT 'active',  -- active | disabled | expired | refunded
  customer_email     TEXT,
  product_name       TEXT,
  variant_name       TEXT,
  order_id           INTEGER,
  activation_count   INTEGER NOT NULL DEFAULT 1,
  activated_at       INTEGER NOT NULL,
  last_validated_at  INTEGER,
  created_at         INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_lic_hash_inst
  ON licenses(license_key_hash, instance_id);

-- Raw webhook event log — audit trail for purchases/refunds/key bans.
CREATE TABLE IF NOT EXISTS events (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  event_name   TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  received_at  INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_events_name ON events(event_name);
