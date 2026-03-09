interface Env {
	LINEAR_API_KEY: string;
	LINEAR_TEAM_ID: string;
	LINEAR_PROJECT_ID: string;
	HOTLINE_PROXY_TOKEN?: string;
}

interface AttachmentRequest {
	filename: string;
	contentType: string;
	data: string;
	encoding?: "text" | "base64";
}

interface BugReportRequest {
	title: string;
	description: string;
	attachments?: AttachmentRequest[];
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

		const issue = data?.data?.issueCreate?.issue;
		const url = issue?.url;
		const issueId = issue?.id;
		if (!url || !issueId) {
			return new Response(
				`Unexpected Linear response: ${JSON.stringify(data)}`,
				{ status: 502 },
			);
		}

		// Upload attachments if present
		if (body.attachments?.length) {
			for (const att of body.attachments) {
				try {
					await uploadAttachment(env.LINEAR_API_KEY, issueId, att);
				} catch (err) {
					// Attachment failure is non-fatal — the issue was already created
					console.error(`Failed to attach ${att.filename}:`, err);
				}
			}
		}

		return Response.json({ url });
	},
};

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
