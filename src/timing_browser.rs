#[cfg(not(feature = "wasm-profile"))]
pub(crate) struct Span;

#[cfg(not(feature = "wasm-profile"))]
pub(crate) fn span(_: &'static str, _: &[(&'static str, usize)]) -> Span {
    Span
}

#[cfg(feature = "wasm-profile")]
pub(crate) use profile::span;

#[cfg(feature = "wasm-profile")]
mod profile {
    use std::cell::RefCell;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_name = Function)]
        pub type ProfileCallback;
        #[wasm_bindgen(method, js_name = call)]
        fn notify(this: &ProfileCallback, context: &JsValue, stage: &str, enter: bool);
    }

    thread_local! {
        static OBSERVER: RefCell<Option<ProfileCallback>> = const { RefCell::new(None) };
    }

    #[wasm_bindgen]
    pub fn set_profile_callback(observer: Option<ProfileCallback>) {
        OBSERVER.with(|slot| slot.replace(observer));
    }

    fn boundary(stage: &str, enter: bool) {
        OBSERVER.with_borrow(|slot| {
            if let Some(callback) = slot {
                callback.notify(&JsValue::UNDEFINED, stage, enter);
            }
        });
    }

    pub(crate) struct Span(&'static str);

    pub(crate) fn span(stage: &'static str, _: &[(&'static str, usize)]) -> Span {
        boundary(stage, true);
        Span(stage)
    }

    impl Drop for Span {
        fn drop(&mut self) {
            boundary(self.0, false);
        }
    }
}
