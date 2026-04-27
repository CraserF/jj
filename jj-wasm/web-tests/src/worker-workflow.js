import { createJjWorkerClient } from "../../../npm/jj-wasm/worker-client.js";

const AUTHOR = {
  name: "jj-wasm",
  email: "jj-wasm@example.invalid",
};

export async function runWorkerWorkflow() {
  const suffix = globalThis.crypto?.randomUUID?.() ?? Date.now();
  const lockResult = await runLockCheck(suffix);
  const progressEvents = [];
  const jj = createJjWorkerClient({ requestTimeoutMs: 30_000 });
  const offProgress = jj.onProgress((event) => progressEvents.push(event));

  try {
    await jj.init({
      fsName: `jj-wasm-worker-${suffix}`,
      dir: "/repo",
      defaultBranch: "main",
    });
    await jj.writeFile(".jj/manual-ignore-check", "ignored jj metadata\n");
    await jj.writeFile(".git/manual-ignore-check", "ignored git metadata\n");
    await jj.writeFile("README.md", "hello from jj-wasm worker\n");

    const initialStatus = await jj.status();
    const snapshot = await jj.snapshot({
      message: "initial worker snapshot",
      author: AUTHOR,
    });
    const afterSnapshotStatus = await jj.status();
    const described = await jj.describe({
      message: "describe worker snapshot",
      author: AUTHOR,
    });
    const describedRaw = await jj.readRawObject(described.commitId);
    const describedCommitText = decodeCommitObject(describedRaw.wrapped);

    await jj.writeFile("README.md", "hello again from jj-wasm worker\n");
    const modifiedStatus = await jj.status();
    const nextChange = await jj.new({
      description: "worker follow-up change",
    });
    const restored = await jj.restore({
      paths: ["README.md"],
      source: "HEAD",
      message: "restore README",
      author: AUTHOR,
    });
    const restoredText = await jj.readFile("README.md", "utf8");
    const undo = await jj.undo();
    const opLog = await jj.opLog();
    const log = await jj.log({ limit: 10 });
    const rebaseError = await captureError(() => jj.rebase({}));

    return {
      lockResult,
      progressEvents,
      initialStatus,
      snapshot,
      afterSnapshotStatus,
      described,
      modifiedStatus,
      nextChange,
      restored,
      restoredText,
      undo,
      opLog,
      log,
      rebaseError,
      rawCommitMetadata: {
        kind: describedRaw.kind,
        hasChangeIdHeader: /^change-id [0-9a-f]{32}$/m.test(describedCommitText),
      },
    };
  } finally {
    offProgress();
    await jj.dispose().catch(() => undefined);
  }
}

async function runLockCheck(suffix) {
  const fsName = `jj-wasm-lock-${suffix}`;
  const holder = createJjWorkerClient({ requestTimeoutMs: 30_000 });
  const contender = createJjWorkerClient({ requestTimeoutMs: 30_000 });
  try {
    await holder.init({ fsName, dir: "/repo", defaultBranch: "main" });
    const error = await captureError(() =>
      contender.init({ fsName, dir: "/repo", defaultBranch: "main" }),
    );
    return {
      error,
    };
  } finally {
    await contender.dispose().catch(() => undefined);
    await holder.dispose().catch(() => undefined);
  }
}

async function captureError(callback) {
  try {
    await callback();
    return null;
  } catch (error) {
    return {
      name: error?.name ?? "Error",
      message: error?.message ?? String(error),
      code: error?.code,
    };
  }
}

function decodeCommitObject(wrapped) {
  const bytes = wrapped instanceof Uint8Array ? wrapped : new Uint8Array(wrapped);
  const nulIndex = bytes.indexOf(0);
  const payload = nulIndex === -1 ? bytes : bytes.slice(nulIndex + 1);
  return new TextDecoder().decode(payload);
}
