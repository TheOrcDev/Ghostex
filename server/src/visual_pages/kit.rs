//! The question kit every published page carries: an agent writes its questions as markup and the
//! kit draws them the same way on every page.
//!
//! ```html
//! <gx-questions submit="Send to the agent">
//!   <h1>Settings redesign</h1>
//!   <gx-wide>…a full-width mockup the questions refer to…</gx-wide>
//!   <gx-question tag="Q1" title="Which layout?">
//!     <p>Why this matters.</p>
//!     <gx-choice recommended>Tabs<small>One page per area.</small></gx-choice>
//!     <gx-choice>One long page</gx-choice>
//!   </gx-question>
//!   <gx-question title="Anything else?" type="text"></gx-question>
//! </gx-questions>
//! ```
//!
//! A mockup can stand for one choice: an element with `data-question` (the question's `name`,
//! title or tag) and `data-choice` (the choice's value, label or letter), usually the element
//! holding the variant's title and its mockup, gets a Pick button by its first heading that picks
//! that choice (and shows it picked, both there and in the question), and a note box under it whose
//! text arrives as `"<question> · <choice> (note)"`.
//!
//! ```html
//! <section data-question="Q1" data-choice="tabs"><h3>A. Tabs</h3>…mockup…</section>
//! ```
//!
//! `<gx-questions>` lays its children out in one centred column (`--gx-column`, 720px) and lets a
//! `<gx-wide>` child span the page's full width. A choice question (`multiple` for several picks)
//! letters its choices, ends with "Other / none of these", and has a note box; a `type="text"`
//! question is a text box. The kit adds the send button, posts the answers with `ghostex.submit`
//! keyed by each question's `name` (its title when it has none), with a non-empty note as
//! `"<key> (note)"`, and shows a sent page's answers locked when it is opened again. Until then it
//! keeps the reader's picks as the page's draft on the host (theme.rs `PAGE_API`), so closing and
//! reopening the page brings them back.
//!
//! CDXC:SessionChat 2026-10-11 DECISION:
//! User picked from the looks page: a question's tag is a filled chip above its title ("make it show like eyebrow"), and a choice keeps its radio or checkbox with the letter as a muted keycap on the right; the in-chat form (apps/desktop/src/app/native_chat/question_form.rs, the phone's QuestionForm.tsx) matches.
//! User: a reader picks an option right at its mockup (a Pick button by the mockup's title that picks that choice in its question, shown picked on both and cleared from either), and writes a note under each mockup that arrives with the answers keyed to that question and choice, is kept as a draft, and shows in the answers card.
//! User: every question page offers "Other / none of these" with a note box, consistently, instead of each agent hand-rolling it, and the reader may leave a question unanswered. No choice is ever preselected; an agent marks the one it recommends with a Recommended badge and the reader picks. Mockups can use the full width of the page between or above the questions, and the Answered note is as wide as the question cards.
//!
//! CDXC:SessionChat 2026-10-11 SEE-ALSO:
//! The answers card folds the `"<key> (note)"` entries into their question's row (packages/gx-chat-core/src/transcript/page_answers.rs); skills/ghostex-visuals/SKILL.md teaches agents this markup. Change them together.

