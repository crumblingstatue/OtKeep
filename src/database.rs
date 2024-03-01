use {
    crate::fs_util::ensure_dir_exists,
    anyhow::bail,
    rusqlite::{named_params, params, Connection, OptionalExtension},
    std::{
        collections::HashSet,
        ffi::OsStr,
        path::{Path, PathBuf},
        process::ExitStatus,
    },
    thiserror::Error,
};

/// Contains all the blobs
pub struct Database {
    conn: Connection,
}

const DB_FILENAME: &str = "otkeep.sqlite3";

pub struct ScriptInfo {
    pub name: String,
    pub description: String,
}

pub struct TreeRootInfo {
    pub id: i64,
    pub path: PathBuf,
}

impl Database {
    pub fn load(dir: &Path) -> anyhow::Result<Self> {
        ensure_dir_exists(dir)?;
        let mut conn = Connection::open(dir.join(DB_FILENAME))?;
        let tx = conn.transaction()?;
        tx.execute_batch(include_str!("create_tables.sql"))?;
        tx.commit()?;
        Ok(Self { conn })
    }

    pub fn add_script(&mut self, tree_id: i64, name: &str, body: Vec<u8>) -> anyhow::Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("INSERT INTO blobs (body) VALUES (?)", params![body])?;
        let blob_id = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO tree_scripts (tree_id, name, blob_id) VALUES (?1, ?2, ?3)",
            params![tree_id, name, blob_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn update_script(&mut self, tree_id: i64, name: &str, body: Vec<u8>) -> anyhow::Result<()> {
        match self.query_script_id_from_name(tree_id, name)? {
            Some(blob_id) => {
                self.conn.execute(
                    "UPDATE blobs SET body=?1 WHERE _rowid_=?2",
                    params![body, blob_id],
                )?;
            }
            None => bail!("No such script"),
        }
        Ok(())
    }

    /// Removes a script with `name` from the current tree and returns whether it actually
    /// removed anything
    pub fn remove_script(&mut self, tree_id: i64, name: &str) -> anyhow::Result<bool> {
        Ok(self.conn.execute(
            "DELETE FROM tree_scripts WHERE tree_id=?1 AND name=?2",
            params![tree_id, name],
        )? > 0)
    }

    pub fn run_script(
        &self,
        tree_id: i64,
        name: &str,
        args: impl Iterator<Item = impl AsRef<OsStr>>,
    ) -> anyhow::Result<ExitStatus> {
        match self.query_script_id_from_name(tree_id, name)? {
            Some(id) => {
                let script = self.fetch_blob(id)?;
                let status = crate::run::run_script(&script, args)?;
                Ok(status)
            }
            None => bail!(NoSuchScriptForCurrentTree),
        }
    }

    pub fn blob_is_null(&self, id: i64) -> anyhow::Result<bool> {
        self.conn.query_row_and_then(
            "SELECT body FROM blobs WHERE _rowid_=?",
            params![id],
            |row| {
                let blob: Option<Vec<u8>> = row.get(0)?;
                Ok(blob.is_none())
            },
        )
    }

    pub fn fetch_blob(&self, id: i64) -> Result<Vec<u8>, anyhow::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT body FROM blobs WHERE _rowid_=?")?;
        let blob: Vec<u8> = stmt.query_row(params![id], |row| row.get(0))?;
        Ok(blob)
    }

    fn query_script_id_from_name(&self, tree_id: i64, name: &str) -> anyhow::Result<Option<i64>> {
        let mut stmt = self
            .conn
            .prepare("SELECT blob_id FROM tree_scripts WHERE tree_id=?1 AND name=?2")?;
        let blob_id: Option<i64> = stmt
            .query_row(params![tree_id, name], |row| row.get(0))
            .optional()?;
        Ok(blob_id)
    }

    fn query_file_id_from_name(&self, tree_id: i64, name: &str) -> anyhow::Result<Option<i64>> {
        let mut stmt = self
            .conn
            .prepare("SELECT blob_id FROM tree_files WHERE tree_id=?1 AND name=?2")?;
        let blob_id: Option<i64> = stmt
            .query_row(params![tree_id, name], |row| row.get(0))
            .optional()?;
        Ok(blob_id)
    }

