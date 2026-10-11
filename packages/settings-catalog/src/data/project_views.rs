use crate::json::J;

/// CDXC:Extensions 2026-09-09 DECISION:
/// User approved Website, Dev server, and HTML report primitives with reusable templates and per-project values, including a Linear URL. Storybook is now built in (2026-09-22), superseding its template entry; existing custom Storybook views keep working.
/// Existing fixed URL views keep their identity and ordering; templates create editable copies rather than live links.
pub const BUILTIN_PROJECT_VIEW_TEMPLATES: J = J::Arr(&[
    J::Obj(&[
        ("id", J::Str("github-issues")),
        ("name", J::Str("GitHub Issues")),
        ("url", J::Str("")),
        ("availability", J::Str("matching")),
        (
            "source",
            J::Obj(&[
                ("kind", J::Str("website")),
                ("destination", J::Str("github-issues")),
                ("discovery", J::Str("command")),
                ("command", J::Str("")),
                ("cwd", J::Str(".")),
                ("readinessUrl", J::Str("")),
                ("reportDirectory", J::Str("")),
                ("entry", J::Str("index.html")),
                ("timeoutSeconds", J::Num(60.0)),
            ]),
        ),
    ]),
    J::Obj(&[
        ("id", J::Str("github-actions")),
        ("name", J::Str("GitHub Actions")),
        ("url", J::Str("")),
        ("availability", J::Str("matching")),
        (
            "source",
            J::Obj(&[
                ("kind", J::Str("website")),
                ("destination", J::Str("github-actions")),
                ("discovery", J::Str("command")),
                ("command", J::Str("")),
                ("cwd", J::Str(".")),
                ("readinessUrl", J::Str("")),
                ("reportDirectory", J::Str("")),
                ("entry", J::Str("index.html")),
                ("timeoutSeconds", J::Num(60.0)),
            ]),
        ),
    ]),
    J::Obj(&[
        ("id", J::Str("linear")),
        ("name", J::Str("Linear")),
        ("url", J::Str("")),
        ("availability", J::Str("matching")),
        (
            "source",
            J::Obj(&[
                ("kind", J::Str("website")),
                ("destination", J::Str("project")),
                ("discovery", J::Str("command")),
                ("command", J::Str("")),
                ("cwd", J::Str(".")),
                ("readinessUrl", J::Str("")),
                ("reportDirectory", J::Str("")),
                ("entry", J::Str("index.html")),
                ("timeoutSeconds", J::Num(60.0)),
            ]),
        ),
    ]),
]);

pub const DEFAULT_PROJECT_VIEW_SOURCE: J = J::Obj(&[
    ("kind", J::Str("website")),
    ("destination", J::Str("fixed")),
    ("discovery", J::Str("command")),
    ("command", J::Str("")),
    ("cwd", J::Str(".")),
    ("readinessUrl", J::Str("")),
    ("reportDirectory", J::Str("")),
    ("entry", J::Str("index.html")),
    ("timeoutSeconds", J::Num(60.0)),
]);
