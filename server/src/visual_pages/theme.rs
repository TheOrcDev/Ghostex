/// The `<style id="ghostex-theme">` every published page starts with: Ghostex's default dark
/// palette, the light palette when the viewer asks for it or the viewer's system is light, and a plain base so an unstyled
/// page already reads like the app. It comes first in `<head>`, so the page's own CSS wins.
///
/// Neutrals come from the app's default theme (`packages/core-ui/styles/theme.css` for dark,
/// `packages/core-ui/styles/modals-light.css` for light); status colors from the chat's status
/// tones (`packages/gx-chat-core/visual/status-tone.json`).
pub(super) const THEME_STYLE: &str = concat!(
    "<style id=\"ghostex-theme\">",
    ":root{color-scheme:dark;",
    "--background:#0e0e0e;--foreground:#c8cdd5;",
    "--muted:#2a2a2a;--muted-foreground:#747b85;",
    "--card:#252525;--card-foreground:#c8cdd5;",
    "--border:rgba(255,255,255,0.11);--input:rgba(255,255,255,0.15);--ring:#737373;",
    "--primary:#7da4f8;--primary-foreground:#0e0e0e;",
    "--accent:#2a2a2a;--accent-foreground:#e5e7eb;",
    "--destructive:#fb7185;--warning:#fbbf24;--success:#34d399;--info:#86d3f8;",
    "--code-background:#161616;--code-foreground:#e6edf3;",
    "--chart-1:#60a5fa;--chart-2:#f59e0b;--chart-3:#34d399;",
    "--chart-4:#f472b6;--chart-5:#a78bfa;--chart-6:#22d3ee;",
    "--radius:8px;",
    "--font-sans:-apple-system, BlinkMacSystemFont, \"Segoe UI\", system-ui, sans-serif;",
    "--font-mono:\"SF Mono\", Menlo, Consolas, \"Liberation Mono\", monospace}",
    // The light palette applies when the viewer asks for it (`#gx-theme=light`, see
    // `PAGE_BOOTSTRAP`) or, without that, when the system is light.
    "@media (prefers-color-scheme: light){:root:not([data-gx-theme=dark]){color-scheme:light;",
    "--background:#ffffff;--foreground:#262626;",
    "--muted:#f1f1f1;--muted-foreground:#626262;",
    "--card:#ffffff;--card-foreground:#262626;",
    "--border:rgba(0,0,0,0.14);--input:rgba(0,0,0,0.16);--ring:#737373;",
    "--primary:#2563eb;--primary-foreground:#ffffff;",
    "--accent:#e9e9e9;--accent-foreground:#262626;",
    "--destructive:#b91c1c;--warning:#b45309;--success:#047857;--info:#0369a1;",
    "--code-background:#f6f8fa;--code-foreground:#1f2328;",
    "--chart-1:#2563eb;--chart-2:#d97706;--chart-3:#059669;",
    "--chart-4:#db2777;--chart-5:#7c3aed;--chart-6:#0891b2}}",
    ":root[data-gx-theme=light]{color-scheme:light;",
    "--background:#ffffff;--foreground:#262626;",
    "--muted:#f1f1f1;--muted-foreground:#626262;",
    "--card:#ffffff;--card-foreground:#262626;",
    "--border:rgba(0,0,0,0.14);--input:rgba(0,0,0,0.16);--ring:#737373;",
    "--primary:#2563eb;--primary-foreground:#ffffff;",
    "--accent:#e9e9e9;--accent-foreground:#262626;",
    "--destructive:#b91c1c;--warning:#b45309;--success:#047857;--info:#0369a1;",
    "--code-background:#f6f8fa;--code-foreground:#1f2328;",
    "--chart-1:#2563eb;--chart-2:#d97706;--chart-3:#059669;",
    "--chart-4:#db2777;--chart-5:#7c3aed;--chart-6:#0891b2}",
    "html{background:var(--background);color:var(--foreground);font-family:var(--font-sans);",
    "font-size:14px;line-height:1.5;-webkit-font-smoothing:antialiased}",
    "body{margin:0;padding:16px}",
    "code,kbd,pre,samp{font-family:var(--font-mono)}",
    "</style>",
);

