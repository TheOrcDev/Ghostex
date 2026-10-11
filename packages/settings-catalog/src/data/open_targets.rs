use crate::json::J;

/// CDXC:Titlebar 2026-05-11-00:22
/// The titlebar Open In menu uses the shared editor command catalog so
/// installed local IDEs appear without maintaining a second, smaller
/// ghostex-only list.
///
/// CDXC:Titlebar 2026-05-16-23:02
/// Embedded Editor is a first-party Code surface reached from the titlebar Code
/// mode, not an external Open In selection. Keep the Open In catalog focused on
/// installed editors and the first-party folder opener so the dropdown does not duplicate Code mode.
///
/// CDXC:Titlebar 2026-06-04-13:39:
/// The built-in filesystem target remains internally keyed as finder for protocol compatibility, but user-facing labels must say Open File/Folder Location so the macOS app copy is OS-agnostic.
pub const BUILT_IN_WORKSPACE_OPEN_TARGETS: J = J::Arr(&[
    J::Obj(&[
        ("commands", J::Arr(&[J::Str("cursor")])),
        ("id", J::Str("cursor")),
        ("label", J::Str("Cursor")),
        ("launchStyle", J::Str("goto")),
        ("macOSAppNames", J::Arr(&[J::Str("Cursor")])),
    ]),
    J::Obj(&[
        ("commands", J::Arr(&[J::Str("trae")])),
        ("id", J::Str("trae")),
        ("label", J::Str("Trae")),
        ("launchStyle", J::Str("goto")),
        ("macOSAppNames", J::Arr(&[J::Str("Trae")])),
    ]),
    J::Obj(&[
        ("baseArgs", J::Arr(&[J::Str("ide")])),
        ("commands", J::Arr(&[J::Str("kiro")])),
        ("id", J::Str("kiro")),
        ("label", J::Str("Kiro")),
        ("launchStyle", J::Str("goto")),
        ("macOSAppNames", J::Arr(&[J::Str("Kiro")])),
    ]),
    J::Obj(&[
        ("commands", J::Arr(&[J::Str("code")])),
        ("id", J::Str("vscode")),
        ("label", J::Str("VS Code")),
        ("launchStyle", J::Str("goto")),
        ("macOSAppNames", J::Arr(&[J::Str("Visual Studio Code")])),
        ("targetApp", J::Str("vscode")),
    ]),
    J::Obj(&[
        ("commands", J::Arr(&[J::Str("code-insiders")])),
        ("id", J::Str("vscode-insiders")),
        ("label", J::Str("VS Code Insiders")),
        ("launchStyle", J::Str("goto")),
        (
            "macOSAppNames",
            J::Arr(&[J::Str("Visual Studio Code - Insiders")]),
        ),
        ("targetApp", J::Str("vscode-insiders")),
    ]),
    J::Obj(&[
        ("commands", J::Arr(&[J::Str("codium")])),
        ("id", J::Str("vscodium")),
        ("label", J::Str("VSCodium")),
        ("launchStyle", J::Str("goto")),
        ("macOSAppNames", J::Arr(&[J::Str("VSCodium")])),
    ]),
    J::Obj(&[
        ("commands", J::Arr(&[J::Str("zed"), J::Str("zeditor")])),
        ("id", J::Str("zed")),
        ("label", J::Str("Zed")),
        ("launchStyle", J::Str("direct-path")),
        (
            "macOSAppNames",
            J::Arr(&[J::Str("Zed"), J::Str("Zed Preview")]),
        ),
        ("targetApp", J::Str("zed")),
    ]),
    J::Obj(&[
        ("commands", J::Arr(&[J::Str("agy-ide")])),
        ("id", J::Str("antigravity")),
        ("label", J::Str("Antigravity")),
        ("launchStyle", J::Str("goto")),
        ("macOSAppNames", J::Arr(&[J::Str("Antigravity")])),
    ]),
    J::Obj(&[
        ("commands", J::Arr(&[J::Str("idea")])),
        ("id", J::Str("idea")),
        ("label", J::Str("IntelliJ IDEA")),
        ("launchStyle", J::Str("line-column")),
        ("macOSAppNames", J::Arr(&[J::Str("IntelliJ IDEA")])),
    ]),
    J::Obj(&[
        ("commands", J::Arr(&[J::Str("aqua")])),
        ("id", J::Str("aqua")),
        ("label", J::Str("Aqua")),
        ("launchStyle", J::Str("line-column")),
        ("macOSAppNames", J::Arr(&[J::Str("Aqua")])),
    ]),
    J::Obj(&[
        ("commands", J::Arr(&[J::Str("clion")])),
        ("id", J::Str("clion")),
        ("label", J::Str("CLion")),
        ("launchStyle", J::Str("line-column")),
        ("macOSAppNames", J::Arr(&[J::Str("CLion")])),
    ]),
    J::Obj(&[
        ("commands", J::Arr(&[J::Str("datagrip")])),
        ("id", J::Str("datagrip")),
        ("label", J::Str("DataGrip")),
        ("launchStyle", J::Str("line-column")),
        ("macOSAppNames", J::Arr(&[J::Str("DataGrip")])),
    ]),
    J::Obj(&[
        ("commands", J::Arr(&[J::Str("dataspell")])),
        ("id", J::Str("dataspell")),
        ("label", J::Str("DataSpell")),
        ("launchStyle", J::Str("line-column")),
        ("macOSAppNames", J::Arr(&[J::Str("DataSpell")])),
    ]),
    J::Obj(&[
        ("commands", J::Arr(&[J::Str("goland")])),
        ("id", J::Str("goland")),
        ("label", J::Str("GoLand")),
        ("launchStyle", J::Str("line-column")),
        ("macOSAppNames", J::Arr(&[J::Str("GoLand")])),
    ]),
    J::Obj(&[
        ("commands", J::Arr(&[J::Str("phpstorm")])),
        ("id", J::Str("phpstorm")),
        ("label", J::Str("PhpStorm")),
        ("launchStyle", J::Str("line-column")),
        ("macOSAppNames", J::Arr(&[J::Str("PhpStorm")])),
    ]),
    J::Obj(&[
        ("commands", J::Arr(&[J::Str("pycharm")])),
        ("id", J::Str("pycharm")),
        ("label", J::Str("PyCharm")),
        ("launchStyle", J::Str("line-column")),
        ("macOSAppNames", J::Arr(&[J::Str("PyCharm")])),
    ]),
    J::Obj(&[
        ("commands", J::Arr(&[J::Str("rider")])),
        ("id", J::Str("rider")),
        ("label", J::Str("Rider")),
        ("launchStyle", J::Str("line-column")),
        ("macOSAppNames", J::Arr(&[J::Str("Rider")])),
    ]),
    J::Obj(&[
        ("commands", J::Arr(&[J::Str("rubymine")])),
        ("id", J::Str("rubymine")),
        ("label", J::Str("RubyMine")),
        ("launchStyle", J::Str("line-column")),
        ("macOSAppNames", J::Arr(&[J::Str("RubyMine")])),
    ]),
    J::Obj(&[
        ("commands", J::Arr(&[J::Str("rustrover")])),
        ("id", J::Str("rustrover")),
        ("label", J::Str("RustRover")),
        ("launchStyle", J::Str("line-column")),
        ("macOSAppNames", J::Arr(&[J::Str("RustRover")])),
    ]),
    J::Obj(&[
        ("commands", J::Arr(&[J::Str("webstorm")])),
        ("id", J::Str("webstorm")),
        ("label", J::Str("WebStorm")),
        ("launchStyle", J::Str("line-column")),
        ("macOSAppNames", J::Arr(&[J::Str("WebStorm")])),
    ]),
    // CDXC:OsIntegration 2026-09-16 DECISION: User: file manager actions say Open File/Folder Location on every OS.
    J::Obj(&[
        ("commands", J::Null),
        ("id", J::Str("finder")),
        ("label", J::Str("Open File/Folder Location")),
        ("launchStyle", J::Str("direct-path")),
    ]),
]);

pub const CUSTOM_WORKSPACE_OPEN_TARGET_ID_PREFIX: &str = "custom:";
