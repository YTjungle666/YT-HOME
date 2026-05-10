# Codex Fix Pass: 2026-05-10

Scope: fixed only actionable findings from `.hermes/reviews/2026-05-10-codex-review.md`.

## Fixes

- Added runtime stats collection in `domain-stats` and `infra-scheduler`.
  - Uses sing-box V2Ray gRPC `StatsService.QueryStats` when the resolved sing-box binary advertises `with_v2ray_api`.
  - Computes deltas from cumulative counters, handles counter resets, inserts rows into `stats`, and increments matching client `up`/`down` counters for user traffic.
  - If the V2Ray API is not available, records only real app-level client counter deltas as a conservative fallback; it does not synthesize traffic.
  - `trafficAge <= 0` deletes stats and clears collector baselines.
- Reworked runtime config generation to ignore removed legacy runtime sections.
  - Runtime outbounds are fixed to `direct`.
  - Runtime services/endpoints are empty.
  - DNS, route, rule sets, and cache file are overwritten with backend-safe defaults.
  - Malformed hidden rows in legacy outbounds/services/endpoints no longer affect config generation.
- Fixed `/api/stats` period semantics.
  - `limit` is now treated as hours, clamped, filtered by `date_time >= now - hours * 3600`, and row-capped.
- Fixed frontend stats chart behavior.
  - Empty API results show the existing empty warning state.
  - Buckets now sum all upload/download rows in the bucket.
- Added focused tests for runtime defaults, malformed hidden rows, V2Ray stats decoding/deltas, and stats time-window filtering.

## Limits

- Official sing-box 1.13.5 release assets do not include `with_v2ray_api`; live per-user traffic therefore requires a sing-box build with that tag. Without it, the fallback only records deltas from client counters that are already changed by the app or another trusted path.
- No commit or push was performed.

## Post-review PVE Follow-up

- PVE container validation found two Alpine/OpenSSH portability issues outside the original review findings:
  - Alpine `sshd` rejected the generated `UsePAM no` option, so the option was removed.
  - Alpine OCI images keep `root` locked by default; `container-init.sh` now unlocks root without setting a password before starting sshd, while `PermitEmptyPasswords no` keeps empty-password logins denied.
- PVE Docker validation also showed OpenSSH sessions close before key exchange under the host's default nested Docker security policy; validation passes with `--privileged`, and README documents that Docker caveat. CT deployment remains the preferred PVE path.