/// Runs first in every published page, before the page's own styles and scripts.
///
/// It reads the theme a Ghostex window hands the page in its address (`#gx-theme=light` or
/// `#gx-theme=dark`), so a page opened over the chat follows the app's appearance rather than the
/// system's from its first paint, and drops that fragment so the page's own hash routing never
/// sees it. A web link the reader clicks opens in a new window, which the floating window and the
/// browser both send to the reader's browser, so the page itself is never navigated away.
pub(super) const PAGE_BOOTSTRAP: &str = concat!(
    "<script>(function(){var d=document.documentElement;",
    "try{var m=/[#&]gx-theme=(light|dark)/.exec(location.hash);",
    "if(m){d.setAttribute(\"data-gx-theme\",m[1]);",
    "history.replaceState(history.state,\"\",location.pathname+location.search);}}catch(e){}",
    "document.addEventListener(\"click\",function(e){var t=e.target,l=t&&t.closest?t.closest(\"a[href]\"):null,u;",
    "if(!l)return;try{u=new URL(l.getAttribute(\"href\"),document.baseURI);}catch(x){return;}",
    "if(!/^https?:$/.test(u.protocol)||u.href.split(\"#\")[0]===location.href.split(\"#\")[0])return;",
    "l.setAttribute(\"target\",\"_blank\");l.setAttribute(\"rel\",\"noopener\");},true);})();</script>",
);

/// Where `serve` puts the page's answered state for `PAGE_API` to read.
pub(crate) const PAGE_STATE_MARKER: &str = "<!--gx-page-state-->";

/// The page's side of answering: `ghostex.submit(answers)` posts any JSON to the page's own
/// `/answers` route, and `ghostex.answered` / `ghostex.answers` tell a reopened page what was sent.
/// A page that was answered also shows a short note, so a reader who reopens it sees the answers
/// went through even when the page itself ignores the state. The note goes into the page's
/// `[data-gx-answered]` element when it has one, else at the top of the question kit's column
/// (`<gx-questions>`, kit.rs), so it is as wide as the questions, else at the top of the page.
/// The route is answers.rs; serve.rs injects the state.
pub(super) const PAGE_API: &str = r#"<script>(function(){var st=null;
function note(){if(!st||!st.answered||!document.body||document.getElementById("gx-answered"))return;var b=document.createElement("div");b.id="gx-answered";b.setAttribute("role","status");b.setAttribute("style","margin:0 0 12px;padding:8px 12px;border-radius:8px;font:500 12.5px/1.4 var(--font-sans);color:var(--success);background:color-mix(in srgb,var(--success) 12%,transparent);border:1px solid color-mix(in srgb,var(--success) 32%,transparent)");var when="";try{when=st.answeredAt?" on "+new Date(st.answeredAt).toLocaleString():"";}catch(e){}b.textContent="Answered"+when+". The agent has your answers.";var slot=document.querySelector("[data-gx-answered]"),kit=document.querySelector("gx-questions");if(slot)slot.appendChild(b);else if(kit){b.style.margin="0";kit.insertBefore(b,kit.firstChild);}else document.body.insertBefore(b,document.body.firstChild);}
window.ghostex={get answered(){return!!(st&&st.answered);},get answers(){return st&&st.answered?st.answers:null;},
submit:function(answers){return fetch(location.pathname.replace(/\/+$/,"")+"/answers",{method:"POST",body:JSON.stringify({answers:answers===undefined?null:answers})}).then(function(r){return r.json().catch(function(){return{};}).then(function(j){if(!r.ok||!j.ok)throw new Error(j.error||"Your answers were not sent.");st={answered:true,answers:answers,answeredAt:j.answeredAt};note();return j;});});},
__state:function(s){st=s;if(s&&s.answered){if(document.body)note();else document.addEventListener("DOMContentLoaded",note);}}};})();</script>"#;
