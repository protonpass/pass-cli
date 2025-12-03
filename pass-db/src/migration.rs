pub struct Migration {
    pub id: i64,
    pub description: &'static str,
    pub sql: &'static str,
}

pub fn get_migrations() -> Vec<Migration> {
    vec![
        Migration {
            id: 1,
            description: "Create telemetry_events table",
            sql: "
                    CREATE TABLE telemetry_events (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        timestamp INTEGER NOT NULL,
                        event_type TEXT NOT NULL,
                        extra_data TEXT,
                        user_id TEXT
                    );
                    CREATE INDEX idx_telemetry_events_user_id ON telemetry_events(user_id);
                ",
        },
        Migration {
            id: 2,
            description: "Create activity_time table",
            sql: "
                    CREATE TABLE activity_time (
                        user_id TEXT,
                        activity TEXT NOT NULL,
                        timestamp INTEGER NOT NULL,
                        PRIMARY KEY (user_id, activity)
                    );
                    CREATE INDEX idx_activity_time_user_id ON activity_time(user_id);
                ",
        },
        Migration {
            id: 3,
            description: "Create share_keys table",
            sql: "
                    CREATE TABLE share_keys (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        user_id TEXT NOT NULL,
                        share_id TEXT NOT NULL,
                        key_rotation INTEGER NOT NULL,
                        share_key BLOB NOT NULL,
                        created_at INTEGER NOT NULL
                    );
                    CREATE UNIQUE INDEX idx_share_keys_unique ON share_keys(user_id, share_id, key_rotation);
                    CREATE INDEX idx_share_keys_lookup ON share_keys(user_id, share_id);
                ",
        },
        Migration {
            id: 4,
            description: "Create folder_keys table",
            sql: "
                    CREATE TABLE folder_keys (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        user_id TEXT NOT NULL,
                        share_id TEXT NOT NULL,
                        folder_id TEXT NOT NULL,
                        key_rotation INTEGER NOT NULL,
                        folder_key BLOB NOT NULL,
                        created_at INTEGER NOT NULL
                    );
                    CREATE UNIQUE INDEX idx_folder_keys_unique ON folder_keys(user_id, share_id, folder_id, key_rotation);
                    CREATE INDEX idx_folder_keys_lookup ON folder_keys(user_id, share_id, folder_id);
                ",
        },
    ]
}
