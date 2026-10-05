use crate::checkpoint::Engine;
use crate::diff::DiffEngine;
use colored::*;
use std::process::Command;
use std::time::Instant;

pub struct AgentRunner<'a> {
    engine: &'a Engine,
}

impl<'a> AgentRunner<'a> {
    pub fn new(engine: &'a Engine) -> Self {
        Self { engine }
    }

    pub fn execute(&self, cmd_args: &[String]) -> std::io::Result<i32> {
        if cmd_args.is_empty() {
            eprintln!("{}", "Error: No command specified to run.".red());
            return Ok(1);
        }

        let cmd_display = cmd_args.join(" ");
        println!(
            "{}",
            format!("► [agent-rewind] Checkpointing before: '{}'...", cmd_display)
                .cyan()
                .bold()
        );

        let pre_cp = self.engine.create_checkpoint(
            &format!("Pre-exec: {}", cmd_display),
            None,
            None,
        )?;
        println!(
            "{}",
            format!("  Captured checkpoint '{}' ({} files tracked)", pre_cp.id, pre_cp.files.len()).dimmed()
        );

        let start_time = Instant::now();
        let program = &cmd_args[0];
        let args = &cmd_args[1..];

        let mut child = Command::new(program)
            .args(args)
            .current_dir(&self.engine.root)
            .spawn()
            .map_err(|e| {
                eprintln!("{}", format!("Failed to spawn command '{}': {}", program, e).red());
                e
            })?;

        let status = child.wait()?;
        let elapsed = start_time.elapsed();
        let exit_code = status.code().unwrap_or(-1);

        println!(
            "\n{}",
            format!(
                "► [agent-rewind] Command finished in {:.2?} with status {}",
                elapsed, exit_code
            )
            .cyan()
        );

        let diff_engine = DiffEngine::new(self.engine);
        let deltas = diff_engine.compute_working_tree_diff(&pre_cp)?;
        if deltas.is_empty() {
            println!("{}", "  No workspace files were modified.".green());
        } else {
            println!(
                "{}",
                format!("  ⚠ {} file(s) modified by command:", deltas.len()).yellow().bold()
            );
            diff_engine.print_diff_summary(&deltas);
            println!(
                "{}",
                format!("  To undo all changes, run: agent-rewind restore {}", pre_cp.id).magenta().bold()
            );
        }

        Ok(exit_code)
    }
}
