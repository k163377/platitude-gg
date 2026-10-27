//! The condition of an `if`, read by the grammar the beats proof allows,
//! and what its truth or falsity implies of a named wait ([`guards`]).

/// A condition, as the grammar the proof allows reads it: `||` over `&&`
/// over `!`, parentheses, a named wait (`Awaited.all` / `still` /
/// `drawnSince`, called with nothing hung on the call), and a leaf — any
/// other operand, read for nothing inside it: a wait a leaf holds is not
/// read. A condition the grammar does not read whole — an operand with a
/// `?:`, an assignment, a `,`, an arrow, a `??` or a `!` of its own at its
/// top level, which would bend what the `&&` and `||` around it mean —
/// proves nothing at all: no part of it is picked out.
#[derive(Debug, PartialEq, Eq)]
enum Cond {
    Wait,
    Leaf,
    Not(Box<Cond>),
    And(Vec<Cond>),
    Or(Vec<Cond>),
}

impl Cond {
    /// Whether the condition coming out `value` means a wait answered
    /// yes. A wait is its own answer and a leaf says nothing; `!` turns
    /// the value; an `&&` true has every part true, so one part's wait
    /// proves it, and false has some part false, so every part's must;
    /// an `||` the other way about.
    fn implies(&self, value: bool) -> bool {
        match self {
            Cond::Wait => value,
            Cond::Leaf => false,
            Cond::Not(inner) => inner.implies(!value),
            Cond::And(parts) if value => parts.iter().any(|part| part.implies(true)),
            Cond::And(parts) => parts.iter().all(|part| part.implies(false)),
            Cond::Or(parts) if value => parts.iter().all(|part| part.implies(true)),
            Cond::Or(parts) => parts.iter().any(|part| part.implies(false)),
        }
    }
}

/// What a wait proves of an `if` on a condition: `then`, that its
/// then-branch runs on a wait answering yes; `els`, that its else-branch
/// does — and so does whatever follows an `if` whose then-branch always
/// leaves. A condition the grammar does not read proves neither.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Guards {
    pub then: bool,
    pub els: bool,
}

pub(super) fn guards(cond: &[crate::lex::Token]) -> Guards {
    read_cond(cond).map_or(Guards::default(), |cond| Guards {
        then: cond.implies(true),
        els: cond.implies(false),
    })
}

/// The tokens that end an operand at its top level: the two connectives,
/// and the closer of a group.
const ENDS_AN_OPERAND: [&str; 3] = ["&&", "||", ")"];
/// The tokens an operand may not hold at its top level: an operator
/// binding looser than the connectives, an assignment, or a `!` past its
/// start — with one of them the connectives around the operand are not
/// what they look like, and the condition is not read.
const NOT_IN_AN_OPERAND: [&str; 11] = ["?", ":", "=", "+=", "-=", "*=", "/=", ",", "=>", "??", "!"];

/// `cond` read whole by the grammar, or nothing.
fn read_cond(cond: &[crate::lex::Token]) -> Option<Cond> {
    let (read, next) = parse_or(cond, 0)?;
    (next == cond.len()).then_some(read)
}

fn parse_or(cond: &[crate::lex::Token], i: usize) -> Option<(Cond, usize)> {
    let (first, mut next) = parse_and(cond, i)?;
    let mut parts = vec![first];
    while cond.get(next).is_some_and(|t| t.text == "||") {
        let (part, after) = parse_and(cond, next + 1)?;
        parts.push(part);
        next = after;
    }
    Some((one_or(parts, Cond::Or), next))
}

fn parse_and(cond: &[crate::lex::Token], i: usize) -> Option<(Cond, usize)> {
    let (first, mut next) = parse_not(cond, i)?;
    let mut parts = vec![first];
    while cond.get(next).is_some_and(|t| t.text == "&&") {
        let (part, after) = parse_not(cond, next + 1)?;
        parts.push(part);
        next = after;
    }
    Some((one_or(parts, Cond::And), next))
}

fn one_or(mut parts: Vec<Cond>, joined: fn(Vec<Cond>) -> Cond) -> Cond {
    if parts.len() == 1 {
        parts.remove(0)
    } else {
        joined(parts)
    }
}

fn parse_not(cond: &[crate::lex::Token], i: usize) -> Option<(Cond, usize)> {
    if cond.get(i).is_some_and(|t| t.text == "!") {
        let (inner, next) = parse_not(cond, i + 1)?;
        return Some((Cond::Not(Box::new(inner)), next));
    }
    parse_operand(cond, i)
}

