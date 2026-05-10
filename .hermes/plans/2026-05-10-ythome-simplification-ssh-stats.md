# YT-HOME Simplification, SSH, Admin Account, and Runtime Stats Implementation Plan

> **For Hermes:** Use project-builder orchestration and Codex no-sandbox workers for implementation, review, fix, local validation, PVE acceptance, and GitHub Actions verification.

**Goal:** Ship YT-HOME v2.0.11 with a simplified proxy-home panel, optional OpenSSH runtime access, fixed admin account management UI, backend-fixed sing-box defaults, and working online/traffic reporting where the current code supports it.

**Architecture:** Preserve the existing Rust workspace + Vue/Vuetify frontend. First remove/hide S-UI legacy surface area from the frontend while retaining the downstream mobile subscription/import flow. Move unused outbound/route/DNS/upstream-subscription knobs behind backend defaults so generated sing-box config remains valid. Add optional OpenSSH startup to the OCI/CT container init path controlled only by environment variables.

**Tech Stack:** Rust 1.88 workspace, SQLite/sqlx, axum, Vue 3 + Vuetify + Vite, Alpine 3.22 container, OpenSSH.

---

## Non-negotiable Requirements

1. Do not break phone/downstream client import/subscription/QR behavior. The removed subscription is only the panel server's own upstream-subscription/client feature.
2. First version should remove/hide useless frontend screens while keeping useful screens mostly unchanged.
3. Backend must still generate valid sing-box config with fixed defaults for outbounds, routes, DNS, rule/rule_set, and experimental/API/stats as needed.
4. Admin frontend must become a clear account-security page: show/change username, change password, logout. No multi-admin/role management.
5. SSH must use OpenSSH, not Dropbear.
6. SSH must default off and be controlled by environment variables:
   - `YTHOME_ENABLE_SSH=1` enables startup.
   - `YTHOME_SSH_PUBLIC_KEY` may contain one or more public keys separated by newlines.
   - `YTHOME_SSH_AUTHORIZED_KEYS` may point to a file with authorized keys.
   - `YTHOME_SSH_PASSWORD_LOGIN=1` allows password login; default must be key-only/password disabled.
7. Do not hard-code YT's public key or any private host/IP secret in the source.
8. Increment project version from 2.0.10 to 2.0.11 across manifests/docs/release-visible places.
9. Validate locally, then in PVE test environment, then push to GitHub and inspect GitHub Actions.
10. Keep the project-local plan current if implementation strategy changes.

## Backup Baseline

Before modifications, Hermes created:

- `/home/ytjungle/YT-HOME-backups/YT-HOME-source-before-simplification-20260510-192632.tar.gz`
- SHA256: `7c2130a2baa56a07b81f176bf5f4679005e4247cc335a0248b861bdf837e2abd`

## Phase 0: Discovery

### Task 0.1: Inspect existing frontend routes and drawer

**Objective:** Identify exact routes/menu items to remove/hide.

**Files:**
- Read: `frontend/src/router/index.ts`
- Read: `frontend/src/layouts/default/Drawer.vue`
- Read: `frontend/src/locales/zhcn.ts`

**Expected classification:**

Keep:
- Home/dashboard/runtime info
- Clients/users
- Inbounds
- Phone/downstream subscription/import/QR flows
- Login/logout

Remove or hide from navigation and direct routes:
- Outbounds
- Endpoints/nodes
- Services
- Basics/basic information
- Rules/routes
- DNS
- panel-server upstream subscription settings

Transform:
- Admins -> Account Security page for username/password changes.

### Task 0.2: Inspect backend APIs, schema, and config generation

**Objective:** Locate admin APIs, settings APIs, config generation, stats collection, and subscription conversion.

**Files:**
- Read: `crates/http-api/src/lib.rs`
- Read: `crates/domain-auth/src/lib.rs`
- Read: `crates/domain-config/src/lib.rs`
- Read: `crates/domain-stats/src/lib.rs`
- Read: `crates/shared/src/settings.rs`
- Read: `crates/infra-db/src/lib.rs`
- Read migrations under `crates/infra-db/migrations/` if present.

**Expected output:** Notes in the implementation summary or plan update naming the exact APIs used by the new admin page and the exact points where defaults are fixed.

## Phase 1: Frontend Simplification

### Task 1.1: Simplify navigation and routes

**Objective:** Remove/hide unused UI screens from the app shell and prevent direct navigation to removed pages.

