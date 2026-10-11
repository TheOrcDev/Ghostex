use crate::json::J;

/// CDXC:StatusPet 2026-07-21:
/// Keep the pet implementation available for a possible return, but hide its
/// sidebar-menu and Settings controls while the feature is out of the product UI.
pub const PET_CONTROLS_VISIBLE: bool = false;

pub const PET_OPTIONS: J = J::Arr(&[
    J::Obj(&[
        (
            "description",
            J::Str("A friendly ghost for quiet workspace focus."),
        ),
        ("displayName", J::Str("Boo")),
        ("id", J::Str("boo")),
    ]),
    J::Obj(&[
        ("description", J::Str("The original Codex companion.")),
        ("displayName", J::Str("Codex")),
        ("id", J::Str("codex")),
    ]),
    J::Obj(&[
        (
            "description",
            J::Str("A tidy duck for calm workspace days."),
        ),
        ("displayName", J::Str("Dewey")),
        ("id", J::Str("dewey")),
    ]),
    J::Obj(&[
        ("description", J::Str("Hot path energy for fast iteration.")),
        ("displayName", J::Str("Fireball")),
        ("id", J::Str("fireball")),
    ]),
    J::Obj(&[
        (
            "description",
            J::Str("A steady rock when the diff gets large."),
        ),
        ("displayName", J::Str("Rocky")),
        ("id", J::Str("rocky")),
    ]),
    J::Obj(&[
        ("description", J::Str("Small green shoots for new ideas.")),
        ("displayName", J::Str("Seedy")),
        ("id", J::Str("seedy")),
    ]),
    J::Obj(&[
        ("description", J::Str("A balanced stack for deep work.")),
        ("displayName", J::Str("Stacky")),
        ("id", J::Str("stacky")),
    ]),
    J::Obj(&[
        ("description", J::Str("A tiny blue-screen companion.")),
        ("displayName", J::Str("BSOD")),
        ("id", J::Str("bsod")),
    ]),
    J::Obj(&[
        ("description", J::Str("Quiet signal from the void.")),
        ("displayName", J::Str("Null Signal")),
        ("id", J::Str("null-signal")),
    ]),
]);
