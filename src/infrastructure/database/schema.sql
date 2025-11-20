-- Table for tasks
CREATE TABLE IF NOT EXISTS tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    guild_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    scheduled_time INTEGER,
    recurrence_type TEXT,
    recurrence_data TEXT,
    notification_method TEXT NOT NULL,
    channel_id INTEGER,
    mention TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);

-- Table for user preferences
CREATE TABLE IF NOT EXISTS user_preferences (
    user_id INTEGER PRIMARY KEY,
    timezone TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Table for server configurations
CREATE TABLE IF NOT EXISTS server_configs (
    guild_id INTEGER PRIMARY KEY,
    notification_channel INTEGER NOT NULL
);

-- Table for scheduled tasks used by the persistent scheduler
-- Use task_id as PRIMARY KEY (one scheduled entry per task). If you want
-- multiple scheduled entries per task, change PRIMARY KEY to an AUTOINCREMENT id.
CREATE TABLE IF NOT EXISTS scheduled_tasks (
    task_id         INTEGER PRIMARY KEY,   -- references Task.id (domain)
    scheduled_time  INTEGER NOT NULL,      -- unix timestamp (seconds since epoch, UTC)
    user_id         INTEGER NOT NULL,
    guild_id        INTEGER NOT NULL,
    title           TEXT NOT NULL,
    notification_method TEXT NOT NULL,     -- "dm" | "channel" | "both"
    is_recurring    INTEGER NOT NULL DEFAULT 0,  -- 0 = false, 1 = true
    is_deleted      INTEGER NOT NULL DEFAULT 0,  -- soft-delete flag
    mention         TEXT
);

-- Index to quickly fetch the next pending (non-deleted) task
CREATE INDEX IF NOT EXISTS idx_scheduled_tasks_time ON scheduled_tasks (is_deleted, scheduled_time);
