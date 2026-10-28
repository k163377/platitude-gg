//! The rules, held over what each context reaches: every code an
//! operation's beat, an input's press or a reading runs — the calls it
//! makes followed, so that code moved into a function, or into a function
//! of another file, is judged as it was where it stood.

use std::collections::{BTreeSet, HashMap, HashSet};

use super::Context;
use super::parse::{UnitKind, parse};
use super::product::slots;
use super::program::{Effect, Program, Value};
use crate::waits::Finding;

/// The findings of the reach rule over the harness's files (`harness`),
/// the product's QML (`product`) and the app's Rust (`rust`), each as
/// `(path, text)`. `machinery` names the files whose functions may
/// declare their reach.
pub(in crate::waits) fn findings(
    harness: &[(String, String)],
    product: &[(String, String)],
    rust: &[String],
    machinery: &[&'static str],
) -> Vec<Finding> {
    let mut docs = Vec::new();
    for (path, text) in harness.iter().chain(product.iter()) {
        docs.push(parse(path, text));
    }
    let slots = rust.iter().flat_map(|text| slots(text)).collect();
    let mut program = Program::new(docs, harness.len(), slots, machinery.to_vec());
    judge(&mut program)
}

/// How many times the program is read again before a reading that has not
/// stood still is named.
const PASSES: usize = 16;

pub(super) fn judge(program: &mut Program) -> Vec<Finding> {
    let mut found: BTreeSet<(usize, usize, &'static str)> = BTreeSet::new();
    let mut what: HashMap<(usize, usize, &'static str), String> = HashMap::new();
    for d in 0..program.harness {
        for line in &program.docs[d].unread {
            found.insert((d, *line, "unread"));
        }
    }
    for (d, line) in program.misdeclared.clone() {
        found.insert((d, line, "reach"));
    }
    // Read until what the calls hand their parameters, and what the containers hold, stands still: a helper handed
    // an operation is read as the operation's own code, and what a callee puts inside its caller's array reaches the
    // caller a reading later. A reading that never stands still is not passed as safe.
    let mut passes = 0;
    let (beats, declared_reads) = loop {
        let read = read_roots(program);
        if !program.learned() {
            break read;
        }
        if passes == PASSES {
            found.insert((0, 1, "unread"));
            what.insert(
                (0, 1, "unread"),
                format!("what the calls hand on and the containers hold did not stand still in {PASSES} readings"),
            );
            break read;
        }
        program.again();
        passes += 1;
    };
    let mut roots: Vec<(usize, Context)> = beats.iter().map(|c| (*c, Context::Beat)).collect();
    roots.extend(declared_reads.iter().map(|c| (*c, Context::Read)));
    roots.extend(program.roots.iter().map(|r| (r.code, r.context)));
    let mut counters: HashSet<(usize, usize, String)> = HashSet::new();
    for (root, context) in &roots {
        for (effect, (d, line), via) in reach(program, *root) {
            if *context == Context::Beat
                && let Effect::Write {
                    target, step: true, ..
                } = &effect
            {
                counters.insert(target.clone());
            }
            if let Some(rule) = verdict(&effect, *context, declared_reads.contains(root)) {
                found.insert((d, line, rule));
                what.entry((d, line, rule)).or_insert_with(|| {
                    let through = via.map_or(String::new(), |(vd, vl)| {
                        format!(", reached from {}:{vl}", program.docs[vd].name)
                    });
                    format!(
                        "{} in a {}{through}",
                        said(program, &effect),
                        context.name()
                    )
                });
            }
        }
    }
    for code in &program.codes {
        for (value, _, line) in &code.compares {
            if let Value::Prop(d, o, p) = value
                && counters.contains(&(*d, *o, p.clone()))
            {
                found.insert((code.doc, *line, "beats"));
            }
        }
    }
    found
        .into_iter()
        .map(|(d, line, rule)| {
            let source = program.docs[d]
                .lines
                .get(line.saturating_sub(1))
                .map(|l| l.trim().to_string())
                .unwrap_or_default();
            Finding {
                file: program.docs[d].path.clone(),
                line,
                rule,
                excerpt: match what.get(&(d, line, rule)) {
                    Some(detail) => format!("{source}  ← {detail}"),
                    None => source,
                },
            }
        })
        .collect()
}

/// Reads every code an operation's beat reaches, and every declared
/// reading of the machinery's: the beats' codes, and the readings'.
fn read_roots(program: &mut Program) -> (Vec<usize>, Vec<usize>) {
    let mut beats = Vec::new();
    for d in 0..program.harness {
        for o in 0..program.docs[d].objects.len() {
            if !program.is_operation(d, o) {
                continue;
            }
            for u in 0..program.docs[d].units.len() {
                let unit = &program.docs[d].units[u];
                if unit.object == o && unit.kind == UnitKind::Handler && unit.name == "onTriggered"
                {
                    beats.push(program.code_of(d, u));
                }
            }
        }
    }
    // A declared reading reads.
    let declared: Vec<(usize, usize)> = program
        .declared
        .iter()
        .filter(|(_, d)| d.wheres.contains(&Context::Read))
        .map(|(key, _)| *key)
        .collect();
    let mut declared_reads = Vec::new();
    for (d, u) in declared {
        declared_reads.push(program.code_of(d, u));
    }
    // A signal's handler is in no context of its own, but what it hands the machinery to read or press later is:
    // reading it carves those out as the stretches they are.
    for d in 0..program.harness {
        for u in 0..program.docs[d].units.len() {
            let unit = &program.docs[d].units[u];
            let handler = match unit.kind {
                UnitKind::Handler => {
                    !program.is_operation(d, unit.object) || unit.name != "onTriggered"
                }
                UnitKind::Function => unit
                    .name
                    .strip_prefix("on")
                    .is_some_and(|rest| rest.chars().next().is_some_and(char::is_uppercase)),
            };
            if handler {
                program.code_of(d, u);
            }
        }
    }
    (beats, declared_reads)
}

/// An effect in words, for the finding's line.
fn said(program: &Program, effect: &Effect) -> String {
    match effect {
        Effect::Write { target, step, .. } => format!(
            "{} {}.{}",
            if *step { "steps" } else { "writes" },
            program.docs[target.0].objects[target.1]
                .id
                .clone()
                .unwrap_or_else(|| program.docs[target.0].name.clone()),
            target.2
        ),
        Effect::Smuggle(name) => format!("writes `{name}` of the code it was carved from"),
        Effect::Input(what) => format!("an input `{what}`"),
        Effect::Captured(name) => format!("reads `{name}` from before the reading"),
        Effect::Calls(_) => "a call".to_string(),
        Effect::Declared(name, wheres) => format!(
            "`{name}`, declared for {}",
            wheres
                .iter()
                .map(|w| w.name())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Effect::Start => "an operation started".to_string(),
        Effect::Unread(what) => format!("`{what}` not followed"),
    }
}

/// A place: a file and a line.
type Place = (usize, usize);

/// Every effect `root` reaches, with the place it stands at: its own, and
/// those of each harness function it calls — with the call in the root's
/// own file it was reached through, where it stands in another.
fn reach(program: &Program, root: usize) -> Vec<(Effect, Place, Option<Place>)> {
    let home = program.codes[root].doc;
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut stack: Vec<(usize, Option<Place>)> = vec![(root, None)];
    while let Some((code, via)) = stack.pop() {
        if !seen.insert(code) {
            continue;
        }
        let doc = program.codes[code].doc;
        for (effect, line) in &program.codes[code].effects {
            if let Effect::Calls(callee) = effect {
                let callee_doc = program.codes[*callee].doc;
                let via = via.or((callee_doc != home && doc == home).then_some((doc, *line)));
                stack.push((*callee, via));
            } else {
                out.push((effect.clone(), (doc, *line), via));
            }
        }
    }
    out
}

/// The rule an effect breaks in a context, if it breaks one.
/// `whole` is a declared reading, held to reading itself: a value it
/// reads from its caller is its argument, not a capture.
fn verdict(effect: &Effect, context: Context, whole: bool) -> Option<&'static str> {
    match (context, effect) {
        (_, Effect::Unread(_)) => Some("unread"),
        (_, Effect::Calls(_)) => None,
        (Context::Read, Effect::Captured(_)) if !whole => Some("stale"),
        // A declared reading of the machinery's keeps its own books (a record made on first ask).
        (Context::Read, Effect::Write { .. }) if whole => None,
        (_, Effect::Captured(_)) => None,
        // What only reads may be read anywhere.
        (_, Effect::Declared(_, wheres)) if wheres.contains(&Context::Read) => None,
        (Context::Read, _) => Some(if whole { "reach" } else { "read" }),
        (_, Effect::Declared(_, wheres)) if wheres.contains(&context) => None,
        (Context::Beat, Effect::Declared(_, wheres)) if wheres.contains(&Context::Press) => {
            Some("input")
        }
        (_, Effect::Declared(..)) => Some("stage"),
        (_, Effect::Write { config: true, .. }) => None,
        (_, Effect::Write { .. } | Effect::Smuggle(_)) => Some("stage"),
        (Context::Beat, Effect::Input(_)) => Some("input"),
        (_, Effect::Input(_) | Effect::Start) => None,
    }
}
