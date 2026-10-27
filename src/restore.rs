use crate::checkpoint::{Checkpoint, Engine};
use crate::diff::{ChangeType, DiffEngine};
use crate::safe_paths::{checked_path, destination, invalid, workspace};
use colored::*;
use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;

pub struct RestoreEngine<'a> {
    engine: &'a Engine,
}

impl<'a> RestoreEngine<'a> {
    pub fn new(engine: &'a Engine) -> Self {
        Self { engine }
    }

    pub fn restore_checkpoint(&self, checkpoint: &Checkpoint) -> io::Result<()> {
        let root = workspace(&self.engine.root)?;
        if checkpoint.workspace_root.as_ref() != Some(&root) {
            return Err(invalid("Checkpoint belongs to another workspace or predates workspace binding; preserve it for manual recovery"));
        }
        // Validate every destination and content blob before any workspace writes/deletes.
        for (relative, record) in &checkpoint.files {
            destination(&root, relative)?;
            if record.path != *relative {
                return Err(invalid("Checkpoint file path does not match its key"));
            }
            let bytes = self.engine.get_blob_content(&record.hash)?;
            if bytes.len() as u64 != record.size {
                return Err(invalid("Checkpoint file size mismatch"));
            }
        }
        for relative in checkpoint.files.keys() {
            let path = Path::new(relative);
            if path.ancestors().skip(1).any(|parent| {
                checkpoint
                    .files
                    .contains_key(&parent.to_string_lossy().replace('\\', "/"))
            }) {
                return Err(invalid("Checkpoint file paths conflict with directories"));
            }
        }
        let diff_engine = DiffEngine::new(self.engine);
        let deltas = diff_engine.compute_working_tree_diff(checkpoint)?;

        if deltas.is_empty() {
            println!(
                "{}",
                "Workspace is already matching this checkpoint.".cyan()
            );
            return Ok(());
        }

        for delta in &deltas {
            destination(&root, &delta.path)?;
        }
        let mut restored_count = 0;
        let mut deleted_count = 0;

        for delta in &deltas {
            let target_path = destination(&root, &delta.path)?;

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
                        checked_path(&target_path)?;
                        let mut temporary =
                            tempfile::NamedTempFile::new_in(target_path.parent().unwrap())?;
                        temporary.write_all(&content)?;
                        temporary.as_file().sync_all()?;
                        if target_path.exists() {
                            temporary
                                .as_file()
                                .set_permissions(fs::metadata(&target_path)?.permissions())?;
                        }
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            if checkpoint.files[&delta.path].is_executable {
                                let mode =
                                    temporary.as_file().metadata()?.permissions().mode() | 0o100;
                                temporary
                                    .as_file()
                                    .set_permissions(fs::Permissions::from_mode(mode))?;
                            }
                        }
                        checked_path(&target_path)?;
                        // Replace the directory entry instead of truncating a shared hard link.
                        temporary
                            .persist(&target_path)
                            .map_err(|error| error.error)?;
                        restored_count += 1;
                    }
                }
                ChangeType::Unchanged => {}
            }
        }

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
}
