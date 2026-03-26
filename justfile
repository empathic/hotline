check:
    cargo fmt --check
    cargo clippy
    cargo test
    cd worker && npm run fmt
    cd worker && npm run check
    cd hotln-ts && npm test
    cd hotln-ts && npx biome format src/

dev:
    cd worker && npx wrangler dev

deploy:
    cd worker && npx wrangler deploy
