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
use js_sys::Function;
use js_sys::Object;
use js_sys::Promise;
use js_sys::Reflect;
use serde::Serialize;
use wasm_bindgen::JsCast as _;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

pub fn error(message: impl AsRef<str>) -> JsValue {
    js_sys::Error::new(message.as_ref()).into()
}

pub fn unsupported(name: &str) -> JsValue {
    error(format!(
        "`{name}` is exported by jj-wasm, but the first browser milestone only \
         wires initialization, clone/fetch, raw Git object access, log, status, \
         snapshot metadata, and op-log metadata. Full jj operation semantics \
         still need the browser working-copy/op-store/index adapters to be \
         connected to jj-lib."
    ))
}

pub fn get_prop(object: &JsValue, name: &str) -> Result<JsValue, JsValue> {
    Reflect::get(object, &JsValue::from_str(name))
}

pub fn set_prop(object: &Object, name: &str, value: &JsValue) -> Result<(), JsValue> {
    Reflect::set(object, &JsValue::from_str(name), value).map(|_| ())
}

pub fn required_prop(object: &JsValue, name: &str) -> Result<JsValue, JsValue> {
    let value = get_prop(object, name)?;
    if value.is_null() || value.is_undefined() {
        Err(error(format!("missing required option `{name}`")))
    } else {
        Ok(value)
    }
}

pub fn optional_prop(object: &JsValue, name: &str) -> Result<Option<JsValue>, JsValue> {
    let value = get_prop(object, name)?;
    if value.is_null() || value.is_undefined() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

pub fn required_string(object: &JsValue, name: &str) -> Result<String, JsValue> {
    required_prop(object, name)?
        .as_string()
        .ok_or_else(|| error(format!("option `{name}` must be a string")))
}

pub fn optional_string(object: &JsValue, name: &str) -> Result<Option<String>, JsValue> {
    optional_prop(object, name)?
        .map(|value| {
            value
                .as_string()
                .ok_or_else(|| error(format!("option `{name}` must be a string")))
        })
        .transpose()
}

pub fn optional_u32(object: &JsValue, name: &str) -> Result<Option<u32>, JsValue> {
    optional_prop(object, name)?
        .map(|value| {
            let number = value
                .as_f64()
                .ok_or_else(|| error(format!("option `{name}` must be a number")))?;
            if number.is_sign_negative() || number > f64::from(u32::MAX) {
                Err(error(format!("option `{name}` is outside the u32 range")))
            } else {
                Ok(number as u32)
            }
        })
        .transpose()
}

pub fn optional_bool(object: &JsValue, name: &str) -> Result<Option<bool>, JsValue> {
    optional_prop(object, name)?
        .map(|value| {
            value
                .as_bool()
                .ok_or_else(|| error(format!("option `{name}` must be a boolean")))
        })
        .transpose()
}

pub fn call_method(object: &JsValue, method: &str, args: &Array) -> Result<JsValue, JsValue> {
    let function = get_prop(object, method)?;
    let function = function
        .dyn_into::<Function>()
        .map_err(|_| error(format!("`{method}` is not a function")))?;
    function.apply(object, args)
}

pub async fn call_promise(
    object: &JsValue,
    method: &str,
    args: &Array,
) -> Result<JsValue, JsValue> {
    let value = call_method(object, method, args)?;
    JsFuture::from(Promise::from(value)).await
}

pub fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(value)
        .map_err(|err| error(format!("failed to serialize value for JavaScript: {err}")))
}

pub fn join_path(base: &str, child: &str) -> String {
    if base == "/" {
        format!("/{child}")
    } else {
        format!("{}/{child}", base.trim_end_matches('/'))
    }
}

pub fn set_if_some(object: &Object, name: &str, value: Option<&JsValue>) -> Result<(), JsValue> {
    if let Some(value) = value {
        set_prop(object, name, value)?;
    }
    Ok(())
}

pub fn set_str_if_some(object: &Object, name: &str, value: Option<&str>) -> Result<(), JsValue> {
    if let Some(value) = value {
        set_prop(object, name, &JsValue::from_str(value))?;
    }
    Ok(())
}

pub fn set_u32_if_some(object: &Object, name: &str, value: Option<u32>) -> Result<(), JsValue> {
    if let Some(value) = value {
        set_prop(object, name, &JsValue::from_f64(f64::from(value)))?;
    }
    Ok(())
}

pub fn set_bool_if_some(object: &Object, name: &str, value: Option<bool>) -> Result<(), JsValue> {
    if let Some(value) = value {
        set_prop(object, name, &JsValue::from_bool(value))?;
    }
    Ok(())
}