**Files:**
- Modify: `frontend/src/router/index.ts`
- Modify: `frontend/src/layouts/default/Drawer.vue`
- Modify: locale files as needed.

**Implementation rules:**
- Remove lazy route entries for Outbounds, Endpoints, Services, Basics, Rules, DNS unless still needed internally.
- Remove drawer/menu entries for those pages.
- Redirect unknown/removed paths to `/` or dashboard.
- Keep downstream client import/subscription route behavior intact.

**Verification:** `npm --prefix frontend run typecheck` and `npm --prefix frontend run build` must pass.

### Task 1.2: Remove panel-server upstream subscription settings from visible UI

**Objective:** Hide/remove only the settings controlling the panel server acting as a proxy client of other servers.

**Files:**
- Modify: `frontend/src/views/Settings.vue` or related components.
- Modify: `frontend/src/types/config.ts` only if safe.

**Implementation rules:**
- Do not remove downstream mobile subscription/import settings.
- If a backend setting remains in DB for compatibility, simply stop rendering/editing it.

**Verification:** Build passes and generated settings form still saves necessary panel settings.

## Phase 2: Admin Account Security Page

### Task 2.1: Replace unclear admin page with account-security UI

**Objective:** Make `Admins.vue` a simple account page.

**Files:**
- Modify or replace: `frontend/src/views/Admins.vue`
- Modify: locale files.
- Modify: frontend API utility only if required.

**UI requirements:**
- Shows current username if API exposes it; otherwise show editable current/new username field with clear helper text.
- Allows changing username.
- Allows changing password with confirmation.
- Requires current password if backend already supports/requires it; otherwise use backend's existing command/API semantics.
- Has logout action or links to existing logout.

### Task 2.2: Add or wire backend account API if missing

**Objective:** Use existing backend account logic where possible, adding minimal HTTP wrappers if necessary.

**Files:**
- Modify: `crates/http-api/src/lib.rs`
- Modify: `crates/domain-auth/src/lib.rs`
- Possibly modify shared DTO/model files.

**Rules:**
- Reuse existing password hashing and admin update command logic.
- Never return password hashes.
- Validate non-empty username and password length.
- Require authenticated session.

**Verification:** Rust tests/checks pass; manual API smoke verifies update works.

## Phase 3: Backend-fixed sing-box Defaults

### Task 3.1: Fix outbounds/routes/DNS/defaults in backend generation

**Objective:** Remove frontend dependency for unused config areas while keeping valid sing-box output.

**Files:**
- Modify: `crates/domain-config/src/lib.rs`
- Modify: `crates/shared/src/settings.rs`
- Modify tests near config generation.

**Rules:**
- Ensure generated config has a usable default outbound, route, DNS, and experimental/API/stats fields if needed by runtime stats.
- Preserve inbound/user/client generation.
- Do not expose removed UI fields as required inputs.
- Keep compatibility with existing DB rows: ignore or override obsolete rows rather than crashing.

**Verification:** Add/update tests proving generated config contains default outbound/route/DNS/stats and still includes users/inbounds.

### Task 3.2: Disable panel-server upstream subscription path by default

**Objective:** The panel server should not expect upstream proxy subscriptions for itself.

**Files:**
- Modify: config/settings and generation/conversion only where needed.

**Rules:**
- Downstream mobile subscription/import must remain.
- Existing DB keys may remain but should not be surfaced in UI or required by generation.

## Phase 4: Online Users and Traffic Stats

### Task 4.1: Trace current stats pipeline

**Objective:** Identify why runtime info online users and user management traffic are blank.

**Files:**
- Read/modify: `crates/domain-stats/src/lib.rs`
- Read/modify: `crates/infra-scheduler/src/lib.rs`
- Read/modify: `crates/http-api/src/lib.rs`
- Read/modify: frontend dashboard/users views.

**Rules:**
- Prefer real sing-box stats/connection data if available.
- If connection-level online data is unavailable, implement safe fallback: recent traffic delta within a window marks a user online.
- Map sing-box counters to panel users via stable user identifier/email/name/tag.
- Store deltas robustly across sing-box restarts/counter resets.

**Verification:** Unit tests for counter delta/reset logic and API shape; runtime smoke in PVE after deployment.

### Task 4.2: Frontend display binding

**Objective:** Ensure dashboard and user management display API-provided online/traffic fields.

**Files:**
- Modify: `frontend/src/views/Home.vue`
- Modify: `frontend/src/views/Clients.vue`
- Modify types/store modules as needed.

