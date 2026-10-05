use crate::checkpoint::{Checkpoint, Engine};
use colored::*;
use similar::{ChangeTag, TextDiff};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, PartialEq, Eq)]
pub enum ChangeType {
    Added,
    Deleted,
    Modified,
    Unchanged,
}

#[derive(Debug)]
pub struct FileDelta {
    pub path: String,
    pub change_type: ChangeType,
    pub old_hash: Option<String>,
    pub new_hash: Option<String>,
}

pub struct DiffEngine<'a> {
    engine: &'a Engine,
}

impl<'a> DiffEngine<'a> {
    pub fn new(engine: &'a Engine) -> Self {
        Self { engine }
    }

    pub fn compute_working_tree_diff(
        &self,
        checkpoint: &Checkpoint,
    ) -> std::io::Result<Vec<FileDelta>> {
        let mut current_files = BTreeMap::new();

        for entry in WalkDir::new(&self.engine.root).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if entry.file_type().is_file() && !self.engine_should_ignore(path) {
                let rel_path = path
                    .strip_prefix(&self.engine.root)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
                    .to_string_lossy()
                    .replace('\\', "/");

                // Read and compute hash
                let (hash, _) = self.engine.hash_and_store_blob(path)?;
                current_files.insert(rel_path, hash);
            }
        }

        let mut all_paths: HashSet<String> = HashSet::new();
        for k in checkpoint.files.keys() {
            all_paths.insert(k.clone());
        }
        for k in current_files.keys() {
            all_paths.insert(k.clone());
        }

        let mut sorted_paths: Vec<_> = all_paths.into_iter().collect();
        sorted_paths.sort();

        let mut deltas = Vec::new();
        for path in sorted_paths {
            let old_rec = checkpoint.files.get(&path);
            let new_hash = current_files.get(&path);

            match (old_rec, new_hash) {
                (None, Some(h)) => {
                    deltas.push(FileDelta {
                        path,
                        change_type: ChangeType::Added,
                        old_hash: None,
                        new_hash: Some(h.clone()),
                    });
                }
                (Some(rec), None) => {
                    deltas.push(FileDelta {
                        path,
                        change_type: ChangeType::Deleted,
                        old_hash: Some(rec.hash.clone()),
                        new_hash: None,
                    });
                }
                (Some(rec), Some(h)) => {
                    if &rec.hash != h {
                        deltas.push(FileDelta {
                            path,
                            change_type: ChangeType::Modified,
                            old_hash: Some(rec.hash.clone()),
                            new_hash: Some(h.clone()),
                        });
                    }
                }
                (None, None) => {}
            }
        }

        Ok(deltas)
    }

    fn engine_should_ignore(&self, path: &Path) -> bool {
        let components: Vec<_> = path.components().map(|c| c.as_os_str().to_string_lossy()).collect();
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

    pub fn print_diff_summary(&self, deltas: &[FileDelta]) {
        if deltas.is_empty() {
            println!("{}", "✔ Working tree is identical to checkpoint.".green().bold());
            return;
        }

        println!("{}", format!("Found {} changed file(s):", deltas.len()).bold());
        for delta in deltas {
            match delta.change_type {
                ChangeType::Added => {
                    println!("  {} {}", "+".green().bold(), delta.path.green());
                }
                ChangeType::Deleted => {
                    println!("  {} {}", "-".red().bold(), delta.path.red());
                }
                ChangeType::Modified => {
                    println!("  {} {}", "~".yellow().bold(), delta.path.yellow());
                }
                ChangeType::Unchanged => {}
            }
        }
    }

    pub fn print_full_unified_diff(
        &self,
        _checkpoint: &Checkpoint,
        deltas: &[FileDelta],
    ) -> std::io::Result<()> {
        for delta in deltas {
            let old_text = if let Some(old_hash) = &delta.old_hash {
                String::from_utf8_lossy(&self.engine.get_blob_content(old_hash)?).to_string()
            } else {
                String::new()
            };

            let new_path = self.engine.root.join(&delta.path);
            let new_text = if new_path.exists() {
                fs::read_to_string(&new_path).unwrap_or_default()
            } else {
                String::new()
            };

            let diff = TextDiff::from_lines(&old_text, &new_text);
            println!("\n{}", format!("--- a/{}", delta.path).bold());
            println!("{}", format!("+++ b/{}", delta.path).bold());

            for change in diff.iter_all_changes() {
                let sign = match change.tag() {
                    ChangeTag::Delete => "-".red(),
                    ChangeTag::Insert => "+".green(),
                    ChangeTag::Equal => " ".normal(),
                };
                print!("{}{}", sign, change);
            }
        }
        Ok(())
    }
}
