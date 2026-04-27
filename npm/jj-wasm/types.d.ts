import type { JjSession } from "./pkg/jj_wasm.js";

export type CommitId = string;
export type ChangeId = string;
export type Revset = "@" | "HEAD" | "::" | "all()" | string;

export interface CommitPerson {
  name: string;
  email: string;
}

export interface JjSessionOptions {
  git: unknown;
  fs: unknown;
  http?: unknown;
  cache?: unknown;
  dir: string;
  gitdir?: string;
  defaultBranch?: string;
  noCheckout?: boolean;
  corsProxy?: string;
  ref?: string;
  singleBranch?: boolean;
  depth?: number;
}

export interface WorkerSessionOptions {
  fsName?: string;
  lockName?: string;
  dir: string;
  gitdir?: string;
  defaultBranch?: string;
  noCheckout?: boolean;
  corsProxy?: string;
  ref?: string;
  singleBranch?: boolean;
  depth?: number;
}

export interface CloneOptions extends WorkerSessionOptions {
  url: string;
}

export interface LogOptions {
  revset?: Revset;
  ref?: string;
  limit?: number;
}

export interface SnapshotOptions {
  message?: string;
  author?: CommitPerson;
  committer?: CommitPerson;
}

export interface DescribeOptions {
  rev?: Revset;
  ref?: string;
  message: string;
  amend?: boolean;
  author?: CommitPerson;
  committer?: CommitPerson;
}

export interface NewOptions {
  revisions?: Revset[];
  description?: string;
}

export interface RestoreOptions {
  source?: Revset;
  from?: Revset;
  paths?: string[];
  filepaths?: string[];
  force?: boolean;
  snapshot?: boolean;
  message?: string;
  author?: CommitPerson;
  committer?: CommitPerson;
}

export interface FetchOptions {
  url?: string;
  remote?: string;
  corsProxy?: string;
  ref?: string;
  singleBranch?: boolean;
  depth?: number;
}

export interface PushOptions {
  url?: string;
  remote?: string;
  corsProxy?: string;
  ref?: string;
}

export interface StatusFile {
  path: string;
  status: "absent" | "added" | "deleted" | "modified" | "staged" | "clean";
  head: number;
  workdir: number;
  stage: number;
}

export interface StatusSummary {
  added: number;
  modified: number;
  deleted: number;
  staged: number;
  clean: number;
}

export interface ChangeRecord {
  id: ChangeId;
  description: string;
  parents: CommitId[];
  commitId?: CommitId | null;
  hidden: boolean;
}

export interface RepoState {
  schemaVersion: number;
  head?: CommitId | null;
  currentChangeId?: ChangeId | null;
  workingCopyCommit?: CommitId | null;
  changes: ChangeRecord[];
}

export interface StatusResult {
  currentChange?: ChangeRecord | null;
  head?: CommitId | null;
  changedFiles: StatusFile[];
  summary: StatusSummary;
}

export interface OperationRecord {
  id: string;
  kind: string;
  timestampMs: number;
  detail: Record<string, unknown>;
  beforeState?: RepoState | null;
  afterState?: RepoState | null;
  previousHead?: CommitId | null;
  newHead?: CommitId | null;
}

export interface SnapshotResult {
  operation: OperationRecord;
  commitId?: CommitId | null;
  currentChange?: ChangeRecord | null;
  changedFiles: StatusFile[];
}

export interface DescribeResult {
  operation: OperationRecord;
  currentChange?: ChangeRecord | null;
  commitId?: CommitId | null;
}

export interface NewResult {
  operation: OperationRecord;
  currentChange: ChangeRecord;
}

export type RestoreResult = SnapshotResult | { operation: OperationRecord };

export interface UndoResult {
  operation: OperationRecord;
  state: RepoState;
}

export interface RawObject {
  oid: CommitId;
  kind: "blob" | "commit" | "tree" | "tag" | string;
  wrapped: Uint8Array;
}

export interface LogResult {
  commits: unknown[];
  currentChange?: ChangeRecord | null;
  head?: CommitId | null;
}

export interface WorkerProgressEvent {
  method: string;
  phase: "start" | "done" | "error";
  detail?: unknown;
}

export interface WorkerError {
  name: string;
  message: string;
  stack?: string;
  code?: string;
  caller?: string;
  data?: unknown;
}

export interface JjWorkerClient {
  init(options: WorkerSessionOptions): Promise<{ dir: string; gitdir?: string }>;
  open(options: WorkerSessionOptions): Promise<{ dir: string; gitdir?: string }>;
  clone(url: string, options: WorkerSessionOptions): Promise<{ dir: string; gitdir?: string }>;
  fetch(options?: FetchOptions): Promise<unknown>;
  push(options?: PushOptions): Promise<unknown>;
  log(options?: LogOptions): Promise<LogResult>;
  status(): Promise<StatusResult>;
  snapshot(options?: SnapshotOptions): Promise<SnapshotResult>;
  describe(options: DescribeOptions): Promise<DescribeResult>;
  new(options?: NewOptions): Promise<NewResult>;
  rebase(options?: unknown): Promise<unknown>;
  restore(options?: RestoreOptions): Promise<RestoreResult>;
  opLog(): Promise<OperationRecord[]>;
  undo(): Promise<UndoResult>;
  listRefs(refPrefix?: string | null): Promise<unknown>;
  resolveRef(refName: string): Promise<CommitId>;
  writeRef(refName: string, oid: CommitId): Promise<unknown>;
  readRawObject(oid: CommitId): Promise<RawObject>;
  writeRawObject(objectType: string, wrapped: Uint8Array): Promise<CommitId>;
  writeFile(path: string, contents: string | Uint8Array): Promise<void>;
  readFile(path: string, encoding?: string): Promise<string | Uint8Array>;
  removeFile(path: string): Promise<void>;
  mkdir(path: string): Promise<void>;
  onProgress(listener: (event: WorkerProgressEvent) => void): () => void;
  dispose(): Promise<void>;
}

export interface CreateWorkerClientOptions {
  worker?: Worker;
  workerUrl?: string | URL;
  name?: string;
  requestTimeoutMs?: number;
}

export interface CreateSessionOptions extends JjSessionOptions {
  mode?: "open" | "init" | "clone";
  url?: string;
  wasm?: Parameters<typeof import("./pkg/jj_wasm.js").default>[0];
}

export type DirectJjSession = JjSession;