**Verification:** Build/typecheck and browser/PVE smoke should show fields populated or clear zero/empty state instead of broken blank UI.

## Phase 5: Optional OpenSSH Runtime

### Task 5.1: Add OpenSSH packages to final Alpine image

**Objective:** Make sshd available without enabling it by default.

**Files:**
- Modify: `Dockerfile`

**Rules:**
- Install `openssh-server` and `openssh-keygen`.
- Expose port 22 only if consistent with Docker/CT deployment. It is acceptable to add `EXPOSE 22` while keeping runtime disabled by default.

### Task 5.2: Add environment-controlled sshd startup to container init

**Objective:** Start OpenSSH only when requested.

**Files:**
- Modify: `scripts/container-init.sh`

**Startup semantics:**
- If `YTHOME_ENABLE_SSH` is not `1`, do nothing.
- If enabled, generate host keys with `ssh-keygen -A`.
- Build `/root/.ssh/authorized_keys` from `YTHOME_SSH_PUBLIC_KEY` and/or `YTHOME_SSH_AUTHORIZED_KEYS`.
- Set root `.ssh` permissions.
- Write an sshd config with:
  - `PubkeyAuthentication yes`
  - `AuthorizedKeysFile .ssh/authorized_keys`
  - `PermitRootLogin prohibit-password` by default
  - `PasswordAuthentication no` by default
  - if `YTHOME_SSH_PASSWORD_LOGIN=1`, allow password auth and `PermitRootLogin yes` only if required by OpenSSH semantics.
- Validate with `/usr/sbin/sshd -t`.
- Start `/usr/sbin/sshd` once; do not supervise it with an extra long-lived wrapper.

**Verification:** `sh -n scripts/container-init.sh`; container smoke with SSH disabled and enabled.

## Phase 6: Versioning and Documentation

### Task 6.1: Bump version to 2.0.11

**Objective:** Keep release-visible versions aligned.

**Files:**
- Modify: `Cargo.toml`
- Modify: `frontend/package.json`
- Modify: `frontend/package-lock.json` if it contains project version.
- Modify docs/readme/release notes if version is mentioned.

### Task 6.2: Update docs for simplified UI and SSH env vars

**Objective:** Operators can discover the new SSH env vars and simplified-scope behavior.

**Files:**
- Modify: `README.md` or deployment docs.

**Required docs:**
- SSH disabled by default.
- Env vars: `YTHOME_ENABLE_SSH`, `YTHOME_SSH_PUBLIC_KEY`, `YTHOME_SSH_AUTHORIZED_KEYS`, `YTHOME_SSH_PASSWORD_LOGIN`.
- Password login default disabled.
- Phone/downstream subscription remains.
- Removed UI pages are intentionally backend-fixed defaults.

## Phase 7: Validation

### Task 7.1: Local validation

Run from repo root:

```bash
cargo fmt --check
cargo check --locked
cargo test --locked
cargo clippy --locked -- -D warnings
npm --prefix frontend run lint
npm --prefix frontend run typecheck
npm --prefix frontend run build
sh -n scripts/container-init.sh
```

If a command is too slow or unavailable, record the reason and run the strongest practical substitute.

### Task 7.2: Independent review and fix pass

Use Codex no-sandbox review pass to compare final diff against this plan. Then run a separate fix pass scoped to review findings and validation failures. Re-run validation after fixes.

### Task 7.3: PVE acceptance

Use PVE validation host `root@10.10.10.200` in a task-owned temporary directory. Copy source excluding `.git`, `target`, `node_modules`, and build artifacts. Validate at least:

- Docker image build succeeds or the available project build path succeeds.
- Container starts with SSH disabled; port 22 not listening inside container.
- Container starts with `YTHOME_ENABLE_SSH=1` and a test public key; sshd listens and config validates.
- If feasible, API/web frontend smoke responds.
- Cleanup all PVE validation resources after acceptance.

## Phase 8: GitHub Delivery

### Task 8.1: Commit and push

**Rules:**
- Review `git status --short` and `git diff` before commit.
- Ensure no backup archives, generated artifacts, secrets, local IP-specific credentials, or temp validation files are staged.
- Commit with clear message, push `main` to origin.
- Tag v2.0.11 only if release workflow/tag practice in repo supports tags for releases.

### Task 8.2: Actions verification

Use `gh` to inspect GitHub Actions for the pushed branch/tag. Fix actionable failures, push fixes, and re-check until clean or blocked by external infrastructure.

## Current Handoff Notes

