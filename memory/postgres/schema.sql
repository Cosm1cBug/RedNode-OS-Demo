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
