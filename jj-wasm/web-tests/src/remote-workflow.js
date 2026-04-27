import { createJjWorkerClient } from "../../../npm/jj-wasm/worker-client.js";

const AUTHOR = {
  name: "jj-wasm remote",
  email: "jj-wasm-remote@example.invalid",
};

export async function runRemoteWorkflow({ remoteUrl, controlUrl, pushRef }) {
  const suffix = globalThis.crypto?.randomUUID?.() ?? Date.now();
  const progressEvents = [];
  const jj = createJjWorkerClient({ requestTimeoutMs: 45_000 });
  const offProgress = jj.onProgress((event) => progressEvents.push(event));

  try {
    await jj.clone(remoteUrl, {
      fsName: `jj-wasm-remote-${suffix}`,
      dir: "/repo",
      ref: "main",
      singleBranch: true,
    });

    const statusAfterClone = await jj.status();
    const logAfterClone = await jj.log({ ref: "refs/heads/main", limit: 5 });
    const headRefs = await jj.listRefs("refs/heads");
    const headId = await jj.resolveRef("refs/heads/main");
    const rawHead = await jj.readRawObject(headId);

    const secondCommit = await addFixtureCommit(controlUrl);
    const fetchResult = await jj.fetch({
      url: remoteUrl,
      ref: "main",
      remoteRef: "main",
      singleBranch: true,
    });
    const fetchedHeadId = await jj.resolveRef("refs/remotes/origin/main");

    await jj.writeFile("PUSH.md", "created in the browser worker\n");
    const snapshot = await jj.snapshot({
      message: "browser remote push commit",
      author: AUTHOR,
    });
    const snapshotRaw = await jj.readRawObject(snapshot.commitId);
    const snapshotCommitText = decodeCommitObject(snapshotRaw.wrapped);
    const pushResult = await jj.push({
      url: remoteUrl,
      ref: "main",
      remoteRef: pushRef,
    });
    const pushedRefId = await resolveFixtureRef(controlUrl, pushRef);
    const afterPushStatus = await jj.status();

    return {
      progressEvents,
      statusAfterClone,
      logAfterClone,
      headRefs,
      headId,
      rawHeadKind: rawHead.kind,
      secondCommit,
      fetchResult,
      fetchedHeadId,
      snapshot,
      snapshotRawKind: snapshotRaw.kind,
      snapshotHasChangeIdHeader: /^change-id [0-9a-f]{32}$/m.test(snapshotCommitText),
      pushResult,
      pushRef,
      pushedRefId,
      afterPushStatus,
    };
  } finally {
    offProgress();
    await jj.dispose().catch(() => undefined);
  }
}

async function addFixtureCommit(controlUrl) {
  const response = await fetch(`${controlUrl}/add-commit`, { method: "POST" });
  const body = await response.json();
  if (!response.ok) {
    throw new Error(`fixture add-commit failed: ${body.error ?? response.statusText}`);
  }
  return body.oid;
}

async function resolveFixtureRef(controlUrl, ref) {
  const url = new URL(`${controlUrl}/resolve`);
  url.searchParams.set("ref", ref);
  const response = await fetch(url);
  const body = await response.json();
  if (!response.ok) {
    throw new Error(`fixture resolve failed: ${body.error ?? response.statusText}`);
  }
  return body.oid;
}

function decodeCommitObject(wrapped) {
  const bytes = wrapped instanceof Uint8Array ? wrapped : new Uint8Array(wrapped);
  const nulIndex = bytes.indexOf(0);
  const payload = nulIndex === -1 ? bytes : bytes.slice(nulIndex + 1);
  return new TextDecoder().decode(payload);
}
