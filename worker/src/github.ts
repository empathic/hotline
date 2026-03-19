import { createAppAuth } from "@octokit/auth-app";

export interface GitHubEnv {
	GITHUB_TOKEN?: string;
	GITHUB_REPO?: string;
	GITHUB_APP_ID?: string;
	GITHUB_APP_PRIVATE_KEY?: string;
	GITHUB_INSTALLATION_ID?: string;
}

interface GitHubRequest {
	title: string;
	description: string;
}

const GITHUB_API_URL = "https://api.github.com";

export async function handleGitHub(
	request: Request,
	env: GitHubEnv,
): Promise<Response> {
	if (!env.GITHUB_REPO) {
		return new Response("GitHub backend not configured: missing GITHUB_REPO", {
			status: 500,
		});
	}

	let token: string;
	if (
		env.GITHUB_APP_ID &&
		env.GITHUB_APP_PRIVATE_KEY &&
		env.GITHUB_INSTALLATION_ID
	) {
		try {
			const auth = createAppAuth({
				appId: env.GITHUB_APP_ID,
				privateKey: env.GITHUB_APP_PRIVATE_KEY,
				installationId: env.GITHUB_INSTALLATION_ID,
			});
			const { token: t } = await auth({ type: "installation" });
			token = t;
		} catch (err) {
			return new Response(`GitHub App auth failed: ${err}`, { status: 502 });
		}
	} else if (env.GITHUB_TOKEN) {
		token = env.GITHUB_TOKEN;
	} else {
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
				Authorization: `Bearer ${token}`,
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
