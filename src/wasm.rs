use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn inspect_modules(json: &str) -> Result<String, JsValue> {
    let result = crate::inspect_modules(json).map_err(error)?;
    serde_json::to_string(&result).map_err(|_| error(crate::Error::GenerationFailed))
}

#[wasm_bindgen]
pub fn generate(json: &str, options: &str) -> Result<String, JsValue> {
    let options = serde_json::from_str(options).map_err(|_| error(crate::Error::InvalidOptions))?;
    let result = crate::generate(json, &options).map_err(error)?;
    serde_json::to_string(&result).map_err(|_| error(crate::Error::GenerationFailed))
}

fn error(error: crate::Error) -> JsValue {
    JsValue::from_str(&serde_json::json!({"code": error, "message": error.to_string()}).to_string())
}
