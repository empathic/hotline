import { handleGitHub } from "./github";
import { handleLinear } from "./linear";

export { handleGitHub, type GitHubEnv } from "./github";
export { handleLinear, type LinearEnv } from "./linear";

export interface Env {
	LINEAR_API_KEY?: string;
	LINEAR_TEAM_ID?: string;
	LINEAR_PROJECT_ID?: string;
	GITHUB_TOKEN?: string;
	GITHUB_REPO?: string;
	GITHUB_APP_ID?: string;
	GITHUB_APP_PRIVATE_KEY?: string;
	GITHUB_INSTALLATION_ID?: string;
	HOTLINE_PROXY_TOKEN?: string;
	RATE_LIMIT_MAX?: string;
	RATE_LIMIT_WINDOW_MS?: string;
	CORS_ORIGIN?: string;
}

const hits = new Map<string, number[]>();

function isRateLimited(ip: string, max: number, windowMs: number): boolean {
	const now = Date.now();
	const cutoff = now - windowMs;
	const timestamps = (hits.get(ip) ?? []).filter((t) => t > cutoff);
	if (timestamps.length >= max) {
		hits.set(ip, timestamps);
		return true;
	}
	timestamps.push(now);
	hits.set(ip, timestamps);
	return false;
}

function resolveEnv(platformEnv?: Env): Env {
	if (platformEnv) return platformEnv;
	if (typeof process !== "undefined") return process.env as Env;
	return {};
}

function clientIp(request: Request): string | null {
	return (
		request.headers.get("cf-connecting-ip") ??
		request.headers.get("x-forwarded-for")?.split(",")[0]?.trim() ??
		null
	);
}

async function handleRequest(request: Request, env: Env): Promise<Response> {
	if (request.method !== "POST") {
		return new Response("Method not allowed", { status: 405 });
	}

	if (env.HOTLINE_PROXY_TOKEN) {
		const authHeader = request.headers.get("Authorization");
		if (!authHeader || authHeader !== `Bearer ${env.HOTLINE_PROXY_TOKEN}`) {
			return new Response("Unauthorized", { status: 401 });
		}
	}

	const ip = clientIp(request);
	const max = Number(env.RATE_LIMIT_MAX) || 5;
	const windowMs = Number(env.RATE_LIMIT_WINDOW_MS) || 60_000;
	if (ip && isRateLimited(ip, max, windowMs)) {
		return new Response("Rate limit exceeded", { status: 429 });
	}

	const url = new URL(request.url);
	switch (url.pathname) {
		case "/": // for backwards compatibility with v0.1
		case "/linear":
			return handleLinear(request, env);
		case "/github":
			return handleGitHub(request, env);
		default:
			return new Response("Not found", { status: 404 });
	}
}

export default {
	async fetch(request: Request, platformEnv?: Env): Promise<Response> {
		const env = resolveEnv(platformEnv);
		const origin = env.CORS_ORIGIN || "*";

		if (request.method === "OPTIONS") {
			return new Response(null, {
				status: 204,
				headers: {
					"Access-Control-Allow-Origin": origin,
					"Access-Control-Allow-Methods": "POST",
					"Access-Control-Allow-Headers": "Content-Type, Authorization",
					"Access-Control-Max-Age": "86400",
				},
			});
		}

		const response = await handleRequest(request, env);
		response.headers.set("Access-Control-Allow-Origin", origin);
		return response;
	},
};
