// A tool loop with a deliberately tiny capability surface: read-only access to
// the clone, nothing else. There is no shell, no network tool and no write tool,
// so a successful prompt injection can at worst waste tokens.

import { chat } from "./console.mjs";
import { parseJsonLoose, validate } from "./schema.mjs";
import { wrapUntrusted, UNTRUSTED_NOTE } from "./untrusted.mjs";

const MAX_TOOL_OUTPUT = 8_000;

export async function runAgent({ system, user, tools = [], handlers = {}, session, turns = 10, maxTokens = 8_000 }) {
  const messages = [
    { role: "system", content: system },
    { role: "user", content: user },
  ];

  for (let turn = 1; turn <= turns; turn++) {
    const message = await chat({ messages, tools, session, maxTokens });
    const calls = Array.isArray(message.tool_calls) ? message.tool_calls : [];

    if (calls.length === 0) return String(message.content ?? "");

    messages.push({
      role: "assistant",
      content: message.content ?? "",
      tool_calls: calls,
    });

    for (const call of calls) {
      const name = call?.function?.name ?? "";
      let result;
      try {
        const args = JSON.parse(call?.function?.arguments || "{}");
        const handler = handlers[name];
        if (!handler) throw new Error(`unknown tool "${name}"`);
        result = await handler(args);
      } catch (error) {
        result = `ERROR: ${error.message}`;
      }
      messages.push({
        role: "tool",
        tool_call_id: call.id,
        content: wrapUntrusted("tool_output", result, { max: MAX_TOOL_OUTPUT }),
      });
    }
  }

  throw new Error(`agent did not return a final answer within ${turns} turns`);
}

// Runs the agent, then insists on a JSON object that satisfies `schema` and
// `extraChecks`. One repair attempt is allowed; after that we fail closed.
export async function askJson({ system, user, tools, handlers, schema, extraChecks = () => [], session, turns = 10 }) {
  let prompt = user;
  let lastErrors = [];

  for (let attempt = 0; attempt < 2; attempt++) {
    const reply = await runAgent({ system, user: prompt, tools, handlers, session, turns });
    let parsed;
    try {
      parsed = parseJsonLoose(reply);
    } catch (error) {
      lastErrors = [error.message];
      prompt = `${user}\n\n${UNTRUSTED_NOTE}\n\nYour previous reply was not valid JSON (${error.message}). Reply with the JSON object only.`;
      continue;
    }

    const errors = [...validate(parsed, schema), ...extraChecks(parsed)];
    if (errors.length === 0) return parsed;

    lastErrors = errors;
    prompt =
      `${user}\n\n${UNTRUSTED_NOTE}\n\n` +
      `Your previous reply did not satisfy the schema:\n` +
      errors.map((error) => `- ${error}`).join("\n") +
      `\n\nReply with a corrected JSON object only.`;
  }

  throw new Error(`model output never satisfied the schema: ${lastErrors.join("; ")}`);
}
