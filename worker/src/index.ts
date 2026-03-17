interface Env {
	LINEAR_API_KEY?: string;
	LINEAR_TEAM_ID?: string;
	LINEAR_PROJECT_ID?: string;
	GITHUB_TOKEN?: string;
	GITHUB_REPO?: string;
	HOTLINE_PROXY_TOKEN?: string;
}

interface AttachmentRequest {
	filename: string;
	contentType: string;
	data: string;
	encoding?: "text" | "base64";
}

interface LinearRequest {
	title: string;
	description: string;
	attachments?: AttachmentRequest[];
}

interface GitHubRequest {
	title: string;
	description: string;
}

const LINEAR_API_URL = "https://api.linear.app/graphql";
const GITHUB_API_URL = "https://api.github.com";
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

		const url = new URL(request.url);
		switch (url.pathname) {
			case "/linear":
				return handleLinear(request, env);
			case "/github":
				return handleGitHub(request, env);
			default:
				return new Response("Not found", { status: 404 });
		}
	},
};

async function handleLinear(request: Request, env: Env): Promise<Response> {
	if (!env.LINEAR_API_KEY || !env.LINEAR_TEAM_ID || !env.LINEAR_PROJECT_ID) {
		return new Response("Linear backend not configured", { status: 500 });
	}

	let body: LinearRequest;
	try {
		body = await request.json();
	} catch {
		return new Response("Invalid JSON", { status: 400 });
	}

	if (!body.title || !body.description) {
		return new Response("Missing title or description", { status: 400 });
	}

	const query = `mutation IssueCreate($input: IssueCreateInput!) {
		issueCreate(input: $input) {
			success
			issue { id url }
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
					teamId: env.LINEAR_TEAM_ID,
					projectId: env.LINEAR_PROJECT_ID,
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

	const issue = data?.data?.issueCreate?.issue;
	const url = issue?.url;
	const issueId = issue?.id;
	if (!url || !issueId) {
		return new Response(`Unexpected Linear response: ${JSON.stringify(data)}`, {
			status: 502,
		});
	}

	if (body.attachments?.length) {
		for (const att of body.attachments) {
			try {
				await uploadAttachment(env.LINEAR_API_KEY, issueId, att);
			} catch (err) {
				console.error(`Failed to attach ${att.filename}:`, err);
			}
		}
	}

	return Response.json({ url });
}

async function handleGitHub(request: Request, env: Env): Promise<Response> {
	if (!env.GITHUB_TOKEN || !env.GITHUB_REPO) {
		return new Response("GitHub backend not configured", { status: 500 });
	}

	let body: GitHubRequest;
	try {
		body = await request.json();
	} catch {
		return new Response("Invalid JSON", { status: 400 });
	}

	if (!body.title || !body.description) {
		return new Response("Missing title or description", { status: 400 });
	}

	const resp = await fetch(
		`${GITHUB_API_URL}/repos/${env.GITHUB_REPO}/issues`,
		{
			method: "POST",
			headers: {
				Authorization: `Bearer ${env.GITHUB_TOKEN}`,
				Accept: "application/vnd.github+json",
				"User-Agent": "hotline",
				"Content-Type": "application/json",
			},
			body: JSON.stringify({
				title: body.title,
				body: body.description,
			}),
		},
	);

	if (!resp.ok) {
		const text = await resp.text();
		return new Response(`GitHub API returned ${resp.status}: ${text}`, {
			status: 502,
		});
	}

	const data: any = await resp.json();
	const url = data?.html_url;
	if (!url) {
		return new Response(`Unexpected GitHub response: ${JSON.stringify(data)}`, {
			status: 502,
		});
	}

	return Response.json({ url });
}

async function uploadAttachment(
	apiKey: string,
	issueId: string,
	att: AttachmentRequest,
): Promise<void> {
	const bytes =
		att.encoding === "text"
			? new TextEncoder().encode(att.data)
			: Uint8Array.from(atob(att.data), (c) => c.charCodeAt(0));

	// Step 1: Get presigned upload URL
	const uploadResp = await fetch(LINEAR_API_URL, {
		method: "POST",
		headers: {
			Authorization: apiKey,
			"Content-Type": "application/json",
		},
		body: JSON.stringify({
			query: `mutation FileUpload($contentType: String!, $filename: String!, $size: Int!) {
				fileUpload(contentType: $contentType, filename: $filename, size: $size) {
					uploadFile {
						uploadUrl
						assetUrl
						headers { key value }
					}
				}
			}`,
			variables: {
				contentType: att.contentType,
				filename: att.filename,
				size: bytes.length,
			},
		}),
	});

	const uploadData: any = await uploadResp.json();
	const uploadFile = uploadData?.data?.fileUpload?.uploadFile;
	if (!uploadFile) {
		throw new Error(`fileUpload failed: ${JSON.stringify(uploadData)}`);
	}

	// Step 2: PUT file bytes to presigned URL
	const putHeaders: Record<string, string> = {
		"Content-Type": att.contentType,
		"Content-Length": String(bytes.length),
	};
	for (const h of uploadFile.headers ?? []) {
		putHeaders[h.key] = h.value;
	}
	const putResp = await fetch(uploadFile.uploadUrl, {
		method: "PUT",
		headers: putHeaders,
		body: bytes,
	});
	if (!putResp.ok) {
		throw new Error(`PUT upload failed: ${putResp.status}`);
	}

	// Step 3: Link attachment to issue
	const attachResp = await fetch(LINEAR_API_URL, {
		method: "POST",
		headers: {
			Authorization: apiKey,
			"Content-Type": "application/json",
		},
		body: JSON.stringify({
			query: `mutation AttachmentCreate($issueId: String!, $url: String!, $title: String!) {
				attachmentCreate(input: { issueId: $issueId, url: $url, title: $title }) {
					success
				}
			}`,
			variables: {
				issueId,
				url: uploadFile.assetUrl,
				title: att.filename,
			},
		}),
	});

	const attachData: any = await attachResp.json();
	if (attachData.errors) {
		throw new Error(
			`attachmentCreate failed: ${JSON.stringify(attachData.errors)}`,
		);
	}
}
