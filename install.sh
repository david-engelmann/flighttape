#!/bin/sh
# Install prebuilt flighttape binary from GitHub Releases with SHA-256 verification.
#
# Environment variables:
#   FLIGHTTAPE_INSTALL_DIR  Target directory (default: $HOME/.local/bin)
#   FLIGHTTAPE_VERSION      Release tag such as v1.0.0 (default: latest)
#   FLIGHTTAPE_REPO         GitHub owner/repo (default: david-engelmann/flighttape)
#   FLIGHTTAPE_BASE_URL     Override release download base URL (for offline mirrors/testing)

set -eu

REPO="${FLIGHTTAPE_REPO:-david-engelmann/flighttape}"
INSTALL_DIR="${FLIGHTTAPE_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${FLIGHTTAPE_VERSION:-latest}"

err() {
    printf 'flighttape-install: error: %s\n' "$1" >&2
    exit 1
}

need_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        err "required command not found: $1"
    fi
}

detect_target() {
    os_name="$(uname -s 2>/dev/null || echo unknown)"
    arch_name="$(uname -m 2>/dev/null || echo unknown)"

    case "$os_name" in
        Linux)
            os_part="unknown-linux-gnu"
            ;;
        Darwin)
            os_part="apple-darwin"
            ;;
        *)
            err "unsupported operating system: $os_name (use prebuilt Windows zip or cargo install)"
            ;;
    esac

    case "$arch_name" in
        x86_64|amd64)
            arch_part="x86_64"
            ;;
        aarch64|arm64)
            arch_part="aarch64"
            ;;
        *)
            err "unsupported CPU architecture: $arch_name"
            ;;
    esac

    printf '%s-%s' "$arch_part" "$os_part"
}

download_file() {
    url="$1"
    dest="$2"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$url" -o "$dest"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$dest" "$url"
    else
        err "either curl or wget is required to download release archives"
    fi
}

compute_sha256() {
    file="$1"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$file" | awk '{print tolower($1)}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$file" | awk '{print tolower($1)}'
    else
        err "either sha256sum or shasum is required to verify SHA256SUMS"
    fi
}

need_cmd uname
need_cmd tar
need_cmd awk
need_cmd mktemp

TARGET="$(detect_target)"
ARCHIVE="flighttape-${TARGET}.tar.gz"

if [ -n "${FLIGHTTAPE_BASE_URL:-}" ]; then
    BASE_URL="${FLIGHTTAPE_BASE_URL%/}"
elif [ "$VERSION" = "latest" ]; then
    BASE_URL="https://github.com/${REPO}/releases/latest/download"
else
    case "$VERSION" in
        v*) TAG="$VERSION" ;;
        *)  TAG="v$VERSION" ;;
    esac
    BASE_URL="https://github.com/${REPO}/releases/download/${TAG}"
fi

TMP_DIR="$(mktemp -d 2>/dev/null || mktemp -d -t flighttape-install)"
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

printf 'Downloading %s/%s\n' "$BASE_URL" "$ARCHIVE"
download_file "${BASE_URL}/${ARCHIVE}" "${TMP_DIR}/${ARCHIVE}"
download_file "${BASE_URL}/SHA256SUMS" "${TMP_DIR}/SHA256SUMS"

EXPECTED_SHA="$(awk -v name="$ARCHIVE" '
    NF >= 2 {
        file = $2
        sub(/^\*/, "", file)
        sub(/^\.\//, "", file)
        if (file == name) {
            print tolower($1)
            exit
        }
    }
' "${TMP_DIR}/SHA256SUMS")"

if [ -z "$EXPECTED_SHA" ]; then
    err "archive ${ARCHIVE} not listed in SHA256SUMS"
fi

ACTUAL_SHA="$(compute_sha256 "${TMP_DIR}/${ARCHIVE}")"
if [ "$ACTUAL_SHA" != "$EXPECTED_SHA" ]; then
    err "SHA-256 mismatch for ${ARCHIVE}: expected ${EXPECTED_SHA}, got ${ACTUAL_SHA}"
fi

mkdir -p "${TMP_DIR}/unpack"
tar -xzf "${TMP_DIR}/${ARCHIVE}" -C "${TMP_DIR}/unpack"

if [ -f "${TMP_DIR}/unpack/flighttape" ]; then
    BIN_SRC="${TMP_DIR}/unpack/flighttape"
elif [ -f "${TMP_DIR}/unpack/flighttape-${TARGET}/flighttape" ]; then
    BIN_SRC="${TMP_DIR}/unpack/flighttape-${TARGET}/flighttape"
else
    err "flighttape binary not found inside ${ARCHIVE}"
fi

mkdir -p "$INSTALL_DIR"
cp "$BIN_SRC" "${INSTALL_DIR}/flighttape"
chmod 0755 "${INSTALL_DIR}/flighttape"

printf 'Verified SHA-256 (%s) and installed flighttape to %s/flighttape\n' "$ACTUAL_SHA" "$INSTALL_DIR"

case ":${PATH:-}:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
        printf 'Note: add %s to your PATH if it is not already present.\n' "$INSTALL_DIR"
        ;;
esac
