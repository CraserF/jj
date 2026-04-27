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

use js_sys::Array;
use js_sys::Object;
use js_sys::Reflect;
use js_sys::Uint8Array;
use wasm_bindgen::JsValue;

use crate::browser_fs::BrowserFs;
use crate::js_util;

#[derive(Clone)]
pub struct IsomorphicGitBackend {
    git: JsValue,
    fs: BrowserFs,
    dir: String,
    gitdir: Option<String>,
    http: Option<JsValue>,
    cache: Option<JsValue>,
}

impl IsomorphicGitBackend {
    pub fn new(
        git: JsValue,
        fs: BrowserFs,
        dir: String,
        gitdir: Option<String>,
        http: Option<JsValue>,
        cache: Option<JsValue>,
    ) -> Self {
        Self {
            git,
            fs,
            dir,
            gitdir,
            http,
            cache,
        }
    }

    pub fn fs(&self) -> &BrowserFs {
        &self.fs
    }

    pub fn dir(&self) -> &str {
        &self.dir
    }

    pub fn gitdir(&self) -> Option<&str> {
        self.gitdir.as_deref()
    }

    pub fn base_args(&self) -> Result<Object, JsValue> {
        let args = Object::new();
        js_util::set_prop(&args, "fs", self.fs.as_isomorphic_git_arg())?;
        js_util::set_prop(&args, "dir", &JsValue::from_str(&self.dir))?;
        js_util::set_str_if_some(&args, "gitdir", self.gitdir.as_deref())?;
        js_util::set_if_some(&args, "cache", self.cache.as_ref())?;
        Ok(args)
    }

    pub async fn init(
        &self,
        default_branch: Option<&str>,
        no_checkout: Option<bool>,
    ) -> Result<JsValue, JsValue> {
        let args = self.base_args()?;
        js_util::set_str_if_some(&args, "defaultBranch", default_branch)?;
        js_util::set_bool_if_some(&args, "noCheckout", no_checkout)?;
        self.call("init", args).await
    }

    pub async fn clone_repo(
        &self,
        url: &str,
        cors_proxy: Option<&str>,
        ref_name: Option<&str>,
        single_branch: Option<bool>,
        depth: Option<u32>,
    ) -> Result<JsValue, JsValue> {
        let args = self.base_args()?;
        let http = self.http.as_ref().ok_or_else(|| {
            js_util::error("`http` is required for browser clone/fetch/push operations")
        })?;
        js_util::set_prop(&args, "http", http)?;
        js_util::set_prop(&args, "url", &JsValue::from_str(url))?;
        js_util::set_str_if_some(&args, "corsProxy", cors_proxy)?;
        js_util::set_str_if_some(&args, "ref", ref_name)?;
        js_util::set_bool_if_some(&args, "singleBranch", single_branch)?;
        js_util::set_u32_if_some(&args, "depth", depth)?;
        self.call("clone", args).await
    }

    pub async fn fetch(
        &self,
        url: Option<&str>,
        remote: Option<&str>,
        cors_proxy: Option<&str>,
        ref_name: Option<&str>,
        single_branch: Option<bool>,
        depth: Option<u32>,
    ) -> Result<JsValue, JsValue> {
        let args = self.base_args()?;
        let http = self.http.as_ref().ok_or_else(|| {
            js_util::error("`http` is required for browser clone/fetch/push operations")
        })?;
        js_util::set_prop(&args, "http", http)?;
        js_util::set_str_if_some(&args, "url", url)?;
        js_util::set_str_if_some(&args, "remote", remote)?;
        js_util::set_str_if_some(&args, "corsProxy", cors_proxy)?;
        js_util::set_str_if_some(&args, "ref", ref_name)?;
        js_util::set_bool_if_some(&args, "singleBranch", single_branch)?;
        js_util::set_u32_if_some(&args, "depth", depth)?;
        self.call("fetch", args).await
    }

    pub async fn push(
        &self,
        remote: Option<&str>,
        url: Option<&str>,
        ref_name: Option<&str>,
        cors_proxy: Option<&str>,
    ) -> Result<JsValue, JsValue> {
        let args = self.base_args()?;
        let http = self.http.as_ref().ok_or_else(|| {
            js_util::error("`http` is required for browser clone/fetch/push operations")
        })?;
        js_util::set_prop(&args, "http", http)?;
        js_util::set_str_if_some(&args, "remote", remote)?;
        js_util::set_str_if_some(&args, "url", url)?;
        js_util::set_str_if_some(&args, "ref", ref_name)?;
        js_util::set_str_if_some(&args, "corsProxy", cors_proxy)?;
        self.call("push", args).await
    }

