#!/bin/sh

set -eu

normalize_os() {
  case "$1" in
    linux|Linux) printf '%s\n' "linux" ;;
    darwin|Darwin) printf '%s\n' "darwin" ;;
    android|Android) printf '%s\n' "android" ;;
    windows|Windows|mingw*|msys*|cygwin*) printf '%s\n' "windows" ;;
    *)
      printf 'unsupported sing-box os: %s\n' "$1" >&2
      exit 1
      ;;
  esac
}

normalize_arch() {
  case "$1" in
    amd64|x86_64) printf '%s\n' "amd64:" ;;
    386|i386|i686) printf '%s\n' "386:" ;;
    arm64|aarch64) printf '%s\n' "arm64:" ;;
    armv7|armv7l|arm/v7) printf '%s\n' "arm:7" ;;
    *)
      printf 'unsupported sing-box arch: %s\n' "$1" >&2
      exit 1
      ;;
  esac
}

need_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf '%s is required to build sing-box from source\n' "$1" >&2
    exit 1
  fi
}

append_tag() {
  case " $TAGS " in
    *" $1 "*) ;;
    *) TAGS="${TAGS} $1" ;;
  esac
}

truthy_not_disabled() {
  case "${1:-}" in
    0|off|false|disabled|no) return 1 ;;
    *) return 0 ;;
  esac
}

prepare_linux_naive_toolchain() {
  if ! truthy_not_disabled "${SING_BOX_PREPARE_NAIVE_TOOLCHAIN:-1}"; then
    return 0
  fi

  need_command bash

  CRONET_VERSION=""
  if [ -f "$TMP_DIR/sing-box/.github/CRONET_GO_VERSION" ]; then
    CRONET_VERSION="$(tr -d '\r\n' < "$TMP_DIR/sing-box/.github/CRONET_GO_VERSION")"
  fi
  if [ -z "$CRONET_VERSION" ]; then
    printf 'missing upstream cronet-go version for Linux naive outbound build\n' >&2
    exit 1
  fi

  CRONET_DIR="$TMP_DIR/cronet-go"
  printf 'preparing cronet-go toolchain %s for linux/%s (%s)\n' "$CRONET_VERSION" "$GOARCH" "$LINUX_LIBC"
  git init "$CRONET_DIR" >/dev/null
  git -C "$CRONET_DIR" remote add origin https://github.com/sagernet/cronet-go.git
  git -C "$CRONET_DIR" fetch --depth=1 origin "$CRONET_VERSION"
  git -C "$CRONET_DIR" checkout -q FETCH_HEAD
  git -C "$CRONET_DIR" submodule update --init --recursive --depth=1 naiveproxy

  if [ -x "$CRONET_DIR/naiveproxy/src/build/linux/sysroot_scripts/generate_keyring.sh" ]; then
    rm -f "$CRONET_DIR/naiveproxy/src/build/linux/sysroot_scripts/keyring.gpg"
    (cd "$CRONET_DIR" && GPG_TTY=/dev/null ./naiveproxy/src/build/linux/sysroot_scripts/generate_keyring.sh)
  fi

  (
    cd "$CRONET_DIR"
    go run ./cmd/build-naive --target="linux/$GOARCH" --libc="$LINUX_LIBC" download-toolchain
  )

  # build-naive emits POSIX-compatible export lines with quoting for CC/CXX.
  eval "$(
    cd "$CRONET_DIR"
    go run ./cmd/build-naive --target="linux/$GOARCH" --libc="$LINUX_LIBC" env --export
  )"
  export CC CXX CGO_LDFLAGS QEMU_LD_PREFIX
}

OS_INPUT="${1:-$(uname -s)}"
ARCH_INPUT="${2:-$(uname -m)}"
OUTPUT_DIR="${3:-.}"
VERSION="${4:-${SING_BOX_VERSION:-1.13.11}}"

OS="$(normalize_os "$OS_INPUT")"
ARCH_PAIR="$(normalize_arch "$ARCH_INPUT")"
GOARCH="${ARCH_PAIR%%:*}"
GOARM="${ARCH_PAIR#*:}"
[ "$GOARM" = "$GOARCH" ] && GOARM=""

case "$OS" in
  windows) BINARY_NAME="sing-box.exe" ;;
  *) BINARY_NAME="sing-box" ;;
esac

need_command git
need_command go

TMP_DIR="$(mktemp -d)"

cleanup() {
  chmod -R u+w "$TMP_DIR" 2>/dev/null || true
  rm -rf "$TMP_DIR"
}

trap cleanup EXIT INT TERM

mkdir -p "$OUTPUT_DIR"
OUTPUT_DIR="$(cd "$OUTPUT_DIR" && pwd)"
OUTPUT_PATH="$OUTPUT_DIR/$BINARY_NAME"

