use crate::rows::{row, section, Page, Section};

pub(crate) fn page() -> Page {
    Page {
        id: "openTargets",
        title: "Open In",
        sections: vec![open_in(), custom_open_targets()],
    }
}

pub(crate) fn open_in() -> Section {
    section(
        "openIn",
        "Open In",
        vec![
            row(
                "builtin:cursor",
                "Cursor",
                "Show or hide this app on session Open In menus.",
            ),
            row(
                "builtin:trae",
                "Trae",
                "Show or hide this app on session Open In menus.",
            ),
            row(
                "builtin:kiro",
                "Kiro",
                "Show or hide this app on session Open In menus.",
            ),
            row(
                "builtin:vscode",
                "VS Code",
                "Show or hide this app on session Open In menus.",
            ),
            row(
                "builtin:vscode-insiders",
                "VS Code Insiders",
                "Show or hide this app on session Open In menus.",
            ),
            row(
                "builtin:vscodium",
                "VSCodium",
                "Show or hide this app on session Open In menus.",
            ),
            row(
                "builtin:zed",
                "Zed",
                "Show or hide this app on session Open In menus.",
            ),
            row(
                "builtin:antigravity",
                "Antigravity",
                "Show or hide this app on session Open In menus.",
            ),
            row(
                "builtin:idea",
                "IntelliJ IDEA",
                "Show or hide this app on session Open In menus.",
            ),
            row(
                "builtin:aqua",
                "Aqua",
                "Show or hide this app on session Open In menus.",
            ),
            row(
                "builtin:clion",
                "CLion",
                "Show or hide this app on session Open In menus.",
            ),
            row(
                "builtin:datagrip",
                "DataGrip",
                "Show or hide this app on session Open In menus.",
            ),
            row(
                "builtin:dataspell",
                "DataSpell",
                "Show or hide this app on session Open In menus.",
            ),
            row(
                "builtin:goland",
                "GoLand",
                "Show or hide this app on session Open In menus.",
            ),
            row(
                "builtin:phpstorm",
                "PhpStorm",
                "Show or hide this app on session Open In menus.",
            ),
            row(
                "builtin:pycharm",
                "PyCharm",
                "Show or hide this app on session Open In menus.",
            ),
            row(
                "builtin:rider",
                "Rider",
                "Show or hide this app on session Open In menus.",
            ),
            row(
                "builtin:rubymine",
                "RubyMine",
                "Show or hide this app on session Open In menus.",
            ),
            row(
                "builtin:rustrover",
                "RustRover",
                "Show or hide this app on session Open In menus.",
            ),
            row(
                "builtin:webstorm",
                "WebStorm",
                "Show or hide this app on session Open In menus.",
            ),
            row(
                "builtin:finder",
                "Open File/Folder Location",
                "Show or hide this app on session Open In menus.",
            ),
        ],
    )
}

pub(crate) fn custom_open_targets() -> Section {
    section(
        "customOpenTargets",
        "Custom Open Targets",
        vec![row(
            "addTarget",
            "Add target",
            "Add a custom command Ghostex uses to open workspaces.",
        )],
    )
}
