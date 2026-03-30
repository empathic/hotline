# hotln-proxy

[![npm](https://img.shields.io/npm/v/hotln-proxy)](https://www.npmjs.com/package/hotln-proxy)

A proxy server that forwards bug report requests to Linear and GitHub Issues.
Holds API credentials server-side so distributed clients never need direct
access to your issue tracker.

Works on Cloudflare Workers, Deno, Bun, or any platform with standard
`Request`/`Response` APIs.

## Usage

```typescript
// Cloudflare Workers / Bun
export { default } from "hotln-proxy";

// Deno
import hotline from "hotln-proxy";
Deno.serve(hotline.fetch);
```

Set environment variables however your platform does it. The handler reads
them from the passed `env` parameter (Cloudflare) or `process.env` (everywhere
else).

## Routes

| Route | Description |
|-------|-------------|
| `POST /linear` | Create a Linear issue |
| `POST /github` | Create a GitHub issue |

Any other path returns 404.

## Environment variables

Only configure the backends you want to enable. Requests to an unconfigured
backend return 500.

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
| `RATE_LIMIT_MAX` | Max requests per window per IP (default: `5`) |
| `RATE_LIMIT_WINDOW_MS` | Rate limit window in milliseconds (default: `60000`) |
| `CORS_ORIGIN` | `Access-Control-Allow-Origin` value (default: `*`) |

## Custom routing

The `hotln` SDKs ([Rust](https://crates.io/crates/hotln),
[TypeScript](https://www.npmjs.com/package/hotln)) expect the `POST /linear`
and `POST /github` routes provided by the default export. If you use the
individual handlers, make sure your router matches these paths:

```typescript
import { handleLinear, handleGitHub } from "hotln-proxy";
```

## Rate limiting

Requests are rate limited per IP (based on `cf-connecting-ip` or
`x-forwarded-for`). Defaults to 5 requests per minute. Configure with
`RATE_LIMIT_MAX` and `RATE_LIMIT_WINDOW_MS`.
