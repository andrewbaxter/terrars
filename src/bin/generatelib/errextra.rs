use anyhow::{Context, Result};

#[inline(always)]
pub fn err_stop<R, F: FnOnce() -> Result<R>>(f: F) -> Result<R> {
    f()
}

#[macro_export]
macro_rules! es {
    ($b:expr) => {
        $crate::errextra::err_stop(|| $b)
    };
}
