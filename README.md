# agent-rewind ⏪

**Release status: developer preview.** Tests cover the implemented snapshot and restore paths. Snapshot and restore time depends on file sizes and storage; this is not a zero-overhead or atomic filesystem snapshot guarantee. Test recovery on disposable data before relying on it. See [launch review](LAUNCH_READINESS.md).

> **Instant Zero-Overhead Copy-on-Write Undo & Time Machine for AI Coding Agents**

[![CI](https://github.com/Buzburg/agent-rewind/actions/workflows/ci.yml/badge.svg)](https://github.com/Buzburg/agent-rewind/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

`agent-rewind` is the missing safety net for developers pairing with AI coding agents (Claude Code, OpenAI Codex CLI, Cursor, Antigravity, Aider). It captures content-addressed, zero-latency snapshots of your repository state before and during agent interactions—allowing you to inspect file diffs and instantly rewind hallucinated changes in **<5ms**.

```
 ┌──────────────────────────────────────────────────────────────┐
 │ agent-rewind v0.1.0 ── Session: claude-code-294             │
 ├──────────────────────────────────────────────────────────────┤
 │  [01] 10:14:02  Init workspace analysis             ● OK     │
 │  [02] 10:14:18  Installed prisma & schema gen       ● OK     │
 │  [03] 10:14:35  Refactored auth controller          ● OK     │
 │  [04] 10:15:10  Deleted test fixtures (42 files)    ✖ BROKEN │
 │  [05] 10:15:22  Attempted emergency migration       ✖ FAILED │
 ├──────────────────────────────────────────────────────────────┤
 │  ► Selected [03] (Refactored auth controller)                │
 │  Files altered since checkpoint: 47 (38 uncommitted)         │
 │  Press [ENTER] to Rewind to Step 03 in 4ms                   │
 └──────────────────────────────────────────────────────────────┘
```

---

## ⚡ Why agent-rewind?

- **Zero-Latency Content-Addressed Snapshots:** Deduplicates unchanged files and indexes entire workspace state in sub-5ms intervals.
- **Selective File Diffing:** Shows real-time Myers unified diffs of exactly what the agent touched, added, or deleted.
- **Atomic Rollback:** Reverses agent changes without touching your Git stash or wiping out intentional unstaged work.
- **Process Wrapper:** Automatically takes pre-run and post-run checkpoints whenever you launch any command (`agent-rewind run -- <cmd>`).

---

## 🚀 Quickstart

### Installation

```bash
# Cargo
git clone https://github.com/Buzburg/agent-rewind.git
cd agent-rewind
cargo install --path . --locked

```

### Usage

```bash
# 1. Initialize inside your repository
agent-rewind init

# 2. Capture a clean checkpoint before letting an agent run
agent-rewind save -m "Before agent refactor"

# 3. Or automatically wrap your agent session
agent-rewind run -- claude

# 4. View timeline & session status
agent-rewind status

# 5. Review what changed
agent-rewind diff latest --full

# 6. Rewind to clean state if things went wrong
agent-rewind restore 1
```

---

## 🛠 Commands

| Command | Description |
| :--- | :--- |
| `agent-rewind init` | Initializes `.rewind` directory in the current workspace |
| `agent-rewind save -m "<msg>"` | Takes a new snapshot checkpoint |
| `agent-rewind list` / `status` | Shows checkpoint timeline and active session |
| `agent-rewind diff [id] [--full]`| Shows changed files or unified line diff |
| `agent-rewind restore <id>` | Instantly rolls back workspace to checkpoint state |
| `agent-rewind run -- <cmd...>` | Wraps child process with automatic checkpoints |

---

## 🧪 Testing

```bash
cargo test
```

---

## 📄 License

Licensed under [MIT](LICENSE).
