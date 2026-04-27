import initWasm, { JjSession, initSync } from "./pkg/jj_wasm.js";

export { JjSession, initSync };
export { default as initWasm } from "./pkg/jj_wasm.js";

export async function initializeJjWasm(moduleOrPath) {
  return initWasm(moduleOrPath);
}

export async function createSession(options) {
  await initWasm(options?.wasm);
  const mode = options?.mode ?? "open";
  if (mode === "init") {
    return JjSession.init(options);
  }
  if (mode === "clone") {
    if (!options?.url) {
      throw new Error("createSession({ mode: 'clone' }) requires `url`");
    }
    return JjSession.clone(options.url, options);
  }
  return JjSession.open(options);
}
