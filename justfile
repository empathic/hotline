check:
    cargo fmt
    cargo clippy
    cargo test
    cd hotln-proxy && npm run fmt
    cd hotln-proxy && npm run check
    cd hotln-ts && npm run fmt
    cd hotln-ts && npm run check
    cd hotln-ts && npm test

dev:
    cd hotln-proxy && npx wrangler dev

deploy:
    cd hotln-proxy && npx wrangler deploy
