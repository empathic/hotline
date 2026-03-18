check:
    cargo fmt --check
    cargo clippy
    cargo test
    cd worker && npm run fmt
    cd worker && npm run check
