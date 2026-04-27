import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(fileURLToPath(new URL("../..", import.meta.url)));
const jjWasmCryptoShim = path.join(repoRoot, "npm/jj-wasm/shims/crypto.js");

export default {
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
      crypto: jjWasmCryptoShim,
    },
  },
  server: {
    fs: {
      allow: [repoRoot],
    },
  },
};
