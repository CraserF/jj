// @ts-ignore Vite resolves this query to a bundled worker asset URL.
import defaultWorkerUrl from "./worker.js?worker&url";

const SESSION_METHODS = [
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
  "writeFile",
  "readFile",
  "removeFile",
  "mkdir",
];

export function createJjWorkerClient(options = {}) {
  const requestTimeoutMs = options.requestTimeoutMs ?? 30_000;
  const worker =
    options.worker ??
    new Worker(options.workerUrl ?? defaultWorkerUrl ?? new URL("./worker.js", import.meta.url), {
      type: "module",
      name: options.name ?? "jj-wasm",
    });
  let nextId = 1;
  const pending = new Map();
  const progressListeners = new Set();

  worker.addEventListener("message", (event) => {
    const message = event.data ?? {};
    if (message.type === "progress") {
      for (const listener of progressListeners) {
        listener(message.event);
      }
      return;
    }
    const request = pending.get(message.id);
    if (!request) {
      return;
    }
    pending.delete(message.id);
    globalThis.clearTimeout(request.timeout);
    if (message.type === "error") {
      request.reject(toWorkerError(message.error));
    } else {
      request.resolve(message.result);
    }
  });
  worker.addEventListener("error", (event) => {
    rejectAll(createWorkerError("WORKER_LOAD_FAILURE", event.message || "jj-wasm worker failed to load"));
  });
  worker.addEventListener("messageerror", () => {
    rejectAll(createWorkerError("WORKER_LOAD_FAILURE", "jj-wasm worker sent an unreadable message"));
  });

  function call(method, ...args) {
    const id = nextId++;
    return new Promise((resolve, reject) => {
      const timeout =
        requestTimeoutMs > 0
          ? globalThis.setTimeout(() => {
              pending.delete(id);
              reject(
                createWorkerError(
                  "REQUEST_TIMEOUT",
                  `jj-wasm worker request ${method} timed out after ${requestTimeoutMs}ms`,
                ),
              );
            }, requestTimeoutMs)
          : undefined;
      pending.set(id, { resolve, reject, timeout });
      worker.postMessage({ id, method, args });
    });
  }

  const client = {
    init: (options) => call("init", options),
    open: (options) => call("open", options),
    clone: (url, options) => call("clone", url, options),
    onProgress(listener) {
      progressListeners.add(listener);
      return () => progressListeners.delete(listener);
    },
    async dispose() {
      try {
        await call("dispose");
      } finally {
        worker.terminate();
        rejectAll(createWorkerError("WORKER_DISPOSED", "jj-wasm worker was disposed"));
        progressListeners.clear();
      }
    },
  };

  for (const method of SESSION_METHODS) {
    client[method] = (...args) => call(method, ...args);
  }

  return client;

  function rejectAll(error) {
    for (const request of pending.values()) {
      globalThis.clearTimeout(request.timeout);
      request.reject(error);
    }
    pending.clear();
  }
}

function createWorkerError(code, message) {
  const error = /** @type {Error & { code?: string }} */ (new Error(message));
  error.code = code;
  return error;
}

function toWorkerError(serialized) {
  const error = /** @type {Error & { code?: string, caller?: string, data?: unknown }} */ (
    new Error(serialized?.message ?? "jj-wasm worker error")
  );
  error.name = serialized?.name ?? "Error";
  error.stack = serialized?.stack ?? error.stack;
  error.code = serialized?.code;
  error.caller = serialized?.caller;
  error.data = serialized?.data;
  return error;
}