/// An operand: a group `( … )` or a wait call, each standing on its own
/// — ended by a connective, a closer or the end — or else a leaf to the
/// next such token, its brackets crossed whole.
fn parse_operand(cond: &[crate::lex::Token], i: usize) -> Option<(Cond, usize)> {
    let alone =
        |past: usize| past == cond.len() || ENDS_AN_OPERAND.contains(&cond[past].text.as_str());
    if cond.get(i).is_some_and(|t| t.text == "(")
        && let Some(close) = crate::lex::matching_close(cond, i, "(", ")")
        && alone(close + 1)
    {
        let (inner, next) = parse_or(cond, i + 1)?;
        return (next == close).then_some((inner, close + 1));
    }
    if let Some(past) = wait_call_end(cond, i)
        && alone(past)
    {
        return Some((Cond::Wait, past));
    }
    let mut depth = 0i32;
    let mut k = i;
    while k < cond.len() {
        let text = cond[k].text.as_str();
        match text {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" if depth > 0 => depth -= 1,
            _ if depth == 0 && ENDS_AN_OPERAND.contains(&text) => break,
            _ if depth == 0 && NOT_IN_AN_OPERAND.contains(&text) => return None,
            _ => {}
        }
        k += 1;
    }
    (k > i && depth == 0).then_some((Cond::Leaf, k))
}

/// The index past the `Awaited.X(…)` beginning at `i`, for X one of the
/// named waits.
fn wait_call_end(cond: &[crate::lex::Token], i: usize) -> Option<usize> {
    (cond.get(i).is_some_and(|t| t.text == "Awaited")
        && cond.get(i + 1).is_some_and(|t| t.text == ".")
        && cond
            .get(i + 2)
            .is_some_and(|t| matches!(t.text.as_str(), "all" | "still" | "drawnSince"))
        && cond.get(i + 3).is_some_and(|t| t.text == "("))
    .then(|| crate::lex::matching_close(cond, i + 3, "(", ")").map(|close| close + 1))
    .flatten()
}

#[cfg(test)]
mod tests {
    use super::{Guards, guards};
    use crate::lex::{Lang, tokens};

    fn of(cond: &str) -> Guards {
        guards(&tokens(cond, Lang::Qml))
    }

    /// A wait is its own answer; `!` turns it, `&&` and `||` carry it the way their truth tables allow, by
    /// precedence, and parentheses group.
    #[test]
    fn a_wait_is_its_own_answer_and_the_connectives_carry_it() {
        let both = |then, els| Guards { then, els };
        assert_eq!(of("Awaited.all(\"r\", { \"r\": x })"), both(true, false));
        assert_eq!(of("!Awaited.still(\"l\", y)"), both(false, true));
        assert_eq!(of("Awaited.drawnSince(\"d\") && y"), both(true, false));
        assert_eq!(of("!Awaited.all(\"r\", {}) || y"), both(false, true));
        assert_eq!(of("!(Awaited.all(\"r\", {}) && y)"), both(false, true));
        assert_eq!(of("!(!Awaited.all(\"r\", {}) || y)"), both(true, false));
        assert_eq!(
            of("y || !Awaited.all(\"r\", {}) || z && w"),
            both(false, true)
        );
        assert_eq!(of("((Awaited.all(\"r\", {})))"), both(true, false));
        assert_eq!(of("!!Awaited.all(\"r\", {})"), both(true, false));
        assert_eq!(of("!Awaited.all(\"r\", {}) && y"), both(false, false));
        assert_eq!(of("Awaited.all(\"r\", {}) || y"), both(false, false));
        assert_eq!(of("y"), both(false, false));
    }

    /// A condition the grammar does not read whole proves nothing, whatever wait it holds: no `&&` or `||` of it
    /// is picked out.
    #[test]
    fn a_condition_outside_the_grammar_proves_nothing_whatever_it_holds() {
        for cond in [
            "!Awaited.all(\"r\", {}) || y ? z : w",
            "x = !Awaited.all(\"r\", {})",
            "Awaited.all(\"r\", {}) === true",
            "x === !Awaited.all(\"r\", {})",
            "!Awaited.all(\"r\", {}) || (y, z)",
            "!Awaited.all(\"r\", {}) ?? y",
            "Awaited.all(\"r\", {}).length",
            "f(!Awaited.all(\"r\", {}))",
            "!Awaited.all(\"r\", {}) ||",
            "",
        ] {
            assert_eq!(of(cond), Guards::default(), "{cond}");
        }
    }
}
