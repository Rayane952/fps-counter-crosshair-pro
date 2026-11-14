# Rust — squelette de migration

État : **structure uniquement**, aucun module fonctionnel (voir ../MIGRATION.md).

## Prérequis

1. `rustup` (toolchain stable MSVC) — https://rustup.rs
2. Visual Studio Build Tools 2022 (C++ Desktop) pour `link.exe`
3. `cargo build --release`

## Build

```powershell
cd rust
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo run -p ovl-app
```

Cible : `target/release/overlay.exe` — binaire unique, < 10 Mo.
