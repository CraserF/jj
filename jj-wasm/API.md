# jj-wasm API Reference

This is the hand-authored public API for the private `@craserf/jj-wasm`
package. The generated `wasm-bindgen` declarations are still available in
`npm/jj-wasm/pkg`, but production-test apps should import from the package
entrypoints below.

## Entrypoints

- `@craserf/jj-wasm`: direct ESM bindings and helpers.
- `@craserf/jj-wasm/worker-client`: main-thread worker client.
- `@craserf/jj-wasm/worker`: worker module for bundlers.
- `@craserf/jj-wasm/types`: type-only API surface.
- `@craserf/jj-wasm/shims/crypto`: Vite/browser alias target for the small
  `crypto.createHash("sha1")` surface isomorphic-git uses while reading packs.

## Worker Client

```ts
import { createJjWorkerClient } from "@craserf/jj-wasm/worker-client";

const jj = createJjWorkerClient();
// Optional: createJjWorkerClient({ requestTimeoutMs: 30_000 })
await jj.init({ fsName: "demo", dir: "/repo", defaultBranch: "main" });
await jj.writeFile("README.md", "hello\n");
await jj.snapshot({ message: "initial" });
await jj.dispose();
```

The worker client mirrors `JjSession` and adds:

- `writeFile(path, contents)`
- `readFile(path, encoding?)`
- `removeFile(path)`
- `mkdir(path)`
- `onProgress(listener)`
- `dispose()`

Progress events are currently lifecycle events for `init`, `open`, `clone`,
`fetch`, `push`, and `snapshot`: `start`, `done`, and `error`.

## Session Methods

- `init(options)`, `open(options)`, `clone(url, options)`
- `fetch(options)`, `push(options)`
- `status()`, `snapshot(options)`, `describe(options)`
- `new(options)`, `restore(options)`, `undo()`
- `log(options)`, `opLog()`
- `listRefs(prefix)`, `resolveRef(ref)`, `writeRef(ref, oid)`
- `readRawObject(oid)`, `writeRawObject(type, wrapped)`

### Remote Options

`clone(url, options)` uses the session options `ref`, `singleBranch`, `depth`,
and `corsProxy` to pass through to isomorphic-git.

`fetch(options)` accepts:

- `url`: explicit remote URL. If omitted, isomorphic-git uses the configured
  remote.
- `remote`: configured remote name, usually `origin`.
- `ref`: local branch/ref context.
- `remoteRef`: branch/ref to fetch from the remote. Use this with
  `singleBranch`.
- `corsProxy`: optional CORS proxy URL.
- `singleBranch` and `depth`: shallow/single-branch fetch controls.

`push(options)` accepts:

- `url`: explicit receiving remote URL.
- `remote`: configured remote name, usually `origin`.
- `ref`: local branch/ref to push.
- `remoteRef`: receiving branch/ref on the remote, for example
  `refs/heads/jj-wasm-test`.
- `corsProxy`: optional CORS proxy URL.

## Core Types

The exported type file includes:

- `JjSessionOptions`
- `WorkerSessionOptions`
- `StatusResult`
- `SnapshotResult`
- `DescribeResult`
- `NewResult`
- `RestoreResult`
- `UndoResult`
- `OperationRecord`
- `RawObject`
- `JjWorkerClient`
- `WorkerProgressEvent`
- `WorkerError`

See `npm/jj-wasm/types.d.ts` for the exact TypeScript declarations.
