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
  const worker =
    options.worker ??
    new Worker(options.workerUrl ?? new URL("./worker.js", import.meta.url), {
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
    if (message.type === "error") {
      request.reject(toWorkerError(message.error));
    } else {
      request.resolve(message.result);
    }
  });
  worker.addEventListener("error", (event) => {
    rejectAll(new Error(event.message || "jj-wasm worker failed to load"));
  });
  worker.addEventListener("messageerror", () => {
    rejectAll(new Error("jj-wasm worker sent an unreadable message"));
  });

  function call(method, ...args) {
    const id = nextId++;
    return new Promise((resolve, reject) => {
      pending.set(id, { resolve, reject });
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
        rejectAll(new Error("jj-wasm worker was disposed"));
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
      request.reject(error);
    }
    pending.clear();
  }
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
