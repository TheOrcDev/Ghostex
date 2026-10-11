use crate::json::J;

pub const DEFAULT_TERMINAL_FONT_PRESET: &str = "JetBrains Mono";

pub const MONOSPACE_TERMINAL_FONT_FAMILY: &str =
    "monospace, \"MesloLGL Nerd Font Mono\", monospace";

pub const TERMINAL_FONT_PRESETS: J = J::Arr(&[
    J::Obj(&[
        ("preset", J::Str("Monospace")),
        ("fontFamily", J::Str("monospace, \"MesloLGL Nerd Font Mono\", monospace")),
    ]),
    J::Obj(&[
        ("preset", J::Str("UI Monospace")),
        ("fontFamily", J::Str("ui-monospace, \"MesloLGL Nerd Font Mono\", monospace")),
    ]),
    J::Obj(&[
        ("preset", J::Str("Meslo")),
        ("fontFamily", J::Str("\"MesloLGL Nerd Font Mono\", Menlo, Monaco, \"Courier New\", monospace")),
    ]),
    J::Obj(&[
        ("preset", J::Str("Cross Platform Mono")),
        ("fontFamily", J::Str("Consolas, Menlo, Monaco, \"Liberation Mono\", \"DejaVu Sans Mono\", \"Courier New\", \"MesloLGL Nerd Font Mono\", monospace")),
    ]),
    J::Obj(&[
        ("preset", J::Str("Consolas (Windows Default)")),
        ("fontFamily", J::Str("Consolas, \"Courier New\", \"MesloLGL Nerd Font Mono\", monospace")),
    ]),
    J::Obj(&[
        ("preset", J::Str("Menlo")),
        ("fontFamily", J::Str("Menlo, \"MesloLGL Nerd Font Mono\", monospace")),
    ]),
    J::Obj(&[
        ("preset", J::Str("Monaco")),
        ("fontFamily", J::Str("Monaco, \"Courier New\", \"MesloLGL Nerd Font Mono\", monospace")),
    ]),
    J::Obj(&[
        ("preset", J::Str("Droid Sans Mono (Linux Default)")),
        ("fontFamily", J::Str("\"Droid Sans Mono\", \"monospace\", \"MesloLGL Nerd Font Mono\", monospace")),
    ]),
    J::Obj(&[
        ("preset", J::Str("Liberation Mono")),
        ("fontFamily", J::Str("\"Liberation Mono\", \"MesloLGL Nerd Font Mono\", monospace")),
    ]),
    J::Obj(&[
        ("preset", J::Str("DejaVu Sans Mono")),
        ("fontFamily", J::Str("\"DejaVu Sans Mono\", \"MesloLGL Nerd Font Mono\", monospace")),
    ]),
    J::Obj(&[
        ("preset", J::Str("Courier New")),
        ("fontFamily", J::Str("\"Courier New\", \"MesloLGL Nerd Font Mono\", monospace")),
    ]),
    J::Obj(&[
        ("preset", J::Str("Cascadia Mono")),
        ("fontFamily", J::Str("\"Cascadia Mono\", \"MesloLGL Nerd Font Mono\", monospace")),
    ]),
    J::Obj(&[
        ("preset", J::Str("Cascadia Code")),
        ("fontFamily", J::Str("\"Cascadia Code\", \"MesloLGL Nerd Font Mono\", monospace")),
    ]),
    J::Obj(&[
        ("preset", J::Str("JetBrains Mono")),
        ("fontFamily", J::Str("\"JetBrains Mono\", \"MesloLGL Nerd Font Mono\", monospace")),
    ]),
    J::Obj(&[
        ("preset", J::Str("Fira Code")),
        ("fontFamily", J::Str("\"Fira Code\", \"MesloLGL Nerd Font Mono\", monospace")),
    ]),
    J::Obj(&[
        ("preset", J::Str("Source Code Pro")),
        ("fontFamily", J::Str("\"Source Code Pro\", \"MesloLGL Nerd Font Mono\", monospace")),
    ]),
    J::Obj(&[
        ("preset", J::Str("IBM Plex Mono")),
        ("fontFamily", J::Str("\"IBM Plex Mono\", \"MesloLGL Nerd Font Mono\", monospace")),
    ]),
    J::Obj(&[
        ("preset", J::Str("Roboto Mono")),
        ("fontFamily", J::Str("\"Roboto Mono\", \"MesloLGL Nerd Font Mono\", monospace")),
    ]),
    J::Obj(&[
        ("preset", J::Str("Noto Sans Mono")),
        ("fontFamily", J::Str("\"Noto Sans Mono\", \"MesloLGL Nerd Font Mono\", monospace")),
    ]),
    J::Obj(&[
        ("preset", J::Str("Ubuntu Mono")),
        ("fontFamily", J::Str("\"Ubuntu Mono\", \"MesloLGL Nerd Font Mono\", monospace")),
    ]),
]);
