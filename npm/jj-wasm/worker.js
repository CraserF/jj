import { Buffer } from "buffer";
import LightningFS from "@isomorphic-git/lightning-fs";
import git from "isomorphic-git";
import http from "isomorphic-git/http/web";
import initWasm, { JjSession } from "./pkg/jj_wasm.js";

globalThis.Buffer ??= Buffer;

const SESSION_METHODS = new Set([
  "fetch",
  "push",
  "log",
  "status",
  "snapshot",
  "describe",
  "new",
  "rebase",
  "restore",
  "opLog",
  "undo",
  "listRefs",
  "resolveRef",
  "writeRef",
  "readRawObject",
  "writeRawObject",
]);
const PROGRESS_METHODS = new Set(["init", "open", "clone", "fetch", "push", "snapshot"]);

let wasmReady;
let session;
let fs;
let pfs;
let releaseRepoLock;

globalThis.addEventListener("message", (event) => {
  void handleMessage(event.data);
});

async function handleMessage(message) {
  const { id, method, args = [] } = message ?? {};
  try {
    if (!method) {
      throw createError("BAD_WORKER_MESSAGE", "jj-wasm worker message is missing `method`");
    }
    emitProgress(method, "start");
    const result = await dispatch(method, args);
    emitProgress(method, "done");
    globalThis.postMessage({ id, type: "result", result });
  } catch (error) {
    emitProgress(method, "error", serializeError(error));
    globalThis.postMessage({ id, type: "error", error: serializeError(error) });
  }
}

async function dispatch(method, args) {
  if (method === "init") {
    await ensureRuntime();
    const options = args[0] ?? {};
    await prepareRepository(options);
    session = await JjSession.init(sessionOptions(options));
    return sessionInfo();
  }
  if (method === "open") {
    await ensureRuntime();
    const options = args[0] ?? {};
    await prepareRepository(options);
    session = await JjSession.open(sessionOptions(options));
    return sessionInfo();
  }
  if (method === "clone") {
    await ensureRuntime();
    const [url, options = {}] = args;
    if (!url) {
      throw createError("BAD_WORKER_MESSAGE", "clone(url, options) requires `url`");
    }
    await prepareRepository(options);
    session = await JjSession.clone(url, sessionOptions(options));
    return sessionInfo();
  }
  if (method === "dispose") {
    await disposeSession();
    return undefined;
  }
  if (method === "writeFile") {
    await requireFs().writeFile(resolvePath(args[0]), args[1]);
    return undefined;
  }
  if (method === "readFile") {
    return requireFs().readFile(resolvePath(args[0]), args[1]);
  }
  if (method === "removeFile") {
    await requireFs().unlink(resolvePath(args[0]));
    return undefined;
  }
  if (method === "mkdir") {
    await requireFs().mkdir(resolvePath(args[0]));
    return undefined;
  }
  if (SESSION_METHODS.has(method)) {
    await ensureRuntime();
    if (!session) {
      throw createError(
        "SESSION_MISSING",
        `JjSession must be init/open/clone'd before calling ${method}()`,
      );
    }
    return session[method](...args);
  }
  throw createError("UNSUPPORTED_WORKER_METHOD", `unsupported jj-wasm worker method: ${method}`);
}

async function ensureRuntime() {
  globalThis.Buffer ??= Buffer;
  wasmReady ??= initWasm();
  await wasmReady;
}

async function prepareRepository(options) {
  if (session) {
    throw createError(
      "SESSION_ACTIVE",
      "this jj-wasm worker already owns an active session; call dispose() first",
    );
  }
  const fsName = options.fsName ?? "jj-wasm";
  fs = new LightningFS(fsName);
  pfs = fs.promises;
  await acquireRepoLock(options);
}

async function acquireRepoLock(options) {
  if (releaseRepoLock) {
    throw createError("REPO_LOCKED", "this jj-wasm worker already holds a repository lock");
  }
  const lockName = options.lockName ?? `jj-wasm:${options.fsName ?? "jj-wasm"}:${options.dir}`;
  if (!globalThis.navigator?.locks?.request) {
    releaseRepoLock = () => {};
    return;
  }

  let release;
  const released = new Promise((resolve) => {
    release = resolve;
  });
  const acquired = new Promise((resolve) => {
    globalThis.navigator.locks
      .request(lockName, { ifAvailable: true }, async (lock) => {
        resolve(Boolean(lock));
        if (lock) {
          await released;
        }
      })
      .catch(() => resolve(false));
  });
  const lockResult = await Promise.race([
    acquired,
    new Promise((resolve) => {
      globalThis.setTimeout(() => resolve("timeout"), 5000);
    }),
  ]);
  if (lockResult === "timeout") {
    throw createError("REPO_LOCK_TIMEOUT", `timed out while acquiring jj-wasm repository lock ${lockName}`);
  }
  if (!lockResult) {
    throw createError("REPO_LOCKED", `another active jj-wasm session already holds ${lockName}`);
  }
  releaseRepoLock = release;
}

function sessionOptions(options) {
  return {
    ...options,
    git,
    fs,
    http,
  };
}

function sessionInfo() {
  return {
    dir: session.dir,
    gitdir: session.gitdir,
  };
}

function requireFs() {
  if (!pfs) {
    throw createError("SESSION_MISSING", "filesystem is not initialized; call init/open/clone first");
  }
  return pfs;
}

function resolvePath(path) {
  if (!path || typeof path !== "string") {
    throw createError("BAD_WORKER_MESSAGE", "file path must be a string");
  }
  if (path.startsWith("/")) {
    return path;
  }
  if (!session?.dir) {
    throw createError("SESSION_MISSING", "relative file paths require an active session");
  }
  return `${session.dir.replace(/\/$/, "")}/${path}`;
}

async function disposeSession() {
  session?.free?.();
  session = undefined;
  fs = undefined;
  pfs = undefined;
  releaseRepoLock?.();
  releaseRepoLock = undefined;
}

function emitProgress(method, phase, detail) {
  if (PROGRESS_METHODS.has(method)) {
    globalThis.postMessage({
      type: "progress",
      event: { method, phase, detail },
    });
  }
}

function serializeError(error) {
  const message = error?.message ?? String(error);
  return {
    name: error?.name ?? "Error",
    message,
    stack: error?.stack,
    code: error?.code ?? inferErrorCode(message),
    caller: error?.caller,
    data: error?.data,
  };
}

function createError(code, message) {
  const error = /** @type {Error & { code?: string }} */ (new Error(message));
  error.code = code;
  return error;
}

function inferErrorCode(message) {
  if (message.includes("exported by jj-wasm")) {
    return "UNSUPPORTED_OPERATION";
  }
  if (message.includes("failed to decode jj-wasm metadata") || message.includes("unsupported jj-wasm state schema")) {
    return "CORRUPT_STATE";
  }
  if (message.includes("no undoable operation")) {
    return "NOTHING_TO_UNDO";
  }
  if (message.includes("clone failed") || message.includes("fetch failed") || message.includes("push failed")) {
    return "REMOTE_FAILURE";
  }
  return undefined;
}
