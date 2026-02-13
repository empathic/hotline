# hotline

A Rust library for filing bug reports to [Linear](https://linear.app) from
distributed applications. Supports calling the Linear API directly or going
through a proxy server that holds the API key.

## Usage

```rust
// Via proxy (recommended for distributed binaries)
hotln::proxy("https://your-worker.example.com")
    .create_issue("crash on startup", Some("details..."), &[("OS", "macos")])?;

// Direct
hotln::direct("lin_api_...", "team-id", "project-id")
    .create_issue("crash on startup", Some("details..."), &[("OS", "macos")])?;
```

## Proxy protocol

The client POSTs JSON to the proxy URL. The proxy creates the issue on Linear
and returns the URL.

### Request

```typescript
interface Request {
  title: string;        // Issue title
  description: string;  // Formatted markdown body
}
```

### Response

```typescript
interface Response {
  url: string; // URL of the created Linear issue
}
```

## Worker

A reference proxy implementation lives in `worker/`. It's a Cloudflare Worker
that holds the Linear API key as a secret, rate limits by IP, and only exposes
issue creation.

```
cd worker
npm install
wrangler secret put LINEAR_API_KEY
wrangler secret put LINEAR_TEAM_ID
wrangler secret put LINEAR_PROJECT_ID
wrangler deploy
```
