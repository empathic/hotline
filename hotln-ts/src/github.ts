import { HotlineError } from "./errors.js";
import { inlineFile } from "./helpers.js";

export class GitHubIssue {
	private proxyUrl: string;
	private token?: string;
	private issueTitle = "Untitled";
	private description = "";

	constructor(proxyUrl: string) {
		this.proxyUrl = proxyUrl;
	}

	withToken(token: string): this {
		this.token = token;
		return this;
	}

	title(title: string): this {
		this.issueTitle = title;
		return this;
	}

	text(text: string): this {
		if (this.description) {
			this.description += "\n\n";
		}
		this.description += text;
		return this;
	}

	file(filename: string, content: string): this {
		if (this.description) {
			this.description += "\n\n";
		}
		this.description += inlineFile(filename, content);
		return this;
	}

	async create(): Promise<string> {
		const headers: Record<string, string> = {
			"Content-Type": "application/json",
		};
		if (this.token) {
			headers.Authorization = `Bearer ${this.token}`;
		}

		const resp = await fetch(`${this.proxyUrl}/github`, {
			method: "POST",
			headers,
			body: JSON.stringify({
				title: this.issueTitle,
				description: this.description,
			}),
		});

		if (!resp.ok) {
			const body = await resp.text();
			throw new HotlineError(
				`Proxy returned error ${resp.status}: ${body}`,
				resp.status,
				body,
			);
		}

		const json = (await resp.json()) as { url?: string };
		if (typeof json.url !== "string") {
			throw new HotlineError("Proxy response missing url");
		}
		return json.url;
	}
}
