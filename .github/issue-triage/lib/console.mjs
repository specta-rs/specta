// OpenCode Console, called over its OpenAI-compatible endpoint.
//
//   POST ${OPENCODE_BASE_URL}/openai/v1/chat/completions
//
// The model id and gateway are repository variables so swapping models never
// needs a code change. Only `OPENCODE_API_KEY` is a secret, and it is only ever
// present in the two jobs that talk to the model.

import { required } from "./env.mjs";

const DEFAULT_MODEL = "kimi-k3";
const RETRYABLE = new Set([408, 409, 425, 429, 500, 502, 503, 504]);

function endpoint() {
  const base = (process.env.OPENCODE_BASE_URL || "https://opencode.ai/inference").replace(/\/+$/, "");
  return `${base}/openai/v1/chat/completions`;
}

export function modelName() {
  // Accept the `opencode/<id>` form used in OpenCode configs as well as the bare
  // model id the inference API expects.
  return (process.env.OPENCODE_MODEL || DEFAULT_MODEL).replace(/^opencode\//, "");
}

async function post(body, session) {
  const key = required("OPENCODE_API_KEY");
  let lastError;

  for (let attempt = 0; attempt < 4; attempt++) {
    if (attempt > 0) await new Promise((resolve) => setTimeout(resolve, 2_000 * 2 ** (attempt - 1)));

    let response;
    try {
      response = await fetch(endpoint(), {
        method: "POST",
        headers: {
          authorization: `Bearer ${key}`,
          "content-type": "application/json",
          "user-agent": "specta-issue-triage",
          // Console uses this to route a conversation to the same machine and
          // reuse its prompt cache.
          ...(session ? { "x-opencode-session": session } : {}),
        },
        body: JSON.stringify(body),
        signal: AbortSignal.timeout(240_000),
      });
    } catch (error) {
      lastError = error;
      continue;
    }

    if (response.ok) return response.json();

    const detail = (await response.text()).slice(0, 600);
    lastError = new Error(`Console request failed: ${response.status} ${detail}`);
    if (!RETRYABLE.has(response.status)) throw lastError;
  }

  throw lastError;
}

export async function chat({ messages, tools, session, maxTokens = 8_000 }) {
  const body = {
    model: modelName(),
    messages,
    temperature: 0,
    max_tokens: maxTokens,
  };
  if (tools?.length) {
    body.tools = tools;
    body.tool_choice = "auto";
  }

  const payload = await post(body, session);
  const message = payload?.choices?.[0]?.message;
  if (!message) throw new Error("Console response did not contain a message");
  return message;
}
