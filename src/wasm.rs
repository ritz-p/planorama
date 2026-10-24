use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = Function)]
    pub type ProgressCallback;

    #[wasm_bindgen(method, catch, js_name = call)]
    fn notify(this: &ProgressCallback, context: &JsValue, stage: &str) -> Result<JsValue, JsValue>;
}

#[wasm_bindgen]
pub fn generate_with_progress(
    json: &str,
    options: &str,
    callback: ProgressCallback,
) -> Result<String, JsValue> {
    crate::generation::progress::observe(
        std::rc::Rc::new(move |stage| {
            // Observer failures must not change the generated result.
            let _ = callback.notify(&JsValue::UNDEFINED, stage);
        }),
        || generate(json, options),
    )
}

#[wasm_bindgen]
pub fn memory_capabilities() -> String {
    use crate::generation::memory;
    serde_json::json!({
        "allowed_mib": memory::ALLOWED_MIB,
        "default_mib": memory::DEFAULT_MIB,
        "maximum_bytes": memory::MAX_BYTES,
        "scope": "instance_linear_memory",
        "enforcement": "imported_memory_maximum",
        "synchronous_option": false,
        "requires_new_worker_on_change": true,
    })
    .to_string()
}

#[wasm_bindgen]
pub fn configure_memory_budget(mib: u32) -> Result<(), JsValue> {
    crate::generation::memory::configure(mib).map_err(error)
}

fn decode_options(options: &str) -> Result<crate::Options, JsValue> {
    serde_json::from_str(options).map_err(|_| error(crate::Error::InvalidOptions))
}

#[wasm_bindgen]
pub fn inspect_input(json: &str) -> Result<String, JsValue> {
    let result = crate::inspect_input(json).map_err(error)?;
    serde_json::to_string(&result).map_err(|_| error(crate::Error::GenerationFailed))
}

#[wasm_bindgen]
pub fn inspect_resources(json: &str) -> Result<String, JsValue> {
    let result = crate::inspect_resources(json).map_err(error)?;
    serde_json::to_string(&result).map_err(|_| error(crate::Error::GenerationFailed))
}

#[wasm_bindgen]
pub fn inspect_modules(json: &str) -> Result<String, JsValue> {
    let result = crate::inspect_modules(json).map_err(error)?;
    serde_json::to_string(&result).map_err(|_| error(crate::Error::GenerationFailed))
}

#[wasm_bindgen]
pub fn generate(json: &str, options: &str) -> Result<String, JsValue> {
    let options = decode_options(options)?;
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
        if options.len() > crate::generation::MAX_BROWSER_OPTIONS_BYTES {
            return Err(error(crate::Error::ResourceLimit));
        }
        let options = decode_options(options)?;
        Ok(Self {
            inner: crate::Generator::new(json, options).map_err(error)?,
        })
    }

    pub fn step(&mut self, units: f64) -> Result<Option<String>, JsValue> {
        if !units.is_finite() || units.fract() != 0.0 || !(1.0..=4096.0).contains(&units) {
            return Err(error(crate::Error::InvalidOptions));
        }
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