/// The kit's look. It comes before the page's own styles, and its heading and paragraph defaults
/// have no specificity (`:where`), so a page's own CSS always wins.
pub(super) const KIT_STYLE: &str = r#"<style id="ghostex-kit">
gx-questions{display:grid;grid-template-columns:minmax(0,1fr) minmax(0,var(--gx-column,720px)) minmax(0,1fr);row-gap:12px;align-content:start}
gx-questions>*{grid-column:2;min-width:0}
gx-questions>gx-wide{grid-column:1/-1}
gx-wide,gx-choice{display:block;min-width:0}
:where(gx-questions>h1,gx-questions>h2,gx-questions>h3,gx-questions>p){margin:0}
:where(gx-questions>h1){font-size:18px;font-weight:600;line-height:1.35}
:where(gx-questions>h2){margin-top:8px;font-size:15px;font-weight:600}
:where(gx-questions>p){color:var(--muted-foreground)}
gx-question{display:block;border:1px solid var(--border);background:var(--card);color:var(--card-foreground);border-radius:calc(var(--radius) + 2px);padding:14px 14px 12px}
.gx-q-head{display:flex;flex-direction:column;align-items:flex-start;gap:4px;margin-bottom:4px}
.gx-q-tag{font:500 10.5px/18px var(--font-mono);color:var(--muted-foreground);background:color-mix(in srgb,var(--muted-foreground) 18%,transparent);border-radius:999px;padding:0 7px}
.gx-q-title{align-self:stretch;font-size:14px;font-weight:600;line-height:1.45}
.gx-q-desc{color:var(--muted-foreground);font-size:12.5px;line-height:1.5;margin:2px 0 10px}
.gx-q-desc>:first-child{margin-top:0}.gx-q-desc>:last-child{margin-bottom:0}
.gx-choices{display:flex;flex-direction:column;gap:2px}
.gx-choice{display:flex;gap:10px;align-items:flex-start;width:100%;box-sizing:border-box;margin:0;text-align:left;padding:8px 10px;border:1px solid transparent;border-radius:var(--radius);background:none;color:inherit;font:inherit;font-size:13px;line-height:1.45;cursor:pointer}
.gx-choice:hover:not(:disabled){background:var(--accent)}
.gx-choice:focus-visible{outline:none;border-color:var(--ring);box-shadow:0 0 0 3px color-mix(in srgb,var(--ring) 20%,transparent)}
.gx-choice[aria-checked=true]{border-color:var(--primary);background:color-mix(in srgb,var(--primary) 9%,transparent)}
.gx-choice:disabled{cursor:default}
.gx-choice:disabled:not([aria-checked=true]){opacity:.5}
.gx-mark{flex-shrink:0;width:14px;height:14px;margin-top:3px;box-sizing:border-box;border:1.5px solid var(--muted-foreground);border-radius:50%}
.gx-choice[role=checkbox] .gx-mark{border-radius:4px}
.gx-choice[aria-checked=true] .gx-mark{border-color:var(--primary);background:var(--primary);box-shadow:inset 0 0 0 2.5px var(--card)}
.gx-letter{flex:none;min-width:18px;height:18px;padding:0 4px;box-sizing:border-box;margin-top:.5px;border:1px solid color-mix(in srgb,var(--border) 60%,transparent);border-radius:4px;background:color-mix(in srgb,var(--background) 40%,transparent);font:500 11px/16px var(--font-mono);color:var(--muted-foreground);text-align:center}
.gx-choice[aria-checked=true] .gx-letter{color:var(--foreground)}
.gx-other .gx-choice-body{color:var(--muted-foreground)}
.gx-other[aria-checked=true] .gx-choice-body{color:inherit}
.gx-rec{display:inline-block;margin-left:6px;padding:0 6px;border:1px solid var(--primary);border-radius:999px;color:var(--primary);font-size:10.5px;font-weight:600;line-height:1.5;vertical-align:1px}
.gx-choice-body{flex:1;min-width:0}
.gx-choice-body small{display:block;color:var(--muted-foreground);font-size:12px;margin-top:2px}
.gx-note,.gx-text{display:block;width:100%;box-sizing:border-box;margin:8px 0 0;min-height:34px;resize:vertical;field-sizing:content;font:inherit;font-size:12.5px;color:var(--foreground);background:transparent;border:1px solid var(--border);border-radius:var(--radius);padding:7px 9px}
.gx-text{min-height:64px}
.gx-note:focus,.gx-text:focus{outline:none;border-color:var(--ring);box-shadow:0 0 0 3px color-mix(in srgb,var(--ring) 20%,transparent)}
.gx-note::placeholder,.gx-text::placeholder{color:var(--muted-foreground)}
.gx-note[readonly],.gx-text[readonly]{resize:none}
.gx-note[hidden],.gx-text[hidden]{display:none}
.gx-pick{display:inline-flex;align-items:center;gap:4px;margin-left:10px;padding:2px 10px;vertical-align:middle;border:1px solid var(--border);border-radius:999px;background:transparent;color:var(--foreground);font:500 12px/1.5 var(--font-sans);cursor:pointer}
.gx-pick:hover:not(:disabled){background:var(--accent)}
.gx-pick[aria-pressed=true]{background:var(--primary);border-color:var(--primary);color:var(--primary-foreground)}
.gx-pick:disabled{cursor:default}
.gx-pick:disabled:not([aria-pressed=true]){opacity:.5}
.gx-option-head{display:flex;align-items:center;justify-content:space-between;gap:10px;margin-bottom:8px;font-weight:600}
[data-question][data-choice].gx-picked:not(h1):not(h2):not(h3):not(h4):not(h5):not(h6){outline:2px solid var(--primary);outline-offset:4px;border-radius:calc(var(--radius) + 4px)}
.gx-option-note{display:block;width:100%;box-sizing:border-box;margin:8px 0 0;min-height:34px;resize:vertical;field-sizing:content;font:inherit;font-size:12.5px;color:var(--foreground);background:transparent;border:1px solid var(--border);border-radius:var(--radius);padding:7px 9px}
.gx-option-note:focus{outline:none;border-color:var(--ring);box-shadow:0 0 0 3px color-mix(in srgb,var(--ring) 20%,transparent)}
.gx-option-note::placeholder{color:var(--muted-foreground)}
.gx-option-note[readonly]{resize:none}
.gx-option-note[hidden]{display:none}
.gx-send{display:flex;align-items:center;gap:12px;flex-wrap:wrap;margin-top:4px}
.gx-send-button{font:inherit;font-size:13px;font-weight:600;padding:8px 16px;border-radius:var(--radius);border:1px solid var(--primary);background:var(--primary);color:var(--primary-foreground);cursor:pointer}
.gx-send-button:disabled{opacity:.55;cursor:default}
.gx-send-status{font-size:12.5px;color:var(--muted-foreground)}
.gx-send-status.gx-error{color:var(--destructive)}
</style>"#;

