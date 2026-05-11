# Contributing to YT-HOME

This repository now ships a Rust backend workspace and a Vue 3 frontend. Contributions are expected to keep the current UI and functional behavior stable while improving structure, safety, and maintainability.

## Prerequisites

- Rust `1.88.0` with `rustfmt` and `clippy`
- Node.js `24`
- `npm`
- Go `1.24.x` (or allow `scripts/build-sing-box.sh` to select the upstream `go.mod` toolchain with `GOTOOLCHAIN`) and `git` for building the bundled `sing-box`; the default Linux build is purego/CGO-disabled. `file`, `gnupg`, `unzip`, `xz-utils`, and Python 3 with `requests` are only needed when explicitly building Linux `SING_BOX_LINUX_LIBC=musl`/`glibc` with the upstream naive/cronet CGO toolchain.
- Docker or Podman if you want to validate container builds

## Local Setup

```bash
git clone https://github.com/YTjungle666/YT-HOME
cd YT-HOME
```

Install frontend dependencies and build assets:

```bash
cd frontend
npm ci
npm run build
cd ..
```

Build the Rust backend and source-build the matching stats-capable `sing-box` runtime:

```bash
cargo build --release -p app
sh ./scripts/build-sing-box.sh linux amd64 ./target/release 1.13.11
./target/release/sing-box version | grep -F with_v2ray_api
```

`scripts/fetch-sing-box.sh` remains available only as a manual fallback for upstream release assets. Normal YT-HOME packaging must use `scripts/build-sing-box.sh`, because official `sing-box` binaries usually do not include the `with_v2ray_api` tag required for runtime traffic stats.

For a local debug run:

```bash
YTHOME_SING_BOX_BIN=./target/release/sing-box \
YTHOME_DB_FOLDER=db \
YTHOME_WEB_DIR=frontend/dist \
cargo run -p app
```

Use the explicit environment variables above for local debug runs. Do not rely on legacy helper scripts when validating new changes.

## Quality Gates

All pull requests are expected to pass the same gates enforced in CI:

```bash
cd frontend
npm ci
npm run lint -- --max-warnings=0
npm run typecheck
npm run build
npm audit --audit-level=high
cd ..

cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo audit
cargo deny check
```

## Runtime Stats Contract

YT-HOME injects `experimental.v2ray_api` only when the resolved `sing-box` binary reports `with_v2ray_api`. The default listen address is local-only `127.0.0.1:21085`; set `YTHOME_V2RAY_API_LISTEN=off`, `0`, `false`, or `disabled` to turn injection off. If you override `YTHOME_SING_BOX_BIN`, verify the binary with `sing-box version | grep -F with_v2ray_api`.

Stats store upload and download separately. Quota, used percentage, over-limit checks, and total account usage count client upload plus download only; do not add user, inbound, and outbound stats together because those are separate views of the same traffic.

## Project Structure

- `crates/app`: process startup and runtime wiring
- `crates/http-api`: Axum routes and HTTP DTOs
- `crates/domain-*`: business domains
- `crates/infra-*`: persistence, scheduling, observability
- `frontend/`: Vue 3 + TypeScript + Vuetify UI
- `scripts/`: build and runtime helper scripts

## Contribution Rules

- Keep user-visible behavior stable unless the change is explicitly discussed first.
- Preserve QR code, subscription link, and old-client compatibility behavior.
- Remove dead code and unused files as part of the change.
- Do not introduce warnings into lint, typecheck, clippy, or build output.
- Prefer small, reviewable commits and clear pull request descriptions.

## Pull Requests

When opening a PR, include:

1. What changed.
2. Why it changed.
3. Which validation commands were executed.
4. Any boundary or compatibility considerations.

If your change touches runtime behavior, configuration format, login/session behavior, or subscription output, call that out explicitly in the PR description.
