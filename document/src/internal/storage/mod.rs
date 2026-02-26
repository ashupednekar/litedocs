use std::fs::{self, OpenOptions};
use std::fmt;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::internal::naming::{doc_id_from_title, title_from_doc_id};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DocId(String);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DocTitle(String);

impl DocId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl DocTitle {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for DocId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for DocId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for DocTitle {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for DocTitle {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<DocTitle> for DocId {
    fn from(value: DocTitle) -> Self {
        Self(doc_id_from_title(&value.0))
    }
}

impl From<DocId> for DocTitle {
    fn from(value: DocId) -> Self {
        Self(title_from_doc_id(&value.0))
    }
}

impl fmt::Display for DocId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for DocTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub trait FileStorage {
    fn read(&self, doc_id: &str) -> io::Result<Vec<u8>>;

    fn write_buffer(
        &self,
        doc_id: &str,
        offset: u64,
        data: &[u8],
        truncate_to: Option<u64>,
    ) -> io::Result<()>;

    fn write_full(&self, doc_id: &str, data: &[u8]) -> io::Result<()> {
        self.write_buffer(doc_id, 0, data, Some(data.len() as u64))
    }

    fn list_docs(&self) -> io::Result<Vec<StoredDoc>>;

    fn delete(&self, doc_id: &str) -> io::Result<()>;

    fn receive_remote_change(&self, _doc_id: &str, _offset: u64, _data: &[u8]) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredDoc {
    pub id: DocId,
    pub title: DocTitle,
    pub updated_at: SystemTime,
}

#[derive(Clone)]
pub struct LocalFileStorage {
    root: PathBuf,
}

impl Default for LocalFileStorage {
    fn default() -> Self {
        Self {
            root: default_storage_root(),
        }
    }
}

impl LocalFileStorage {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn docs_dir(&self) -> PathBuf {
        self.root.join("docs")
    }

    fn doc_path(&self, doc_id: &str) -> PathBuf {
        self.docs_dir().join(format!("{doc_id}.md"))
    }

    fn ensure_parent_dir(path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }
}

impl FileStorage for LocalFileStorage {
    fn read(&self, doc_id: &str) -> io::Result<Vec<u8>> {
        let path = self.doc_path(doc_id);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let mut file = OpenOptions::new().read(true).open(path)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        Ok(buf)
    }

    fn write_buffer(
        &self,
        doc_id: &str,
        offset: u64,
        data: &[u8],
        truncate_to: Option<u64>,
    ) -> io::Result<()> {
        let path = self.doc_path(doc_id);
        Self::ensure_parent_dir(&path)?;

        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(path)?;

        file.seek(SeekFrom::Start(offset))?;
        file.write_all(data)?;

        if let Some(new_len) = truncate_to {
            file.set_len(new_len)?;
        }

        file.flush()?;
        Ok(())
    }

    fn list_docs(&self) -> io::Result<Vec<StoredDoc>> {
        let docs_dir = self.docs_dir();
        if !docs_dir.exists() {
            return Ok(Vec::new());
        }

        let mut docs = Vec::new();
        for entry in fs::read_dir(docs_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                continue;
            }
            let id = match path.file_stem().and_then(|stem| stem.to_str()) {
                Some(stem) => DocId::from(stem),
                None => continue,
            };
            let metadata = fs::metadata(&path)?;
            let updated_at = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            docs.push(StoredDoc {
                title: DocTitle::from(id.clone()),
                id,
                updated_at,
            });
        }

        docs.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(docs)
    }

    fn delete(&self, doc_id: &str) -> io::Result<()> {
        let path = self.doc_path(doc_id);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}

fn default_storage_root() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".litedocs");
    }

    PathBuf::from(".litedocs")
}
