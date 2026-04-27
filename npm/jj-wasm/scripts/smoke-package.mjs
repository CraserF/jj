import { spawn } from "node:child_process";
import { mkdir, readdir, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

import { assertPackageSmokeResult } from "../../../jj-wasm/web-tests/scripts/assertions.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "../../..");
const tempRoot = path.join(tmpdir(), `jj-wasm-package-smoke-${Date.now()}`);
const appDir = path.join(tempRoot, "app");
const cacheDir = path.join(tmpdir(), "jj-wasm-npm-cache");
const viteBin = path.join(repoRoot, "node_modules/.bin/vite");

await mkdir(appDir, { recursive: true });
await mkdir(path.join(appDir, "src"), { recursive: true });

await run("npm", ["pack", "--pack-destination", tempRoot], { cwd: repoRoot });
const tarball = (await readdir(tempRoot))
  .filter((entry) => entry.endsWith(".tgz"))
  .map((entry) => path.join(tempRoot, entry))[0];
if (!tarball) {
  throw new Error(`npm pack did not create a tarball in ${tempRoot}`);
}

await writeFile(
  path.join(appDir, "package.json"),
  JSON.stringify(
    {
      private: true,
      type: "module",
      dependencies: {
        "@craserf/jj-wasm": `file:${tarball}`,
      },
    },
    null,
    2,
  ),
);
await writeFile(
  path.join(appDir, "vite.config.js"),
  `export default {
  optimizeDeps: {
    exclude: ["@craserf/jj-wasm"],
    include: [
      "@isomorphic-git/lightning-fs",
      "buffer",
      "isomorphic-git",
      "isomorphic-git/http/web",
    ],
  },
  resolve: {
    alias: {
      crypto: "@craserf/jj-wasm/shims/crypto",
    },
  },
};
`,
);
await writeFile(
  path.join(appDir, "index.html"),
  `<!doctype html>
<html lang="en">
  <head><meta charset="UTF-8" /><title>jj-wasm package smoke</title></head>
  <body>
    <pre id="output" data-status="running">Running...</pre>
    <script type="module" src="/src/main.js"></script>
  </body>
</html>
`,
);
await writeFile(
  path.join(appDir, "src/main.js"),
  `import { createJjWorkerClient } from "@craserf/jj-wasm/worker-client";

const output = document.querySelector("#output");
const progressEvents = [];
const jj = createJjWorkerClient({ requestTimeoutMs: 30_000 });
jj.onProgress((event) => progressEvents.push(event));

try {
  await jj.init({
    fsName: \`jj-wasm-package-\${crypto.randomUUID()}\`,
    dir: "/repo",
    defaultBranch: "main",
  });
  await jj.writeFile("README.md", "hello from package smoke\\n");
  const initialStatus = await jj.status();
  const snapshot = await jj.snapshot({
    message: "package smoke snapshot",
    author: { name: "Smoke", email: "smoke@example.invalid" },
  });
  const afterSnapshotStatus = await jj.status();
  const raw = await jj.readRawObject(snapshot.commitId);
  await jj.dispose();
  output.dataset.status = "done";
  output.textContent = JSON.stringify({
    progressEvents,
    initialStatus,
    afterSnapshotStatus,
    commitId: snapshot.commitId,
    rawKind: raw.kind,
  }, null, 2);
} catch (error) {
  output.dataset.status = "error";
  output.textContent = error?.stack || String(error);
  throw error;
}
`,
);

await run("npm", ["install", "--ignore-scripts"], {
  cwd: appDir,
  env: { npm_config_cache: cacheDir },
});

await runSmokeServer("dev", ["--host", "127.0.0.1", "--port", "5182", "--strictPort"]);
await run(viteBin, ["build"], { cwd: appDir });
await runSmokeServer("preview", ["preview", "--host", "127.0.0.1", "--port", "5183", "--strictPort"]);

console.log(`jj-wasm package smoke passed in ${appDir}`);

async function runSmokeServer(label, args) {
  const port = label === "dev" ? 5182 : 5183;
  const server = spawn(viteBin, args, {
    cwd: appDir,
    stdio: ["ignore", "pipe", "pipe"],
  });
  let serverOutput = "";
  server.stdout.on("data", (chunk) => {
    serverOutput += chunk;
  });
  server.stderr.on("data", (chunk) => {
    serverOutput += chunk;
  });
  try {
    const baseUrl = `http://127.0.0.1:${port}`;
    await waitForServer(baseUrl, server, () => serverOutput);
    const browser = await chromium.launch();
    try {
      const page = await browser.newPage();
      const result = await readHarnessResult(page, baseUrl);
      assertPackageSmokeResult(result, `package ${label}`);
    } finally {
      await browser.close();
    }
  } finally {
    server.kill();
  }
}

async function readHarnessResult(page, url) {
  await page.goto(url);
  await page.waitForFunction(
    () => {
      const output = document.querySelector("#output");
      return output?.dataset.status && output.dataset.status !== "running";
    },
    undefined,
    { timeout: 60_000 },
  );
  const output = page.locator("#output");
  const status = await output.getAttribute("data-status");
  const text = await output.textContent();
  if (status !== "done") {
    throw new Error(`package smoke failed at ${url}:\n${text}`);
  }
  return JSON.parse(text);
}

async function waitForServer(url, server, output) {
  const started = Date.now();
  while (Date.now() - started < 30_000) {
    if (server.exitCode !== null) {
      throw new Error(`Vite ${url} exited early:\n${output()}`);
    }
    try {
      const response = await fetch(url);
      if (response.ok) {
        return;
      }
    } catch {
      // Keep polling until Vite is ready.
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`Vite server did not start at ${url}:\n${output()}`);
}

async function run(command, args, options = {}) {
  const env = { ...process.env, ...(options.env ?? {}) };
  const child = spawn(command, args, {
    cwd: options.cwd ?? repoRoot,
    env,
    stdio: ["ignore", "pipe", "pipe"],
  });
  let output = "";
  child.stdout.on("data", (chunk) => {
    output += chunk;
  });
  child.stderr.on("data", (chunk) => {
    output += chunk;
  });
  const exitCode = await new Promise((resolve) => {
    child.on("close", resolve);
  });
  if (exitCode !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed with exit code ${exitCode}:\n${output}`);
  }
  return output;
}
