/*
 *  Copyright (c) 2026 Proton AG
 *  This file is part of Proton AG and Proton Pass.
 *
 *  Proton Pass is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Proton Pass is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Proton Pass.  If not, see <https://www.gnu.org/licenses/>.
 *
 */

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
        Migration {
            id: 5,
            description: "Create user_settings table",
            sql: "
                    CREATE TABLE user_settings (
                        user_id TEXT NOT NULL,
                        setting_key TEXT NOT NULL,
                        setting_value TEXT,
                        updated_at INTEGER NOT NULL,
                        PRIMARY KEY (user_id, setting_key)
                    );
                    CREATE INDEX idx_user_settings_user_id ON user_settings(user_id);
                ",
        },
        Migration {
            id: 6,
            description: "Create core_event_cursors table",
            sql: "
                    CREATE TABLE core_event_cursors (
                        user_id TEXT NOT NULL PRIMARY KEY,
                        event_id TEXT NOT NULL,
                        updated_at INTEGER NOT NULL
                    );
                ",
        },
    ]
}
