# hotline

[![crates.io](https://img.shields.io/crates/v/hotln)](https://crates.io/crates/hotln)
[![npm](https://img.shields.io/npm/v/hotln)](https://www.npmjs.com/package/hotln)

A library for filing bug reports to [Linear](https://linear.app) and
[GitHub Issues](https://github.com) from distributed applications. Reports
are sent through a proxy server that holds API credentials.

Available for [Rust](https://crates.io/crates/hotln) and [TypeScript/JavaScript](https://www.npmjs.com/package/hotln).

## Usage

```rust
// GitHub
hotln::github("https://your-worker.example.com")
    .with_token("secret")
    .title("crash on startup")
    .text("Something went wrong.")
    .file("config.toml", &toml_str)
    .create()?;

// Linear
hotln::linear("https://your-worker.example.com")
    .with_token("secret")
    .title("crash on startup")
    .text("Details.")
    .attachment("crash.log", &log_bytes)
    .create()?;
```

## Builder API

Both backends use a fluent builder. Call `.create()` to send the request and
get back a `Result<String, Error>` containing the issue URL.

| Method | Description |
|--------|-------------|
| `.title(s)` | Set the issue title |
| `.text(s)` | Append a text block to the body |
| `.file(name, content)` | Append a fenced code block to the body |
| `.attachment(name, data)` | **Linear only.** Upload as a real Linear attachment (binary OK) |
| `.with_token(s)` | Set a bearer token for proxy auth |
| `.create()` | Send the request and return the issue URL |

`.text()` and `.file()` blocks are joined in order, separated by blank lines.

## Proxy protocol

The client POSTs JSON to the proxy. Each backend has its own path:

- `POST /linear` — create a Linear issue
- `POST /github` — create a GitHub issue

If `with_token` is set, the client sends an `Authorization: Bearer <token>` header.

### Linear request

```typescript
interface LinearRequest {
  title: string;
  description: string;
  attachments?: {
    filename: string;
    contentType: string;
    data: string;
    encoding?: "text" | "base64";
  }[];
}
```

### GitHub request

```typescript
interface GitHubRequest {
  title: string;
  description: string;
}
```

### Response (both backends)

```typescript
interface Response {
  url: string; // URL of the created issue
}
```

## Cloudflare Worker

A reference proxy implementation lives in `worker/`. See
[worker/README.md](worker/README.md) for setup and configuration.

## CLI

```
hotln github "crash on startup" --proxy-url https://worker.example.com
hotln linear "crash on startup" --proxy-url https://worker.example.com
hotln linear "crash on startup" --proxy-url https://worker.example.com -f config.toml -a crash.log
```

All flags can also be set via environment variables (`HOTLINE_PROXY_URL`,
`HOTLINE_PROXY_TOKEN`).