- Baseline backup exists at `/home/ytjungle/YT-HOME-backups/YT-HOME-source-before-simplification-20260510-192632.tar.gz` with SHA256 `7c2130a2baa56a07b81f176bf5f4679005e4247cc335a0248b861bdf837e2abd`.
- Implementation pass and Codex fix pass completed in the main worktree.
- Fix pass addressed `.hermes/reviews/2026-05-10-codex-review.md` findings:
  - Runtime config generation now overwrites removed UI areas with backend-fixed DNS, route, direct outbound, empty services/endpoints, empty rule sets, and cache file defaults. Malformed hidden outbound/service/endpoint rows are ignored for runtime generation.
  - Runtime stats collection now runs from `infra-scheduler`: it uses sing-box V2Ray gRPC stats when the resolved binary advertises `with_v2ray_api`, and otherwise falls back to recording real app client-counter deltas only. Official sing-box 1.13.5 release assets do not include `with_v2ray_api`, so per-client live traffic requires a custom sing-box build with that tag.
  - `trafficAge <= 0` deletes stats and clears collector baselines; positive values prune old rows by age.
  - `/api/stats` now treats `limit` as hours, clamps the time window, and applies a safe row cap.
  - The frontend stats graph now reports empty states for empty data and sums all rows inside each bucket.
- Focused tests added for backend runtime defaults, malformed hidden rows, V2Ray stats protobuf/delta handling, and stats hour-window filtering.
- Local validation completed after PVE-discovered SSH portability fixes:
  - `cargo fmt --check`
  - `cargo check --locked`
  - `cargo test --locked`
  - `cargo clippy --locked -- -D warnings`
  - `npm --prefix frontend run typecheck`
  - `npm --prefix frontend run build`
  - `sh -n scripts/container-init.sh`
  - `git diff --check`
- PVE validation passed on `root@10.10.10.200` with temporary stamp `20260510205024` and all validation resources cleaned:
  - Docker image build completed using the prebuilt backend binary path.
  - Default container served web and kept SSH disabled (`sshd` not running, tcp/22 not listening inside).
  - `YTHOME_ENABLE_SSH=1` plus a test public key allowed root key login; generated sshd config validated with `PasswordAuthentication no`, `PermitRootLogin prohibit-password`, `PubkeyAuthentication yes`, and `PermitEmptyPasswords no`.
  - `YTHOME_SSH_PASSWORD_LOGIN=1` toggled sshd config to `PasswordAuthentication yes`, `PermitRootLogin yes`, `KbdInteractiveAuthentication yes`, while still keeping `PermitEmptyPasswords no`.
  - On this PVE Docker environment, OpenSSH sessions require `--privileged` because the default nested/container security policy closes the connection before key exchange; README now documents this Docker-only caveat and CT deployment remains the preferred path for that environment.
  - Remote containers, temporary source directories, validation scripts, validation images, and Docker build cache were removed after acceptance.
- PVE follow-up fixes after the Codex fix pass:
  - Removed unsupported Alpine `UsePAM` sshd option.
  - Unlocked Alpine's default locked root entry without setting a password so public-key auth works; empty-password login remains denied.
  - Dockerfile now skips `rustup target add` when a prebuilt `/app/packaging/docker/YTHOME` binary is supplied for validation builds.
- GitHub CI initially failed on `npm audit --audit-level=high` because axios 1.15.0/1.15.1 is now covered by high-severity advisories. The frontend axios dependency and lock entry were intentionally upgraded to 1.16.0; local `npm --prefix frontend audit --audit-level=high` reports zero vulnerabilities.
- Cargo.lock records direct local-package dependency edges for already locked `reqwest`/`tokio` use, with no crate version upgrades.
- Fix-pass summary is recorded in `.hermes/reviews/2026-05-10-codex-fix-pass.md`.
- Next phase: push the CI fix, move the just-created `v2.0.11` tag to the fixed commit, and re-check GitHub Actions.

## Acceptance Criteria

- Backup archive exists and checksum recorded.
- Frontend no longer shows useless S-UI pages.
- Downstream phone/client import/subscription remains accessible.
- Admin page supports username/password change through backend.
- Generated sing-box config does not depend on removed frontend management pages.
- OpenSSH is available but disabled by default; env vars enable it; password login env var works as designed.
- Dashboard/user pages no longer have broken blank online/traffic UI; they show real data or robust zero/empty states.
- Local validation passes.
- Codex review/fix pass completed.
- PVE acceptance passes and resources are cleaned.
- GitHub push succeeds and Actions are checked.
