use crate::checkpoint::{Checkpoint, Engine};
use crate::diff::{ChangeType, DiffEngine};
use colored::*;
use std::fs;
use std::io;
use std::path::Path;

pub struct RestoreEngine<'a> {
    engine: &'a Engine,
}

impl<'a> RestoreEngine<'a> {
    pub fn new(engine: &'a Engine) -> Self {
        Self { engine }
    }

    pub fn restore_checkpoint(&self, checkpoint: &Checkpoint) -> io::Result<()> {
        let diff_engine = DiffEngine::new(self.engine);
        let deltas = diff_engine.compute_working_tree_diff(checkpoint)?;

        if deltas.is_empty() {
            println!("{}", "Workspace is already matching this checkpoint.".cyan());
            return Ok(());
        }

        let mut restored_count = 0;
        let mut deleted_count = 0;

        for delta in &deltas {
            let target_path = self.engine.root.join(&delta.path);

            match delta.change_type {
                ChangeType::Added => {
                    // File was added after checkpoint -> remove it
                    if target_path.exists() {
                        fs::remove_file(&target_path)?;
                        deleted_count += 1;
                    }
                }
                ChangeType::Deleted | ChangeType::Modified => {
                    // File was modified or deleted -> recreate/overwrite with checkpoint version
                    if let Some(old_hash) = &delta.old_hash {
                        let content = self.engine.get_blob_content(old_hash)?;
                        if let Some(parent) = target_path.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        fs::write(&target_path, content)?;
                        restored_count += 1;
                    }
                }
                ChangeType::Unchanged => {}
            }
        }

        // Clean up empty directories if any were left
        self.cleanup_empty_dirs(&self.engine.root)?;

        println!(
            "{}",
            format!(
                "✔ Successfully rewound to checkpoint '{}'! ({} files restored, {} created files removed)",
                checkpoint.id, restored_count, deleted_count
            )
            .green()
            .bold()
        );

        Ok(())
    }

    fn cleanup_empty_dirs(&self, dir: &Path) -> io::Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                if name != ".git" && name != ".rewind" && name != "target" {
                    let _ = self.cleanup_empty_dirs(&path);
                    let _ = fs::remove_dir(&path); // Only succeeds if empty
                }
            }
        }
        Ok(())
    }
}
