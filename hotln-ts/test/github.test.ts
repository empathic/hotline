import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { github, HotlineError } from "../src/index.js";

const PROXY = "https://proxy.test";

describe("github", () => {
	beforeEach(() => {
		vi.stubGlobal("fetch", vi.fn());
	});

	afterEach(() => {
		vi.restoreAllMocks();
	});

	function mockFetch(url: string, status = 200) {
		const mock = vi.mocked(fetch);
		mock.mockResolvedValueOnce(
			new Response(JSON.stringify({ url }), {
				status,
				headers: { "Content-Type": "application/json" },
			}),
		);
		return mock;
	}

	it("creates an issue and returns the URL", async () => {
		const mock = mockFetch("https://github.com/owner/repo/issues/1");

		const url = await github(PROXY)
			.title("crash on startup")
			.text("Something went wrong")
			.create();

		expect(url).toBe("https://github.com/owner/repo/issues/1");
		expect(mock).toHaveBeenCalledWith(`${PROXY}/github`, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				title: "crash on startup",
				description: "Something went wrong",
			}),
		});
	});

	it("sends Authorization header when token is set", async () => {
		const mock = mockFetch("https://github.com/owner/repo/issues/2");

		await github(PROXY)
			.withToken("my-token")
			.title("auth test")
			.text("details")
			.create();

		expect(mock).toHaveBeenCalledWith(
			expect.any(String),
			expect.objectContaining({
				headers: {
					"Content-Type": "application/json",
					Authorization: "Bearer my-token",
				},
			}),
		);
	});

	it("inlines files into the description", async () => {
		const mock = mockFetch("https://github.com/owner/repo/issues/3");

		await github(PROXY)
			.title("config issue")
			.text("Bad config detected")
			.file("config.toml", 'key = "value"')
			.text("Please investigate")
			.create();

		const body = JSON.parse(mock.mock.calls[0][1]!.body as string);
		expect(body.description).toBe(
			'Bad config detected\n\n**config.toml**\n```toml\nkey = "value"\n```\n\nPlease investigate',
		);
	});

	it("throws HotlineError on proxy error", async () => {
		vi.mocked(fetch).mockResolvedValueOnce(
			new Response("rate limited", { status: 429 }),
		);

		const err = await github(PROXY)
			.title("test")
			.text("desc")
			.create()
			.catch((e) => e);

		expect(err).toBeInstanceOf(HotlineError);
		expect(err.status).toBe(429);
		expect(err.body).toBe("rate limited");
	});

	it("defaults title to Untitled", async () => {
		const mock = mockFetch("https://github.com/owner/repo/issues/4");

		await github(PROXY).text("no title").create();

		const body = JSON.parse(mock.mock.calls[0][1]!.body as string);
		expect(body.title).toBe("Untitled");
	});
});
