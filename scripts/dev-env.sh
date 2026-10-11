#!/usr/bin/env bash
# Local Rust build environment for the Windows `x86_64-pc-windows-gnu` toolchain
# WITHOUT installing an external MinGW or MSVC toolchain.
#
# Background: rustup's bundled `rust-mingw` ships a self-contained linker
# (gcc/ld) but its `dlltool` needs the GNU assembler `as`, which is not bundled,
# so building crates that use `raw-dylib` (e.g. `windows-sys` via Tokio) fails
# with "dlltool: CreateProcess". LLVM's `llvm-dlltool` generates import
# libraries natively without `as`, and the bundled `llvm-ar` acts as
# `llvm-dlltool` when invoked under that name. We point rustc at it via
# `-Cdlltool` and put the self-contained linker on PATH.
#
# Usage (from the repo root):
#   source scripts/dev-env.sh
#   cargo build        # or test / run
#
# This is a local developer convenience. CI and reproducible builds should use
# a real MSVC toolchain (preferred on Windows) or a full MinGW-w64.

set -u

_sr="$(rustc --print sysroot)"
_sc="$_sr/lib/rustlib/x86_64-pc-windows-gnu/bin/self-contained"
_llvmbin="$_sr/lib/rustlib/x86_64-pc-windows-gnu/bin"

if [ ! -x "$_sc/x86_64-w64-mingw32-gcc.exe" ]; then
  echo "dev-env: rust-mingw self-contained toolchain not found; run: rustup component add rust-mingw" >&2
  return 1 2>/dev/null || exit 1
fi

_shimdir="$(pwd)/target/dev-bin"
mkdir -p "$_shimdir"
cp -f "$_llvmbin/llvm-ar.exe" "$_shimdir/llvm-dlltool.exe"

export PATH="$_sc:$PATH"
# Use CARGO_ENCODED_RUSTFLAGS (split only on \x1f) so a dlltool path containing
# spaces — e.g. a repo under "Projet Claude Code" — is passed as one argument.
export CARGO_ENCODED_RUSTFLAGS="-Cdlltool=$_shimdir/llvm-dlltool.exe"
# Keep the debug `target/` dir small (line-table backtraces, no full debuginfo).
export CARGO_PROFILE_DEV_DEBUG="${CARGO_PROFILE_DEV_DEBUG:-line-tables-only}"

echo "dev-env: windows-gnu linker ready (dlltool=llvm-dlltool, self-contained on PATH)"
