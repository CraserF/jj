import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";

const CORS_HEADERS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
  "Access-Control-Allow-Headers": "Content-Type, Accept, Authorization, x-requested-with",
  "Access-Control-Expose-Headers": "Content-Type, Content-Length, ETag",
  "Access-Control-Max-Age": "86400",
};

export async function startGitHttpFixture() {
  const root = await mkdtemp(path.join(tmpdir(), "jj-wasm-git-fixture-"));
  const bareDir = path.join(root, "remote.git");
  const workDir = path.join(root, "work");
  let commitCounter = 0;

  await runGit(["init", "--bare", bareDir]);
  await runGit(["--git-dir", bareDir, "config", "http.receivepack", "true"]);
  await runGit(["init", "--initial-branch=main", workDir]);
  await runGit(["-C", workDir, "config", "user.name", "jj-wasm fixture"]);
  await runGit(["-C", workDir, "config", "user.email", "jj-wasm-fixture@example.invalid"]);
  await writeFile(path.join(workDir, "README.md"), "hello from remote fixture\n", "utf8");
  await runGit(["-C", workDir, "add", "README.md"]);
  await runGit(["-C", workDir, "commit", "-m", "fixture initial commit"]);
  await runGit(["-C", workDir, "remote", "add", "origin", bareDir]);
  await runGit(["-C", workDir, "push", "origin", "main"]);
  await runGit(["--git-dir", bareDir, "symbolic-ref", "HEAD", "refs/heads/main"]);

  const server = createServer(async (request, response) => {
    try {
      if (request.method === "OPTIONS") {
        writeCorsResponse(response, 204);
        response.end();
        return;
      }

      const requestUrl = new URL(request.url ?? "/", "http://127.0.0.1");
      if (requestUrl.pathname === "/__fixture/add-commit") {
        const oid = await addCommit();
        writeJson(response, { oid });
        return;
      }
      if (requestUrl.pathname === "/__fixture/resolve") {
        const ref = requestUrl.searchParams.get("ref");
        if (!ref) {
          writeJson(response, { error: "`ref` query parameter is required" }, 400);
          return;
        }
        const oid = await resolve(ref);
        writeJson(response, { oid });
        return;
      }

      await handleGitHttpBackend(request, response, requestUrl, root);
    } catch (error) {
      writeJson(
        response,
        {
          error: error?.stack ?? String(error),
        },
        500,
      );
    }
  });

  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      server.off("error", reject);
      resolve();
    });
  });

  const address = server.address();
  const port = typeof address === "object" && address ? address.port : undefined;
  if (!port) {
    throw new Error("fixture server did not expose a TCP port");
  }
  const origin = `http://127.0.0.1:${port}`;

  return {
    root,
    url: `${origin}/remote.git`,
    controlUrl: `${origin}/__fixture`,
    addCommit,
    resolve,
    async close() {
      await new Promise((resolve, reject) => {
        server.close((error) => (error ? reject(error) : resolve()));
      });
    },
  };

  async function addCommit(message = "fixture second commit") {
    commitCounter += 1;
    await writeFile(
      path.join(workDir, "REMOTE.md"),
      `fixture commit ${commitCounter}\n`,
      "utf8",
    );
    await runGit(["-C", workDir, "add", "REMOTE.md"]);
    await runGit(["-C", workDir, "commit", "-m", `${message} ${commitCounter}`]);
    await runGit(["-C", workDir, "push", "origin", "main"]);
    return resolve("refs/heads/main");
  }

  async function resolve(ref) {
    return runGit(["--git-dir", bareDir, "rev-parse", ref]);
  }
}

async function handleGitHttpBackend(request, response, requestUrl, projectRoot) {
  const body = await collectBody(request);
  const env = {
    ...process.env,
    GIT_PROJECT_ROOT: projectRoot,
    GIT_HTTP_EXPORT_ALL: "1",
    PATH_INFO: decodeURIComponent(requestUrl.pathname),
    QUERY_STRING: requestUrl.searchParams.toString(),
    REQUEST_METHOD: request.method ?? "GET",
    CONTENT_TYPE: request.headers["content-type"] ?? "",
    CONTENT_LENGTH: String(body.byteLength),
    REMOTE_USER: "jj-wasm",
  };
  const child = spawn("git", ["http-backend"], {
    env,
    stdio: ["pipe", "pipe", "pipe"],
  });
  const stdoutChunks = [];
  const stderrChunks = [];
  child.stdout.on("data", (chunk) => stdoutChunks.push(chunk));
  child.stderr.on("data", (chunk) => stderrChunks.push(chunk));
  child.stdin.end(body);
  const exitCode = await new Promise((resolve) => {
    child.on("close", resolve);
  });
  const stdout = Buffer.concat(stdoutChunks);
  const stderr = Buffer.concat(stderrChunks).toString("utf8");
  if (exitCode !== 0) {
    throw new Error(`git http-backend exited with ${exitCode}: ${stderr}`);
  }
  writeCgiResponse(response, stdout);
}

function writeCgiResponse(response, stdout) {
  const separator = findHeaderSeparator(stdout);
  if (!separator) {
    response.writeHead(500, {
      ...CORS_HEADERS,
      "Content-Type": "text/plain; charset=utf-8",
    });
    response.end("git http-backend did not return CGI headers");
    return;
  }

  const headerText = stdout.subarray(0, separator.index).toString("utf8");
  const body = stdout.subarray(separator.end);
  const headers = { ...CORS_HEADERS };
  let status = 200;
  for (const line of headerText.split(/\r?\n/)) {
    if (!line.trim()) {
      continue;
    }
    const delimiter = line.indexOf(":");
    if (delimiter === -1) {
      continue;
    }
    const name = line.slice(0, delimiter);
    const value = line.slice(delimiter + 1).trim();
    if (name.toLowerCase() === "status") {
      status = Number(value.split(" ")[0]) || status;
    } else {
      headers[name] = value;
    }
  }
  response.writeHead(status, headers);
  response.end(body);
}

function findHeaderSeparator(buffer) {
  const crlf = buffer.indexOf("\r\n\r\n");
  if (crlf !== -1) {
    return { index: crlf, end: crlf + 4 };
  }
  const lf = buffer.indexOf("\n\n");
  if (lf !== -1) {
    return { index: lf, end: lf + 2 };
  }
  return null;
}

function writeJson(response, value, status = 200) {
  writeCorsResponse(response, status, {
    "Content-Type": "application/json; charset=utf-8",
  });
  response.end(JSON.stringify(value));
}

function writeCorsResponse(response, status, headers = {}) {
  response.writeHead(status, {
    ...CORS_HEADERS,
    ...headers,
  });
}

async function collectBody(request) {
  const chunks = [];
  for await (const chunk of request) {
    chunks.push(Buffer.from(chunk));
  }
  return Buffer.concat(chunks);
}

async function runGit(args, options = {}) {
  const child = spawn("git", args, {
    ...options,
    stdio: ["ignore", "pipe", "pipe"],
  });
  const stdoutChunks = [];
  const stderrChunks = [];
  child.stdout.on("data", (chunk) => stdoutChunks.push(chunk));
  child.stderr.on("data", (chunk) => stderrChunks.push(chunk));
  const exitCode = await new Promise((resolve) => {
    child.on("close", resolve);
  });
  const stdout = Buffer.concat(stdoutChunks).toString("utf8");
  const stderr = Buffer.concat(stderrChunks).toString("utf8");
  if (exitCode !== 0) {
    throw new Error(`git ${args.join(" ")} failed with ${exitCode}:\n${stderr}\n${stdout}`);
  }
  return stdout.trim();
}
