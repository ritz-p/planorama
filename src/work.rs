use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

std::thread_local! {
    static REMAINING: Cell<usize> = const { Cell::new(usize::MAX) };
    static LIMITED: Cell<bool> = const { Cell::new(false) };
    static EXCEEDED: Cell<bool> = const { Cell::new(false) };
    static MEMORY_EXCEEDED: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn limited() -> bool {
    LIMITED.get()
}

pub(crate) async fn check_limit(used: usize, maximum: usize) {
    std::future::poll_fn(|_| {
        if LIMITED.get() && used > maximum {
            EXCEEDED.set(true);
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    })
    .await
}

pub(crate) async fn checkpoint() {
    check_memory(0).await;
    std::future::poll_fn(|_| {
        REMAINING.with(|remaining| match remaining.get() {
            0 => Poll::Pending,
            usize::MAX => Poll::Ready(()),
            units => {
                remaining.set(units - 1);
                Poll::Ready(())
            }
        })
    })
    .await
}

pub(crate) async fn check_memory(additional: u64) {
    #[cfg(not(target_arch = "wasm32"))]
    let _ = additional;
    std::future::poll_fn(|_| {
        #[cfg(target_arch = "wasm32")]
        if LIMITED.get() && !crate::generation::memory::available(additional) {
            MEMORY_EXCEEDED.set(true);
            return Poll::Pending;
        }
        Poll::Ready(())
    })
    .await
}

pub(crate) async fn check_allocation(succeeded: bool) {
    std::future::poll_fn(|_| {
        if succeeded {
            Poll::Ready(())
        } else {
            MEMORY_EXCEEDED.set(true);
            Poll::Pending
        }
    })
    .await
}

pub(crate) async fn filled_vec<T: Clone>(value: T, length: usize) -> Vec<T> {
    if !limited() {
        return vec![value; length];
    }
    check_memory((length as u64).saturating_mul(std::mem::size_of::<T>() as u64)).await;
    let mut values = Vec::new();
    check_allocation(values.try_reserve_exact(length).is_ok()).await;
    values.resize(length, value);
    values
}

pub(crate) fn step<F: Future + ?Sized>(
    future: Pin<&mut F>,
    units: usize,
    limited: bool,
) -> Result<Poll<F::Output>, crate::Error> {
    struct Restore(usize, bool, bool, bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            REMAINING.set(self.0);
            LIMITED.set(self.1);
            EXCEEDED.set(self.2);
            MEMORY_EXCEEDED.set(self.3);
        }
    }
    let _restore = Restore(
        REMAINING.replace(units),
        LIMITED.replace(limited),
        EXCEEDED.replace(false),
        MEMORY_EXCEEDED.replace(false),
    );
    let result = future.poll(&mut Context::from_waker(Waker::noop()));
    if MEMORY_EXCEEDED.get() {
        Err(crate::Error::MemoryLimit)
    } else if EXCEEDED.get() {
        Err(crate::Error::ResourceLimit)
    } else {
        Ok(result)
    }
}

pub(crate) fn complete<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    match step(future.as_mut(), usize::MAX, false).expect("synchronous work has no browser budget")
    {
        Poll::Ready(output) => output,
        Poll::Pending => unreachable!("unlimited work completes synchronously"),
    }
}