/// The kit's behaviour: turns the first `<gx-questions>` into a working form once the page has
/// loaded, after `PAGE_API` (theme.rs) has its answered state.
pub(super) const KIT_SCRIPT: &str = r#"<script>(function(){
var OTHER="Other / none of these";
function el(tag,cls,text){var e=document.createElement(tag);if(cls)e.className=cls;if(text!=null)e.textContent=text;return e;}
function kids(node){return Array.prototype.slice.call(node.childNodes);}
function labelText(node){var c=node.cloneNode(true);c.querySelectorAll("small").forEach(function(s){s.remove();});return c.textContent.replace(/\s+/g," ").trim();}
function question(qe,index,used,changed){
  var title=(qe.getAttribute("title")||"").trim(),key=(qe.getAttribute("name")||"").trim()||title||"Question "+(index+1);
  for(var n=2;used[key];n++)key=key.replace(/ \(\d+\)$/,"")+" ("+n+")";
  used[key]=true;
  var multiple=qe.hasAttribute("multiple"),text=(qe.getAttribute("type")||"").toLowerCase()==="text";
  var titleId="gx-q-"+index,head=el("div","gx-q-head"),tag=qe.getAttribute("tag");
  qe.removeAttribute("title");
  if(tag)head.append(el("span","gx-q-tag",tag));
  var titleEl=el("div","gx-q-title",title);titleEl.id=titleId;head.append(titleEl);
  var choiceEls=[],desc=el("div","gx-q-desc");
  kids(qe).forEach(function(node){if(node.nodeName==="GX-CHOICE")choiceEls.push(node);else desc.append(node);});
  qe.textContent="";qe.append(head);
  if(desc.textContent.trim()||desc.querySelector("img,svg,canvas,table"))qe.append(desc);
  if(text){
    var area=el("textarea","gx-text");area.placeholder=qe.getAttribute("placeholder")||"Your answer (optional)";area.setAttribute("aria-labelledby",titleId);area.addEventListener("input",changed);qe.append(area);
    return{key:key,tag:(tag||"").trim(),choices:[],collect:function(out){out[key]=area.value.trim()||null;},
      restore:function(answers){if(typeof answers[key]==="string")area.value=answers[key];},
      lock:function(){area.readOnly=true;if(!area.value)area.hidden=true;}};
  }
  var list=el("div","gx-choices"),choices=[];
  list.setAttribute("role",multiple?"group":"radiogroup");list.setAttribute("aria-labelledby",titleId);
  var note=el("textarea","gx-note");note.rows=1;note.setAttribute("aria-label","Note");note.addEventListener("input",changed);
  function sync(){note.placeholder=other.on()?"What would you prefer instead? (optional)":"Note (optional)";}
  function choice(nodes,value,recommended,label){
    var button=el("button","gx-choice"),body=el("span","gx-choice-body"),index=choices.length,letter=index<26?String.fromCharCode(97+index):String(index+1);
    button.type="button";button.setAttribute("role",multiple?"checkbox":"radio");button.setAttribute("aria-checked","false");

    nodes.forEach(function(node){body.append(node);});
    if(recommended)body.insertBefore(el("span","gx-rec","Recommended"),body.querySelector(":scope>small"));
    button.append(el("span","gx-mark"),body,el("span","gx-letter",letter));list.append(button);
    var c={button:button,value:value,label:label,letter:letter,on:function(){return button.getAttribute("aria-checked")==="true";},set:function(v){button.setAttribute("aria-checked",v?"true":"false");}};
    button.addEventListener("click",function(){var on=!c.on();if(on&&!multiple)choices.forEach(function(o){o.set(false);});c.set(on);sync();changed();if(on&&c===other)note.focus();});
    choices.push(c);return c;
  }
  choiceEls.forEach(function(ce){var label=labelText(ce);choice(kids(ce),(ce.getAttribute("value")||"").trim()||label,ce.hasAttribute("recommended"),label);});
  var other=choice([document.createTextNode(OTHER)],OTHER,false,OTHER);other.button.classList.add("gx-other");
  sync();qe.append(list,note);
  return{key:key,tag:(tag||"").trim(),choices:choices,collect:function(out){var on=choices.filter(function(c){return c.on();}).map(function(c){return c.value;});
      out[key]=multiple?(on.length?on:null):on[0]||null;var written=note.value.trim();if(written)out[key+" (note)"]=written;},
    restore:function(answers){var v=answers[key],picked=Array.isArray(v)?v:v==null?[]:[v];
      choices.forEach(function(c){c.set(picked.indexOf(c.value)>=0);});
      if(typeof answers[key+" (note)"]==="string")note.value=answers[key+" (note)"];sync();},
    lock:function(){choices.forEach(function(c){c.button.disabled=true;});note.readOnly=true;if(!note.value)note.hidden=true;}};
}
/* A mockup tied to a choice (`data-question` + `data-choice`): a Pick button by its title that
   clicks that choice, and a note box under it, sent as "<question> · <choice> (note)". */
