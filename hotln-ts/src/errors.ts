export class HotlineError extends Error {
	status?: number;
	body?: string;

	constructor(message: string, status?: number, body?: string) {
		super(message);
		this.name = "HotlineError";
		this.status = status;
		this.body = body;
	}
}