printf 'building sing-box v%s for %s/%s\n' "$VERSION" "$OS" "$GOARCH"
git clone --depth 1 --branch "v${VERSION}" https://github.com/SagerNet/sing-box "$TMP_DIR/sing-box"

if [ -z "${GOTOOLCHAIN:-}" ]; then
  GO_MOD_VERSION=""
  while read -r key value _; do
    if [ "$key" = "go" ]; then
      GO_MOD_VERSION="$value"
      break
    fi
  done < "$TMP_DIR/sing-box/go.mod"
  if [ -n "$GO_MOD_VERSION" ]; then
    # Newer Go toolchains can break sing-box's badlinkname internals before
    # upstream adapts; default to the upstream go.mod toolchain unless callers
    # explicitly provide GOTOOLCHAIN.
    export GOTOOLCHAIN="go${GO_MOD_VERSION}"
  fi
fi

# Default to purego/CGO-disabled Linux builds so CI, Docker source builds, and
# constrained nested-PVE validation do not need the large cronet/naive CGO
# toolchain. Set SING_BOX_LINUX_LIBC=musl or glibc explicitly when that
# toolchain is required.
LINUX_LIBC="${SING_BOX_LINUX_LIBC:-purego}"
EXTRA_TAGS=""
CGO_DEFAULT="0"
case "$OS" in
  linux)
    TAG_FILE="release/DEFAULT_BUILD_TAGS"
    case "$LINUX_LIBC" in
      musl)
        EXTRA_TAGS="with_musl"
        CGO_DEFAULT="1"
        ;;
      glibc)
        CGO_DEFAULT="1"
        ;;
      purego)
        EXTRA_TAGS="with_purego"
        CGO_DEFAULT="0"
        ;;
      *)
        printf 'unsupported SING_BOX_LINUX_LIBC: %s (expected musl, glibc, or purego)\n' "$LINUX_LIBC" >&2
        exit 1
        ;;
    esac
    ;;
  darwin|android)
    TAG_FILE="release/DEFAULT_BUILD_TAGS"
    CGO_DEFAULT="1"
    ;;
  windows)
    TAG_FILE="release/DEFAULT_BUILD_TAGS_WINDOWS"
    CGO_DEFAULT="0"
    ;;
  *)
    TAG_FILE="release/DEFAULT_BUILD_TAGS_OTHERS"
    CGO_DEFAULT="0"
    ;;
esac

if [ ! -f "$TMP_DIR/sing-box/$TAG_FILE" ]; then
  printf 'missing upstream build tag file: %s\n' "$TAG_FILE" >&2
  exit 1
fi

TAGS="$(tr ',\n' '  ' < "$TMP_DIR/sing-box/$TAG_FILE" | sed 's/[[:space:]]\+/ /g; s/^ //; s/ $//')"
for tag in $EXTRA_TAGS; do
  append_tag "$tag"
done
append_tag with_v2ray_api

export CGO_ENABLED="${CGO_ENABLED:-$CGO_DEFAULT}"
if [ -n "$GOARM" ]; then
  export GOARM="$GOARM"
fi

case " $TAGS " in
  *" with_naive_outbound "*) HAS_NAIVE=1 ;;
  *) HAS_NAIVE=0 ;;
esac

if [ "$OS" = "linux" ] && [ "$LINUX_LIBC" != "purego" ] && [ "$HAS_NAIVE" = "1" ] && [ "$CGO_ENABLED" = "1" ]; then
  prepare_linux_naive_toolchain
fi

LDFLAGS_SHARED=""
if [ -f "$TMP_DIR/sing-box/release/LDFLAGS" ]; then
  LDFLAGS_SHARED="$(tr '\n' ' ' < "$TMP_DIR/sing-box/release/LDFLAGS" | sed 's/[[:space:]]\+/ /g; s/^ //; s/ $//')"
fi

(
  cd "$TMP_DIR/sing-box"
  export GOOS="$OS"
  export GOARCH="$GOARCH"
  go build -trimpath -ldflags "-X github.com/sagernet/sing-box/constant.Version=${VERSION} ${LDFLAGS_SHARED} -s -w -buildid=" -tags "$TAGS" -o "$OUTPUT_PATH" ./cmd/sing-box
)

chmod +x "$OUTPUT_PATH"
if ! "$OUTPUT_PATH" version 2>&1 | grep -q 'with_v2ray_api'; then
  "$OUTPUT_PATH" version >&2 || true
  printf 'built sing-box binary does not report with_v2ray_api\n' >&2
  exit 1
fi

printf 'built %s with with_v2ray_api\n' "$OUTPUT_PATH"
