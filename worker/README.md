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

| Variable | Required | Description |
|----------|----------|-------------|
| `LINEAR_API_KEY` | For Linear | Linear API key |
| `LINEAR_TEAM_ID` | For Linear | Linear team ID |
| `LINEAR_PROJECT_ID` | For Linear | Linear project ID |
| `GITHUB_TOKEN` | For GitHub | GitHub personal access token |
| `GITHUB_REPO` | For GitHub | Repo in `owner/repo` format |
| `HOTLINE_PROXY_TOKEN` | No | When set, requires `Authorization: Bearer <token>` on all requests |

## Development

```sh
npx wrangler dev
```

## Deploy

```sh
npx wrangler deploy
npx wrangler secret put <var>
```

## Rate limiting

Requests are rate limited to 5 per minute per IP (based on `cf-connecting-ip`).
