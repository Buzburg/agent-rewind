use crate::safe_paths::{checked_path, destination, invalid, workspace};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub const REWIND_DIR: &str = ".rewind";
pub const BLOBS_DIR: &str = ".rewind/blobs";
pub const MANIFEST_FILE: &str = ".rewind/manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub path: String,
    pub hash: String,
    pub size: u64,
    pub is_executable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub files: HashMap<String, FileRecord>,
    pub token_burn: Option<u64>,
    #[serde(default)]
    pub workspace_root: Option<PathBuf>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,
    pub current_session: String,
    pub checkpoints: Vec<Checkpoint>,
}

pub struct Engine {
    pub root: PathBuf,
}

impl Engine {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn init(&self) -> std::io::Result<()> {
        let _rewind_path = self.root.join(REWIND_DIR);
        workspace(&self.root)?;
        let blobs_path = checked_path(&self.root.join(BLOBS_DIR))?;
        fs::create_dir_all(&blobs_path)?;

        let manifest_path = checked_path(&self.root.join(MANIFEST_FILE))?;
        if !manifest_path.exists() {
            let manifest = Manifest {
                version: 1,
                current_session: format!("session-{}", Utc::now().timestamp()),
                checkpoints: Vec::new(),
            };
            self.save_manifest(&manifest)?;
        }
        Ok(())
    }

    pub fn load_manifest(&self) -> std::io::Result<Manifest> {
        let manifest_path = checked_path(&self.root.join(MANIFEST_FILE))?;
        if !manifest_path.exists() {
            self.init()?;
        }
        let data = fs::read_to_string(&manifest_path)?;
        let manifest: Manifest = serde_json::from_str(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(manifest)
    }

    pub fn save_manifest(&self, manifest: &Manifest) -> std::io::Result<()> {
        let manifest_path = checked_path(&self.root.join(MANIFEST_FILE))?;
        let data = serde_json::to_string_pretty(manifest)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(manifest_path, data)
    }

    fn should_ignore(&self, path: &Path) -> bool {
        let components: Vec<_> = path
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect();
        components.iter().any(|comp| {
            comp == ".git"
                || comp == ".rewind"
                || comp == "target"
                || comp == "node_modules"
                || comp == ".build-venv"
                || comp == ".build-cache"
                || comp == ".mypy_cache"
                || comp == ".ruff_cache"
                || comp == "__pycache__"
        })
    }

    pub fn hash_and_store_blob(&self, path: &Path) -> std::io::Result<(String, u64)> {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let mut hasher = Sha256::new();
        hasher.update(&buffer);
        let hash = format!("{:x}", hasher.finalize());

        let blob_path = checked_path(&self.root.join(BLOBS_DIR).join(&hash))?;
        if !blob_path.exists() {
            fs::write(blob_path, &buffer)?;
        }

        Ok((hash, buffer.len() as u64))
    }

    pub fn get_blob_content(&self, hash: &str) -> std::io::Result<Vec<u8>> {
        if hash.len() != 64
            || !hash
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(invalid("Invalid checkpoint blob identifier"));
        }
        let blob_path = checked_path(&self.root.join(BLOBS_DIR).join(hash))?;
        if !blob_path.is_file() {
            return Err(invalid("Missing checkpoint blob or non-regular file"));
        }
        let bytes = fs::read(blob_path)?;
        if format!("{:x}", Sha256::digest(&bytes)) != hash {
            return Err(invalid("Checkpoint blob checksum mismatch"));
        }
        Ok(bytes)
    }

    pub fn create_checkpoint(
        &self,
        message: &str,
        session_id: Option<&str>,
        token_burn: Option<u64>,
    ) -> std::io::Result<Checkpoint> {
        self.init()?;
        let mut manifest = self.load_manifest()?;

        let mut files = HashMap::new();
        for entry in WalkDir::new(&self.root).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if entry.file_type().is_file() && !self.should_ignore(path) {
                let rel_path = path
                    .strip_prefix(&self.root)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
                    .to_str()
                    .ok_or_else(|| invalid("Checkpoint paths must be UTF-8"))?
                    .replace(std::path::MAIN_SEPARATOR, "/");
                destination(&self.root, &rel_path)?;

                let (hash, size) = self.hash_and_store_blob(path)?;
                files.insert(
                    rel_path.clone(),
                    FileRecord {
                        path: rel_path,
                        hash,
                        size,
                        is_executable: false,
                    },
                );
            }
        }

        let now = Utc::now();
        let checkpoint_id = format!("cp-{}-{}", manifest.checkpoints.len() + 1, now.timestamp());
        let session = session_id.unwrap_or(&manifest.current_session).to_string();

        let checkpoint = Checkpoint {
            id: checkpoint_id,
            session_id: session,
            timestamp: now,
            message: message.to_string(),
            files,
            token_burn,
            workspace_root: Some(workspace(&self.root)?),
        };

        manifest.checkpoints.push(checkpoint.clone());
        self.save_manifest(&manifest)?;

        Ok(checkpoint)
    }

    pub fn list_checkpoints(&self) -> std::io::Result<Vec<Checkpoint>> {
        let manifest = self.load_manifest()?;
        Ok(manifest.checkpoints)
    }

    pub fn get_checkpoint(&self, id_or_index: &str) -> std::io::Result<Option<Checkpoint>> {
        let manifest = self.load_manifest()?;

        // Match exact ID
        if let Some(cp) = manifest.checkpoints.iter().find(|cp| cp.id == id_or_index) {
            return Ok(Some(cp.clone()));
        }

        // Match index (e.g. "1" or "0")
        if let Ok(idx) = id_or_index.parse::<usize>() {
            if idx > 0 && idx <= manifest.checkpoints.len() {
                return Ok(Some(manifest.checkpoints[idx - 1].clone()));
            }
        }

        Ok(None)
    }

    pub fn get_latest_checkpoint(&self) -> std::io::Result<Option<Checkpoint>> {
        let manifest = self.load_manifest()?;
        Ok(manifest.checkpoints.last().cloned())
    }
}
