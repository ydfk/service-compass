$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm -C frontend install --frozen-lockfile
pnpm -C frontend lint
pnpm -C frontend format:check
pnpm -C frontend build