    pub fn scripts_for_tree(&self, tree_id: i64) -> anyhow::Result<Vec<ScriptInfo>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, desc FROM tree_scripts WHERE tree_id=?")?;
        let rows = stmt.query_map(params![tree_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
        let mut vec = Vec::new();
        for result in rows {
            let (name, description) = result?;
            let description: Option<String> = description;
            vec.push(ScriptInfo {
                name,
                description: description.unwrap_or_default(),
            });
        }
        Ok(vec)
    }

    pub fn files_for_tree(&self, tree_id: i64) -> anyhow::Result<Vec<ScriptInfo>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, desc FROM tree_files WHERE tree_id=?")?;
        let rows = stmt.query_map(params![tree_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
        let mut vec = Vec::new();
        for result in rows {
            let (name, description) = result?;
            let description: Option<String> = description;
            vec.push(ScriptInfo {
                name,
                description: description.unwrap_or_default(),
            });
        }
        Ok(vec)
    }

    pub fn query_tree(&self, path: &Path) -> anyhow::Result<Option<i64>> {
        let mut stmt = self
            .conn
            .prepare("SELECT _rowid_ FROM trees where root=?")?;
        Ok(stmt
            .query_row(params![paths_as_strings::encode_path(&path)], |row| {
                row.get(0)
            })
            .optional()?)
    }

    pub fn add_new_tree(&self, path: &Path) -> anyhow::Result<()> {
        let str = paths_as_strings::encode_path(&path);
        self.conn
            .execute("INSERT INTO trees (root) VALUES (?)", params![str])?;
        Ok(())
    }

    pub fn remove_tree(&mut self, tree_id: i64) -> anyhow::Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM trees WHERE _rowid_=?", params![tree_id])?;
        tx.execute("DELETE FROM tree_scripts WHERE tree_id=?", params![tree_id])?;
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
            "UPDATE tree_scripts SET desc=?1 WHERE tree_id=?2 AND name=?3",
            params![desc, tree_id, name],
        )?;
        Ok(())
    }

    pub fn get_tree_roots(&self) -> anyhow::Result<Vec<TreeRootInfo>> {
        let mut stmt = self.conn.prepare("SELECT _rowid_, root FROM trees")?;
        let mut vec = Vec::new();
        for result in stmt.query_map([], |row| {
            let id = row.get(0)?;
            let root_path: String = row.get(1)?;
            Ok((id, root_path))
        })? {
            let (id, root) = result?;
            let pb = paths_as_strings::decode_path(&root)?;
            vec.push(TreeRootInfo { id, path: pb });
        }
        Ok(vec)
    }

    pub fn get_script_by_name(&self, tree_id: i64, name: &str) -> anyhow::Result<Vec<u8>> {
        match self.query_script_id_from_name(tree_id, name)? {
            Some(id) => Ok(self.fetch_blob(id)?),
            None => bail!("No such script"),
        }
    }

    pub fn get_file_by_name(&self, tree_id: i64, name: &str) -> anyhow::Result<Vec<u8>> {
        match self.query_file_id_from_name(tree_id, name)? {
            Some(id) => Ok(self.fetch_blob(id)?),
            None => bail!("No such file"),
        }
    }

    pub fn rename_script(&self, old_name: &str, new_name: &str) -> Result<(), anyhow::Error> {
        self.conn.execute(
            "UPDATE tree_scripts SET name=?1 WHERE name=?2",
            params![new_name, old_name],
        )?;
        Ok(())
    }

    pub fn add_file(&mut self, tree_id: i64, path: &str, bytes: Vec<u8>) -> anyhow::Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT OR REPLACE INTO blobs (body) VALUES (?)",
            params![bytes],
        )?;
        let blob_id = tx.last_insert_rowid();
        tx.execute(
            "INSERT OR REPLACE INTO tree_files (tree_id, name, blob_id) VALUES (?1, ?2, ?3)",
            params![tree_id, path, blob_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn clone_tree(&mut self, src_tree: i64, dst_tree: i64) -> anyhow::Result<()> {
        self.conn.execute(
            include_str!("clone_tree_table.sql"),
            named_params! {
                ":src": src_tree,
                ":dst": dst_tree,
            },
        )?;
        Ok(())
    }
    /// Returns a set of blob ids that are referenced by trees
    ///
    /// Can be used to check whether a blob is part of any tree
    pub fn tree_script_blob_ids(&self) -> anyhow::Result<HashSet<i64>> {
        let mut stmt = self.conn.prepare("SELECT blob_id FROM tree_scripts")?;
        let mut set = HashSet::new();
        let rows = stmt.query_map(params![], |row| {
            let id: i64 = row.get(0)?;
            Ok(id)
        })?;
        for result in rows {
            let id = result?;
            set.insert(id);
        }
        Ok(set)
    }
    pub fn blobs_table_len(&self) -> anyhow::Result<i64> {
        let result = self
            .conn
            .query_row("SELECT COUNT() FROM blobs", params![], |row| row.get(0))?;
        Ok(result)
    }

    pub fn nullify_blob(&self, rowid: i64) -> anyhow::Result<()> {
        self.conn.execute(
            "UPDATE blobs SET body = NULL where _rowid_=?",
            params![rowid],
        )?;
        Ok(())
    }
}

#[derive(Error, Debug)]
#[error("No such script found for current tree")]
pub struct NoSuchScriptForCurrentTree;
