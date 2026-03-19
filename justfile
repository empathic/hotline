check:
    cargo fmt --check
    cargo clippy
    cargo test
    cd worker && npm run fmt
    cd worker && npm run check

dev:
    cd worker && npx wrangler dev

deploy:
    cd worker && npx wrangler deploy
