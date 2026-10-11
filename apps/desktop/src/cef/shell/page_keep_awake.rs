//! Whether a hidden page has to stay awake: it is playing sound, or the box the user last typed
//! into still holds text they have not sent.
//!
//! CDXC:CefRuntime 2026-09-30 WHY:
//! CEF's AudioHandler cannot answer "is this page playing": attaching one captures the page's
//! audio streams instead of sending them to the speakers. The page is asked instead, over the
//! DevTools protocol the appearance code already drives. Comparing form fields with their
//! defaults does not work either, because React keeps an input's default in step with its value,
//! so every page gets a listener that remembers the last field typed into, and the question asks
//! whether that field still holds text. Answers land in a thread-local that the app's sweep reads
//! on its next pass: observer callbacks run inside CEF's message pump, which can be inside a gpui
//! update, so they never touch the App.
//!
//! SEE-ALSO: apps/desktop/src/app/web_page_sleep.rs (the sweep that asks and acts).

use super::*;
use cef::{
    DevToolsMessageObserver, ImplDevToolsMessageObserver, WrapDevToolsMessageObserver,
    wrap_dev_tools_message_observer,
};
use std::time::{Duration, Instant};

/// Remembers the last text field or editor the user typed into. Search boxes are left out: a
/// query left in one is not work that sleeping the page would lose.
const PAGE_INPUT_TRACKER_SOURCE: &str = r#"(() => {
  const key = Symbol.for('ghostex.lastEditedField');
  const skipped = new Set(['search', 'checkbox', 'radio', 'range', 'color', 'file', 'hidden', 'submit', 'button', 'reset', 'image']);
  addEventListener('input', (event) => {
    const field = event.composedPath ? event.composedPath()[0] : event.target;
    if (!(field instanceof Element)) return;
    const editable = field.isContentEditable
      || field.tagName === 'TEXTAREA'
      || (field.tagName === 'INPUT' && !skipped.has(String(field.type).toLowerCase()));
    if (editable) window[key] = new WeakRef(field);
  }, true);
})();"#;

/// `true` while the page plays unmuted media or its last edited field still holds text, in the
/// page itself or a same-origin frame.
const PAGE_KEEP_AWAKE_QUESTION: &str = r#"(() => {
  const key = Symbol.for('ghostex.lastEditedField');
  const windows = [window];
  for (const frame of document.querySelectorAll('iframe')) {
    try { if (frame.contentWindow && frame.contentWindow.document) windows.push(frame.contentWindow); } catch (_) {}
  }
  for (const win of windows) {
    try {
      const field = win[key] && win[key].deref();
      if (field && field.isConnected) {
        const text = field.isContentEditable ? field.textContent : field.value;
        if (typeof text === 'string' && text.trim() !== '') return true;
      }
      for (const media of win.document.querySelectorAll('video, audio')) {
        if (!media.paused && !media.ended && !media.muted && media.volume > 0) return true;
      }
    } catch (_) {}
  }
  return false;
})()"#;

/// How long an answer stands before the page is asked again.
const PAGE_KEEP_AWAKE_ANSWER_TTL: Duration = Duration::from_secs(60);
/// A question with no answer by then is treated as lost and asked again.
const PAGE_KEEP_AWAKE_QUESTION_TIMEOUT: Duration = Duration::from_secs(10);

/// What the page last said about staying awake.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PageKeepAwake {
    /// No fresh answer yet; ask and look again on the next pass.
    Unknown,
    Keep,
    Release,
}

#[derive(Default)]
struct PageKeepAwakeProbe {
    registration: Option<cef::Registration>,
    question: Option<(c_int, Instant)>,
    answer: Option<(bool, Instant)>,
}

thread_local! {
    static PAGE_KEEP_AWAKE_PROBES: RefCell<HashMap<c_int, PageKeepAwakeProbe>> = RefCell::new(HashMap::new());
}

wrap_dev_tools_message_observer! {
    struct PageKeepAwakeObserver {}

    impl DevToolsMessageObserver {
        fn on_dev_tools_method_result(
            &self,
            browser: Option<&mut cef::Browser>,
            message_id: c_int,
            success: c_int,
            result: Option<&[u8]>,
        ) {
            let Some(browser) = browser else {
                return;
            };
            let browser_id = browser.identifier();
            PAGE_KEEP_AWAKE_PROBES.with(|probes| {
                let mut probes = probes.borrow_mut();
                let Some(probe) = probes.get_mut(&browser_id) else {
                    return;
                };
                if probe.question.map(|(id, _)| id) != Some(message_id) {
                    return;
                }
                probe.question = None;
                let keep = success != 0 && result.is_some_and(page_keep_awake_answer);
                probe.answer = Some((keep, Instant::now()));
            });
        }
    }
}

