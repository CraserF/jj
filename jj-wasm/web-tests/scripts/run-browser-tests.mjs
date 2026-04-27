import { spawn } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

import {
  assertRemoteWorkflowResult,
  assertWorkflowResult,
  assertWorkerWorkflowResult,
} from "./assertions.mjs";
import { startGitHttpFixture } from "./git-http-fixture.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const webTestsDir = path.resolve(scriptDir, "..");
const repoRoot = path.resolve(webTestsDir, "../..");
const viteBin = path.join(repoRoot, "node_modules/.bin/vite");
const port = Number(process.env.JJ_WASM_BROWSER_PORT ?? 5181);
const baseUrl = `http://127.0.0.1:${port}`;

const gitFixture = await startGitHttpFixture();
const server = spawn(
  viteBin,
  ["--host", "127.0.0.1", "--port", String(port), "--strictPort"],
  {
    cwd: webTestsDir,
    stdio: ["ignore", "pipe", "pipe"],
  },
);
let serverOutput = "";
server.stdout.on("data", (chunk) => {
  serverOutput += chunk;
});
server.stderr.on("data", (chunk) => {
  serverOutput += chunk;
});

try {
  await waitForServer(baseUrl);
  const browser = await chromium.launch();
  try {
    const directPage = await browser.newPage();
    const direct = await readHarnessResult(directPage, `${baseUrl}/`);
    assertWorkflowResult(direct, "direct");

    const workerPage = await browser.newPage();
    const worker = await readHarnessResult(workerPage, `${baseUrl}/worker.html`);
    assertWorkerWorkflowResult(worker);

    const remotePage = await browser.newPage();
    const remoteUrl = new URL(`${baseUrl}/remote.html`);
    remoteUrl.searchParams.set("remoteUrl", gitFixture.url);
    remoteUrl.searchParams.set("controlUrl", gitFixture.controlUrl);
    remoteUrl.searchParams.set("pushRef", "refs/heads/jj-wasm-test");
    const remote = await readHarnessResult(remotePage, String(remoteUrl));
    assertRemoteWorkflowResult(remote);
  } finally {
    await browser.close();
  }
  console.log("jj-wasm browser assertions passed");
} finally {
  server.kill();
  await gitFixture.close();
}

async function readHarnessResult(page, url) {
  const consoleMessages = [];
  const pageErrors = [];
  page.on("console", (message) => {
    consoleMessages.push(`${message.type()}: ${message.text()}`);
  });
  page.on("pageerror", (error) => {
    pageErrors.push(error?.stack ?? String(error));
  });
  await page.goto(url);
  try {
    await page.waitForFunction(
      () => {
        const output = document.querySelector("#output");
        return output?.dataset.status && output.dataset.status !== "running";
      },
      undefined,
      { timeout: 60_000 },
    );
  } catch (error) {
    const text = await page.locator("#output").textContent().catch(() => "");
    throw new Error(
      `browser harness timed out at ${url}:\n${text}\n${pageErrors.join("\n")}\n${consoleMessages.join("\n")}`,
      { cause: error },
    );
  }
  const output = page.locator("#output");
  const status = await output.getAttribute("data-status");
  const text = await output.textContent();
  if (status !== "done") {
    throw new Error(`browser harness failed at ${url}:\n${text}\n${consoleMessages.join("\n")}`);
  }
  return JSON.parse(text);
}

async function waitForServer(url) {
  const started = Date.now();
  while (Date.now() - started < 30_000) {
    if (server.exitCode !== null) {
      throw new Error(`Vite server exited early:\n${serverOutput}`);
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
  throw new Error(`Vite server did not start at ${url}:\n${serverOutput}`);
}
