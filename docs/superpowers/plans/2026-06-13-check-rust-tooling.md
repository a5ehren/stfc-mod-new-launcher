# Rust Check Script Update Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `pnpm check` validate both the frontend and Rust backend by running Biome, `cargo fmt --check`, and `cargo clippy` from `src-tauri`.

**Architecture:** Keep the existing top-level script entrypoint and extend it with Rust verification commands executed in the backend crate directory. Update the repo guidance so the documented check command matches the actual script behavior.

**Tech Stack:** `pnpm`, Biome, Cargo, Rustfmt, Clippy

---

### Task 1: Update the root check script

**Files:**
- Modify: `package.json`

- [ ] **Step 1: Replace the existing check command**

```json
{
  "scripts": {
    "check": "biome check && cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings"
  }
}
```

- [ ] **Step 2: Verify the command sequence is correct**

Run: `pnpm check`
Expected: Biome runs first, then `cargo fmt --check`, then `cargo clippy`; the command fails if any stage fails.

### Task 2: Keep repo instructions in sync

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Update the check command note**

```md
pnpm check               # check only (Biome + Rust fmt/clippy)
```

- [ ] **Step 2: Verify the wording matches the script**

Run: `sed -n '20,35p' CLAUDE.md`
Expected: The command list reflects that `pnpm check` now covers both Biome and Rust backend checks.

