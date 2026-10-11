pub(crate) mod native_chat {
    pub(crate) mod fonts {
        pub(crate) const CHAT_MONO: &str = "JetBrainsMono Nerd Font";
    }
    pub(crate) mod markdown_style {
        use gpui_component::highlighter::HighlightTheme;
        use std::sync::{Arc, LazyLock};

        #[derive(serde::Deserialize)]
        struct CodeThemes {
            dark: HighlightTheme,
            light: HighlightTheme,
        }

        static THEMES: LazyLock<(Arc<HighlightTheme>, Arc<HighlightTheme>)> = LazyLock::new(|| {
            let themes: CodeThemes = serde_json::from_str(include_str!(
                "../../../../../packages/gx-chat-core/visual/code-theme.json"
            ))
            .expect("shared code theme");
            (Arc::new(themes.dark), Arc::new(themes.light))
        });

        pub(crate) fn highlight_theme(light: bool) -> Arc<HighlightTheme> {
            if light {
                THEMES.1.clone()
            } else {
                THEMES.0.clone()
            }
        }
    }
}
#[path = "helpers.rs"]
pub(crate) mod helpers;
#[path = "native_docs.rs"]
pub(crate) mod native_docs;
#[path = "window.rs"]
pub(crate) mod window;
