---
name: sync-types
description: Audit that Rust models.rs and src/types/launcher.ts are in sync across the Tauri IPC boundary
---

Read both `src-tauri/src/models.rs` and `src/types/launcher.ts` in full.

For each Rust struct that derives `Serialize`/`Deserialize` and corresponds to a TypeScript type:
1. Verify all fields are present on both sides with no extras or missing fields.
2. Verify camelCase mapping is correct — Rust `snake_case` fields must appear as `camelCase` in TypeScript (the Rust side uses `#[serde(rename_all = "camelCase")]` globally).
3. Verify enum variants match — Rust `PascalCase` variants map to `camelCase` string literals in TypeScript (e.g. `ModChannel::Stable` → `"stable"`).
4. Verify `Option<T>` on the Rust side maps to `T | null` in TypeScript.

Report any mismatches, missing fields, or type-level divergences. If everything is aligned, say so explicitly.
