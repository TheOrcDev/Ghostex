//! Deterministic, chat-shaped rows for the demo transcript.
//!
//! Lengths vary from a few words to several paragraphs so the list has to measure rows of very
//! different heights; some rows carry emoji, long unbreakable paths and code-ish text, the three
//! things phone text layout gets wrong first.

use gpui::SharedString;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Role {
    User,
    Assistant,
    Tool,
}

impl Role {
    pub(super) fn label(self) -> &'static str {
        match self {
            Role::User => "You",
            Role::Assistant => "Agent",
            Role::Tool => "Tool",
        }
    }
}

pub(super) struct DemoRow {
    pub id: u64,
    pub role: Role,
    pub body: SharedString,
    /// Shown under the body while the row is expanded.
    pub detail: SharedString,
    pub expanded: bool,
}

const WORDS: &[&str] = &[
    "the",
    "transcript",
    "renders",
    "inside",
    "a",
    "surface",
    "that",
    "React",
    "Native",
    "owns",
    "and",
    "GPUI",
    "draws",
    "every",
    "frame",
    "while",
    "the",
    "composer",
    "stays",
    "native",
    "so",
    "typing",
    "never",
    "waits",
    "on",
    "Rust",
    "I",
    "checked",
    "failing",
    "test",
    "again",
    "after",
    "rebasing",
    "onto",
    "main",
    "it",
    "passes",
    "now",
    "because",
    "the",
    "fixture",
    "was",
    "stale",
    "we",
    "should",
    "keep",
    "one",
    "renderer",
    "for",
    "desktop",
    "web",
    "and",
    "phone",
    "scrolling",
    "feels",
    "right",
    "when",
    "momentum",
    "decays",
    "like",
    "Android's",
    "own",
    "lists",
    "do",
    "let",
    "me",
    "look",
    "at",
    "the",
    "logs",
    "first",
    "then",
    "decide",
    "whether",
    "this",
    "belongs",
    "in",
    "gxserver",
    "or",
    "chat",
    "core",
    "tap",
    "a",
    "row",
    "to",
    "expand",
    "its",
    "details",
];

const TOOLS: &[&str] = &[
    "Ran `cargo test -p gx-chat-core`",
    "Read apps/desktop/src/app/native_chat/transcript.rs",
    "Edited packages/gx-chat-core/src/presentation/rows.rs",
    "Searched for \"ListState::new\" in 14 files",
    "Ran `bun run typecheck`",
    "Listed server/src/session_chat_*.rs",
];

const EMOJI: &[&str] = &["✅", "🚀", "🦀", "👍", "🔥", "⚠️", "🎉", "📦"];

const PATH: &str = "packages/gpui-mobile/host/src/demo/a_really_long_file_name_without_any_break_opportunities_for_wrapping.rs";

/// A small xorshift so every run (and every device) shows the same rows.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
}

fn sentence(rng: &mut Rng, words: u64) -> String {
    let mut out = String::new();
    for i in 0..words {
        let word = WORDS[rng.below(WORDS.len() as u64) as usize];
        if i == 0 {
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                out.extend(first.to_uppercase());
                out.push_str(chars.as_str());
            }
        } else {
            out.push(' ');
            out.push_str(word);
        }
    }
    out.push('.');
    out
}

fn paragraph(rng: &mut Rng, sentences: u64) -> String {
    (0..sentences)
        .map(|_| {
            let words = 4 + rng.below(14);
            sentence(rng, words)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn generated(id: u64) -> DemoRow {
    let mut rng = Rng::new(id + 1);
    let role = match id % 5 {
        0 => Role::User,
        2 => Role::Tool,
        _ => Role::Assistant,
    };
    let mut body = match role {
        Role::User => {
            let sentences = 1 + rng.below(2);
            paragraph(&mut rng, sentences)
        }
        Role::Tool => TOOLS[rng.below(TOOLS.len() as u64) as usize].to_string(),
        Role::Assistant => {
            // Mostly short replies, sometimes a long multi-paragraph one.
            let paragraphs = if rng.below(6) == 0 {
                3 + rng.below(3)
            } else {
                1
            };
            (0..paragraphs)
                .map(|_| {
                    let sentences = 1 + rng.below(4);
                    paragraph(&mut rng, sentences)
                })
                .collect::<Vec<_>>()
                .join("\n\n")
        }
    };
    if id % 7 == 3 {
        body.push(' ');
        body.push_str(EMOJI[rng.below(EMOJI.len() as u64) as usize]);
        body.push_str(EMOJI[rng.below(EMOJI.len() as u64) as usize]);
    }
    if id % 11 == 6 {
        body.push('\n');
        body.push_str(PATH);
    }
    let detail = match role {
        Role::Tool => {
            let millis = 40 + rng.below(4000);
            let sentences = 2 + rng.below(3);
            format!(
                "$ exit 0 after {millis} ms\n{}",
                paragraph(&mut rng, sentences)
            )
        }
        _ => {
            let sentences = 1 + rng.below(3);
            format!(
                "Row {id}. Details appear only while the row is expanded, so the list has to remeasure this row and keep the rows around it in place. {}",
                paragraph(&mut rng, sentences)
            )
        }
    };
    DemoRow {
        id,
        role,
        body: body.into(),
        detail: detail.into(),
        expanded: false,
    }
}

pub(super) fn typed(id: u64, text: String) -> DemoRow {
    DemoRow {
        id,
        role: Role::User,
        body: text.into(),
        detail: format!("Row {id}, sent from the React Native composer.").into(),
        expanded: false,
    }
}
