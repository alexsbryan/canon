#!/bin/sh
# canon installer.
#
#   curl -fsSL https://raw.githubusercontent.com/alexsbryan/canon/main/install.sh | sh
#
# Works out your platform, downloads the matching prebuilt `canon` from the
# newest GitHub Release, checks it against the release's SHA256SUMS, and
# puts it in ~/.local/bin. One binary, about 5 MB, no dependencies.
#
# Read it first if you like; it is short. Overrides, as environment
# variables:
#
#   CANON_VERSION      a release, e.g. v0.1.0 (default: latest)
#   CANON_INSTALL_DIR  where the binary goes  (default: ~/.local/bin)
#   CANON_REPO         owner/name to fetch from (default: alexsbryan/canon)
#
# No Rust required. If you'd rather build from source, GETTING_STARTED.md
# in the repository says how.

set -eu

REPO="${CANON_REPO:-alexsbryan/canon}"
INSTALL_DIR="${CANON_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${CANON_VERSION:-latest}"

say() { printf '  %s\n' "$1"; }
err() { printf 'install: %s\n' "$1" >&2; exit 1; }

command -v curl >/dev/null 2>&1 || err "curl is required"
command -v tar  >/dev/null 2>&1 || err "tar is required"

# ── Platform → release target ─────────────────────────────────────────────
os="$(uname -s)"
arch="$(uname -m)"
case "$os" in
  Darwin)
    # A shell running under Rosetta reports x86_64 on an Apple Silicon
    # machine. Ask the kernel, and install the native binary.
    if [ "$arch" = "x86_64" ] && [ "$(sysctl -n sysctl.proc_translated 2>/dev/null || echo 0)" = "1" ]; then
      arch="arm64"
    fi
    case "$arch" in
      arm64 | aarch64) target="aarch64-apple-darwin" ;;
      x86_64)          target="x86_64-apple-darwin" ;;
      *) err "no prebuilt canon for macOS/$arch; build from source: https://github.com/$REPO" ;;
    esac ;;
  Linux)
    # Static musl builds: no glibc version to get wrong.
    case "$arch" in
      x86_64 | amd64)  target="x86_64-unknown-linux-musl" ;;
      aarch64 | arm64) target="aarch64-unknown-linux-musl" ;;
      *) err "no prebuilt canon for Linux/$arch; build from source: https://github.com/$REPO" ;;
    esac ;;
  MINGW* | MSYS* | CYGWIN*)
    err "on Windows, download canon-x86_64-pc-windows-msvc.zip from https://github.com/$REPO/releases/latest" ;;
  *)
    err "unsupported OS '$os'; build from source: https://github.com/$REPO" ;;
esac

# ── Where to fetch from ───────────────────────────────────────────────────
# "latest" goes through GitHub's own redirect, so there is no API call to
# rate-limit and no version list to sort. A pinned version is accepted with
# or without its leading v.
if [ "$VERSION" = "latest" ]; then
  base="https://github.com/$REPO/releases/latest/download"
else
  case "$VERSION" in v*) ;; *) VERSION="v$VERSION" ;; esac
  base="https://github.com/$REPO/releases/download/$VERSION"
fi
archive="canon-$target.tar.gz"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

say "downloading $archive ($VERSION)"
curl -fsSL "$base/$archive" -o "$tmp/$archive" 2>/dev/null \
  || err "download failed: $base/$archive (no release for $target yet?)"

# ── Verify against the release's SHA256SUMS ───────────────────────────────
# Every release carries one, so its absence is an error, not a shrug.
curl -fsSL "$base/SHA256SUMS" -o "$tmp/SHA256SUMS" \
  || err "download failed: $base/SHA256SUMS"
want="$(grep " $archive\$" "$tmp/SHA256SUMS" | awk '{print $1}' | head -n1)"
[ -n "$want" ] || err "SHA256SUMS has no entry for $archive"
if command -v sha256sum >/dev/null 2>&1; then
  got="$(sha256sum "$tmp/$archive" | awk '{print $1}')"
elif command -v shasum >/dev/null 2>&1; then
  got="$(shasum -a 256 "$tmp/$archive" | awk '{print $1}')"
else
  err "neither sha256sum nor shasum found; cannot verify the download"
fi
[ "$got" = "$want" ] || err "checksum mismatch for $archive (expected $want, got $got)"
say "checksum ok"

# ── Install ───────────────────────────────────────────────────────────────
tar -xzf "$tmp/$archive" -C "$tmp"
[ -f "$tmp/canon" ] || err "unexpected archive layout: no canon binary in $archive"

mkdir -p "$INSTALL_DIR"
if command -v install >/dev/null 2>&1; then
  install -m 0755 "$tmp/canon" "$INSTALL_DIR/canon"
else
  cp "$tmp/canon" "$INSTALL_DIR/canon" && chmod 0755 "$INSTALL_DIR/canon"
fi

say "installed $("$INSTALL_DIR/canon" --version) → $INSTALL_DIR/canon"

# ── PATH hint and the next step ───────────────────────────────────────────
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *)
    printf '\n  %s is not on your PATH. Add this to your shell profile:\n' "$INSTALL_DIR"
    printf '    export PATH="%s:$PATH"\n' "$INSTALL_DIR" ;;
esac

printf '\n  next:  canon init --profile house\n'
printf '         https://github.com/%s/blob/main/GETTING_STARTED.md\n\n' "$REPO"
