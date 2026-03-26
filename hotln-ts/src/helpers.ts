export function inlineFile(filename: string, content: string): string {
	const ext = filename.split(".").pop() ?? filename;
	return `**${filename}**\n\`\`\`${ext}\n${content}\n\`\`\``;
}

export function mimeForExt(filename: string): string {
	const ext = filename.split(".").pop()?.toLowerCase() ?? "";
	switch (ext) {
		case "png":
			return "image/png";
		case "jpg":
		case "jpeg":
			return "image/jpeg";
		case "gif":
			return "image/gif";
		case "json":
			return "application/json";
		case "pdf":
			return "application/pdf";
		case "txt":
		case "log":
			return "text/plain";
		default:
			return "application/octet-stream";
	}
}
