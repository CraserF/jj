// Copyright 2026 The Jujutsu Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use serde::Deserialize;
use serde::Serialize;
use std::fmt::Write as _;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::browser_fs::BrowserFs;
use crate::isomorphic_git_backend::IsomorphicGitBackend;
use crate::js_util;

const JJ_WASM_DIR: &str = ".jj/wasm";
const SESSION_FILE: &str = "session.json";
const OP_LOG_FILE: &str = "operations.json";
const STATE_FILE: &str = "state.json";
const CURRENT_SCHEMA_VERSION: u32 = 1;

#[wasm_bindgen]
pub struct JjSession {
    backend: IsomorphicGitBackend,
    metadata_dir: String,
}

#[wasm_bindgen]
impl JjSession {
    #[wasm_bindgen(js_name = init)]
    pub async fn init(options: JsValue) -> Result<Self, JsValue> {
        let options = SessionOptions::from_js(options)?;
        options.fs.mkdirp(&options.dir).await?;
        let backend = options.backend();
        backend
            .init(options.default_branch.as_deref(), options.no_checkout)
            .await?;
        let session = Self::from_options(options);
        session.write_session_file("init").await?;
        let state = session.initial_state().await?;
        session.write_state(&state).await?;
        session
            .record_operation("init", serde_json::json!({}), None, Some(state), None, None)
            .await?;
        Ok(session)
    }

    #[wasm_bindgen(js_name = open)]
    pub async fn open(options: JsValue) -> Result<Self, JsValue> {
        let options = SessionOptions::from_js(options)?;
        let session = Self::from_options(options);
        session.ensure_metadata_dir().await?;
        let state = session.read_state_or_default().await?;
        session.write_state(&state).await?;
        Ok(session)
    }

    #[wasm_bindgen(js_name = clone)]
    pub async fn clone_repo(url: String, options: JsValue) -> Result<Self, JsValue> {
        let options = SessionOptions::from_js(options)?;
        options.fs.mkdirp(&options.dir).await?;
        let backend = options.backend();
        backend
            .clone_repo(
                &url,
                options.cors_proxy.as_deref(),
                options.ref_name.as_deref(),
                options.single_branch,
                options.depth,
            )
            .await?;
        let session = Self::from_options(options);
        session.write_session_file("clone").await?;
        let state = session.initial_state().await?;
        session.write_state(&state).await?;
        session
            .record_operation(
                "clone",
                serde_json::json!({ "url": url }),
                None,
                Some(state),
                None,
                None,
            )
            .await?;
        Ok(session)
    }

    pub async fn fetch(&self, options: JsValue) -> Result<JsValue, JsValue> {
        let options = FetchOptions::from_js(options)?;
        let result = self
            .backend
            .fetch(
                options.url.as_deref(),
                options.remote.as_deref(),
                options.cors_proxy.as_deref(),
                options.ref_name.as_deref(),
                options.single_branch,
                options.depth,
            )
            .await?;
        let before_state = self.read_state_or_default().await?;
        let mut after_state = before_state.clone();
        after_state.head = self.resolve_head().await.ok().flatten();
        self.write_state(&after_state).await?;
        self.record_operation(
            "fetch",
            serde_json::json!({
                "url": options.url,
                "remote": options.remote,
                "ref": options.ref_name,
            }),
            Some(before_state),
            Some(after_state),
            None,
            None,
        )
        .await?;
        Ok(result)
    }

    pub async fn push(&self, options: JsValue) -> Result<JsValue, JsValue> {
        let options = PushOptions::from_js(options)?;
        let result = self
            .backend
            .push(
                options.remote.as_deref(),
                options.url.as_deref(),
                options.ref_name.as_deref(),
                options.cors_proxy.as_deref(),
            )
            .await?;
        let before_state = self.read_state_or_default().await?;
        let mut after_state = before_state.clone();
        after_state.head = self.resolve_head().await.ok().flatten();
        self.write_state(&after_state).await?;
        self.record_operation(
            "push",
            serde_json::json!({
                "remote": options.remote,
                "url": options.url,
                "ref": options.ref_name,
            }),
            Some(before_state),
            Some(after_state),
            None,
            None,
        )
        .await?;
        Ok(result)
    }

    pub async fn log(&self, options: JsValue) -> Result<JsValue, JsValue> {
        let options = LogOptions::from_js(options)?;
        let ref_name = log_ref_from_options(&options);
        let log = self.backend.log(ref_name.as_deref(), options.limit).await?;
        let state = self.read_state_or_default().await?;
        js_util::to_js(&serde_json::json!({
            "commits": serde_wasm_bindgen::from_value::<serde_json::Value>(log)
                .unwrap_or(serde_json::Value::Null),
            "currentChange": state.current_change(),
            "head": state.head,
        }))
    }

