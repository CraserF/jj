import initWasm, { JjSession, initSync } from "./pkg/jj_wasm.js";
import type { CreateSessionOptions } from "./types.d.ts";

export * from "./types.d.ts";
export { JjSession, initSync };
export { initWasm };

export declare function initializeJjWasm(
  moduleOrPath?: Parameters<typeof initWasm>[0],
): ReturnType<typeof initWasm>;

export declare function createSession(options: CreateSessionOptions): Promise<JjSession>;
