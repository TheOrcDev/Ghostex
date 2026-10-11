pub(crate) mod window;
#[allow(dead_code)]
pub(crate) mod model {
    include!(concat!(env!("OUT_DIR"), "/floating_reveal_model.rs"));
}
