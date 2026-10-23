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

#[wasm_bindgen]
pub struct Generation {
    inner: crate::Generator,
}

#[wasm_bindgen]
impl Generation {
    #[wasm_bindgen(constructor)]
    pub fn new(json: String, options: &str) -> Result<Generation, JsValue> {
        let options =
            serde_json::from_str(options).map_err(|_| error(crate::Error::InvalidOptions))?;
        Ok(Self {
            inner: crate::Generator::new(json, options).map_err(error)?,
        })
    }

    pub fn step(&mut self, units: u32) -> Result<Option<String>, JsValue> {
        self.inner
            .step(units as usize)
            .map_err(error)?
            .map(|result| {
                serde_json::to_string(&result).map_err(|_| error(crate::Error::GenerationFailed))
            })
            .transpose()
    }

    pub fn cancel(&mut self) {
        self.inner.cancel();
    }
}
