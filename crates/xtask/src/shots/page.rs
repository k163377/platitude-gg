//! The board's page.
//!
//! This is developer tooling, not the product: the token rule of
//! デザイン規約.md governs what the app paints, and nothing here ends up
//! in a build. What the page owes the reader is only this — the seat
//! that took each picture, stated where it cannot be missed, and a
//! magnifier that does not interpolate. Integer ratios with
//! `image-rendering: pixelated` are the point: a 1px design call read off
//! a smoothed enlargement is a guess, and the system image viewers all
//! smooth.
//!
//! The seat filter shows the whole roster whatever the board holds, the
//! seats with nothing on it disabled. Built from the seats that happen to
//! have runs, the row would reorder itself every time a seat's last run
//! swept away or a new seat took its first picture — a button that moves
//! between two readings of the same board is one nobody can aim at.
//!
//! Which of them is chosen rides in the page's fragment. F5 is how the
//! board is re-read (`window.rs`) and it carries nothing else over, so a
//! filter held only in a variable comes back as `all` on the one press
//! whose whole point was the picture the reader has just taken.

use super::Run;

const STYLE: &str = r#"
 html,body{margin:0;height:100%;background:#0b1020;color:#c8d3f0;overflow:hidden;
  font:13px/1.5 "Segoe UI",system-ui,sans-serif}
 #wrap{display:flex;height:100%}
 #side{width:286px;flex:none;overflow-y:auto;background:#0e1428;border-right:1px solid #24305a}
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
 #imgs{position:absolute;transform-origin:0 0;display:flex;gap:24px;align-items:flex-start}
 #imgs figure{margin:0}
 #imgs figcaption{height:28px;font:600 19px/28px "Segoe UI",system-ui,sans-serif;
  color:#8494c0;letter-spacing:.04em;white-space:nowrap}
 #imgs img{display:block;image-rendering:pixelated;background:#000}
 #empty{position:absolute;inset:0;display:grid;place-items:center;color:#5c6a99}
"#;

/// The gap between two pictures read abreast, and the band their words
/// sit in — the same numbers the style above uses, because `fit()` has
/// to know how big the view is before a single picture has loaded.
const GAP: u32 = 24;
const CAP: u32 = 28;

const SCRIPT: &str = r#"
const INK={a:'#7aa2ff',b:'#5ecf9a',c:'#e0a35c',d:'#d47ba8',e:'#8f7ae0',f:'#4fc3d1',main:'#8b97bd'};
const ink=s=>INK[s]||'#8b97bd';
// One entry per *view*, not per picture: a run read abreast is a single
// view holding all of its pictures in a row, so one zoom and one pan
// move both halves of a before/after together.
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
  // clicking either brings up the comparison rather than a half of it.
  im.dataset.k=String(FLAT.findIndex(f=>f.ri===ri&&f.si===(r.abreast?0:si)));
  im.onclick=()=>show(+im.dataset.k);t.append(im)});
 d.append(top,meta,t);side.append(d)});
const ROSTER=['a','b','c','d','e','f','main'];
const held=new Set(RUNS.map(r=>r.seat));
const seats=ROSTER.concat([...held].filter(s=>!ROSTER.includes(s)));
const filter=document.getElementById('filter');
const chips=new Map();
function pick(seat){chips.forEach((b,s)=>b.classList.toggle('on',s===seat));
 side.querySelectorAll('.run').forEach(
  r=>r.classList.toggle('off',!!seat&&r.dataset.seat!==seat))}
// The chosen seat goes in the fragment, which is what the reload F5 makes
// carries over. replaceState rather than location.hash: a file:// page is
// allowed it (measured) and it leaves no history entry behind every chip.
// A browser that refuses simply does not remember, so this is written
// after the filter has already moved.
function remember(seat){try{history.replaceState(null,'',seat?'#seat-'+seat:'#')}catch(e){}}
function chip(seat,text,empty){const b=document.createElement('button');b.textContent=text;
 if(empty){b.disabled=true;b.title='nothing from seat '+seat+' on the board'}
 else{if(seat)b.style.color=ink(seat);chips.set(seat,b);
  b.onclick=()=>{pick(seat);remember(seat)}}
 filter.append(b)}
chip('','all',false);
seats.forEach(s=>chip(s,'seat '+s,!held.has(s)));
// A seat whose last picture swept away between two readings has a
// disabled chip and nothing behind it: fall back to the whole board
// rather than to a filter that hides every run on it.
const asked=/^#seat-(.+)$/.exec(location.hash);
pick(asked&&chips.has(asked[1])?asked[1]:'');
let i=0,z=1,x=0,y=0;
function draw(){imgs.style.transform='translate('+x+'px,'+y+'px) scale('+z+')';
 document.getElementById('zoom').textContent=(z*100).toFixed(0)+'%'}
function fit(){const s=FLAT[i];if(!s)return;
 z=Math.min(stage.clientWidth/s.w,stage.clientHeight/s.h,1);
 x=(stage.clientWidth-s.w*z)/2;y=(stage.clientHeight-s.h*z)/2;draw()}
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
 const on=side.querySelector('.thumbs img.on');if(on)on.scrollIntoView({block:'nearest'});fit()}
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
addEventListener('resize',fit);
if(FLAT.length){document.getElementById('empty').remove();show(0)}else{imgs.style.display='none'}
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

/// The whole page: a few KB whatever the board holds, because the runs
/// carry paths rather than pictures.
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

/// A JavaScript string literal for text nobody vetted. Labels are written
/// by hand, so `<` and `>` leave as escapes too: `</script>` inside one
/// would otherwise end the block and put the rest of the board on the page
/// as markup.
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

    /// The one thing the page must never get wrong.
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

    /// The filter's buttons stand in one order however few seats have
    /// taken anything, so a chip is in the same place on every reading.
    #[test]
    fn the_filter_carries_the_whole_roster() {
        let roster = "const ROSTER=['a','b','c','d','e','f','main']";
        assert!(render(&[]).contains(roster));
        assert!(render(&[run("a", "chip padding")]).contains(roster));
    }

    /// F5 carries the fragment and nothing else, so the chosen seat has
    /// to be written there and read back out of it: the reader who
    /// filtered the board down to their own seat pressed it to see the
    /// picture they had just taken, not everybody's.
    #[test]
    fn the_chosen_seat_survives_a_reload() {
        let page = render(&[run("a", "chip padding")]);
        assert!(page.contains("history.replaceState(null,'',seat?'#seat-'+seat:'#')"));
        assert!(page.contains("/^#seat-(.+)$/.exec(location.hash)"));
    }

    /// A pair reaches the page as one view: the flag the script reads to
    /// put the two side by side, and the word that goes over each.
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
        // The row's width is the script's to work out, and it needs the
        // gap and the caption band the style leaves room for.
        assert!(page.contains("GAP=24"));
        assert!(page.contains("CAP=28"));
    }

    /// Everything else stays one picture per view.
    #[test]
    fn an_ordinary_run_is_read_one_picture_at_a_time() {
        assert!(render(&[run("a", "chip padding")]).contains("abreast:false"));
    }

    #[test]
    fn a_control_character_becomes_an_escape() {
        assert_eq!(js("a\u{1}b"), "\"a\\u0001b\"");
    }
}
