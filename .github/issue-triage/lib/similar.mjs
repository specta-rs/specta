// Deterministic pre-filter so the model only sees issues that share vocabulary
// with the report. This runs before any model call and cannot be influenced by
// the issue text beyond ordinary word overlap.

const STOP_WORDS = new Set(
  ("the a an and or of to in on for with is are was were be been it this that i we you they my our as at by from " +
    "not no but if then than so do does did can could would should will bug issue error problem help please thanks")
    .split(" ")
);

export function tokens(text) {
  const matches = String(text ?? "").toLowerCase().match(/[a-z0-9_]{3,}/g) ?? [];
  return new Set(matches.filter((token) => !STOP_WORDS.has(token)));
}

export function rankCandidates(issue, candidates, limit = 10) {
  const titleTokens = tokens(issue.title);
  const bodyTokens = tokens(String(issue.body ?? "").slice(0, 8_000));

  return candidates
    .filter((candidate) => candidate.number !== issue.number)
    .map((candidate) => {
      const candidateTitle = tokens(candidate.title);
      const candidateBody = tokens(candidate.body);
      let score = 0;
      for (const token of titleTokens) {
        if (candidateTitle.has(token)) score += 3;
        else if (candidateBody.has(token)) score += 1;
      }
      for (const token of bodyTokens) {
        if (candidateTitle.has(token)) score += 1;
        else if (candidateBody.has(token)) score += 0.25;
      }
      return { number: candidate.number, title: candidate.title, state: candidate.state, body: candidate.body, score };
    })
    .filter((candidate) => candidate.score > 0)
    .sort((a, b) => b.score - a.score || b.number - a.number)
    .slice(0, limit);
}

export function candidateDigest(candidates, { bodyChars = 400 } = {}) {
  if (!candidates.length) return "(none)";
  return candidates
    .map((candidate) => {
      const body = String(candidate.body ?? "").replace(/\s+/g, " ").slice(0, bodyChars);
      return `#${candidate.number} [${candidate.state}] ${candidate.title}\n${body}`;
    })
    .join("\n\n");
}
