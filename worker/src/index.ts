interface Env {
	LINEAR_API_KEY: string;
	LINEAR_TEAM_ID: string;
	LINEAR_PROJECT_ID: string;
	HOTLINE_PROXY_TOKEN?: string;
}

interface BugReportRequest {
	title: string;
	description: string;
}

const LINEAR_API_URL = "https://api.linear.app/graphql";
const RATE_LIMIT_MAX = 5;
const RATE_LIMIT_WINDOW_MS = 60_000; // 1 minute

const hits = new Map<string, number[]>();

function isRateLimited(ip: string): boolean {
	const now = Date.now();
	const cutoff = now - RATE_LIMIT_WINDOW_MS;
	const timestamps = (hits.get(ip) ?? []).filter((t) => t > cutoff);
	if (timestamps.length >= RATE_LIMIT_MAX) {
		hits.set(ip, timestamps);
		return true;
	}
	timestamps.push(now);
	hits.set(ip, timestamps);
	return false;
}

export default {
	async fetch(request: Request, env: Env): Promise<Response> {
		if (request.method !== "POST") {
			return new Response("Method not allowed", { status: 405 });
		}

		if (env.HOTLINE_PROXY_TOKEN) {
			const authHeader = request.headers.get("Authorization");
			if (!authHeader || authHeader !== `Bearer ${env.HOTLINE_PROXY_TOKEN}`) {
				return new Response("Unauthorized", { status: 401 });
			}
		}

		const ip = request.headers.get("cf-connecting-ip");
		if (ip && isRateLimited(ip)) {
			return new Response("Rate limit exceeded", { status: 429 });
		}

		let body: BugReportRequest;
		try {
			body = await request.json();
		} catch {
			return new Response("Invalid JSON", { status: 400 });
		}

		if (!body.title || !body.description) {
			return new Response("Missing title or description", { status: 400 });
		}

		// Use server-side team/project IDs, ignoring whatever the client sent.
		const teamId = env.LINEAR_TEAM_ID;
		const projectId = env.LINEAR_PROJECT_ID;

		if (!teamId || !projectId || !env.LINEAR_API_KEY) {
			return new Response("Proxy not configured", { status: 500 });
		}

		const query = `mutation IssueCreate($input: IssueCreateInput!) {
			issueCreate(input: $input) {
				success
				issue { url }
			}
		}`;

		const resp = await fetch(LINEAR_API_URL, {
			method: "POST",
			headers: {
				Authorization: env.LINEAR_API_KEY,
				"Content-Type": "application/json",
			},
			body: JSON.stringify({
				query,
				variables: {
					input: {
						teamId,
						projectId,
						title: body.title,
						description: body.description,
					},
				},
			}),
		});

		if (!resp.ok) {
			const text = await resp.text();
			return new Response(`Linear API returned ${resp.status}: ${text}`, {
				status: 502,
			});
		}

		const data: any = await resp.json();

		if (data.errors) {
			const errMsg = JSON.stringify(data.errors);
			return new Response(`Linear GraphQL errors: ${errMsg}`, { status: 502 });
		}

		const url = data?.data?.issueCreate?.issue?.url;
		if (!url) {
			return new Response(
				`Unexpected Linear response: ${JSON.stringify(data)}`,
				{ status: 502 },
			);
		}

		return Response.json({ url });
	},
};
