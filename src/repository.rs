use std::path::PathBuf;

use chrono::{Local, NaiveDateTime};
use rusqlite::{params, Connection, OptionalExtension};

use crate::model::{
    validate_entry, validate_text, AppError, AppResult, CurrentActivity, TimeEntry,
};

const DB_FILE: &str = "time_tracker.db";

pub(crate) struct Repository {
    conn: Connection,
}

impl Repository {
    pub(crate) fn open() -> AppResult<Self> {
        let db_path = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(DB_FILE);
        let conn = Connection::open(db_path)?;
        let repo = Self { conn };
        repo.init()?;
        Ok(repo)
    }

    fn init(&self) -> AppResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS time_entry (
                id INTEGER NOT NULL PRIMARY KEY,
                start_time DATETIME NOT NULL,
                end_time DATETIME NOT NULL,
                activity VARCHAR(200) NOT NULL,
                category VARCHAR(50) NOT NULL,
                location TEXT NOT NULL DEFAULT '',
                description TEXT
            );

            CREATE TABLE IF NOT EXISTS current_activity (
                id INTEGER NOT NULL PRIMARY KEY CHECK (id = 1),
                start_time DATETIME NOT NULL,
                activity VARCHAR(200) NOT NULL,
                category VARCHAR(50) NOT NULL,
                location TEXT NOT NULL DEFAULT '',
                description TEXT
            );
            ",
        )?;
        self.ensure_column("time_entry", "location", "TEXT NOT NULL DEFAULT ''")?;
        self.ensure_column("current_activity", "location", "TEXT NOT NULL DEFAULT ''")?;
        Ok(())
    }

    fn ensure_column(&self, table: &str, column: &str, definition: &str) -> AppResult<()> {
        let mut stmt = self.conn.prepare(&format!("PRAGMA table_info({table})"))?;
        let has_column = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .any(|name| name == column);

        if !has_column {
            self.conn.execute(
                &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
                [],
            )?;
        }

        Ok(())
    }

    pub(crate) fn list_between(
        &self,
        start: NaiveDateTime,
        end: NaiveDateTime,
    ) -> AppResult<Vec<TimeEntry>> {
        let mut stmt = self.conn.prepare(
            "
            SELECT id, start_time, end_time, activity, category, COALESCE(location, ''), COALESCE(description, '')
            FROM time_entry
            WHERE start_time >= ?1 AND start_time < ?2
            ORDER BY start_time DESC
            ",
        )?;

        let entries = stmt
            .query_map(params![start, end], row_to_entry)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(entries)
    }

    pub(crate) fn insert_entry(
        &self,
        start_time: NaiveDateTime,
        end_time: NaiveDateTime,
        activity: &str,
        category: &str,
        location: &str,
        description: &str,
    ) -> AppResult<()> {
        validate_entry(start_time, end_time, activity, category)?;
        self.conn.execute(
            "
            INSERT INTO time_entry (start_time, end_time, activity, category, location, description)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ",
            params![
                start_time,
                end_time,
                activity.trim(),
                category.trim(),
                location.trim(),
                description.trim()
            ],
        )?;
        Ok(())
    }

    pub(crate) fn update_entry(&self, entry: &TimeEntry) -> AppResult<()> {
        validate_entry(
            entry.start_time,
            entry.end_time,
            &entry.activity,
            &entry.category,
        )?;
        self.conn.execute(
            "
            UPDATE time_entry
            SET start_time = ?1, end_time = ?2, activity = ?3, category = ?4, location = ?5, description = ?6
            WHERE id = ?7
            ",
            params![
                entry.start_time,
                entry.end_time,
                entry.activity.trim(),
                entry.category.trim(),
                entry.location.trim(),
                entry.description.trim(),
                entry.id
            ],
        )?;
        Ok(())
    }

    pub(crate) fn delete_entry(&self, id: i64) -> AppResult<()> {
        self.conn
            .execute("DELETE FROM time_entry WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub(crate) fn get_current_activity(&self) -> AppResult<Option<CurrentActivity>> {
        self.conn
            .query_row(
                "
                SELECT start_time, activity, category, COALESCE(location, ''), COALESCE(description, '')
                FROM current_activity
                WHERE id = 1
                ",
                [],
                |row| {
                    Ok(CurrentActivity {
                        start_time: row.get(0)?,
                        activity: row.get(1)?,
                        category: row.get(2)?,
                        location: row.get(3)?,
                        description: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(AppError::from)
    }

    pub(crate) fn start_activity(
        &self,
        activity: &str,
        category: &str,
        location: &str,
        description: &str,
    ) -> AppResult<()> {
        let now = Local::now().naive_local();
        validate_text(activity, category)?;
        self.conn.execute(
            "
            INSERT INTO current_activity (id, start_time, activity, category, location, description)
            VALUES (1, ?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(id) DO UPDATE SET
                start_time = excluded.start_time,
                activity = excluded.activity,
                category = excluded.category,
                location = excluded.location,
                description = excluded.description
            ",
            params![
                now,
                activity.trim(),
                category.trim(),
                location.trim(),
                description.trim()
            ],
        )?;
        Ok(())
    }

    pub(crate) fn end_activity(&self) -> AppResult<()> {
        let current = self
            .get_current_activity()?
            .ok_or(AppError::NoActiveActivity)?;
        let now = Local::now().naive_local();
        self.insert_entry(
            current.start_time,
            now,
            &current.activity,
            &current.category,
            &current.location,
            &current.description,
        )?;
        self.conn
            .execute("DELETE FROM current_activity WHERE id = 1", [])?;
        Ok(())
    }
}

fn row_to_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<TimeEntry> {
    Ok(TimeEntry {
        id: row.get(0)?,
        start_time: row.get(1)?,
        end_time: row.get(2)?,
        activity: row.get(3)?,
        category: row.get(4)?,
        location: row.get(5)?,
        description: row.get(6)?,
    })
}
