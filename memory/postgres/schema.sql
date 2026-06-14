-- RedNode-OS PostgreSQL Schema
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE IF NOT EXISTS intentions (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  ts TIMESTAMPTZ DEFAULT now(),
  session_id TEXT,
  intent TEXT NOT NULL,
  plan JSONB,
  result JSONB,
  status TEXT
);
CREATE TABLE IF NOT EXISTS memory_longterm (key TEXT PRIMARY KEY, value JSONB, ts TIMESTAMPTZ DEFAULT now());
CREATE TABLE IF NOT EXISTS memory_working (key TEXT PRIMARY KEY, value JSONB, ts TIMESTAMPTZ DEFAULT now(), expires_at TIMESTAMPTZ);
CREATE TABLE IF NOT EXISTS memory_episodic (id BIGSERIAL PRIMARY KEY, ts TIMESTAMPTZ DEFAULT now(), key TEXT, value JSONB);
CREATE TABLE IF NOT EXISTS memory_security (key TEXT PRIMARY KEY, value JSONB, ts TIMESTAMPTZ DEFAULT now(), severity TEXT);

CREATE TABLE IF NOT EXISTS audit_log (
  id BIGSERIAL PRIMARY KEY,
  ts TIMESTAMPTZ DEFAULT now(),
  actor TEXT NOT NULL,
  action TEXT NOT NULL,
  tool TEXT,
  args JSONB,
  risk TEXT,
  approved BOOLEAN,
  result TEXT,
  prev_hash TEXT,
  hash TEXT
);

CREATE TABLE IF NOT EXISTS security_events (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  ts TIMESTAMPTZ DEFAULT now(),
  severity TEXT NOT NULL,
  source TEXT NOT NULL,
  summary TEXT NOT NULL,
  raw JSONB,
  acknowledged BOOLEAN DEFAULT false
);

CREATE TABLE IF NOT EXISTS documents (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  source TEXT,
  content TEXT NOT NULL,
  embedding vector(768),
  metadata JSONB,
  created_at TIMESTAMPTZ DEFAULT now()
);

-- Knowledge Graph (Postgres fallback when Kuzu not compiled)
CREATE TABLE IF NOT EXISTS kg_entities (
  name TEXT PRIMARY KEY,
  kind TEXT NOT NULL,
  properties JSONB DEFAULT '{}',
  created_at TIMESTAMPTZ DEFAULT now(),
  updated_at TIMESTAMPTZ DEFAULT now()
);

CREATE TABLE IF NOT EXISTS kg_relationships (
  id BIGSERIAL PRIMARY KEY,
  from_entity TEXT NOT NULL REFERENCES kg_entities(name) ON DELETE CASCADE,
  to_entity TEXT NOT NULL REFERENCES kg_entities(name) ON DELETE CASCADE,
  relation TEXT NOT NULL,
  properties JSONB DEFAULT '{}',
  created_at TIMESTAMPTZ DEFAULT now(),
  UNIQUE(from_entity, to_entity, relation)
);

CREATE INDEX IF NOT EXISTS idx_kg_entities_kind ON kg_entities(kind);
CREATE INDEX IF NOT EXISTS idx_kg_rel_from ON kg_relationships(from_entity);
CREATE INDEX IF NOT EXISTS idx_kg_rel_to ON kg_relationships(to_entity);
