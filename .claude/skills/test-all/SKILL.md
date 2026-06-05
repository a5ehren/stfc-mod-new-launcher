---
name: test-all
description: Run both the Vitest frontend tests and the Rust cargo tests, report pass/fail for each
disable-model-invocation: true
---

Run both test suites in sequence:

1. Run `pnpm test` from the repo root (Vitest — frontend + lib tests)
2. Run `cd src-tauri && cargo test` (Rust unit tests)

Report pass/fail for each suite. If either fails, show the failing output in full.