    pub async fn status(&self) -> Result<JsValue, JsValue> {
        let status_matrix = self.backend.status_matrix().await?;
        let rows = status_rows_from_js(status_matrix)?;
        let state = self.read_state_or_default().await?;
        let visible_rows = rows
            .iter()
            .filter(|row| !is_vcs_metadata_path(&row.filepath))
            .cloned()
            .collect::<Vec<_>>();
        let changed_files = rows
            .iter()
            .filter(|row| !is_vcs_metadata_path(&row.filepath))
            .filter(|row| row.head != row.workdir || row.workdir != row.stage)
            .map(StatusFile::from)
            .collect::<Vec<_>>();
        let value = BrowserStatus {
            current_change: state.current_change(),
            head: state.head,
            changed_files,
            summary: StatusSummary::from_rows(&visible_rows),
        };
        js_util::to_js(&value)
    }

    pub async fn snapshot(&self, options: JsValue) -> Result<JsValue, JsValue> {
        let options = SnapshotOptions::from_js(options)?;
        let author = options.author.unwrap_or_default();
        let result = self
            .snapshot_internal(
                "snapshot",
                options.message.as_deref(),
                &author,
                options.committer.as_ref(),
            )
            .await?;
        js_util::to_js(&result)
    }

    pub async fn describe(&self, options: JsValue) -> Result<JsValue, JsValue> {
        let options = DescribeOptions::from_js(options)?;
        let before_state = self.read_state_or_default().await?;
        let previous_head = self.resolve_head().await.ok().flatten();
        let mut after_state = before_state.clone();
        after_state.ensure_current_change(previous_head.clone());
        after_state.describe_current_change(options.message.clone());

        let author = options.author.unwrap_or_default();
        let new_head = if options.amend.unwrap_or(true) && previous_head.is_some() {
            let commit_id = self
                .backend
                .commit(
                    &options.message,
                    &js_util::to_js(&author)?,
                    options
                        .committer
                        .as_ref()
                        .map(js_util::to_js)
                        .transpose()?
                        .as_ref(),
                    true,
                    options.ref_name.as_deref(),
                    &[],
                )
                .await?;
            let commit_id = self
                .write_change_id_header(commit_id, after_state.current_change_id.as_deref())
                .await?;
            if let Some(ref_name) = options.ref_name.as_deref() {
                self.backend.write_ref(ref_name, &commit_id).await?;
            } else {
                self.reset_current_ref(&commit_id).await?;
            }
            after_state.set_current_commit(commit_id.clone());
            after_state.head = Some(commit_id.clone());
            Some(commit_id)
        } else {
            after_state.head = previous_head.clone();
            None
        };

        self.write_state(&after_state).await?;
        let operation = self
            .record_operation(
                "describe",
                serde_json::json!({
                    "message": options.message,
                    "rev": options.rev,
                    "amend": options.amend.unwrap_or(true),
                }),
                Some(before_state),
                Some(after_state.clone()),
                previous_head,
                new_head.clone(),
            )
            .await?;
        js_util::to_js(&serde_json::json!({
            "operation": operation,
            "currentChange": after_state.current_change(),
            "commitId": new_head,
        }))
    }

    #[wasm_bindgen(js_name = new)]
    pub async fn new_change(&self, options: JsValue) -> Result<JsValue, JsValue> {
        let options = NewOptions::from_js(options)?;
        let before_state = self.read_state_or_default().await?;
        let previous_head = self.resolve_head().await.ok().flatten();
        let parents = if options.revisions.is_empty() {
            previous_head.iter().cloned().collect()
        } else {
            self.resolve_revisions(&options.revisions).await?
        };
        let mut after_state = before_state.clone();
        let change = ChangeRecord::new(
            new_change_id("change"),
            options.description.unwrap_or_default(),
            parents,
            None,
        );
        after_state.current_change_id = Some(change.id.clone());
        after_state.changes.push(change.clone());
        after_state.head = previous_head.clone();
        self.write_state(&after_state).await?;
        let operation = self
            .record_operation(
                "new",
                serde_json::json!({ "revisions": options.revisions }),
                Some(before_state),
                Some(after_state),
                previous_head,
                None,
            )
            .await?;
        js_util::to_js(&serde_json::json!({
            "operation": operation,
            "currentChange": change,
        }))
    }

