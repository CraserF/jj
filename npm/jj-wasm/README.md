# @craserf/jj-wasm

Private package wrapper for the experimental `jj-wasm` browser bindings.

Install from the fork while this package is private:

```json
{
  "dependencies": {
    "@craserf/jj-wasm": "github:CraserF/jj#codex/jj-wasm-browser"
  }
}
```

For reproducible test deploys, pin the dependency to a commit SHA instead of the
branch name.

## Worker-first usage

```js
import { createJjWorkerClient } from "@craserf/jj-wasm/worker-client";

const jj = createJjWorkerClient({ requestTimeoutMs: 30_000 });
await jj.init({ fsName: "demo", dir: "/repo", defaultBranch: "main" });
await jj.writeFile("README.md", "hello from jj-wasm\n");
await jj.snapshot({
  message: "initial snapshot",
  author: { name: "Demo", email: "demo@example.invalid" },
});

console.log(await jj.status());
await jj.dispose();
```

The worker owns the `LightningFS`, `isomorphic-git`, and `JjSession` instances.
Use the file helpers for simple test workflows, or add app-specific worker
messages around the same filesystem if a richer editor integration needs it.

For Vite apps, keep this package out of dependency pre-bundling so the worker
URL stays package-relative, while still pre-bundling the CommonJS browser
dependencies used inside the worker:

```js
// vite.config.js
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
      crypto: "@craserf/jj-wasm/shims/crypto",
    },
  },
};
```

## Direct ESM usage

The direct exports are intended for debugging and integration tests. Production
test usage should prefer the worker client.

```js
import { initWasm, JjSession } from "@craserf/jj-wasm";
import LightningFS from "@isomorphic-git/lightning-fs";
import git from "isomorphic-git";

await initWasm();
const fs = new LightningFS("direct-demo");
const session = await JjSession.init({ git, fs, dir: "/repo" });
```

## Remote browser constraints

Remote operations are backed by isomorphic-git HTTP. Browser Git servers must
serve CORS headers for `info/refs`, `git-upload-pack`, and `git-receive-pack`,
or requests must go through a trusted CORS proxy. Use `remoteRef` on `fetch()`
and `push()` when the remote branch differs from the local branch, for example
pushing a browser-created commit to `refs/heads/jj-wasm-test` during production
tests.