    pub async fn list_refs(&self, ref_prefix: Option<&str>) -> Result<JsValue, JsValue> {
        let args = self.base_args()?;
        js_util::set_str_if_some(&args, "filepath", ref_prefix)?;
        self.call("listRefs", args).await
    }

    pub async fn resolve_ref(&self, ref_name: &str) -> Result<JsValue, JsValue> {
        let args = self.base_args()?;
        js_util::set_prop(&args, "ref", &JsValue::from_str(ref_name))?;
        self.call("resolveRef", args).await
    }

    pub async fn write_ref(&self, ref_name: &str, oid: &str) -> Result<JsValue, JsValue> {
        let args = self.base_args()?;
        js_util::set_prop(&args, "ref", &JsValue::from_str(ref_name))?;
        js_util::set_prop(&args, "value", &JsValue::from_str(oid))?;
        js_util::set_bool_if_some(&args, "force", Some(true))?;
        self.call("writeRef", args).await
    }

    pub async fn current_branch(&self) -> Result<Option<String>, JsValue> {
        let args = self.base_args()?;
        js_util::set_prop(&args, "fullname", &JsValue::from_bool(false))?;
        let branch = self.call("currentBranch", args).await?;
        Ok(branch.as_string())
    }

    pub async fn add(&self, filepath: &str) -> Result<JsValue, JsValue> {
        let args = self.base_args()?;
        js_util::set_prop(&args, "filepath", &JsValue::from_str(filepath))?;
        self.call("add", args).await
    }

    pub async fn remove(&self, filepath: &str) -> Result<JsValue, JsValue> {
        let args = self.base_args()?;
        js_util::set_prop(&args, "filepath", &JsValue::from_str(filepath))?;
        self.call("remove", args).await
    }

    pub async fn commit(
        &self,
        message: &str,
        author: &JsValue,
        committer: Option<&JsValue>,
        amend: bool,
        ref_name: Option<&str>,
        parents: &[String],
    ) -> Result<String, JsValue> {
        let args = self.base_args()?;
        js_util::set_prop(&args, "message", &JsValue::from_str(message))?;
        js_util::set_prop(&args, "author", author)?;
        js_util::set_if_some(&args, "committer", committer)?;
        js_util::set_bool_if_some(&args, "amend", Some(amend))?;
        js_util::set_bool_if_some(&args, "noUpdateBranch", amend.then_some(true))?;
        js_util::set_str_if_some(&args, "ref", ref_name)?;
        if !parents.is_empty() {
            let parent_array = Array::new();
            for parent in parents {
                parent_array.push(&JsValue::from_str(parent));
            }
            js_util::set_prop(&args, "parent", &parent_array)?;
        }
        let oid = self.call("commit", args).await?;
        oid.as_string()
            .ok_or_else(|| js_util::error("`commit` did not return an object id"))
    }

    pub async fn checkout_paths(
        &self,
        source_ref: Option<&str>,
        filepaths: &[String],
        force: bool,
    ) -> Result<JsValue, JsValue> {
        let args = self.base_args()?;
        js_util::set_str_if_some(&args, "ref", source_ref)?;
        js_util::set_bool_if_some(&args, "force", Some(force))?;
        js_util::set_bool_if_some(&args, "noUpdateHead", Some(true))?;
        if !filepaths.is_empty() {
            let filepath_array = Array::new();
            for filepath in filepaths {
                filepath_array.push(&JsValue::from_str(filepath));
            }
            js_util::set_prop(&args, "filepaths", &filepath_array)?;
        }
        self.call("checkout", args).await
    }

    pub async fn log(
        &self,
        ref_name: Option<&str>,
        depth: Option<u32>,
    ) -> Result<JsValue, JsValue> {
        let args = self.base_args()?;
        js_util::set_str_if_some(&args, "ref", ref_name)?;
        js_util::set_u32_if_some(&args, "depth", depth)?;
        self.call("log", args).await
    }

    pub async fn status_matrix(&self) -> Result<JsValue, JsValue> {
        let args = self.base_args()?;
        self.call("statusMatrix", args).await
    }

