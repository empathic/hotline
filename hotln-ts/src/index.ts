import { GitHubIssue } from "./github.js";
import { LinearIssue } from "./linear.js";

export { GitHubIssue } from "./github.js";
export { LinearIssue } from "./linear.js";
export { HotlineError } from "./errors.js";

export function github(proxyUrl: string): GitHubIssue {
	return new GitHubIssue(proxyUrl);
}

export function linear(proxyUrl: string): LinearIssue {
	return new LinearIssue(proxyUrl);
}

export default { github, linear };
