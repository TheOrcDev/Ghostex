//! Only `clip_above` of the desktop's `element/`; the other two wrap CEF surfaces and native window corners.
pub(crate) mod clip_above;
#[allow(unused_imports)]
pub(crate) use clip_above::*;
