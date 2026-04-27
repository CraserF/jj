# jj-wasm

Experimental browser/WebAssembly bindings for Jujutsu.

This crate exposes a JavaScript-first `JjSession` API through `wasm-bindgen`.
It uses the same browser integration model as `isomorphic-git`: callers pass a
Git client object, a Node-like `fs.promises` implementation, and optionally an
HTTP client. The first milestone keeps the integration isolated from the native
`jj` CLI and focuses on browser-safe storage, raw Git-object bridging, and a
small session API that can run in a Web Worker.

```js
import init, { JjSession } from "./pkg/jj_wasm.js";
import git from "isomorphic-git";
import http from "isomorphic-git/http/web";
import LightningFS from "@isomorphic-git/lightning-fs";

await init();

const fs = new LightningFS("jj");
const session = await JjSession.clone("https://github.com/example/project", {
  git,
  fs,
  http,
  dir: "/project",
  corsProxy: "https://cors.isomorphic-git.org",
  depth: 10,
});

console.log(await session.log({ limit: 10 }));
console.log(await session.status());
```

## Browser workflow harness

The `web-tests` directory contains a small Vite harness that exercises the
browser working-copy loop with LightningFS and isomorphic-git.

```sh
cd jj-wasm/web-tests
npm install
npm run build:wasm
npm run dev
```

Open the Vite URL and check that the page reports `data-status="done"`.