    pub async fn rebase(&self, _options: JsValue) -> Result<JsValue, JsValue> {
        Err(js_util::unsupported("rebase"))
    }

    pub async fn restore(&self, options: JsValue) -> Result<JsValue, JsValue> {
        let options = RestoreOptions::from_js(options)?;
        self.backend
            .checkout_paths(
                options.source.as_deref(),
                &options.paths,
                options.force.unwrap_or(true),
            )
            .await?;
        if options.snapshot.unwrap_or(true) {
            let author = options.author.unwrap_or_default();
            let message = options
                .message
                .unwrap_or_else(|| format!("restore {}", options.paths.join(" ")));
            let result = self
                .snapshot_internal(
                    "restore",
                    Some(&message),
                    &author,
                    options.committer.as_ref(),
                )
                .await?;
            js_util::to_js(&result)
        } else {
            let before_state = self.read_state_or_default().await?;
            let mut after_state = before_state.clone();
            after_state.head = self.resolve_head().await.ok().flatten();
            self.write_state(&after_state).await?;
            let operation = self
                .record_operation(
                    "restore",
                    serde_json::json!({
                        "source": options.source,
                        "paths": options.paths,
                        "snapshot": false,
                    }),
                    Some(before_state),
                    Some(after_state),
                    None,
                    None,
                )
                .await?;
            js_util::to_js(&serde_json::json!({ "operation": operation }))
        }
    }

    #[wasm_bindgen(js_name = opLog)]
    pub async fn op_log(&self) -> Result<JsValue, JsValue> {
        let operations = self.read_operations_or_empty().await?;
        js_util::to_js(&operations)
    }

    pub async fn undo(&self) -> Result<JsValue, JsValue> {
        let operations = self.read_operations_or_empty().await?;
        let Some(operation_to_undo) = operations
            .iter()
            .rev()
            .find(|operation| operation.before_state.is_some())
            .cloned()
        else {
            return Err(js_util::error("no undoable operation found"));
        };
        let before_undo = self.read_state_or_default().await?;
        let restore_state = operation_to_undo.before_state.clone().unwrap();
        if let Some(previous_head) = operation_to_undo.previous_head.as_deref() {
            self.reset_current_ref(previous_head).await?;
        }
        self.write_state(&restore_state).await?;
        let undo_operation = self
            .record_operation(
                "undo",
                serde_json::json!({ "undid": operation_to_undo.id }),
                Some(before_undo),
                Some(restore_state.clone()),
                operation_to_undo.new_head,
                operation_to_undo.previous_head,
            )
            .await?;
        js_util::to_js(&serde_json::json!({
            "operation": undo_operation,
            "state": restore_state,
        }))
    }

    #[wasm_bindgen(js_name = listRefs)]
    pub async fn list_refs(&self, ref_prefix: Option<String>) -> Result<JsValue, JsValue> {
        self.backend.list_refs(ref_prefix.as_deref()).await
    }

    #[wasm_bindgen(js_name = resolveRef)]
    pub async fn resolve_ref(&self, ref_name: String) -> Result<JsValue, JsValue> {
        self.backend.resolve_ref(&ref_name).await
    }

    #[wasm_bindgen(js_name = writeRef)]
    pub async fn write_ref(&self, ref_name: String, oid: String) -> Result<JsValue, JsValue> {
        let before_state = self.read_state_or_default().await?;
        let result = self.backend.write_ref(&ref_name, &oid).await?;
        let mut after_state = before_state.clone();
        after_state.head = self.resolve_head().await.ok().flatten();
        self.write_state(&after_state).await?;
        self.record_operation(
            "write-ref",
            serde_json::json!({
                "ref": ref_name,
                "oid": oid,
            }),
            Some(before_state),
            Some(after_state),
            None,
            None,
        )
        .await?;
        Ok(result)
    }

    #[wasm_bindgen(js_name = readRawObject)]
    pub async fn read_raw_object(&self, oid: String) -> Result<JsValue, JsValue> {
        let raw = self.backend.read_raw_object(&oid).await?;
        js_util::to_js(&RawObjectForJs {
            oid: raw.oid,
            kind: raw.kind,
            wrapped: raw.wrapped,
        })
    }

    #[wasm_bindgen(js_name = writeRawObject)]
    pub async fn write_raw_object(
        &self,
        object_type: String,
        wrapped: Vec<u8>,
    ) -> Result<String, JsValue> {
        self.backend.write_raw_object(&object_type, &wrapped).await
    }

