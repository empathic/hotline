import { HotlineError } from "./errors.js";
import { inlineFile, mimeForExt } from "./helpers.js";

interface Attachment {
	filename: string;
	data: Uint8Array;
}

export class LinearIssue {
	private proxyUrl: string;
	private token?: string;
	private issueTitle = "Untitled";
	private description = "";
	private attachments: Attachment[] = [];

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

	attachment(filename: string, data: Uint8Array): this {
		this.attachments.push({ filename, data });
		return this;
	}

	async create(): Promise<string> {
		const headers: Record<string, string> = {
			"Content-Type": "application/json",
		};
		if (this.token) {
			headers.Authorization = `Bearer ${this.token}`;
		}

		const encodedAttachments = this.attachments.map(({ filename, data }) => {
			const contentType = mimeForExt(filename);
			try {
				const text = new TextDecoder("utf-8", { fatal: true }).decode(data);
				return {
					filename,
					contentType,
					data: text,
					encoding: "text" as const,
				};
			} catch {
				// Not valid UTF-8, encode as base64
				let binary = "";
				for (let i = 0; i < data.length; i++) {
					binary += String.fromCharCode(data[i]);
				}
				return {
					filename,
					contentType,
					data: btoa(binary),
					encoding: "base64" as const,
				};
			}
		});

		const resp = await fetch(`${this.proxyUrl}/linear`, {
			method: "POST",
			headers,
			body: JSON.stringify({
				title: this.issueTitle,
				description: this.description,
				attachments: encodedAttachments,
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
