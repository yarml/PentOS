use {
    alloc::boxed::Box,
    core::{
        mem,
        pin::Pin,
        task::{Context, Poll},
    },
};

pub enum ManualFuture<T> {
    Ready(T),
    Pending(Pin<Box<dyn Future<Output = T> + Send>>),
}

impl<T> ManualFuture<T> {
    pub fn make(future: impl Future<Output = T> + 'static + Send) -> Self {
        Self::Pending(Box::pin(future))
    }

    pub fn remake(&mut self, future: impl Future<Output = T> + 'static + Send) -> Option<T> {
        let old_self = mem::replace(self, Self::Pending(Box::pin(future)));
        match old_self {
            ManualFuture::Ready(val) => Some(val),
            ManualFuture::Pending(_) => None,
        }
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Option<&mut T> {
        match self {
            ManualFuture::Ready(val) => Some(val),
            ManualFuture::Pending(future) => {
                match future.as_mut().poll(cx) {
                    Poll::Ready(val) => {
                        *self = Self::Ready(val);
                    }
                    Poll::Pending => {}
                };
                match self {
                    ManualFuture::Ready(val) => Some(val),
                    ManualFuture::Pending(_) => None,
                }
            }
        }
    }
}
