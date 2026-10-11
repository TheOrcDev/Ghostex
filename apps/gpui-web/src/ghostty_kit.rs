//! The desktop's shared constants are typed with libghostty's C enums. The browser has no libghostty, so these are the same integer types under the same paths.
#[allow(non_camel_case_types)]
pub(crate) mod ffi {
    pub(crate) type ghostty_input_mods_e = std::ffi::c_uint;
    pub(crate) type ghostty_input_action_e = std::ffi::c_uint;
    pub(crate) type ghostty_input_scroll_mods_t = std::ffi::c_int;
}
