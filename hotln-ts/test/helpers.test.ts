import { describe, expect, it } from "vitest";
import { inlineFile, mimeForExt } from "../src/helpers.js";

describe("inlineFile", () => {
	it("formats a file as a markdown code block", () => {
		const result = inlineFile("config.toml", 'key = "value"');
		expect(result).toBe('**config.toml**\n```toml\nkey = "value"\n```');
	});

	it("uses filename as language when no extension", () => {
		const result = inlineFile("Makefile", "all: build");
		expect(result).toBe("**Makefile**\n```Makefile\nall: build\n```");
	});
});

describe("mimeForExt", () => {
	it("returns correct MIME types", () => {
		expect(mimeForExt("photo.png")).toBe("image/png");
		expect(mimeForExt("photo.jpg")).toBe("image/jpeg");
		expect(mimeForExt("photo.jpeg")).toBe("image/jpeg");
		expect(mimeForExt("anim.gif")).toBe("image/gif");
		expect(mimeForExt("data.json")).toBe("application/json");
		expect(mimeForExt("doc.pdf")).toBe("application/pdf");
		expect(mimeForExt("log.txt")).toBe("text/plain");
		expect(mimeForExt("output.log")).toBe("text/plain");
	});

	it("returns octet-stream for unknown extensions", () => {
		expect(mimeForExt("archive.tar.gz")).toBe("application/octet-stream");
		expect(mimeForExt("noext")).toBe("application/octet-stream");
	});
});
