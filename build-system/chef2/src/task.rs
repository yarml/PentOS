use std::{
    hint,
    task::{Context, Poll, Waker},
};

pub fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = Box::pin(future);
    loop {
        if let Poll::Ready(output) = future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            return output;
        }
        hint::spin_loop();
    }
}