function bindOptions(questions,changed){
  var found=[];
  Array.prototype.forEach.call(document.querySelectorAll("[data-question][data-choice]"),function(node){
    if(node.closest("gx-question"))return;
    var asked=(node.getAttribute("data-question")||"").trim(),wanted=(node.getAttribute("data-choice")||"").trim();
    var q=questions.filter(function(x){return x.key===asked||(x.tag&&x.tag===asked);})[0];
    var c=q&&q.choices.filter(function(x){return x.value===wanted||x.label===wanted||x.letter===wanted.toLowerCase();})[0];
    if(!c){if(window.console)console.warn("ghostex: no choice \""+wanted+"\" in question \""+asked+"\"");return;}
    var heading=/^H[1-6]$/.test(node.tagName)?node:node.querySelector("h1,h2,h3,h4,h5,h6");
    var pick=el("button","gx-pick");pick.type="button";
    if(heading)heading.append(pick);
    else{var head=el("div","gx-option-head");head.append(el("span","gx-option-label",c.label),pick);node.insertBefore(head,node.firstChild);}
    var note=el("textarea","gx-option-note");note.rows=1;note.placeholder="Note about this option (optional)";note.setAttribute("aria-label","Note about "+c.label);
    if(heading===node){var mockup=node.nextElementSibling;node.parentNode.insertBefore(note,mockup?mockup.nextSibling:null);}else node.append(note);
    var key=q.key+" \u00b7 "+c.value+" (note)";
    pick.addEventListener("click",function(){c.button.click();});
    note.addEventListener("input",changed);
    found.push({
      sync:function(){var on=c.on();node.classList.toggle("gx-picked",on);pick.setAttribute("aria-pressed",on?"true":"false");pick.textContent=on?"\u2713 Picked":"Pick";pick.title=on?"Click again to clear":"Pick \""+c.label+"\"";pick.disabled=c.button.disabled;},
      collect:function(out){var written=note.value.trim();if(written)out[key]=written;},
      restore:function(answers){if(typeof answers[key]==="string")note.value=answers[key];},
      lock:function(){note.readOnly=true;if(!note.value)note.hidden=true;}});
  });
  return found;
}
function setup(root){
  var used={},questions=[],options=[];
  function collect(){var out={};questions.forEach(function(q){q.collect(out);});options.forEach(function(o){o.collect(out);});return out;}
  function restore(answers){questions.forEach(function(q){q.restore(answers);});options.forEach(function(o){o.restore(answers);});}
  function syncOptions(){options.forEach(function(o){o.sync();});}
  /* The reader's picks so far are kept on the host (PAGE_API's draft), so closing and reopening the page loses nothing. */
  function changed(){syncOptions();if(window.ghostex&&ghostex.__saveDraft)ghostex.__saveDraft("kit",collect());}
  questions=Array.prototype.map.call(root.querySelectorAll("gx-question"),function(qe,index){return question(qe,index,used,changed);});
  options=bindOptions(questions,changed);syncOptions();
  var bar=el("div","gx-send"),button=el("button","gx-send-button",root.getAttribute("submit")||"Send answers"),status=el("span","gx-send-status","Questions you leave blank are sent as unanswered.");
  button.type="button";bar.append(button,status);root.append(bar);
  function lock(){questions.forEach(function(q){q.lock();});options.forEach(function(o){o.lock();});syncOptions();button.disabled=true;button.textContent="Sent";status.textContent="";}
  if(!window.ghostex){button.disabled=true;status.textContent="This Ghostex can't send answers from a page; type them in the chat.";return;}
  if(ghostex.answered){restore(ghostex.answers||{});lock();return;}
  var saved=ghostex.__draft&&ghostex.__draft("kit");if(saved&&typeof saved==="object"){restore(saved);syncOptions();}
  button.addEventListener("click",function(){
    var out=collect();
    button.disabled=true;status.classList.remove("gx-error");status.textContent="Sending…";
    ghostex.submit(out).then(lock,function(error){button.disabled=false;status.classList.add("gx-error");status.textContent=error&&error.message||"Your answers were not sent.";});
  });
}
function init(){var root=document.querySelector("gx-questions");if(root)setup(root);}
if(document.readyState==="loading")document.addEventListener("DOMContentLoaded",init);else init();
})();</script>"#;
