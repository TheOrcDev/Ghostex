use crate::json::J;

/// Automatic rules for the account that starts new sessions, in the order the Settings dropdown lists them.
pub const NEW_SESSION_ACCOUNT_RULES: J = J::Arr(&[
    J::Obj(&[
        ("rule", J::Str("auto")),
        ("label", J::Str("Auto (recommended)")),
    ]),
    J::Obj(&[
        ("rule", J::Str("mostRemaining")),
        ("label", J::Str("Most limit remaining")),
    ]),
    J::Obj(&[
        ("rule", J::Str("soonestReset")),
        ("label", J::Str("Soonest reset")),
    ]),
    J::Obj(&[
        ("rule", J::Str("mostUsed")),
        ("label", J::Str("Most used first")),
    ]),
    J::Obj(&[
        ("rule", J::Str("lastUsed")),
        ("label", J::Str("Same as last session")),
    ]),
]);
