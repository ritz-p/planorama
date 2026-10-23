use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

std::thread_local! {
    static REMAINING: Cell<usize> = const { Cell::new(usize::MAX) };
    static LIMITED: Cell<bool> = const { Cell::new(false) };
    static EXCEEDED: Cell<bool> = const { Cell::new(false) };
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

pub(crate) fn step<F: Future + ?Sized>(
    future: Pin<&mut F>,
    units: usize,
    limited: bool,
) -> Result<Poll<F::Output>, crate::Error> {
    struct Restore(usize, bool, bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            REMAINING.set(self.0);
            LIMITED.set(self.1);
            EXCEEDED.set(self.2);
        }
    }
    let _restore = Restore(
        REMAINING.replace(units),
        LIMITED.replace(limited),
        EXCEEDED.replace(false),
    );
    let result = future.poll(&mut Context::from_waker(Waker::noop()));
    if EXCEEDED.get() {
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
