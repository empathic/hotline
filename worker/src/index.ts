import { handleGitHub } from "./github";
import { handleLinear } from "./linear";

interface Env {
	LINEAR_API_KEY?: string;
	LINEAR_TEAM_ID?: string;
	LINEAR_PROJECT_ID?: string;
	GITHUB_TOKEN?: string;
	GITHUB_REPO?: string;
	GITHUB_APP_ID?: string;
	GITHUB_APP_PRIVATE_KEY?: string;
	GITHUB_INSTALLATION_ID?: string;
	HOTLINE_PROXY_TOKEN?: string;
}

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
			case "/":
			case "/linear":
				return handleLinear(request, env);
			case "/github":
				return handleGitHub(request, env);
			default:
				return new Response("Not found", { status: 404 });
		}
	},
};