    #[wasm_bindgen(getter)]
    pub fn dir(&self) -> String {
        self.backend.dir().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn gitdir(&self) -> Option<String> {
        self.backend.gitdir().map(str::to_owned)
    }
}

impl JjSession {
    fn from_options(options: SessionOptions) -> Self {
        let metadata_dir = js_util::join_path(&options.dir, JJ_WASM_DIR);
        Self {
            backend: options.backend(),
            metadata_dir,
        }
    }

    async fn ensure_metadata_dir(&self) -> Result<(), JsValue> {
        self.backend.fs().mkdirp(&self.metadata_dir).await
    }

    async fn write_session_file(&self, created_by: &str) -> Result<(), JsValue> {
        self.ensure_metadata_dir().await?;
        let metadata = SessionMetadata {
            created_by: created_by.to_string(),
            dir: self.backend.dir().to_string(),
            gitdir: self.backend.gitdir().map(str::to_owned),
            schema_version: CURRENT_SCHEMA_VERSION,
        };
        self.backend
            .fs()
            .write_json(
                &js_util::join_path(&self.metadata_dir, SESSION_FILE),
                &metadata,
            )
            .await
    }

    async fn initial_state(&self) -> Result<RepoState, JsValue> {
        let head = self.resolve_head().await.ok().flatten();
        let mut state = RepoState {
            schema_version: CURRENT_SCHEMA_VERSION,
            head: head.clone(),
            ..RepoState::default()
        };
        state.ensure_current_change(head);
        Ok(state)
    }

    async fn read_state_or_default(&self) -> Result<RepoState, JsValue> {
        let path = js_util::join_path(&self.metadata_dir, STATE_FILE);
        match self.backend.fs().read_text(&path).await {
            Ok(text) => {
                let state =
                    serde_json::from_str(&text).map_err(|err| corrupt_state_error(&path, err))?;
                migrate_state(state, &path)
            }
            Err(_) => self.initial_state().await,
        }
    }

    async fn write_state(&self, state: &RepoState) -> Result<(), JsValue> {
        self.ensure_metadata_dir().await?;
        self.backend
            .fs()
            .write_json(&js_util::join_path(&self.metadata_dir, STATE_FILE), state)
            .await
    }

    async fn resolve_head(&self) -> Result<Option<String>, JsValue> {
        match self.backend.resolve_ref("HEAD").await {
            Ok(value) => Ok(value.as_string()),
            Err(_) => Ok(None),
        }
    }

    async fn resolve_revisions(&self, revisions: &[String]) -> Result<Vec<String>, JsValue> {
        let mut resolved = Vec::new();
        for revision in revisions {
            if revision.len() == 40 && revision.bytes().all(|b| b.is_ascii_hexdigit()) {
                resolved.push(revision.clone());
            } else if matches!(revision.as_str(), "@" | "HEAD") {
                let value = self.backend.resolve_ref("HEAD").await?;
                let id = value.as_string().ok_or_else(|| {
                    js_util::error(format!("revision `{revision}` did not resolve to an id"))
                })?;
                resolved.push(id);
            } else {
                let value = self.backend.resolve_ref(revision).await?;
                let id = value.as_string().ok_or_else(|| {
                    js_util::error(format!("revision `{revision}` did not resolve to an id"))
                })?;
                resolved.push(id);
            }
        }
        Ok(resolved)
    }

    async fn reset_current_ref(&self, oid: &str) -> Result<(), JsValue> {
        match self.backend.current_branch().await? {
            Some(branch) => {
                self.backend
                    .write_ref(&format!("refs/heads/{branch}"), oid)
                    .await?;
            }
            None => {
                self.backend.write_ref("HEAD", oid).await?;
            }
        }
        Ok(())
    }

