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

## Worker Client

```ts
import { createJjWorkerClient } from "@craserf/jj-wasm/worker-client";

const jj = createJjWorkerClient();
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
