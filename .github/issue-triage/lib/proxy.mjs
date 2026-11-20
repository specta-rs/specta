// A loopback proxy that injects the model credential.
//
// The agent's process is configured with a dummy key and this proxy's address,
// so the real `OPENCODE_API_KEY` never enters the process that reads a hostile
// issue. If a tool call somehow reached the network, it would find a localhost
// endpoint that can spend a bounded number of requests — not a key it can
// exfiltrate.
//
// The proxy only forwards the endpoints the CLI actually uses, so it is not a
// general purpose tunnel to the internet.

import { createServer } from "node:http";
import { Readable } from "node:stream";

const ALLOWED = [/^\/v1\/chat\/completions$/, /^\/v1\/responses$/, /^\/v1\/models$/];
const MAX_BODY_BYTES = 4 * 1024 * 1024;

function reply(response, status, payload) {
  const body = JSON.stringify(payload);
  response.writeHead(status, { "content-type": "application/json", "content-length": Buffer.byteLength(body) });
  response.end(body);
}

/**
 * @param {{ upstream: string, apiKey: string, maxRequests?: number }} options
 * @returns {Promise<{ url: string, close: () => Promise<void>, count: () => number }>}
 */
export async function startModelProxy({ upstream, apiKey, maxRequests = 64 }) {
  if (!apiKey) throw new Error("the proxy needs the model key");
  // https everywhere, plus loopback so the proxy can be tested without a
  // network. A plaintext remote upstream would put the key on the wire.
  if (!/^https:\/\/|^http:\/\/127\.0\.0\.1(:\d+)?(\/|$)/.test(upstream)) {
    throw new Error("the upstream must be https, or loopback for testing");
  }

  const base = upstream.replace(/\/+$/, "");
  let count = 0;

  const server = createServer(async (request, response) => {
    try {
      const pathname = new URL(request.url ?? "/", "http://127.0.0.1").pathname;
      if (!ALLOWED.some((pattern) => pattern.test(pathname))) {
        return reply(response, 404, { error: "this path is not proxied" });
      }
      if (count >= maxRequests) {
        return reply(response, 429, { error: "the request budget for this job is exhausted" });
      }
      count += 1;

      const chunks = [];
      let size = 0;
      for await (const chunk of request) {
        size += chunk.length;
        if (size > MAX_BODY_BYTES) return reply(response, 413, { error: "request body is too large" });
        chunks.push(chunk);
      }

      const headers = {
        "content-type": request.headers["content-type"] ?? "application/json",
        accept: request.headers.accept ?? "application/json",
        authorization: `Bearer ${apiKey}`,
        "user-agent": "specta-issue-triage-proxy",
      };
      for (const name of ["x-opencode-session", "x-opencode-client"]) {
        if (request.headers[name]) headers[name] = request.headers[name];
      }

      const forwarded = await fetch(`${base}${pathname.slice("/v1".length)}`, {
        method: request.method === "GET" ? "GET" : "POST",
        headers,
        body: request.method === "GET" ? undefined : Buffer.concat(chunks),
        signal: AbortSignal.timeout(600_000),
      });

      response.writeHead(forwarded.status, {
        "content-type": forwarded.headers.get("content-type") ?? "application/json",
        "cache-control": "no-store",
      });

      if (!forwarded.body) return response.end();
      Readable.fromWeb(forwarded.body).pipe(response);
    } catch (error) {
      if (!response.headersSent) reply(response, 502, { error: error.message });
      else response.destroy(error);
    }
  });

  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });

  const { port } = server.address();

  return {
    url: `http://127.0.0.1:${port}/v1`,
    count: () => count,
    close: () =>
      new Promise((resolve) => {
        server.closeAllConnections?.();
        server.close(() => resolve());
      }),
  };
}