    pub async fn read_raw_object(&self, oid: &str) -> Result<RawGitObject, JsValue> {
        let args = self.base_args()?;
        js_util::set_prop(&args, "oid", &JsValue::from_str(oid))?;
        js_util::set_prop(&args, "format", &JsValue::from_str("wrapped"))?;
        let value = self.call("readObject", args).await?;
        let object = Reflect::get(&value, &JsValue::from_str("object"))?;
        let object = Uint8Array::new(&object).to_vec();
        let js_kind = Reflect::get(&value, &JsValue::from_str("type"))?.as_string();
        let kind = match js_kind.as_deref() {
            Some("wrapped") | None => kind_from_wrapped_object(&object)
                .or_else(|| kind_from_commit_content(&object))
                .unwrap_or_else(|| "wrapped".to_string()),
            Some(kind) => kind.to_string(),
        };
        Ok(RawGitObject {
            oid: oid.to_string(),
            kind,
            wrapped: object,
        })
    }

    pub async fn write_raw_object(
        &self,
        object_type: &str,
        wrapped: &[u8],
    ) -> Result<String, JsValue> {
        let args = self.base_args()?;
        let bytes = Uint8Array::from(wrapped);
        js_util::set_prop(&args, "type", &JsValue::from_str(object_type))?;
        js_util::set_prop(&args, "format", &JsValue::from_str("wrapped"))?;
        js_util::set_prop(&args, "object", &bytes)?;
        let oid = self.call("writeObject", args).await?;
        oid.as_string()
            .ok_or_else(|| js_util::error("`writeObject` did not return an object id"))
    }

    pub async fn rewrite_commit_header(
        &self,
        oid: &str,
        name: &str,
        value: &str,
    ) -> Result<String, JsValue> {
        let raw = self.read_raw_object(oid).await?;
        if raw.kind != "commit" && raw.kind != "wrapped" {
            return Err(js_util::error(format!(
                "object `{oid}` is `{}`, not a commit",
                raw.kind
            )));
        }
        let rewritten = rewrite_wrapped_commit_header(&raw.wrapped, name, value)?;
        self.write_raw_object("commit", &rewritten).await
    }

    async fn call(&self, method: &str, object: Object) -> Result<JsValue, JsValue> {
        let args = Array::new();
        args.push(&object);
        js_util::call_promise(&self.git, method, &args).await
    }
}

#[derive(Debug, Clone)]
pub struct RawGitObject {
    pub oid: String,
    pub kind: String,
    pub wrapped: Vec<u8>,
}

fn rewrite_wrapped_commit_header(
    wrapped: &[u8],
    name: &str,
    value: &str,
) -> Result<Vec<u8>, JsValue> {
    let payload = if let Some(nul_index) = wrapped.iter().position(|byte| *byte == 0) {
        let header = std::str::from_utf8(&wrapped[..nul_index])
            .map_err(|err| js_util::error(format!("commit object header was not UTF-8: {err}")))?;
        if !header.starts_with("commit ") {
            return Err(js_util::error(format!(
                "wrapped object header `{header}` was not a commit"
            )));
        }
        &wrapped[nul_index + 1..]
    } else {
        wrapped
    };
    let payload = std::str::from_utf8(payload)
        .map_err(|err| js_util::error(format!("commit object was not UTF-8: {err}")))?;
    if !payload.starts_with("tree ") {
        return Err(js_util::error(
            "commit object did not start with a tree header",
        ));
    }
    let Some(headers_end) = payload.find("\n\n") else {
        return Err(js_util::error(
            "commit object did not contain a message separator",
        ));
    };

    let headers = &payload[..headers_end];
    let message = &payload[headers_end..];
    let filtered_headers = headers
        .lines()
        .filter(|line| !line.starts_with(&format!("{name} ")))
        .collect::<Vec<_>>()
        .join("\n");
    let new_payload = format!("{filtered_headers}\n{name} {value}{message}");
    let mut new_wrapped = format!("commit {}\0", new_payload.len()).into_bytes();
    new_wrapped.extend_from_slice(new_payload.as_bytes());
    Ok(new_wrapped)
}

fn kind_from_wrapped_object(wrapped: &[u8]) -> Option<String> {
    wrapped.iter().position(|byte| *byte == 0)?;
    let space_index = wrapped.iter().position(|byte| *byte == b' ')?;
    std::str::from_utf8(&wrapped[..space_index])
        .ok()
        .map(str::to_string)
}

fn kind_from_commit_content(content: &[u8]) -> Option<String> {
    content.starts_with(b"tree ").then(|| "commit".to_string())
}
