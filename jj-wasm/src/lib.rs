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

#![deny(unused_must_use)]

#[cfg(target_arch = "wasm32")]
mod browser_fs;
#[cfg(target_arch = "wasm32")]
mod isomorphic_git_backend;
#[cfg(target_arch = "wasm32")]
mod js_util;
#[cfg(target_arch = "wasm32")]
mod session;

#[cfg(target_arch = "wasm32")]
pub use session::JjSession;

#[cfg(not(target_arch = "wasm32"))]
/// Marker type for host builds.
///
/// The usable browser API is only compiled for `wasm32-unknown-unknown`.
pub struct JjSession;
