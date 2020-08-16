CREATE TABLE IF NOT EXISTS jobs (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL,
    project TEXT NOT NULL,
    commands TEXT NOT NULL,
    started_at TEXT NOT NULL,
    ended_at TEXT
)
