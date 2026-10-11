//! The desktop asks its embedded Chromium which colour scheme the system is in. The phone's host
//! reports it instead (`ChatInit::light_appearance`, `mobile::set_light_appearance`).

/// The system colour scheme in the words CEF uses.
pub(crate) fn system_page_color_scheme() -> Option<String> {
    Some(
        if crate::mobile::light_appearance() {
            "light"
        } else {
            "dark"
        }
        .to_string(),
    )
}
