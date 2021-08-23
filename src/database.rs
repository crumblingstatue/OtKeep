use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::ExitStatus,
};
use thiserror::Error;

use anyhow::bail;
use os_str_bytes::{OsStrBytes, OsStringBytes};
use rusqlite::{params, Connection, OptionalExtension};

use crate::fs_util::ensure_dir_exists;

/// Contains all the scripts
pub struct Database {
    conn: Connection,
}

const DB_FILENAME: &str = "otkeep.sqlite3";

pub struct ScriptInfo {
    pub name: String,
    pub description: String,
}

impl Database {
    pub fn load(dir: &Path) -> anyhow::Result<Self> {
        ensure_dir_exists(dir)?;
        let mut conn = Connection::open(dir.join(DB_FILENAME))?;
        let tx = conn.transaction()?;
        tx.execute(
            "CREATE TABLE IF NOT EXISTS scripts (
            body BLOB NOT NULL UNIQUE
        )",
            [],
        )?;
        tx.execute(
            "CREATE TABLE IF NOT EXISTS trees (
                root BLOB NOT NULL UNIQUE
            )",
            [],
        )?;
        tx.execute(
            "CREATE TABLE IF NOT EXISTS pairings (
            tree_id   INTEGER NOT NULL,
            script_id INTEGER NOT NULL,
            name      TEXT NOT NULL,
            desc      TEXT,
            UNIQUE(tree_id, name)
        )",
            [],
        )?;
        tx.commit()?;
        Ok(Self { conn })
    }

    pub fn add_script(&mut self, tree_id: i64, name: &str, body: Vec<u8>) -> anyhow::Result<()> {
        let tx = self.conn.transaction()?;
        let script_id = match query_script_by_body(&tx, &body)? {
            Some(id) => id,
            None => {
                tx.execute("INSERT INTO scripts (body) VALUES (?)", params![body])?;
                tx.last_insert_rowid()
            }
        };
        tx.execute(
            "INSERT INTO pairings (tree_id, name, script_id) VALUES (?1, ?2, ?3)",
            params![tree_id, name, script_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Removes a script with `name` from the current tree and returns whether it actually
    /// removed anything
    pub fn remove_script(&mut self, tree_id: i64, name: &str) -> anyhow::Result<bool> {
        Ok(self.conn.execute(
            "DELETE FROM pairings WHERE tree_id=?1 AND name=?2",
            params![tree_id, name],
        )? > 0)
    }

    pub fn run_script(
        &self,
        tree_id: i64,
        name: &str,
        args: impl Iterator<Item = impl AsRef<OsStr>>,
    ) -> anyhow::Result<ExitStatus> {
        let mut stmt = self
            .conn
            .prepare("SELECT script_id FROM pairings WHERE tree_id=?1 AND name=?2")?;
        let script_id: Option<i64> = stmt
            .query_row(params![tree_id, name], |row| row.get(0))
            .optional()?;
        match script_id {
            Some(id) => {
                let mut stmt = self
                    .conn
                    .prepare("SELECT body FROM scripts WHERE _rowid_=?")?;
                let script: Vec<u8> = stmt.query_row(params![id], |row| row.get(0))?;
                let status = crate::run::run_script(&script, args)?;
                Ok(status)
            }
            None => bail!(NoSuchScriptForCurrentTree),
        }
    }

    pub fn scripts_for_tree(&self, tree_id: i64) -> anyhow::Result<Vec<ScriptInfo>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, desc FROM pairings WHERE tree_id=?")?;
        let rows = stmt.query_map(params![tree_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
        let mut vec = Vec::new();
        for result in rows {
            let (name, description) = result?;
            let description: Option<String> = description;
            vec.push(ScriptInfo {
                name,
                description: description.unwrap_or_else(String::new),
            });
        }
        Ok(vec)
    }

    pub fn query_tree(&self, path: &Path) -> anyhow::Result<Option<i64>> {
        let mut stmt = self
            .conn
            .prepare("SELECT _rowid_ FROM trees where root=?")?;
        Ok(stmt
            .query_row(params![path.to_raw_bytes()], |row| row.get(0))
            .optional()?)
    }

    pub fn add_new_tree(&self, path: &Path) -> anyhow::Result<()> {
        let raw = path.to_raw_bytes();
        self.conn
            .execute("INSERT INTO trees (root) VALUES (?)", params![raw])?;
        Ok(())
    }

    pub fn remove_tree(&mut self, tree_id: i64) -> anyhow::Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM trees WHERE _rowid_=?", params![tree_id])?;
        tx.execute("DELETE FROM pairings WHERE tree_id=?", params![tree_id])?;
        tx.commit()?;
        Ok(())
    }

    pub fn add_script_description(
        &self,
        tree_id: i64,
        name: &str,
        desc: &str,
    ) -> anyhow::Result<()> {
        self.conn.execute(
            "UPDATE pairings SET desc=?1 WHERE tree_id=?2 AND name=?3",
            params![desc, tree_id, name],
        )?;
        Ok(())
    }

    pub fn get_tree_roots(&mut self) -> anyhow::Result<Vec<PathBuf>> {
        let mut stmt = self.conn.prepare("SELECT root FROM trees")?;
        let mut vec = Vec::new();
        for root in stmt.query_map([], |row| row.get(0))? {
            let root: Vec<u8> = root?;
            vec.push(PathBuf::from_raw_vec(root)?);
        }
        Ok(vec)
    }
}

fn query_script_by_body(conn: &Connection, body: &[u8]) -> anyhow::Result<Option<i64>> {
    let mut stmt = conn.prepare("SELECT _rowid_ FROM scripts WHERE body=?")?;
    Ok(stmt.query_row(params![body], |row| row.get(0)).optional()?)
}

#[derive(Error, Debug)]
#[error("No such script found for current tree")]
pub struct NoSuchScriptForCurrentTree;
