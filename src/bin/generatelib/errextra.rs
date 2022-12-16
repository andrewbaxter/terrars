use anyhow::Result;

#[inline(always)]
pub fn err_stop<R, F: FnOnce() -> Result<R>>(f: F) -> Result<R> {
    f()
}

#[macro_export]
macro_rules! es{($b: expr) => {
    $crate:: generatelib:: errextra:: err_stop(|| $b)
};}
