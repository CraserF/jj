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
use wasm_bindgen::JsValue;

use crate::js_util;

#[derive(Clone)]
pub struct BrowserFs {
    fs: JsValue,
    promises: JsValue,
}

impl BrowserFs {
    pub fn new(fs: JsValue) -> Result<Self, JsValue> {
        let promises = js_util::get_prop(&fs, "promises")?;
        if promises.is_null() || promises.is_undefined() {
            Err(js_util::error(
                "`fs` must provide a Node-like `promises` API, such as LightningFS.promises",
            ))
        } else {
            Ok(Self { fs, promises })
        }
    }

    pub fn raw_fs(&self) -> &JsValue {
        &self.fs
    }

    async fn call(&self, method: &str, args: &Array) -> Result<JsValue, JsValue> {
        js_util::call_promise(&self.promises, method, args).await
    }

    pub async fn mkdir(&self, path: &str) -> Result<(), JsValue> {
        let args = Array::new();
        args.push(&JsValue::from_str(path));
        self.call("mkdir", &args).await.map(|_| ())
    }

    pub async fn mkdirp(&self, path: &str) -> Result<(), JsValue> {
        let mut current = if path.starts_with('/') {
            "/".to_string()
        } else {
            String::new()
        };
        for part in path.split('/').filter(|part| !part.is_empty()) {
            current = if current.is_empty() || current == "/" {
                format!("{current}{part}")
            } else {
                format!("{current}/{part}")
            };
            if self.mkdir(&current).await.is_err() {
                // Node-compatible browser filesystems vary in their EEXIST
                // surface. Creation failures are ignored here and real errors
                // will be surfaced by the subsequent read/write operation.
            }
        }
        Ok(())
    }

    pub async fn read_text(&self, path: &str) -> Result<String, JsValue> {
        let args = Array::new();
        args.push(&JsValue::from_str(path));
        args.push(&JsValue::from_str("utf8"));
        let value = self.call("readFile", &args).await?;
        value
            .as_string()
            .ok_or_else(|| js_util::error(format!("`readFile({path})` did not return a string")))
    }

    pub async fn write_text(&self, path: &str, contents: &str) -> Result<(), JsValue> {
        if let Some(parent) = path.rsplit_once('/').map(|(parent, _)| parent)
            && !parent.is_empty()
        {
            self.mkdirp(parent).await?;
        }
        let args = Array::new();
        args.push(&JsValue::from_str(path));
        args.push(&JsValue::from_str(contents));
        args.push(&JsValue::from_str("utf8"));
        self.call("writeFile", &args).await.map(|_| ())
    }

    pub async fn write_json<T: serde::Serialize>(
        &self,
        path: &str,
        value: &T,
    ) -> Result<(), JsValue> {
        let text = serde_json::to_string_pretty(value)
            .map_err(|err| js_util::error(format!("failed to encode JSON: {err}")))?;
        self.write_text(path, &text).await
    }

    pub fn as_isomorphic_git_arg(&self) -> &JsValue {
        self.raw_fs()
    }
}
