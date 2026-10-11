//! Same module paths as the desktop's `helpers/`. `indicator_animation` is the desktop file; `titlebar`, `sidebar` and `agents_hub` hold only the items `build.rs` lifts out of
//! the desktop files of the same name (see `extracted-items.txt`), because the rest of those files
//! is native-only; `mobile` answers the helpers whose desktop versions call the operating system.
pub(crate) mod indicator_animation;
#[allow(dead_code, unused_imports)]
pub(crate) mod titlebar {
    use crate::app::helpers::*;
    use crate::*;
    use gpui_component::{Theme, ThemeMode};
    include!(concat!(env!("OUT_DIR"), "/titlebar.rs"));
}
#[allow(dead_code, unused_imports)]
pub(crate) mod sidebar {
    use crate::*;
    include!(concat!(env!("OUT_DIR"), "/sidebar.rs"));
}
#[allow(dead_code, unused_imports)]
pub(crate) mod agents_hub {
    use crate::*;
    include!(concat!(env!("OUT_DIR"), "/agents_hub.rs"));
}
pub(crate) mod mobile;

#[allow(unused_imports)]
pub(crate) use agents_hub::*;
pub(crate) use indicator_animation::*;
pub(crate) use mobile::*;
#[allow(unused_imports)]
pub(crate) use sidebar::*;
pub(crate) use titlebar::*;
