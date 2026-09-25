//! The board's page — developer tooling, outside デザイン規約.md's token
//! rule (nothing here ships). It owes the reader the seat that took each
//! picture, stated where it cannot be missed, and a magnifier that does
//! not interpolate: whole ratios with `image-rendering: pixelated`, since
//! the system image viewers all smooth.
//!
//! One picture opens at 1:1 — nearest neighbour under 1 drops pixels, so
//! a shrink interpolates and the ratio says `smoothed`. A row opens
//! fitted: at 1:1 the picture beside the first is off the edge. The rail
//! stays narrow so the fitted row gets the width.
//!
//! The seat filter shows the whole roster, empty seats disabled, so no
//! chip moves between readings. The choice rides in the fragment: F5
//! (`window.rs`) carries nothing else over. The arrows walk every view in
//! the order the runs went up, filter or no.

use super::Run;

const STYLE: &str = r#"
 html,body{margin:0;height:100%;background:#0b1020;color:#c8d3f0;overflow:hidden;
  font:13px/1.5 "Segoe UI",system-ui,sans-serif}
 #wrap{display:flex;height:100%}
 #side{width:230px;flex:none;overflow-y:auto;background:#0e1428;border-right:1px solid #24305a}
 #head{position:sticky;top:0;background:#0e1428;padding:9px 12px 7px;border-bottom:1px solid #1a2445}
 #head h2{margin:0 0 6px;font-size:11px;letter-spacing:.08em;color:#6f7fae;
  text-transform:uppercase;font-weight:600}
 #filter{display:flex;flex-wrap:wrap;gap:4px}
 #filter button{font:inherit;font-size:11px;padding:1px 8px;border-radius:9px;cursor:pointer;
  background:#151d3a;color:#8494c0;border:1px solid #26315c}
 #filter button.on{background:#1d2b57;color:#e6ecff;border-color:#4a63b8}
 #filter button:disabled{background:none;border-style:dashed;border-color:#2a3560;
  color:#4d598a;cursor:default}
 .run{padding:7px 12px 8px;border-bottom:1px solid #1a2445}
 .run.off{display:none}
 .run .top{display:flex;align-items:baseline;gap:6px}
 .seat{flex:none;min-width:17px;text-align:center;border-radius:3px;padding:0 4px;
  font-weight:700;font-size:11px;color:#0b1020}
 .run .lab{color:#e6ecff;font-weight:600;overflow-wrap:anywhere}
 .run .meta{color:#6f7fae;font-size:11px;margin-left:23px}
 .run .verb{color:#7aa2ff;font-family:Consolas,monospace}
 .thumbs{display:flex;flex-wrap:wrap;gap:5px;margin:5px 0 0 23px}
 .thumbs img{width:58px;height:36px;object-fit:cover;object-position:0 0;border-radius:2px;
  border:1px solid #2b3765;cursor:pointer;background:#000}
 .thumbs img.on{border-color:#5a8cff;box-shadow:0 0 0 1px #5a8cff}
 #main{flex:1;display:flex;flex-direction:column;min-width:0}
 #bar{height:34px;flex:none;display:flex;gap:12px;align-items:center;padding:0 12px;
  background:#111a33;border-bottom:1px solid #24305a;user-select:none;white-space:nowrap}
 #bar #barseat{font-size:12px;padding:1px 7px}
 #bar b{color:#e6ecff;overflow:hidden;text-overflow:ellipsis}
 #bar span{color:#7d8cb8}
 #keys{margin-left:auto}
 kbd{background:#1b2547;border:1px solid #2e3c6e;border-radius:3px;padding:1px 5px;color:#a8b6e0}
 #stage{flex:1;position:relative;overflow:hidden;cursor:grab}
 #stage.drag{cursor:grabbing}
 #imgs{position:absolute;transform-origin:0 0;display:flex;gap:24px;align-items:flex-start;
  image-rendering:pixelated}
 #imgs figure{margin:0}
 #imgs figcaption{height:28px;font:600 19px/28px "Segoe UI",system-ui,sans-serif;
  color:#8494c0;letter-spacing:.04em;white-space:nowrap}
 #imgs img{display:block;background:#000}
 #empty{position:absolute;inset:0;display:grid;place-items:center;color:#5c6a99}
"#;

/// The `#imgs` gap and `figcaption` height in the style above: the view
/// is sized before a single picture has loaded.
const GAP: u32 = 24;
const CAP: u32 = 28;

const SCRIPT: &str = r#"
const INK={a:'#7aa2ff',b:'#5ecf9a',c:'#e0a35c',d:'#d47ba8',e:'#8f7ae0',f:'#4fc3d1',main:'#8b97bd'};
const ink=s=>INK[s]||'#8b97bd';
// One entry per *view*: a run read abreast is a single view holding
// all of its pictures in a row, so one zoom and one pan move both
// halves of a before/after together.
const FLAT=[];
RUNS.forEach(function(r,ri){
 const view=(parts,si)=>({parts:parts,
  w:parts.reduce((n,p)=>n+p.w,0)+GAP*(parts.length-1),
  h:Math.max.apply(null,parts.map(p=>p.h))+(parts.some(p=>p.cap)?CAP:0),
  label:r.label,verb:r.verb,seat:r.seat,ri:ri,si:si});
 if(r.abreast)FLAT.push(view(r.shots,0));
 else r.shots.forEach((s,si)=>FLAT.push(view([s],si)))});
const side=document.getElementById('side'),imgs=document.getElementById('imgs'),
 stage=document.getElementById('stage'),barseat=document.getElementById('barseat');
function badge(seat){const b=document.createElement('span');b.className='seat';
 b.style.background=ink(seat);b.textContent=seat;b.title='taken in seat '+seat;return b}
RUNS.forEach(function(r,ri){
 const d=document.createElement('div');d.className='run';d.dataset.seat=r.seat;
 const top=document.createElement('div');top.className='top';
 const lab=document.createElement('span');lab.className='lab';lab.textContent=r.label;
 top.append(badge(r.seat),lab);
 const meta=document.createElement('div');meta.className='meta';
 meta.textContent=new Date(r.at).toLocaleString();
 if(r.verb){const v=document.createElement('span');v.className='verb';
  v.textContent=' \u00b7 '+r.verb;meta.append(v)}
 const t=document.createElement('div');t.className='thumbs';
 r.shots.forEach(function(s,si){const im=new Image();im.src=s.file;
  im.title=(s.cap?s.cap+'  ':'')+s.from+'  '+s.w+'x'+s.h;
  // Both halves of a pair point at the one view that holds them, so
  // clicking either brings up the comparison.
  im.dataset.k=String(FLAT.findIndex(f=>f.ri===ri&&f.si===(r.abreast?0:si)));
  im.onclick=()=>show(+im.dataset.k);t.append(im)});
 d.append(top,meta,t);side.append(d)});
const ROSTER=['a','b','c','d','e','f','main'];
const held=new Set(RUNS.map(r=>r.seat));
const seats=ROSTER.concat([...held].filter(s=>!ROSTER.includes(s)));
const filter=document.getElementById('filter');
const chips=new Map();
// The seat the board is held to, '' for the whole of it.
let only='';
function pick(seat){only=seat;chips.forEach((b,s)=>b.classList.toggle('on',s===seat));
 side.querySelectorAll('.run').forEach(
  r=>r.classList.toggle('off',!!seat&&r.dataset.seat!==seat))}
// The first view the filter leaves standing: the top of the board as the
// reader now sees it. Named `first`: a declaration at this scope
// cannot shadow `window.top`.
function first(){return only?FLAT.findIndex(f=>f.seat===only):0}
// The stage follows the chip that was just pressed: a
// picture it has hidden is not the one the reader asked
// for, so the top of what is left comes up instead. One
// the chip leaves standing stays put — `all` widens the
// board. Called from the chip alone: the opening runs
// `pick` before `i` is initialised.
function follow(){if(!FLAT.length||!only||FLAT[i].seat===only)return;
 const k=first();if(k>=0)show(k)}
// The chosen seat goes in the fragment, which is what the reload
// F5 makes carries over. replaceState: a file:// page is allowed it
// (measured) and it leaves no history entry behind every chip. A
// browser that refuses simply does not remember, so this is written
// after the filter has already moved.
function remember(seat){try{history.replaceState(null,'',seat?'#seat-'+seat:'#')}catch(e){}}
function chip(seat,text,empty){const b=document.createElement('button');b.textContent=text;
 if(empty){b.disabled=true;b.title='nothing from seat '+seat+' on the board'}
 else{if(seat)b.style.color=ink(seat);chips.set(seat,b);
  b.onclick=()=>{pick(seat);remember(seat);follow()}}
 filter.append(b)}
chip('','all',false);
seats.forEach(s=>chip(s,'seat '+s,!held.has(s)));
// A seat whose last picture swept away between two
// readings has a disabled chip and nothing behind it:
// fall back to the whole board.
const asked=/^#seat-(.+)$/.exec(location.hash);
pick(asked&&chips.has(asked[1])?asked[1]:'');
let i=0,z=1,x=0,y=0;
// Nearest-neighbour is only right where it *adds* pixels at a whole
// ratio. Shrinking with it drops them instead of averaging them, which
// takes the 1px rules out and swaps flat colours for whichever
// neighbour survived, so a view that does not fit is the one place the
// board must interpolate, and it says so where the ratio is written.
// The approximation mark carries the rest of that: a fit one pixel
// short of 1:1 rounds to `100%`, and a reader has to be able to tell
// that from the ratio a 1px call may be read off.
function exact(){return z>=1&&Math.abs(z-Math.round(z))<1e-9}
function draw(){imgs.style.transform='translate('+x+'px,'+y+'px) scale('+z+')';
 imgs.style.imageRendering=exact()?'pixelated':'auto';
 document.getElementById('zoom').textContent=
  (exact()?'':'≈')+(z*100).toFixed(0)+'%'+(exact()?'':' smoothed')}
// Centred while the view holds it, against the top-left corner once it
// does not: a picture read past the window's edges is read from the
// corner the app's own furniture starts at.
function place(){const s=FLAT[i];if(!s)return;
 x=s.w*z<=stage.clientWidth?(stage.clientWidth-s.w*z)/2:0;
 y=s.h*z<=stage.clientHeight?(stage.clientHeight-s.h*z)/2:0;draw()}
// What a view of one picture opens at: one image pixel on one screen
// pixel, whatever it costs in panning — a shrink is the one ratio a
// 1px call cannot be read off. Named `oneToOne`:
// `show()` keeps a local `one` for the first picture in the view,
// and a call from inside it reaches the local (measured: TypeError,
// and no opening view at all).
function oneToOne(){z=1;place()}
// The whole view inside the window, the one thing 1:1 cannot give, and
// the only place a ratio under 1 is reached — from `0`, and from the
// opening of a row that holds more than one picture. Magnifies as well,
// held to a whole ratio so a view smaller than the window fills it
// without going soft.
function fit(){const s=FLAT[i];if(!s)return;
 const raw=Math.min(stage.clientWidth/s.w,stage.clientHeight/s.h);
 z=raw>=1?Math.floor(raw):raw;place()}
// What a view opens at. One picture at 1:1; a row of them at the ratio
// `0` gives, because at 1:1 the picture beside the first stands off the
// window's edge and the comparison is back in the reader's memory.
function opening(){if(FLAT[i].parts.length>1)fit();else oneToOne()}
function show(n){if(!FLAT.length)return;i=(n+FLAT.length)%FLAT.length;const s=FLAT[i];
 imgs.textContent='';imgs.style.display='';
 // The width and height are written on every picture, so the row is
 // laid out to its real size before any of them has loaded: otherwise
 // the first fit() of a view measures nothing and centres it wrongly.
 s.parts.forEach(function(p){const fig=document.createElement('figure');
  if(p.cap){const c=document.createElement('figcaption');c.textContent=p.cap;fig.append(c)}
  const im=new Image();im.src=p.file;im.width=p.w;im.height=p.h;fig.append(im);imgs.append(fig)});
 barseat.textContent=s.seat;barseat.style.background=ink(s.seat);
 barseat.className='seat';barseat.title='taken in seat '+s.seat;
 document.getElementById('name').textContent=s.label;
 const one=s.parts[0];
 document.getElementById('from').textContent=
  (s.parts.length>1?s.parts.map(p=>p.cap||p.from).join(' | ')+'  ':one.from+'  ')
  +one.w+'x'+one.h+(s.parts.length>1?' each':'')+(s.verb?'  \u00b7 '+s.verb:'');
 side.querySelectorAll('.thumbs img').forEach(e=>e.classList.toggle('on',+e.dataset.k===i));
 const on=side.querySelector('.thumbs img.on');
 if(on)on.scrollIntoView({block:'nearest'});opening()}
function zoomAt(cx,cy,nz){x=cx-(cx-x)*(nz/z);y=cy-(cy-y)*(nz/z);z=nz;draw()}
stage.addEventListener('wheel',function(e){e.preventDefault();const r=stage.getBoundingClientRect();
 zoomAt(e.clientX-r.left,e.clientY-r.top,
  Math.min(32,Math.max(.05,z*(e.deltaY<0?1.25:.8))))},{passive:false});
let drag=null;
stage.addEventListener('pointerdown',function(e){drag={x:e.clientX-x,y:e.clientY-y};
 stage.classList.add('drag');stage.setPointerCapture(e.pointerId)});
stage.addEventListener('pointermove',function(e){if(!drag)return;
 x=e.clientX-drag.x;y=e.clientY-drag.y;draw()});
stage.addEventListener('pointerup',function(){drag=null;stage.classList.remove('drag')});
addEventListener('keydown',function(e){
 if(e.key==='ArrowRight')show(i+1);
 else if(e.key==='ArrowLeft')show(i-1);
 else if(e.key==='0')fit();
 else if('1248'.indexOf(e.key)>=0){const r=stage.getBoundingClientRect();
  zoomAt(r.width/2,r.height/2,+e.key)}});
addEventListener('resize',place);
if(FLAT.length){document.getElementById('empty').remove();show(Math.max(first(),0))}
else{imgs.style.display='none'}
"#;

const BODY: &str = r#"<div id="wrap"><div id="side"><div id="head">
 <h2>shots &mdash; read top down</h2><div id="filter"></div></div></div>
 <div id="main"><div id="bar"><span class="seat" id="barseat"></span>
  <b id="name"></b><span id="from"></span><b id="zoom"></b>
  <span id="keys"><kbd>wheel</kbd> zoom <kbd>drag</kbd> pan <kbd>0</kbd> fit
  <kbd>1</kbd><kbd>2</kbd><kbd>4</kbd><kbd>8</kbd> exact
  <kbd>&larr;</kbd><kbd>&rarr;</kbd> next <kbd>F5</kbd> reload</span></div>
  <div id="stage"><div id="imgs"></div>
  <div id="empty">no shots yet &mdash; <kbd>F5</kbd> once there are</div></div></div></div>"#;

pub(super) fn render(runs: &[Run]) -> String {
    let mut out = String::from(
        "<!doctype html><html><head><meta charset=\"utf-8\">\
         <title>platitude shots</title><style>",
    );
    out.push_str(STYLE);
    out.push_str("</style></head><body>");
    out.push_str(BODY);
    out.push_str("<script>const RUNS=");
    out.push_str(&data(runs));
    out.push_str(&format!(",GAP={GAP},CAP={CAP};"));
    out.push_str(SCRIPT);
    out.push_str("</script></body></html>\n");
    out
}

/// The runs as a JavaScript literal.
fn data(runs: &[Run]) -> String {
    let mut out = String::from("[");
    for (n, run) in runs.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{label:{},verb:{},seat:{},at:{},abreast:{},shots:[",
            js(&run.label),
            js(&run.verb),
            js(&run.seat),
            run.at,
            run.side_by_side
        ));
        for (m, shot) in run.shots.iter().enumerate() {
            if m > 0 {
                out.push(',');
            }
            out.push_str(&format!(
                "{{file:{},from:{},cap:{},w:{},h:{}}}",
                js(&shot.file),
                js(&shot.from),
                js(&shot.caption),
                shot.width,
                shot.height
            ));
        }
        out.push_str("]}");
    }
    out.push(']');
    out
}

/// A JavaScript string literal for text nobody vetted. `<` and `>` are
/// escaped too: a `</script>` in a label would end the block.
fn js(text: &str) -> String {
    let mut out = String::from("\"");
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            ch if (ch as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::super::{Run, Shot};
    use super::{js, render};

    fn run(seat: &str, label: &str) -> Run {
        Run {
            label: label.to_string(),
            verb: "row-card".to_string(),
            seat: seat.to_string(),
            at: 1_700_000_000_000,
            side_by_side: false,
            shots: vec![Shot {
                file: "img/x.png".to_string(),
                from: "app.png".to_string(),
                caption: String::new(),
                width: 1440,
                height: 900,
            }],
        }
    }

    /// The one thing the page has to get right.
    #[test]
    fn every_run_carries_its_seat_onto_the_page() {
        let page = render(&[run("a", "chip padding"), run("e", "graph lanes")]);
        assert!(page.contains("seat:\"a\""));
        assert!(page.contains("seat:\"e\""));
    }

    /// A label is written by hand and can hold anything.
    #[test]
    fn a_label_cannot_end_the_script() {
        let page = render(&[run("a", "</script><img src=x onerror=alert(1)>")]);
        assert!(!page.contains("</script><img"));
        assert!(page.contains("\\u003c/script"));
    }

    #[test]
    fn an_empty_board_is_still_a_page() {
        let page = render(&[]);
        assert!(page.contains("const RUNS=[]"));
        assert!(page.contains("no shots yet"));
    }

    /// So a chip is in the same place on every reading.
    #[test]
    fn the_filter_carries_the_whole_roster() {
        let roster = "const ROSTER=['a','b','c','d','e','f','main']";
        assert!(render(&[]).contains(roster));
        assert!(render(&[run("a", "chip padding")]).contains(roster));
    }

    /// F5 carries the fragment and nothing else.
    #[test]
    fn the_chosen_seat_survives_a_reload() {
        let page = render(&[run("a", "chip padding")]);
        assert!(page.contains("history.replaceState(null,'',seat?'#seat-'+seat:'#')"));
        assert!(page.contains("/^#seat-(.+)$/.exec(location.hash)"));
    }

    /// The chip moves the stage as well as the list: a board held to one
    /// seat must not be showing another seat's picture.
    #[test]
    fn a_chip_that_hides_the_shown_picture_hands_over_the_top_of_what_is_left() {
        let page = render(&[run("a", "chip padding"), run("e", "graph lanes")]);
        assert!(page.contains("b.onclick=()=>{pick(seat);remember(seat);follow()}"));
        assert!(page.contains("function first(){return only?FLAT.findIndex(f=>f.seat===only):0}"));
        // And leaves it alone where the chip hides nothing.
        assert!(
            page.contains("function follow(){if(!FLAT.length||!only||FLAT[i].seat===only)return")
        );
        // The same on the way in from F5: the fragment is read before
        // the first picture is chosen.
        assert!(page.contains("show(Math.max(first(),0))"));
    }

    #[test]
    fn a_pair_reaches_the_page_as_one_view() {
        let mut pair = run("a", "the stopped landing");
        pair.side_by_side = true;
        pair.shots[0].caption = "before".to_string();
        pair.shots.push(Shot {
            file: "img/y.png".to_string(),
            from: "app.png".to_string(),
            caption: "after".to_string(),
            width: 1440,
            height: 900,
        });
        let page = render(&[pair]);
        assert!(page.contains("abreast:true"));
        assert!(page.contains("cap:\"before\""));
        assert!(page.contains("cap:\"after\""));
        // The script sizes the row from these.
        assert!(page.contains("GAP=24"));
        assert!(page.contains("CAP=28"));
    }

    #[test]
    fn an_ordinary_run_is_read_one_picture_at_a_time() {
        assert!(render(&[run("a", "chip padding")]).contains("abreast:false"));
    }

    #[test]
    fn a_view_of_one_picture_opens_at_one_image_pixel_per_screen_pixel() {
        let page = render(&[run("a", "chip padding")]);
        assert!(page.contains("function oneToOne(){z=1;place()}"));
        // Both ways in: the picture chosen, and the window reshaped. The
        // name is load-bearing — `one` is a local inside `show()`.
        assert!(page.contains("scrollIntoView({block:'nearest'});opening()}"));
        assert!(page.contains("addEventListener('resize',place)"));
        // No 100% ceiling on the fit: `0` may magnify a small picture as
        // well as shrink.
        assert!(!page.contains("stage.clientHeight/s.h,1)"));
    }

    #[test]
    fn a_row_of_pictures_opens_with_the_whole_row_in_the_window() {
        let page = render(&[run("a", "chip padding")]);
        assert!(
            page.contains("function opening(){if(FLAT[i].parts.length>1)fit();else oneToOne()}")
        );
    }

    #[test]
    fn only_a_whole_ratio_is_read_without_interpolation() {
        let page = render(&[run("a", "chip padding")]);
        assert!(page.contains("function exact(){return z>=1&&Math.abs(z-Math.round(z))<1e-9}"));
        assert!(page.contains("imgs.style.imageRendering=exact()?'pixelated':'auto'"));
        // `≈` keeps a fit that rounds to `100%` from reading as exact.
        assert!(page.contains("(exact()?'':'≈')+(z*100).toFixed(0)+'%'+(exact()?'':' smoothed')"));
        // No `image-rendering` on `#imgs img`: a rule there outranks what
        // `draw()` writes on the parent, and the flip would do nothing.
        assert!(page.contains("#imgs img{display:block;background:#000}"));
    }

    #[test]
    fn a_control_character_becomes_an_escape() {
        assert_eq!(js("a\u{1}b"), "\"a\\u0001b\"");
    }
}