    async fn snapshot_internal(
        &self,
        operation_kind: &str,
        message: Option<&str>,
        author: &CommitPerson,
        committer: Option<&CommitPerson>,
    ) -> Result<SnapshotResult, JsValue> {
        let before_state = self.read_state_or_default().await?;
        let previous_head = self.resolve_head().await.ok().flatten();
        let status_matrix = self.backend.status_matrix().await?;
        let rows = status_rows_from_js(status_matrix)?;
        let changed_rows = rows
            .iter()
            .filter(|row| !is_vcs_metadata_path(&row.filepath))
            .filter(|row| row.head != row.workdir)
            .cloned()
            .collect::<Vec<_>>();

        for row in &changed_rows {
            if row.workdir == 0 {
                self.backend.remove(&row.filepath).await?;
            } else {
                self.backend.add(&row.filepath).await?;
            }
        }

        let mut after_state = before_state.clone();
        after_state.ensure_current_change(previous_head.clone());

        let commit_id = if changed_rows.is_empty() {
            previous_head.clone()
        } else {
            let message = message
                .map(str::to_owned)
                .or_else(|| {
                    after_state
                        .current_change()
                        .map(|change| change.description)
                        .filter(|description| !description.is_empty())
                })
                .unwrap_or_else(|| "snapshot".to_string());
            let mut commit_id = self
                .backend
                .commit(
                    &message,
                    &js_util::to_js(author)?,
                    committer.map(js_util::to_js).transpose()?.as_ref(),
                    false,
                    None,
                    &[],
                )
                .await?;
            commit_id = self
                .write_change_id_header(commit_id, after_state.current_change_id.as_deref())
                .await?;
            self.reset_current_ref(&commit_id).await?;
            after_state.set_current_commit(commit_id.clone());
            after_state.head = Some(commit_id.clone());
            Some(commit_id)
        };

        self.write_state(&after_state).await?;
        let changed_files = changed_rows
            .iter()
            .map(StatusFile::from)
            .collect::<Vec<_>>();
        let operation = self
            .record_operation(
                operation_kind,
                serde_json::json!({
                    "message": message,
                    "changedFiles": changed_files,
                }),
                Some(before_state),
                Some(after_state.clone()),
                previous_head,
                commit_id.clone(),
            )
            .await?;
        Ok(SnapshotResult {
            operation,
            commit_id,
            current_change: after_state.current_change(),
            changed_files,
        })
    }

    async fn read_operations_or_empty(&self) -> Result<Vec<OperationRecord>, JsValue> {
        let path = js_util::join_path(&self.metadata_dir, OP_LOG_FILE);
        match self.backend.fs().read_text(&path).await {
            Ok(text) => serde_json::from_str(&text).map_err(|err| corrupt_state_error(&path, err)),
            Err(_) => Ok(Vec::new()),
        }
    }

    async fn write_change_id_header(
        &self,
        commit_id: String,
        change_id: Option<&str>,
    ) -> Result<String, JsValue> {
        let Some(change_id) = change_id else {
            return Ok(commit_id);
        };
        let header_value = normalize_change_id_header(change_id);
        self.backend
            .rewrite_commit_header(&commit_id, "change-id", &header_value)
            .await
    }

    async fn record_operation(
        &self,
        kind: &str,
        detail: serde_json::Value,
        before_state: Option<RepoState>,
        after_state: Option<RepoState>,
        previous_head: Option<String>,
        new_head: Option<String>,
    ) -> Result<OperationRecord, JsValue> {
        self.ensure_metadata_dir().await?;
        let mut operations = self.read_operations_or_empty().await?;
        let timestamp_ms = js_sys::Date::now();
        let operation = OperationRecord {
            id: format!("wasm-{timestamp_ms:.0}-{}", operations.len()),
            kind: kind.to_string(),
            timestamp_ms,
            detail,
            before_state,
            after_state,
            previous_head,
            new_head,
        };
        operations.push(operation.clone());
        self.backend
            .fs()
            .write_json(
                &js_util::join_path(&self.metadata_dir, OP_LOG_FILE),
                &operations,
            )
            .await?;
        Ok(operation)
    }
}

#[derive(Clone)]
struct SessionOptions {
    git: JsValue,
    fs: BrowserFs,
    http: Option<JsValue>,
    cache: Option<JsValue>,
    dir: String,
    gitdir: Option<String>,
    default_branch: Option<String>,
    no_checkout: Option<bool>,
    cors_proxy: Option<String>,
    ref_name: Option<String>,
    single_branch: Option<bool>,
    depth: Option<u32>,
}

impl SessionOptions {
    fn from_js(options: JsValue) -> Result<Self, JsValue> {
        let git = js_util::required_prop(&options, "git")?;
        let fs = BrowserFs::new(js_util::required_prop(&options, "fs")?)?;
        Ok(Self {
            git,
            fs,
            http: js_util::optional_prop(&options, "http")?,
            cache: js_util::optional_prop(&options, "cache")?,
            dir: js_util::required_string(&options, "dir")?,
            gitdir: js_util::optional_string(&options, "gitdir")?,
            default_branch: js_util::optional_string(&options, "defaultBranch")?,
            no_checkout: js_util::optional_bool(&options, "noCheckout")?,
            cors_proxy: js_util::optional_string(&options, "corsProxy")?,
            ref_name: js_util::optional_string(&options, "ref")?,
            single_branch: js_util::optional_bool(&options, "singleBranch")?,
            depth: js_util::optional_u32(&options, "depth")?,
        })
    }

