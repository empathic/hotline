import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { linear, HotlineError } from "../src/index.js";

const PROXY = "https://proxy.test";

describe("linear", () => {
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
		const mock = mockFetch("https://linear.app/test-org/issue/TEST-99");

		const url = await linear(PROXY)
			.title("Bug Report: test")
			.text("desc")
			.create();

		expect(url).toBe("https://linear.app/test-org/issue/TEST-99");
		expect(mock).toHaveBeenCalledWith(`${PROXY}/linear`, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				title: "Bug Report: test",
				description: "desc",
				attachments: [],
			}),
		});
	});

	it("sends Authorization header when token is set", async () => {
		const mock = mockFetch("https://linear.app/test-org/issue/TEST-100");

		await linear(PROXY)
			.withToken("my-secret-token")
			.title("auth test")
			.text("desc")
			.create();

		expect(mock).toHaveBeenCalledWith(
			expect.any(String),
			expect.objectContaining({
				headers: {
					"Content-Type": "application/json",
					Authorization: "Bearer my-secret-token",
				},
			}),
		);
	});

	it("encodes text attachments with encoding text", async () => {
		const mock = mockFetch("https://linear.app/test-org/issue/TEST-51");

		await linear(PROXY)
			.title("crash")
			.text("details")
			.attachment("crash.log", new TextEncoder().encode("log data"))
			.create();

		const body = JSON.parse(mock.mock.calls[0][1]!.body as string);
		expect(body.attachments).toEqual([
			{
				filename: "crash.log",
				contentType: "text/plain",
				data: "log data",
				encoding: "text",
			},
		]);
	});

	it("encodes binary attachments as base64", async () => {
		const mock = mockFetch("https://linear.app/test-org/issue/TEST-52");
		const binaryData = new Uint8Array([0xff, 0xd8, 0xff, 0xe0]);

		await linear(PROXY)
			.title("binary test")
			.attachment("image.png", binaryData)
			.create();

		const body = JSON.parse(mock.mock.calls[0][1]!.body as string);
		expect(body.attachments[0].encoding).toBe("base64");
		expect(body.attachments[0].contentType).toBe("image/png");
		expect(body.attachments[0].filename).toBe("image.png");
		// Verify the base64 decodes back to the original bytes
		expect(body.attachments[0].data).toBe(btoa("\xff\xd8\xff\xe0"));
	});

	it("throws HotlineError on proxy error", async () => {
		vi.mocked(fetch).mockResolvedValueOnce(
			new Response("rate limited", { status: 429 }),
		);

		const err = await linear(PROXY)
			.title("test")
			.text("desc")
			.create()
			.catch((e) => e);

		expect(err).toBeInstanceOf(HotlineError);
		expect(err.status).toBe(429);
		expect(err.body).toBe("rate limited");
	});

	it("inlines files into the description", async () => {
		const mock = mockFetch("https://linear.app/test-org/issue/TEST-53");

		await linear(PROXY)
			.title("config issue")
			.text("Bad config")
			.file("config.toml", 'key = "value"')
			.create();

		const body = JSON.parse(mock.mock.calls[0][1]!.body as string);
		expect(body.description).toBe(
			'Bad config\n\n**config.toml**\n```toml\nkey = "value"\n```',
		);
	});
});
