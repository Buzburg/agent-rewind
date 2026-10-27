# Agent Rewind ⏪

**AI broke your workspace? Rewind it.**

Capture a checkpoint before a coding agent makes changes, inspect the diff, and restore the saved file contents when an experiment goes wrong. Works around any CLI command and keeps its state in `.rewind/`, separate from Git.

## Try it

Requires a current stable Rust toolchain. Install from this repository:

```sh
git clone https://github.com/Buzburg/agent-rewind.git
cd agent-rewind
cargo install --path . --locked
```

In a disposable project first:

```sh
agent-rewind init
agent-rewind save -m "Before the refactor"
# Make changes, or wrap a coding session:
agent-rewind run -- your-agent-command
agent-rewind list
agent-rewind diff 1 --full
agent-rewind restore 1
```

`restore 1` restores the **whole captured file set** and removes eligible files added after that checkpoint. It cannot distinguish your later edits from an agent's edits. Review the diff and stop writers before restoring.

## What you get

- Content-addressed SHA-256 storage that deduplicates unchanged file bytes.
- Named checkpoints, readable diffs, and a command wrapper for pre-run checkpoints and post-run diffs.
- Workspace binding, path validation, blob checksums, and per-file temporary replacement before recovery.
- Recovery of deleted executable scripts on Unix.

## Recovery boundaries

Developer beta. This is file-content recovery, not an atomic filesystem snapshot or a backup service. A disk failure or interruption during a multi-file restore can leave partial results. Snapshots exclude `.git`, `.rewind`, common dependency/build directories and caches; they do not implement `.gitignore`. Links, ownership, full permission metadata, and concurrent writers are outside the recovery contract. Snapshot timing depends on bytes and storage; there is no zero-overhead or fixed-latency guarantee.

Read [snapshot safety](SNAPSHOT_SAFETY.md) before using recovery on valuable work. Keep an independent backup. Old snapshots without workspace binding require manual recovery.

## Verify

```sh
cargo test --locked
cargo build --release --locked
```

See [release verification](RELEASE_VERIFICATION.md) for checked behavior and remaining limits. MIT licensed. Built by Buzburg.