    fn backend(&self) -> IsomorphicGitBackend {
        IsomorphicGitBackend::new(
            self.git.clone(),
            self.fs.clone(),
            self.dir.clone(),
            self.gitdir.clone(),
            self.http.clone(),
            self.cache.clone(),
        )
    }
}

#[derive(Default)]
struct FetchOptions {
    url: Option<String>,
    remote: Option<String>,
    cors_proxy: Option<String>,
    ref_name: Option<String>,
    single_branch: Option<bool>,
    depth: Option<u32>,
}

impl FetchOptions {
    fn from_js(options: JsValue) -> Result<Self, JsValue> {
        Ok(Self {
            url: js_util::optional_string(&options, "url")?,
            remote: js_util::optional_string(&options, "remote")?,
            cors_proxy: js_util::optional_string(&options, "corsProxy")?,
            ref_name: js_util::optional_string(&options, "ref")?,
            single_branch: js_util::optional_bool(&options, "singleBranch")?,
            depth: js_util::optional_u32(&options, "depth")?,
        })
    }
}

#[derive(Default)]
struct PushOptions {
    url: Option<String>,
    remote: Option<String>,
    cors_proxy: Option<String>,
    ref_name: Option<String>,
}

impl PushOptions {
    fn from_js(options: JsValue) -> Result<Self, JsValue> {
        Ok(Self {
            url: js_util::optional_string(&options, "url")?,
            remote: js_util::optional_string(&options, "remote")?,
            cors_proxy: js_util::optional_string(&options, "corsProxy")?,
            ref_name: js_util::optional_string(&options, "ref")?,
        })
    }
}

#[derive(Default)]
struct LogOptions {
    revset: Option<String>,
    ref_name: Option<String>,
    limit: Option<u32>,
}

impl LogOptions {
    fn from_js(options: JsValue) -> Result<Self, JsValue> {
        Ok(Self {
            revset: js_util::optional_string(&options, "revset")?,
            ref_name: js_util::optional_string(&options, "ref")?,
            limit: js_util::optional_u32(&options, "limit")?,
        })
    }
}

#[derive(Default)]
struct SnapshotOptions {
    message: Option<String>,
    author: Option<CommitPerson>,
    committer: Option<CommitPerson>,
}

impl SnapshotOptions {
    fn from_js(options: JsValue) -> Result<Self, JsValue> {
        Ok(Self {
            message: js_util::optional_string(&options, "message")?,
            author: optional_person(&options, "author")?,
            committer: optional_person(&options, "committer")?,
        })
    }
}

#[derive(Default)]
struct DescribeOptions {
    rev: Option<String>,
    ref_name: Option<String>,
    message: String,
    amend: Option<bool>,
    author: Option<CommitPerson>,
    committer: Option<CommitPerson>,
}

impl DescribeOptions {
    fn from_js(options: JsValue) -> Result<Self, JsValue> {
        Ok(Self {
            rev: js_util::optional_string(&options, "rev")?,
            ref_name: js_util::optional_string(&options, "ref")?,
            message: js_util::required_string(&options, "message")?,
            amend: js_util::optional_bool(&options, "amend")?,
            author: optional_person(&options, "author")?,
            committer: optional_person(&options, "committer")?,
        })
    }
}

#[derive(Default)]
struct NewOptions {
    revisions: Vec<String>,
    description: Option<String>,
}

impl NewOptions {
    fn from_js(options: JsValue) -> Result<Self, JsValue> {
        Ok(Self {
            revisions: optional_string_array(&options, "revisions")?.unwrap_or_default(),
            description: js_util::optional_string(&options, "description")?,
        })
    }
}

#[derive(Default)]
struct RestoreOptions {
    source: Option<String>,
    paths: Vec<String>,
    force: Option<bool>,
    snapshot: Option<bool>,
    message: Option<String>,
    author: Option<CommitPerson>,
    committer: Option<CommitPerson>,
}

impl RestoreOptions {
    fn from_js(options: JsValue) -> Result<Self, JsValue> {
        let paths = optional_string_array(&options, "paths")?
            .or_else(|| optional_string_array(&options, "filepaths").ok().flatten())
            .unwrap_or_default();
        Ok(Self {
            source: js_util::optional_string(&options, "source")?
                .or_else(|| js_util::optional_string(&options, "from").ok().flatten()),
            paths,
            force: js_util::optional_bool(&options, "force")?,
            snapshot: js_util::optional_bool(&options, "snapshot")?,
            message: js_util::optional_string(&options, "message")?,
            author: optional_person(&options, "author")?,
            committer: optional_person(&options, "committer")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RepoState {
    schema_version: u32,
    head: Option<String>,
    current_change_id: Option<String>,
    working_copy_commit: Option<String>,
    changes: Vec<ChangeRecord>,
}

impl Default for RepoState {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            head: None,
            current_change_id: None,
            working_copy_commit: None,
            changes: Vec::new(),
        }
    }
}

impl RepoState {
    fn ensure_current_change(&mut self, parent: Option<String>) {
        if self.current_change().is_some() {
            return;
        }
        let change = ChangeRecord::new(
            new_change_id("change"),
            String::new(),
            parent.into_iter().collect(),
            None,
        );
        self.current_change_id = Some(change.id.clone());
        self.changes.push(change);
    }

    fn current_change(&self) -> Option<ChangeRecord> {
        let current_id = self.current_change_id.as_ref()?;
        self.changes
            .iter()
            .find(|change| &change.id == current_id)
            .cloned()
    }

    fn current_change_mut(&mut self) -> Option<&mut ChangeRecord> {
        let current_id = self.current_change_id.as_ref()?;
        self.changes
            .iter_mut()
            .find(|change| &change.id == current_id)
    }

    fn describe_current_change(&mut self, description: String) {
        if let Some(change) = self.current_change_mut() {
            change.description = description;
        }
    }

    fn set_current_commit(&mut self, commit_id: String) {
        self.working_copy_commit = Some(commit_id.clone());
        if let Some(change) = self.current_change_mut() {
            change.commit_id = Some(commit_id);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChangeRecord {
    id: String,
    description: String,
    parents: Vec<String>,
    commit_id: Option<String>,
    hidden: bool,
}

impl ChangeRecord {
    fn new(
        id: String,
        description: String,
        parents: Vec<String>,
        commit_id: Option<String>,
    ) -> Self {
        Self {
            id,
            description,
            parents,
            commit_id,
            hidden: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommitPerson {
    name: String,
    email: String,
}

impl Default for CommitPerson {
    fn default() -> Self {
        Self {
            name: "jj-wasm".to_string(),
            email: "jj-wasm@example.invalid".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BrowserStatus {
    current_change: Option<ChangeRecord>,
    head: Option<String>,
    changed_files: Vec<StatusFile>,
    summary: StatusSummary,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotResult {
    operation: OperationRecord,
    commit_id: Option<String>,
    current_change: Option<ChangeRecord>,
    changed_files: Vec<StatusFile>,
}

#[derive(Debug, Clone)]
struct StatusRow {
    filepath: String,
    head: u8,
    workdir: u8,
    stage: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusFile {
    path: String,
    status: &'static str,
    head: u8,
    workdir: u8,
    stage: u8,
}

impl From<&StatusRow> for StatusFile {
    fn from(row: &StatusRow) -> Self {
        let status = match (row.head, row.workdir, row.stage) {
            (0, 0, _) => "absent",
            (0, _, _) => "added",
            (_, 0, _) => "deleted",
            _ if row.head != row.workdir => "modified",
            _ if row.workdir != row.stage => "staged",
            _ => "clean",
        };
        Self {
            path: row.filepath.clone(),
            status,
            head: row.head,
            workdir: row.workdir,
            stage: row.stage,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusSummary {
    added: usize,
    modified: usize,
    deleted: usize,
    staged: usize,
    clean: usize,
}

impl StatusSummary {
    fn from_rows(rows: &[StatusRow]) -> Self {
        let mut summary = Self {
            added: 0,
            modified: 0,
            deleted: 0,
            staged: 0,
            clean: 0,
        };
        for row in rows {
            match StatusFile::from(row).status {
                "added" => summary.added += 1,
                "modified" => summary.modified += 1,
                "deleted" => summary.deleted += 1,
                "staged" => summary.staged += 1,
                _ => summary.clean += 1,
            }
        }
        summary
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OperationRecord {
    id: String,
    kind: String,
    timestamp_ms: f64,
    detail: serde_json::Value,
    #[serde(default)]
    before_state: Option<RepoState>,
    #[serde(default)]
    after_state: Option<RepoState>,
    #[serde(default)]
    previous_head: Option<String>,
    #[serde(default)]
    new_head: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct SessionMetadata {
    schema_version: u32,
    created_by: String,
    dir: String,
    gitdir: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct RawObjectForJs {
    oid: String,
    kind: String,
    wrapped: Vec<u8>,
}

fn optional_person(object: &JsValue, name: &str) -> Result<Option<CommitPerson>, JsValue> {
    let Some(value) = js_util::optional_prop(object, name)? else {
        return Ok(None);
    };
    serde_wasm_bindgen::from_value(value)
        .map(Some)
        .map_err(|err| js_util::error(format!("option `{name}` must be a commit person: {err}")))
}

fn optional_string_array(object: &JsValue, name: &str) -> Result<Option<Vec<String>>, JsValue> {
    let Some(value) = js_util::optional_prop(object, name)? else {
        return Ok(None);
    };
    serde_wasm_bindgen::from_value(value)
        .map(Some)
        .map_err(|err| js_util::error(format!("option `{name}` must be a string array: {err}")))
}

fn status_rows_from_js(value: JsValue) -> Result<Vec<StatusRow>, JsValue> {
    let raw_rows: Vec<serde_json::Value> = serde_wasm_bindgen::from_value(value)
        .map_err(|err| js_util::error(format!("failed to decode status matrix: {err}")))?;
    raw_rows
        .into_iter()
        .map(|row| {
            let cells = row
                .as_array()
                .ok_or_else(|| js_util::error("status matrix row was not an array"))?;
            if cells.len() != 4 {
                return Err(js_util::error("status matrix row did not have four cells"));
            }
            Ok(StatusRow {
                filepath: cells[0]
                    .as_str()
                    .ok_or_else(|| js_util::error("status matrix filepath was not a string"))?
                    .to_string(),
                head: status_cell(&cells[1], "HEAD")?,
                workdir: status_cell(&cells[2], "WORKDIR")?,
                stage: status_cell(&cells[3], "STAGE")?,
            })
        })
        .collect()
}

fn status_cell(value: &serde_json::Value, name: &str) -> Result<u8, JsValue> {
    let number = value
        .as_u64()
        .ok_or_else(|| js_util::error(format!("status matrix {name} cell was not a number")))?;
    u8::try_from(number)
        .map_err(|_| js_util::error(format!("status matrix {name} cell was out of range")))
}

fn log_ref_from_options(options: &LogOptions) -> Option<String> {
    if let Some(ref_name) = options.ref_name.as_deref() {
        return Some(ref_name.to_string());
    }
    match options.revset.as_deref().map(str::trim) {
        None | Some("" | "::" | "all()") => None,
        Some("@" | "HEAD") => Some("HEAD".to_string()),
        Some(revset) => Some(revset.to_string()),
    }
}

fn migrate_state(mut state: RepoState, path: &str) -> Result<RepoState, JsValue> {
    if state.schema_version == 0 {
        state.schema_version = CURRENT_SCHEMA_VERSION;
    }
    if state.schema_version != CURRENT_SCHEMA_VERSION {
        return Err(js_util::error(format!(
            "unsupported jj-wasm state schema {} in `{path}`; supported schema is {}",
            state.schema_version, CURRENT_SCHEMA_VERSION
        )));
    }
    Ok(state)
}

fn corrupt_state_error(path: &str, err: serde_json::Error) -> JsValue {
    js_util::error(format!(
        "failed to decode jj-wasm metadata `{path}`: {err}. Back up the browser filesystem if \
         needed, then delete `{path}` or reset the repo to recover."
    ))
}

fn is_vcs_metadata_path(path: &str) -> bool {
    path == ".jj" || path.starts_with(".jj/") || path == ".git" || path.starts_with(".git/")
}

fn normalize_change_id_header(change_id: &str) -> String {
    if change_id.len() == 32 && change_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return change_id.to_ascii_lowercase();
    }

    let mut bytes = [0_u8; 16];
    for (index, byte) in change_id.bytes().enumerate() {
        let slot = index % bytes.len();
        bytes[slot] = bytes[slot].wrapping_mul(31).wrapping_add(byte);
    }
    bytes.iter().fold(String::new(), |mut output, byte| {
        write!(&mut output, "{byte:02x}").expect("writing to a String should not fail");
        output
    })
}

fn new_change_id(prefix: &str) -> String {
    let timestamp = js_sys::Date::now() as u64;
    let random = (js_sys::Math::random() * u64::MAX as f64) as u64;
    if prefix == "change" {
        format!("{timestamp:016x}{random:016x}")
    } else {
        format!("{prefix}-{timestamp:016x}{random:016x}")
    }
}
