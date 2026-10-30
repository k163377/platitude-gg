//! The reach rule of the app's harness: what an operation does is read
//! across every call it makes, so a helper, or a function shared between
//! files, is held to what the operation's own body would be
//! (反映前テストの機械化 §検証時点).
//!
//! Code an operation runs is in one of three contexts, each with its one
//! responsibility:
//!
//! - **beat** — the operation's `onTriggered`, and a continuation the
//!   machinery runs in a beat: it observes the product and moves its own
//!   stages through the operation's API. It puts no input in and keeps no
//!   state of the harness's.
//! - **press** — the function an operation's `input` presses: it puts the
//!   input in, and may start the operations that wait for its effect (a
//!   start condition, with the properties that configure them). It moves
//!   no stage and keeps no state.
//! - **read** — a report, a completion's `when`, a claim: it only reads,
//!   and reads only what is there when it is read — no value carried in
//!   from the code it was written in.
//!
//! A write is judged by what it changes, not by the name it is written
//! through: the origin of the value changed (`origin`) — made by the code
//! itself, a harness property's, the product's — follows a value through
//! locals, arguments and returns, so a local naming the harness's array
//! changes that array when it is pushed to, and an array the code made is
//! its own to build. What a container holds is kept apart from the
//! container, by the site that made it, so a borrowed value a callee puts
//! inside its caller's array is borrowed when the caller reads it out; the
//! language's own functions answer and put by their contracts
//! (`builtins`). A change through a value of no known origin is not passed
//! as safe.
//!
//! A call is followed to what it calls: a harness function's code is read
//! as the caller's own; a function of the machinery's answers by the
//! contexts it declares (`// reach(<contexts>[; <arg>=<context>…])`, only
//! in the machinery's files); the product by its own code, its Rust slots'
//! receivers and the table of the language's and Qt's methods
//! (`product`). A call none of these answers is `unread`: not passed as
//! safe.

#[cfg(test)]
mod bindings;
mod builtins;
mod calls;
mod handed;
mod members;
mod origin;
mod parse;
mod product;
mod program;
mod rules;
mod scan;
mod scope;
#[cfg(test)]
mod tests;

pub(super) use rules::findings;

/// Where code runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Context {
    Beat,
    Press,
    Read,
}

impl Context {
    fn name(self) -> &'static str {
        match self {
            Context::Beat => "beat",
            Context::Press => "press",
            Context::Read => "read",
        }
    }
    fn of(name: &str) -> Option<Context> {
        match name {
            "beat" => Some(Context::Beat),
            "press" => Some(Context::Press),
            "read" => Some(Context::Read),
            _ => None,
        }
    }
}

/// What a machinery function declares: the contexts it may be called in,
/// and the contexts its function arguments run in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Declared {
    wheres: Vec<Context>,
    args: Vec<(usize, program::ArgContext)>,
}

impl Declared {
    /// `beat, press; 1=press, 2=read` (`spec` for a completion's object).
    fn read(marker: &str) -> Option<Declared> {
        let (wheres, args) = marker.split_once(';').unwrap_or((marker, ""));
        let wheres = wheres
            .split(',')
            .map(|w| Context::of(w.trim()))
            .collect::<Option<Vec<_>>>()?;
        if wheres.is_empty() {
            return None;
        }
        let mut taken = Vec::new();
        for arg in args.split(',').map(str::trim).filter(|a| !a.is_empty()) {
            let (index, context) = arg.split_once('=')?;
            let index: usize = index.trim().parse().ok()?;
            let context = match context.trim() {
                "spec" => program::ArgContext::Spec,
                other => program::ArgContext::Of(Context::of(other)?),
            };
            taken.push((index, context));
        }
        Some(Declared {
            wheres,
            args: taken,
        })
    }
}
