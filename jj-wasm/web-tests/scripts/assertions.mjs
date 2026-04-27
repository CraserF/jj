import assert from "node:assert/strict";

const COMMIT_ID = /^[0-9a-f]{40}$/;

export function assertWorkflowResult(result, label) {
  assert.equal(result.initialStatus.summary.added, 1, `${label}: README should start as added`);
  assertNoVcsPaths(result.initialStatus, label);
  assert.match(result.snapshot.commitId, COMMIT_ID, `${label}: snapshot should create a commit`);
  assert.equal(
    result.afterSnapshotStatus.changedFiles.length,
    0,
    `${label}: snapshot should clear working-copy changes`,
  );
  assert.match(result.described.commitId, COMMIT_ID, `${label}: describe should create a commit`);
  assert.notEqual(
    result.described.commitId,
    result.snapshot.commitId,
    `${label}: describe should rewrite the current commit`,
  );
  assert.equal(
    result.rawCommitMetadata.kind,
    "commit",
    `${label}: raw described object should be a commit`,
  );
  assert.equal(
    result.rawCommitMetadata.hasChangeIdHeader,
    true,
    `${label}: described commit should preserve a change-id header`,
  );
  assert.equal(
    result.modifiedStatus.summary.modified,
    1,
    `${label}: editing README should produce one modified file`,
  );
  assert.deepEqual(
    result.nextChange.currentChange.parents,
    [result.described.commitId],
    `${label}: new change should use described commit as parent`,
  );
  assert.equal(result.restored.operation.kind, "restore", `${label}: restore op should be recorded`);
  assert.equal(result.undo.operation.kind, "undo", `${label}: undo op should be recorded`);
  assert.equal(
    result.reopenedStatus?.currentChange?.id ?? result.undo.state.currentChangeId,
    result.undo.state.currentChangeId,
    `${label}: open should reload the undo-restored current change`,
  );
  assert.deepEqual(
    result.opLog.map((operation) => operation.kind),
    ["init", "snapshot", "describe", "new", "restore", "undo"],
    `${label}: op log should record the core workflow in order`,
  );
  assert.deepEqual(
    result.reopenedOpLog?.map((operation) => operation.kind) ?? result.opLog.map((operation) => operation.kind),
    result.opLog.map((operation) => operation.kind),
    `${label}: op log should persist across open`,
  );
  assert.ok(Array.isArray(result.log.commits), `${label}: log should return a commits array`);
}

export function assertWorkerWorkflowResult(result) {
  assertWorkflowResult(result, "worker");
  assert.equal(result.lockResult.error?.code, "REPO_LOCKED", "worker should reject a second session");
  assert.equal(
    result.rebaseError?.code,
    "UNSUPPORTED_OPERATION",
    "rebase should be explicitly unsupported",
  );
  assert.equal(
    result.restoredText,
    "hello from jj-wasm worker\n",
    "restore should write README content from HEAD",
  );
  assertProgress(result.progressEvents, "init");
  assertProgress(result.progressEvents, "snapshot");
}

export function assertPackageSmokeResult(result, label) {
  assert.equal(result.initialStatus.summary.added, 1, `${label}: README should start as added`);
  assert.match(result.commitId, COMMIT_ID, `${label}: snapshot should create a commit`);
  assert.equal(result.afterSnapshotStatus.changedFiles.length, 0, `${label}: snapshot should clear status`);
  assert.equal(result.rawKind, "commit", `${label}: package should read raw commit objects`);
  assertProgress(result.progressEvents, "init");
  assertProgress(result.progressEvents, "snapshot");
}

function assertNoVcsPaths(status, label) {
  for (const file of status.changedFiles) {
    assert.equal(
      file.path === ".jj" ||
        file.path.startsWith(".jj/") ||
        file.path === ".git" ||
        file.path.startsWith(".git/"),
      false,
      `${label}: ${file.path} should not appear in status`,
    );
  }
}

function assertProgress(events, method) {
  assert.ok(
    events.some((event) => event.method === method && event.phase === "start"),
    `${method} should emit start progress`,
  );
  assert.ok(
    events.some((event) => event.method === method && event.phase === "done"),
    `${method} should emit done progress`,
  );
}
