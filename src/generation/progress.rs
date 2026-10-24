use std::cell::RefCell;
use std::rc::Rc;

type Observer = Rc<dyn Fn(&'static str)>;

std::thread_local! {
    static OBSERVER: RefCell<Option<Observer>> = const { RefCell::new(None) };
}

pub(crate) fn report(stage: &'static str) {
    let observer = OBSERVER.with(|slot| slot.borrow().clone());
    if let Some(observer) = observer {
        observer(stage);
    }
}

#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) fn observe<T>(observer: Observer, run: impl FnOnce() -> T) -> T {
    struct Restore(Option<Observer>);
    impl Drop for Restore {
        fn drop(&mut self) {
            OBSERVER.with(|slot| *slot.borrow_mut() = self.0.take());
        }
    }
    let _restore = Restore(OBSERVER.with(|slot| slot.replace(Some(observer))));
    run()
}