/// `Runtime.evaluate` with `returnByValue` answers `{"result":{"type":"boolean","value":…}}`; a
/// page that threw or answered anything else can sleep.
fn page_keep_awake_answer(result: &[u8]) -> bool {
    serde_json::from_slice::<serde_json::Value>(result)
        .ok()
        .and_then(|value| value.pointer("/result/value")?.as_bool())
        == Some(true)
}

/// Installs the input tracker on every document the browser loads, the current one included.
pub(crate) fn install_page_input_tracker(browser: &cef::Browser) {
    let Some(host) = browser.host() else {
        return;
    };
    // Scripts added for new documents only run while the Page domain is enabled.
    host.execute_dev_tools_method(
        next_page_appearance_devtools_message_id(),
        Some(&CefString::from("Page.enable")),
        None,
    );
    let Some(mut params) = cef::dictionary_value_create() else {
        return;
    };
    params.set_string(
        Some(&CefString::from("source")),
        Some(&CefString::from(PAGE_INPUT_TRACKER_SOURCE)),
    );
    params.set_bool(Some(&CefString::from("runImmediately")), 1);
    host.execute_dev_tools_method(
        next_page_appearance_devtools_message_id(),
        Some(&CefString::from("Page.addScriptToEvaluateOnNewDocument")),
        Some(&mut params),
    );
}

pub(crate) fn forget_page_keep_awake_probe(browser_id: c_int) {
    PAGE_KEEP_AWAKE_PROBES.with(|probes| {
        probes.borrow_mut().remove(&browser_id);
    });
}

impl CefBrowser {
    /// The page's latest answer while it is fresh.
    pub fn page_keep_awake(&self) -> PageKeepAwake {
        let browser_id = self.identifier();
        PAGE_KEEP_AWAKE_PROBES.with(|probes| {
            let probes = probes.borrow();
            let Some(probe) = probes.get(&browser_id) else {
                return PageKeepAwake::Unknown;
            };
            if let Some((keep, at)) = probe.answer
                && at.elapsed() < PAGE_KEEP_AWAKE_ANSWER_TTL
            {
                return if keep {
                    PageKeepAwake::Keep
                } else {
                    PageKeepAwake::Release
                };
            }
            // A page that cannot answer (hung or crashed) holds nothing worth keeping awake.
            if probe
                .question
                .is_some_and(|(_, at)| at.elapsed() >= PAGE_KEEP_AWAKE_QUESTION_TIMEOUT)
            {
                return PageKeepAwake::Release;
            }
            PageKeepAwake::Unknown
        })
    }

    /// Asks the page again unless a question is already on its way.
    pub fn ask_page_keep_awake(&self) {
        let browser = self.browser.borrow().clone();
        let Some(host) = browser.host() else {
            return;
        };
        let browser_id = browser.identifier();
        let (asking, needs_observer) = PAGE_KEEP_AWAKE_PROBES.with(|probes| {
            let probes = probes.borrow();
            let probe = probes.get(&browser_id);
            let asking = probe
                .and_then(|probe| probe.question)
                .is_some_and(|(_, at)| at.elapsed() < PAGE_KEEP_AWAKE_QUESTION_TIMEOUT);
            (
                asking,
                probe.is_none_or(|probe| probe.registration.is_none()),
            )
        });
        if asking {
            return;
        }
        // Host calls stay outside the registry borrow: CEF may run callbacks while they execute.
        let registration = if needs_observer {
            let mut observer = PageKeepAwakeObserver::new();
            host.add_dev_tools_message_observer(Some(&mut observer))
        } else {
            None
        };
        let Some(mut params) = cef::dictionary_value_create() else {
            return;
        };
        params.set_string(
            Some(&CefString::from("expression")),
            Some(&CefString::from(PAGE_KEEP_AWAKE_QUESTION)),
        );
        params.set_bool(Some(&CefString::from("returnByValue")), 1);
        let message_id = next_page_appearance_devtools_message_id();
        let sent = host.execute_dev_tools_method(
            message_id,
            Some(&CefString::from("Runtime.evaluate")),
            Some(&mut params),
        ) != 0;
        PAGE_KEEP_AWAKE_PROBES.with(|probes| {
            let mut probes = probes.borrow_mut();
            let probe = probes.entry(browser_id).or_default();
            if registration.is_some() {
                probe.registration = registration;
            }
            if sent {
                probe.question = Some((message_id, Instant::now()));
            }
        });
    }
}
