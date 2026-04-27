# jj-wasm

Experimental browser/WebAssembly bindings for Jujutsu.

This crate exposes a JavaScript-first `JjSession` API through `wasm-bindgen`.
It uses the same browser integration model as `isomorphic-git`: storage is a
Node-like `fs.promises` implementation such as LightningFS, and remotes use
`isomorphic-git` HTTP.

The production-test package is private and lives at the repository root as
`@craserf/jj-wasm`. It is meant to be installed from the fork, not published to
the public npm registry.

## Implemented today

- Browser repo creation/open/clone with isomorphic-git storage.
- `status`, `snapshot`, `describe`, `new`, `restore`, `undo`, `log`, and
  `opLog` for a single active browser session.
- `fetch`, `push`, `listRefs`, `resolveRef`, `writeRef`, `readRawObject`, and
  `writeRawObject` helpers.
- Versioned `.jj/wasm` metadata with explicit corrupt-state errors.
- Filtering for `.jj` and `.git` metadata so those paths are not snapshotted.
- `change-id` Git commit headers for commits written by the browser MVP.
- A Web Worker package wrapper with a typed main-thread client.

## Intentionally unsupported

- Full jj CLI parity and TUI behavior.
- Multi-tab or collaborative editing beyond one active browser session per repo.
- `rebase` and advanced rewrite operations.
- Conflict UI, submodules, symlink fidelity, executable-bit fidelity, GPG/SSH
  signing, external editors, pagers, and Watchman.
- Guaranteed parity with native jj for all revsets. The MVP supports `@`,
  `HEAD`, `::`, `all()`, commit IDs, and Git ref/bookmark names where
  isomorphic-git can resolve them.

## Private package install

Use the branch while iterating:

```json
{
  "dependencies": {
    "@craserf/jj-wasm": "github:CraserF/jj#codex/jj-wasm-browser"
  }
}
```

Pin to a commit SHA for reproducible test deployments:

```json
{
  "dependencies": {
    "@craserf/jj-wasm": "github:CraserF/jj#<commit-sha>"
  }
}
```

The package commits built release WASM artifacts under `npm/jj-wasm/pkg`, so
consuming apps do not need Rust, `wasm-pack`, or the wasm target at install
time. There are no install-time `prepare` or `postinstall` scripts.

## Worker-first quickstart

```js
import { createJjWorkerClient } from "@craserf/jj-wasm/worker-client";

const jj = createJjWorkerClient();
const offProgress = jj.onProgress((event) => console.log(event));

await jj.init({
  fsName: "example-repo",
  dir: "/repo",
  defaultBranch: "main",
});
await jj.writeFile("README.md", "hello from jj-wasm\n");
await jj.snapshot({
  message: "initial browser snapshot",
  author: { name: "Example", email: "example@example.invalid" },
});

console.log(await jj.status());
console.log(await jj.log({ revset: "@", limit: 5 }));

offProgress();
await jj.dispose();
```

The worker owns the `JjSession`, LightningFS, and isomorphic-git instances. Use
the worker file helpers for simple test workflows. For a richer editor, add
app-specific worker messages around the same filesystem.

Vite apps should keep the package out of dependency pre-bundling so the worker
URL remains package-relative, while still pre-bundling the CommonJS browser
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
};
```

## Direct ESM quickstart

Direct imports are useful for debugging and integration tests. Production-test
apps should prefer the worker client so expensive Git and WASM work stays off
the UI thread.

```js
import { initWasm, JjSession } from "@craserf/jj-wasm";
import LightningFS from "@isomorphic-git/lightning-fs";
import git from "isomorphic-git";

await initWasm();

const fs = new LightningFS("direct-demo");
const session = await JjSession.init({
  git,
  fs,
  dir: "/repo",
  defaultBranch: "main",
});

console.log(await session.status());
```

## Browser storage model

The browser MVP stores repository state in the same filesystem passed to
isomorphic-git:

- Git objects, refs, and index data live under `.git`.
- jj-wasm session metadata lives under `.jj/wasm`.
- `.jj/wasm/state.json` tracks current browser change state.
- `.jj/wasm/operations.json` stores the operation log used by `undo`.
- `.jj/wasm/session.json` records metadata about how the session was created.

If metadata becomes corrupt, the API returns a descriptive error instead of
silently resetting state. For disposable test repos, reset by deleting the
LightningFS database or using a new `fsName`.

## Remote setup

`clone`, `fetch`, and `push` use isomorphic-git HTTP. Browser remotes must
support CORS or go through a proxy. For authenticated remotes, add auth handling
in the worker package before using private repositories; the current worker
entrypoint does not transmit credentials by default.

## Maintainer commands

From the repository root:

```sh
npm install
npm run build:jj-wasm
npm run verify:jj-wasm-package
npm run test:jj-wasm-browser
npm run smoke:jj-wasm-package
npm run verify:jj-wasm-all
```

`build:jj-wasm` writes release artifacts to `npm/jj-wasm/pkg`. Commit those
artifacts with the wrapper files when updating the private Git dependency.
`verify:jj-wasm-all` is the production-test gate for this private package.
It runs Rust wasm checks, package build/typecheck, browser assertions, and the
packed-package smoke test.

## Browser workflow harness

The `web-tests` directory contains a Vite harness that exercises the direct
browser working-copy loop with LightningFS and isomorphic-git.

```sh
cd jj-wasm/web-tests
npm install
npm run build:wasm
npm run dev
```

Open the Vite URL and check that the page reports `data-status="done"`.
For automated assertions, run `npm run test:jj-wasm-browser` from the repo
root. Coverage status is tracked in `jj-wasm/COVERAGE.md`.

## Troubleshooting

- WASM loading: make sure your bundler treats
  `@craserf/jj-wasm/wasm` or `npm/jj-wasm/pkg/jj_wasm_bg.wasm` as an asset.
- Worker loading: use a modern ESM bundler that supports
  `new Worker(new URL(..., import.meta.url), { type: "module" })`.
  In Vite, exclude `@craserf/jj-wasm` and include `buffer`,
  `isomorphic-git`, `isomorphic-git/http/web`, and
  `@isomorphic-git/lightning-fs` in `optimizeDeps`.
- Storage quota: large repos can exceed browser storage limits; start with
  shallow clones and small fixtures.
- CORS: browser remotes fail unless the Git server has CORS enabled or a proxy
  is configured.
- Unsupported operations: `rebase` is exported so the public API shape is
  stable, but it currently returns an unsupported-operation error.
- Resetting a test repo: call `dispose()`, then use a new LightningFS `fsName`
  or clear the browser storage for the origin.
