# Codex Review: YT-HOME v2.0.11 Simplification/SSH/Stats

Review-only pass against `.hermes/plans/2026-05-10-ythome-simplification-ssh-stats.md`.

## Findings

### High: Stats and online reporting remain read-only and are not wired to runtime collection

Plan Phase 4 requires working online/traffic reporting, with a real sing-box path or a safe recent-traffic fallback. The implementation only changes `get_onlines()` from always-empty to a query over `stats`, but there is still no code path that inserts traffic rows into `stats`.

- `crates/http-api/src/lib.rs:207` publishes `state.stats.get_onlines()` to the dashboard payload.
- `crates/domain-stats/src/lib.rs:49` queries recent rows from `stats` to decide online users.
- `crates/domain-stats/src/lib.rs:76` serves `/api/stats` from the same table.
- Repo-wide search found no `INSERT INTO stats`; the only stats mutation found is the delete path at `crates/domain-config/src/lib.rs:153`.
- `crates/infra-scheduler/src/lib.rs:1` is still placeholder code, so Phase 4's expected scheduler/stats pipeline was not implemented.
- `crates/shared/src/settings.rs:38` only enables `experimental.cache_file`; no sing-box stats/API defaults are added.

Impact: dashboard online chips, client online status, and traffic graphs will stay empty unless rows are manually seeded. This is a blocker for the stats acceptance criteria.

### High: Runtime config still depends on hidden legacy outbounds/services/endpoints and stale route/DNS values

The plan says removed frontend config areas should be backend-fixed defaults and existing obsolete DB rows should be ignored or overridden rather than crashing. The current generator still loads hidden legacy tables directly into the sing-box runtime config.

- `crates/domain-config/src/lib.rs:123` loads runtime outbounds from DB, then only inserts `direct` if absent.
- `crates/domain-config/src/lib.rs:128` and `crates/domain-config/src/lib.rs:130` still insert DB-backed services and endpoints.
- `crates/domain-config/src/lib.rs:621`, `crates/domain-config/src/lib.rs:630`, and `crates/domain-config/src/lib.rs:642` load those legacy tables unconditionally.
- `crates/domain-config/src/lib.rs:933`, `crates/domain-config/src/lib.rs:940`, and `crates/domain-config/src/lib.rs:953` strictly parse legacy JSON options, so malformed/stale hidden rows can fail config generation.
- `crates/domain-config/src/lib.rs:714` and `crates/domain-config/src/lib.rs:737` preserve existing `dns.final` and `route.final` instead of forcing known-good backend defaults.

Impact: existing v2.0.10 installs can still fail `/api/restartSb` or boot reload because of config areas users can no longer reach in the UI. This violates the backend-fixed defaults and compatibility requirements.

### Medium: Traffic graph semantics are still incorrect even if stats rows exist

The frontend sends `limit` as an hour range, but the backend treats it as a row count. The chart also does not sum all rows in a time bucket.

- `frontend/src/layouts/modals/Stats.vue:77` defines period values as hours.
- `frontend/src/layouts/modals/Stats.vue:134` sends that value as `limit`.
- `crates/domain-stats/src/lib.rs:95` and `crates/domain-stats/src/lib.rs:101` use `LIMIT ?`, not a time-window filter.
- `frontend/src/layouts/modals/Stats.vue:150` and `frontend/src/layouts/modals/Stats.vue:152` use `reduce(u => u)` / `reduce(d => d)`, which returns the first element rather than summing the bucket.

Impact: once collection is added, requested periods and displayed traffic totals will still be wrong.

## Checks Run

- `git diff --check` passed.
- `cargo fmt --check` passed.
- `npm --prefix frontend run typecheck` passed.
- `sh -n scripts/container-init.sh` passed.

## Notes

- Version bump appears aligned across `Cargo.toml`, `Cargo.lock`, `frontend/package.json`, `frontend/package-lock.json`, and README.
- Frontend route/drawer removal matches the intended hidden-page list.
- Admin account page uses authenticated `/api/account` and requires the current password.
- OpenSSH is off by default and environment-gated in `scripts/container-init.sh`; Alpine v3.22 has an `openssh-keygen` package, so the Docker package name is valid.
