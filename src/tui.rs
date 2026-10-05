use crate::checkpoint::Engine;
use colored::*;

pub struct Dashboard<'a> {
    engine: &'a Engine,
}

impl<'a> Dashboard<'a> {
    pub fn new(engine: &'a Engine) -> Self {
        Self { engine }
    }

    pub fn display(&self) -> std::io::Result<()> {
        let manifest = self.engine.load_manifest()?;
        println!("{}", "╔═══════════════════════════════════════════════════════════════════════╗".cyan());
        println!("{}", "║              agent-rewind v0.1.0 — Agent Session Time Machine         ║".cyan().bold());
        println!("{}", "╚═══════════════════════════════════════════════════════════════════════╝".cyan());
        println!(" Active Session: {}", manifest.current_session.bold());
        println!(" Total Checkpoints: {}", manifest.checkpoints.len().to_string().yellow().bold());
        println!();

        if manifest.checkpoints.is_empty() {
            println!("{}", "  No checkpoints recorded yet. Run `agent-rewind save` to capture state.".dimmed());
            return Ok(());
        }

        println!("{:<4} {:<24} {:<20} {:<8} {}", "IDX", "CHECKPOINT ID", "TIMESTAMP", "FILES", "MESSAGE");
        println!("{}", "─".repeat(74).dimmed());

        for (idx, cp) in manifest.checkpoints.iter().enumerate() {
            let is_latest = idx == manifest.checkpoints.len() - 1;
            let marker = if is_latest { "►".green().bold() } else { " ".normal() };
            let ts = cp.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();

            println!(
                "{}{:<3} {:<24} {:<20} {:<8} {}",
                marker,
                idx + 1,
                cp.id.cyan(),
                ts.dimmed(),
                cp.files.len(),
                cp.message.bold()
            );
        }

        println!();
        if let Some(latest) = manifest.checkpoints.last() {
            println!("  Quick rollback to latest: {}", format!("agent-rewind restore {}", latest.id).green());
            println!("  View diff from latest:    {}", format!("agent-rewind diff {}", latest.id).yellow());
        }

        Ok(())
    }
}
