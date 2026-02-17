# hotline

A Rust library for filing bug reports to [Linear](https://linear.app) from
distributed applications. Supports calling the Linear API directly or going
through a proxy server that holds the API key.

## Usage

```rust
// Via proxy (recommended for distributed binaries)
hotln::proxy("https://your-worker.example.com")
    .with_token("your-proxy-token")
    .create_issue("crash on startup", Some("details..."), &[("OS", "macos")])?;

// Direct
hotln::direct("lin_api_...", "team-id", "project-id")
    .create_issue("crash on startup", Some("details..."), &[("OS", "macos")])?;
```

## Proxy protocol

The client POSTs JSON to the proxy URL. The proxy creates the issue on Linear
and returns the URL. Any server that implements this protocol can be used as a
proxy. If `with_token` is set, the client sends an `Authorization: Bearer <token>`
header.

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

## Cloudflare worker

A proxy implementation lives in `worker/`. It's a Cloudflare Worker that holds
the Linear API key server-side, rate limits by IP, and only exposes issue
creation.

### Environment variables

Set these as secrets in the Cloudflare dashboard (Settings > Variables and
Secrets) or via `npx wrangler secret put <NAME>`:

| Variable | Required | Description |
|----------|----------|-------------|
| `LINEAR_API_KEY` | Yes | Linear API key used to create issues |
| `LINEAR_TEAM_ID` | Yes | Linear team ID to file issues under |
| `LINEAR_PROJECT_ID` | Yes | Linear project ID to file issues under |
| `HOTLINE_PROXY_TOKEN` | No | When set, requires clients to send a matching `Authorization: Bearer <token>` header |
