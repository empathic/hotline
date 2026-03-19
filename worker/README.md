# hotline proxy worker

A Cloudflare Worker that proxies bug report requests to Linear and GitHub
Issues. Holds API credentials server-side so distributed clients never need
direct access to your issue tracker.

## Routes

| Route | Description |
|-------|-------------|
| `POST /linear` | Create a Linear issue |
| `POST /github` | Create a GitHub issue |

Any other path returns 404.

## Setup

```sh
npm install
```

Only configure the secrets for the backends you want to enable. Requests to an
unconfigured backend return 500.

## Environment variables

### GitHub

Two auth modes. If both are configured, the GitHub App is used.

**GitHub App (recommended):**

| Variable | Description |
|----------|-------------|
| `GITHUB_APP_ID` | GitHub App numeric ID |
| `GITHUB_APP_PRIVATE_KEY` | GitHub App private key (PKCS#8 PEM) |
| `GITHUB_INSTALLATION_ID` | Installation ID from installing the app on your repo |
| `GITHUB_REPO` | Repo in `owner/repo` format |

GitHub generates PKCS#1 keys by default. Convert to PKCS#8 first:
```sh
openssl pkcs8 -topk8 -inform PEM -outform PEM -nocrypt -in private-key.pem -out private-key-pkcs8.pem
```

**Personal access token:**

| Variable | Description |
|----------|-------------|
| `GITHUB_TOKEN` | GitHub personal access token |
| `GITHUB_REPO` | Repo in `owner/repo` format |

### Linear

| Variable | Description |
|----------|-------------|
| `LINEAR_API_KEY` | Linear API key |
| `LINEAR_TEAM_ID` | Linear team ID |
| `LINEAR_PROJECT_ID` | Linear project ID |

### Shared

| Variable | Description |
|----------|-------------|
| `HOTLINE_PROXY_TOKEN` | When set, requires `Authorization: Bearer <token>` on all requests |

## Development

```sh
just dev
```

## Deploy

```sh
just deploy
npx wrangler secret put <var>
```

## Rate limiting

Requests are rate limited to 5 per minute per IP (based on `cf-connecting-ip`).
