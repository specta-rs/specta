// The only network destination besides the model gateway. Every path is built
// from integers and repository names, never from model text.

const API = "https://api.github.com";
const RETRYABLE = new Set([429, 500, 502, 503, 504]);

export async function gh(path, { method = "GET", body, token } = {}) {
  const auth = token ?? process.env.GITHUB_TOKEN;
  if (!auth) throw new Error("GITHUB_TOKEN is not available");

  let lastError;
  for (let attempt = 0; attempt < 4; attempt++) {
    if (attempt > 0) await new Promise((resolve) => setTimeout(resolve, 1_000 * 2 ** (attempt - 1)));

    let response;
    try {
      response = await fetch(`${API}${path}`, {
        method,
        headers: {
          accept: "application/vnd.github+json",
          authorization: `Bearer ${auth}`,
          "content-type": "application/json",
          "user-agent": "specta-issue-triage",
          "x-github-api-version": "2022-11-28",
        },
        body: body === undefined ? undefined : JSON.stringify(body),
        signal: AbortSignal.timeout(30_000),
      });
    } catch (error) {
      lastError = error;
      continue;
    }

    if (response.ok) return response.status === 204 ? null : response.json();

    const detail = (await response.text()).slice(0, 600);
    lastError = new Error(`${method} ${path} -> ${response.status}: ${detail}`);
    if (!RETRYABLE.has(response.status)) throw lastError;
  }

  throw lastError;
}

export function getIssue(repo, number) {
  return gh(`/repos/${repo}/issues/${number}`);
}

export async function listRecentIssues(repo, { perPage = 100 } = {}) {
  const issues = await gh(`/repos/${repo}/issues?state=all&sort=updated&direction=desc&per_page=${perPage}`);
  return (issues ?? [])
    .filter((issue) => !issue.pull_request)
    .map((issue) => ({
      number: issue.number,
      title: issue.title ?? "",
      state: issue.state ?? "open",
      body: issue.body ?? "",
    }));
}

export async function listComments(repo, number) {
  return (await gh(`/repos/${repo}/issues/${number}/comments?per_page=100`)) ?? [];
}

export function commentOnIssue(repo, number, body) {
  return gh(`/repos/${repo}/issues/${number}/comments`, { method: "POST", body: { body } });
}

export function createDraftPullRequest(repo, body) {
  return gh(`/repos/${repo}/pulls`, { method: "POST", body: { ...body, draft: true } });
}

export function branchExists(repo, branch) {
  return gh(`/repos/${repo}/git/ref/heads/${branch}`)
    .then(() => true)
    .catch(() => false);
}

export function issueUrl(repo, number) {
  return `https://github.com/${repo}/issues/${number}`;
}

export function pullUrl(repo, number) {
  return `https://github.com/${repo}/pull/${number}`;
}
