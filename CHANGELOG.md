# Changelog

All notable changes to this project are documented in this file.

## [0.3.2.1] - 2026-11-29 (feat/rust-core-v0.3)

### Fixed
- **Installation Update Transaction (`skill-lifecycle`):** Fixed bug where updating an intact installed skill to a new commit without `--force` failed during ledger commit because `record_installation()` re-compared the newly replaced destination directory against the previous ledger hash. Added `commit_verified_ledger_record()` which validates concurrent ledger consistency against the pre-replacement hash.
- **Fail-Closed Integrity Verification (`skill-lifecycle`):** Replaced `if let Ok(...)` in `record_installation()` with explicit `Result` propagation (`LifecycleError::TamperDetected`) and blocked recording `REJECTED` / `QUARANTINED` trust states even under `--force`.
- **Regression Tests:** Added `test_v0321_update_intact_reinstall_untracked_and_fail_closed_matrix` covering all 10 transaction, overwrite, fail-closed, rollback, and temp-dir cleanup invariants.

## [0.3.2] - 2026-11-29 (feat/rust-core-v0.3)

### Security & Reliability
- **Upstream Commit Resolution (`skill-lifecycle`):** Removed `"main"` fallback in `resolve_head_commit_sha()`. Validated 40/64-char hex commit SHAs, distinguished HTTP 404, 401/403, 429, timeouts, malformed JSON, and invalid SHAs. `check_for_updates()` now reports `UNKNOWN` on upstream failures and never marks unavailable upstreams as `UP_TO_DATE`.
- **Complete Skill Directory Installation (`skill-lifecycle` & `skill-registry`):** Replaced single-file `SKILL.md` installation with full skill directory installation at pinned commit SHAs, including nested subdirectories (`scripts/`, `references/`, `assets/`), strict path validation (rejecting traversal, absolute paths, drive prefixes, backslashes, symlinks, duplicate normalized paths, and unscannable binary extensions), configurable file/total size and file count limits, multi-file `TrustScanner` inspection, and canonical length-prefixed directory SHA-256 hashing (`compute_directory_sha256`, `compute_entries_sha256`).
- **Transactional Consistency & Rollback (`skill-lifecycle`):** Implemented unique staging directory verification, backup-and-restore directory replacement, and filesystem rollback if ledger commit fails. Removed fabricated `"1.0.0"` and `"Reviewed upstream license"` defaults in favor of parsed frontmatter or `"UNSPECIFIED"`.
- **Tree Truncation & Catalog Hardening (`skill-discovery`):** Enforced `GitTreeResponse.truncated` (`TREE_TRUNCATED` error; never caches truncated trees), added rate-limit/auth error differentiation, rejected `limit == 0` and `limit > MAX_SYNC_LIMIT`, validated catalog JSON schemas, and preserved existing valid catalog indexes on sync failure.
- **Trust & Integrity Enforcement (`skill-activation`, `router-core`, `skill-discovery`):** Verified complete installed directory hashes in `InstalledRegistry::retrieve_content()` before activation and added negative tests for `UNSCANNED` and `SCANNING` states.
- **CLI Correctness (`router-cli`):** Renamed `update-skills` to `check-updates` (preserving `update-skills` / `update:skills` as deprecated check-only aliases), removed unused `--force`, added `--root` and `--dry-run`, honored `--no-network` across lifecycle commands, and added binary integration tests (`crates/router-cli/tests/cli_integration.rs`).

## [0.3.1] - 2026-11-28 (feat/rust-core-v0.3)
- Remediated P0/P1 Rust workspace defects, fixed Clippy warnings, and guarded `undefined` repository handling in Node.js `DirectoryResolver` and `ActivationPlanner`.

## [0.3.0] - 2026-11-28 (feat/rust-core-v0.3)
- Initial 13-crate Rust core workspace implementation alongside V0.2 Node.js baseline.

## [0.2.0] - Frozen Baseline
- Node.js V0.2 routing, ranking, security, orchestration, and lifecycle engine (`33/33` tests passing).
