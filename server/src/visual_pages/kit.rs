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
//! `<gx-questions>` lays its children out in one centred column (`--gx-column`, 720px) and lets a
//! `<gx-wide>` child span the page's full width. A choice question (`multiple` for several picks)
//! letters its choices, ends with "Other / none of these", and has a note box; a `type="text"`
//! question is a text box. The kit adds the send button, posts the answers with `ghostex.submit`
//! keyed by each question's `name` (its title when it has none), with a non-empty note as
//! `"<key> (note)"`, and shows a sent page's answers locked when it is opened again.
//!
//! CDXC:SessionChat 2026-10-11 DECISION:
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
.gx-q-head{display:flex;gap:8px;align-items:baseline;margin-bottom:4px}
.gx-q-tag{flex-shrink:0;font:11px/1.6 var(--font-mono);color:var(--muted-foreground);border:1px solid var(--border);border-radius:999px;padding:0 7px}
.gx-q-title{font-size:14px;font-weight:600;line-height:1.45}
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
.gx-letter{font-weight:600}
.gx-other .gx-choice-body{color:var(--muted-foreground)}
.gx-other[aria-checked=true] .gx-choice-body{color:inherit}
.gx-rec{display:inline-block;margin-left:6px;padding:0 6px;border:1px solid var(--primary);border-radius:999px;color:var(--primary);font-size:10.5px;font-weight:600;line-height:1.5;vertical-align:1px}
.gx-choice-body small{display:block;color:var(--muted-foreground);font-size:12px;margin-top:2px}
.gx-note,.gx-text{display:block;width:100%;box-sizing:border-box;margin:8px 0 0;min-height:34px;resize:vertical;field-sizing:content;font:inherit;font-size:12.5px;color:var(--foreground);background:transparent;border:1px solid var(--border);border-radius:var(--radius);padding:7px 9px}
.gx-text{min-height:64px}
.gx-note:focus,.gx-text:focus{outline:none;border-color:var(--ring);box-shadow:0 0 0 3px color-mix(in srgb,var(--ring) 20%,transparent)}
.gx-note::placeholder,.gx-text::placeholder{color:var(--muted-foreground)}
.gx-note[readonly],.gx-text[readonly]{resize:none}
.gx-note[hidden],.gx-text[hidden]{display:none}
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
function question(qe,index,used){
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
    var area=el("textarea","gx-text");area.placeholder=qe.getAttribute("placeholder")||"Your answer (optional)";area.setAttribute("aria-labelledby",titleId);qe.append(area);
    return{collect:function(out){out[key]=area.value.trim()||null;},
      restore:function(answers){if(typeof answers[key]==="string")area.value=answers[key];},
      lock:function(){area.readOnly=true;if(!area.value)area.hidden=true;}};
  }
  var list=el("div","gx-choices"),choices=[];
  list.setAttribute("role",multiple?"group":"radiogroup");list.setAttribute("aria-labelledby",titleId);
  var note=el("textarea","gx-note");note.rows=1;note.setAttribute("aria-label","Note");
  function sync(){note.placeholder=other.on()?"What would you prefer instead? (optional)":"Note (optional)";}
  function choice(nodes,value,recommended){
    var button=el("button","gx-choice"),body=el("span","gx-choice-body"),index=choices.length;
    button.type="button";button.setAttribute("role",multiple?"checkbox":"radio");button.setAttribute("aria-checked","false");
    body.append(el("b","gx-letter",(index<26?String.fromCharCode(97+index):index+1)+")")," ");
    nodes.forEach(function(node){body.append(node);});
    if(recommended)body.insertBefore(el("span","gx-rec","Recommended"),body.querySelector(":scope>small"));
    button.append(el("span","gx-mark"),body);list.append(button);
    var c={button:button,value:value,on:function(){return button.getAttribute("aria-checked")==="true";},set:function(v){button.setAttribute("aria-checked",v?"true":"false");}};
    button.addEventListener("click",function(){var on=!c.on();if(on&&!multiple)choices.forEach(function(o){o.set(false);});c.set(on);sync();if(on&&c===other)note.focus();});
    choices.push(c);return c;
  }
  choiceEls.forEach(function(ce){choice(kids(ce),(ce.getAttribute("value")||"").trim()||labelText(ce),ce.hasAttribute("recommended"));});
  var other=choice([document.createTextNode(OTHER)],OTHER,false);other.button.classList.add("gx-other");
  sync();qe.append(list,note);
  return{collect:function(out){var on=choices.filter(function(c){return c.on();}).map(function(c){return c.value;});
      out[key]=multiple?(on.length?on:null):on[0]||null;var written=note.value.trim();if(written)out[key+" (note)"]=written;},
    restore:function(answers){var v=answers[key],picked=Array.isArray(v)?v:v==null?[]:[v];
      choices.forEach(function(c){c.set(picked.indexOf(c.value)>=0);});
      if(typeof answers[key+" (note)"]==="string")note.value=answers[key+" (note)"];sync();},
    lock:function(){choices.forEach(function(c){c.button.disabled=true;});note.readOnly=true;if(!note.value)note.hidden=true;}};
}
function setup(root){
  var used={},questions=Array.prototype.map.call(root.querySelectorAll("gx-question"),function(qe,index){return question(qe,index,used);});
  var bar=el("div","gx-send"),button=el("button","gx-send-button",root.getAttribute("submit")||"Send answers"),status=el("span","gx-send-status","Questions you leave blank are sent as unanswered.");
  button.type="button";bar.append(button,status);root.append(bar);
  function lock(){questions.forEach(function(q){q.lock();});button.disabled=true;button.textContent="Sent";status.textContent="";}
  if(!window.ghostex){button.disabled=true;status.textContent="This Ghostex can't send answers from a page; type them in the chat.";return;}
  if(ghostex.answered){var answers=ghostex.answers||{};questions.forEach(function(q){q.restore(answers);});lock();return;}
  button.addEventListener("click",function(){
    var out={};questions.forEach(function(q){q.collect(out);});
    button.disabled=true;status.classList.remove("gx-error");status.textContent="Sending…";
    ghostex.submit(out).then(lock,function(error){button.disabled=false;status.classList.add("gx-error");status.textContent=error&&error.message||"Your answers were not sent.";});
  });
}
function init(){var root=document.querySelector("gx-questions");if(root)setup(root);}
if(document.readyState==="loading")document.addEventListener("DOMContentLoaded",init);else init();
})();</script>"#;
