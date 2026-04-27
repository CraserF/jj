# jj-wasm Coverage Map

This map tracks the private production-test confidence suite for
`@craserf/jj-wasm`.

## Commands

- `npm run test:jj-wasm-browser`: direct ESM, worker-client, and local CORS Git fixture browser workflow coverage.
- `npm run smoke:jj-wasm-package`: packed private package install, dev server, and production preview smoke.
- `npm run verify:jj-wasm-all`: Rust gates, package build/typecheck, browser tests, and package smoke.

## Public API Coverage

| API | Status | Validated by |
| --- | --- | --- |
| `JjSession.init` / worker `init` | covered | `npm run test:jj-wasm-browser`, `npm run smoke:jj-wasm-package` |
| `JjSession.open` / worker `open` | covered | `npm run test:jj-wasm-browser` |
| `JjSession.clone` / worker `clone` | covered | `npm run test:jj-wasm-browser` with local Git HTTP fixture |
| `fetch` | covered | `npm run test:jj-wasm-browser` with local Git HTTP fixture |
| `push` | covered | `npm run test:jj-wasm-browser` with local Git HTTP fixture |
| `status` | covered | `npm run test:jj-wasm-browser`, `npm run smoke:jj-wasm-package` |
| `snapshot` | covered | `npm run test:jj-wasm-browser`, `npm run smoke:jj-wasm-package` |
| `describe` | covered | `npm run test:jj-wasm-browser` |
| `new` | covered | `npm run test:jj-wasm-browser` |
| `restore` | covered | `npm run test:jj-wasm-browser` |
| `undo` | covered | `npm run test:jj-wasm-browser` |
| `log` | covered | `npm run test:jj-wasm-browser` |
| `opLog` | covered | `npm run test:jj-wasm-browser` |
| `rebase` | unsupported-tested | `npm run test:jj-wasm-browser` |
| `listRefs` | covered | `npm run test:jj-wasm-browser` with local Git HTTP fixture |
| `resolveRef` | covered | `npm run test:jj-wasm-browser` with local Git HTTP fixture |
| `writeRef` | smoke-covered | `npm run verify:jj-wasm-all` through package type/build gates |
| `readRawObject` | covered | `npm run test:jj-wasm-browser`, `npm run smoke:jj-wasm-package` |
| `writeRawObject` | smoke-covered | `npm run verify:jj-wasm-all` through package type/build gates |
| Worker file helpers | covered | `npm run test:jj-wasm-browser`, `npm run smoke:jj-wasm-package` |
| Worker progress events | covered | `npm run test:jj-wasm-browser`, `npm run smoke:jj-wasm-package` |
| Worker repo lock rejection | covered | `npm run test:jj-wasm-browser` |

## Current Gaps

- Authenticated/private remotes and deployed CORS proxy behavior.
- Native jj parity checks for selected commit/tree IDs.
- Multi-tab locking beyond the single-repo worker lock rejection case.
- Conflict, submodule, symlink, executable bit, signing, and advanced rewrite behavior.
