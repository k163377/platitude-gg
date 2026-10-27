//! The QML rules, by [`Kind`].
//!
//! A `tst_*.qml` file, where the QtTest runner holds one budget: a
//! `wait(ms)` spent in place of an answer, a `tryCompare` / `tryVerify`
//! given a deadline of its own, and a rendering or polish wait whose
//! verdict nobody reads.
//!
//! The app's harness has no runner: its beats all come from the one type
//! that names the cadence, and the run's own clock is its budget, the way
//! `crate::wait` is this runner's. What is named is a span of time a file
//! spelled for itself ([`SPANS`]), a count of milliseconds it keeps
//! ([`keeps_a_count`]), frames it counts for itself ([`counts_frames`])
//! and a count of its own beats held against a number ([`beats`]).

use super::source::{self, Lang, has_token, line_of};
use super::{Candidate, Exception, Finding};

/// The waits that answer with a bool, which a test has to read: a false
/// left unread is a paint that never happened, read as one that did.
const ANSWERED_WAITS: [&str; 2] = ["waitForRendering(", "waitForItemPolished("];

/// The QtTest calls that take a timeout, and which argument it is.
const OWN_DEADLINES: [(&str, usize); 2] = [("tryCompare(", 4), ("tryVerify(", 2)];

/// A span of time spelled in the file itself: a `Timer` whatever its
/// `interval` says (unspelled, Qt's default second), an animation's
/// `duration`, and `Date` — a second clock beside the run's own
/// (`PerfProbe.clockMs`). `SampleTimer`, the harness's one cadence, is
/// held out ([`spelled`]).
const SPANS: [&str; 4] = ["Timer", "duration:", "Date.now(", "new Date("];

/// Which rules a QML file is read by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    /// A QtTest file: the runner stands behind every wait in it.
    Test,
    /// The app's automation harness (`src/auto`): no runner, and the
    /// beat is the product's own event loop.
    Harness,
}

/// Judges one QML file by the rules of its [`Kind`].
pub(super) fn scan(file: &str, text: &str, kind: Kind) -> (Vec<Finding>, Vec<Exception>) {
    let code = source::code_view(text, Lang::Qml);
    let chars: Vec<char> = code.chars().collect();
    let mut candidates = Vec::new();
    // What the lexical reading could not settle is not passed: the rest of the file is unread, and said so.
    if let Some((line, _)) = source::unread(text, Lang::Qml) {
        candidates.push(Candidate {
            first: line,
            last: line,
            shown: line,
            rule: "unread",
        });
    }
    if kind == Kind::Harness {
        candidates.extend(beats(file, &code));
    }
    for (at, line) in code.lines().enumerate() {
        let number = at + 1;
        let mut named = |rule| {
            candidates.push(Candidate {
                first: number,
                last: number,
                shown: number,
                rule,
            });
        };
        if has_token(line, "wait(") || has_token(line, "sleep(") {
            named("sleep");
        }
        if kind == Kind::Harness {
            if SPANS.iter().any(|span| spelled(line, span)) {
                named("clock");
            }
            if keeps_a_count(line) {
                named("elapsed");
            }
            if counts_frames(file, line) {
                named("frames");
            }
        }
    }
    if kind == Kind::Harness {
        candidates.sort_by_key(|c| c.first);
        return source::judged(file, text, &code, candidates);
    }
    for name in ANSWERED_WAITS {
        for call in calls(&code, name) {
            if unread_call(&chars, &call) {
                candidates.push(Candidate {
                    first: call.first,
                    last: call.last,
                    shown: call.first,
                    rule: "ignored",
                });
            }
        }
    }
    for (name, position) in OWN_DEADLINES {
        for call in calls(&code, name) {
            // `undefined` in the timeout's seat is how a message is passed
            // without one: the runner's own budget stands.
            let own = call
                .args
                .get(position - 1)
                .map(|arg| arg.trim())
                .is_some_and(|arg| !arg.is_empty() && arg != "undefined");
            if own {
                candidates.push(Candidate {
                    first: call.first,
                    last: call.last,
                    shown: call.first,
                    rule: "deadline",
                });
            }
        }
    }
    candidates.sort_by_key(|c| c.first);
    source::judged(file, text, &code, candidates)
}

/// Whether `line` spells `span`. A bare name is a type, and counts only
/// where an object of it is built — followed by `{`, or ending the line
/// (the brace on the next) — and not after a word character, which is
/// the whole of what tells `SampleTimer {` from `Timer {`. A name with its
/// own punctuation (`duration:`, `Date.now(`) counts wherever it stands.
fn spelled(line: &str, span: &str) -> bool {
    if !span.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return has_token(line, span);
    }
    let mut from = 0;
    while let Some(found) = line[from..].find(span) {
        let at = from + found;
        from = at + span.len();
        let after_a_word = at > 0 && is_word(line[..at].chars().last());
        let rest = line[from..].trim_start();
        if !after_a_word && (rest.starts_with('{') || rest.is_empty()) {
            return true;
        }
    }
    false
}

/// Whether `line` keeps a count of milliseconds by adding a beat onto
/// it. Keeping one at all is reading the clock; what it is used for is
/// the marker's to say.
fn keeps_a_count(line: &str) -> bool {
    line.split_once("+=")
        .is_some_and(|(_, added)| has_token(added, "interval"))
}

/// The files that count the window's frames for everyone else: the one
/// render boundary a wait on a layout reads is `Awaited.still` /
/// `Awaited.drawnSince`, fed by `WindowHarness`; the shot driver reads
/// frames for the ceiling's line, and the perf drivers measure them.
const FRAME_KEEPERS: [&str; 5] = [
    "Awaited.qml",
    "WindowHarness.qml",
    "AutoShotDriver.qml",
    "PagePerfDriver.qml",
    "WindowPerfDriver.qml",
];

/// Whether `line` counts frames of its own — `frameSwapped`,
/// `framesSwapped` or the window's `frameCounter` — outside the files
/// that keep the count for everyone: a wait "two frames from here" written
/// again, in place of `Awaited.drawnSince`.
fn counts_frames(file: &str, line: &str) -> bool {
    let name = file.rsplit(['/', '\\']).next().unwrap_or(file);
    !FRAME_KEEPERS.contains(&name)
        && ["frameSwapped", "framesSwapped", "frameCounter"]
            .iter()
            .any(|word| has_token(line, word))
}

/// The beat counts of a harness file held against a number, as
/// candidates: a wait of "so many beats from here", which is the
/// sampler's interval times a count — a clock.
///
/// What is read, and what is guaranteed: a count is a name an
/// `onTriggered:` handler steps anywhere in its body — `x++`, `++x`,
/// `x--`, `x += …`, `x -= …`, `x = x + …`, `x = x - …`, under any guard,
/// with or without a `;` — that is neither a `let` / `const` / `var` of
/// the handler's own (a local does not outlive the beat, so it counts
/// nothing across beats; a `for` index is one) nor a stage's step: one
/// inside a block an `if` on that same name selects, with a named wait
/// read in that block before it (`Awaited.all` / `still` / `drawnSince`,
/// the one way a stage asks the product — `if (x === 1) { if
/// (!Awaited.all(…)) return; x++ }`). A block its counter's value alone
/// selects, stepped on nothing the product answered, counts firings:
/// `if (x === 0) { x++ }` is one. A name is the whole `.`-joined path,
/// so `a.step` is not `b.step`; a bare name is the same property as a
/// qualified one ending in it. Such a count held with `<`, `>`, `<=`,
/// `>=`, `===`, `!==`, `==` or `!=` against a number other than zero,
/// anywhere in the file, is named at that line — whatever the comparison
/// then decides, since a count of beats is a reading of the clock
/// wherever it is read. Held against another value (the beat it was
/// asked in, `asked !== beats - 1`) it is a reading of order, not time,
/// and zero is "none yet", an edge, not a count. A handler whose braces
/// never close is not read, and is named as such (`unread`): what cannot
/// be read is not passed. The files that keep frames for everyone
/// ([`FRAME_KEEPERS`]) are not read for beats either.
fn beats(file: &str, code: &str) -> Vec<Candidate> {
    let name = file.rsplit(['/', '\\']).next().unwrap_or(file);
    if FRAME_KEEPERS.contains(&name) {
        return Vec::new();
    }
    let tokens = crate::lex::tokens(code);
    let mut counters: Vec<String> = Vec::new();
    let mut candidates = Vec::new();
    for (open, close) in handlers(&tokens) {
        let Some(close) = close else {
            candidates.push(Candidate {
                first: tokens[open].line,
                last: tokens[open].line,
                shown: tokens[open].line,
                rule: "unread",
            });
            continue;
        };
        let body = &tokens[open..close];
        let locals = declared_in(body);
        for stepped in stepped_in(body) {
            if !locals.contains(&stepped) && !counters.contains(&stepped) {
                counters.push(stepped);
            }
        }
    }
    for line in compared_with_a_number(&tokens, &counters) {
        if candidates
            .iter()
            .any(|c| c.first == line && c.rule == "beats")
        {
            continue;
        }
        candidates.push(Candidate {
            first: line,
            last: line,
            shown: line,
            rule: "beats",
        });
    }
    candidates
}

/// Each `onTriggered:` handler of `tokens`, as the index of its first
/// token and one past its last: a braced body to its matching brace (or
/// `None` where that brace never comes), a bare expression to the end of
/// its line.
fn handlers(tokens: &[crate::lex::Token]) -> Vec<(usize, Option<usize>)> {
    let mut out = Vec::new();
    for (at, token) in tokens.iter().enumerate() {
        if token.text != "onTriggered" || tokens.get(at + 1).is_none_or(|t| t.text != ":") {
            continue;
        }
        let first = at + 2;
        if tokens.get(first).is_some_and(|t| t.text == "{") {
            let mut depth = 0;
            let mut close = None;
            for (i, t) in tokens.iter().enumerate().skip(first) {
                match t.text.as_str() {
                    "{" => depth += 1,
                    "}" => {
                        depth -= 1;
                        if depth == 0 {
                            close = Some(i + 1);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            out.push((first, close));
        } else {
            let line = token.line;
            let end = tokens[first..]
                .iter()
                .position(|t| t.line != line)
                .map_or(tokens.len(), |n| first + n);
            out.push((first, Some(end)));
        }
    }
    out
}

/// The names a `let` / `const` / `var` of `body` declares, a
/// destructuring's names among them.
fn declared_in(body: &[crate::lex::Token]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < body.len() {
        if matches!(body[i].text.as_str(), "let" | "const" | "var") {
            i += 1;
            while i < body.len() {
                let text = body[i].text.as_str();
                if is_name(text) && !matches!(text, "of" | "in") {
                    out.push(text.to_string());
                } else if !matches!(text, "," | "[" | "]" | "{" | "}" | ":") {
                    break;
                }
                i += 1;
            }
            continue;
        }
        i += 1;
    }
    out
}

fn is_name(text: &str) -> bool {
    text.chars()
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_' || c == '$')
}

/// The `.`-joined name ending at `last`, with the index of its first
/// token, if a name ends there.
fn name_ending_at(tokens: &[crate::lex::Token], last: usize) -> Option<(String, usize)> {
    if !tokens.get(last).is_some_and(|t| is_name(&t.text)) {
        return None;
    }
    let mut first = last;
    while first >= 2 && tokens[first - 1].text == "." && is_name(&tokens[first - 2].text) {
        first -= 2;
    }
    Some((joined(&tokens[first..=last]), first))
}

/// The `.`-joined name beginning at `first`, with the index past it, if
/// a name begins there.
fn name_beginning_at(tokens: &[crate::lex::Token], first: usize) -> Option<(String, usize)> {
    if !tokens.get(first).is_some_and(|t| is_name(&t.text)) {
        return None;
    }
    let mut last = first;
    while tokens.get(last + 1).is_some_and(|t| t.text == ".")
        && tokens.get(last + 2).is_some_and(|t| is_name(&t.text))
    {
        last += 2;
    }
    Some((joined(&tokens[first..=last]), last + 1))
}

fn joined(tokens: &[crate::lex::Token]) -> String {
    tokens.iter().map(|t| t.text.as_str()).collect()
}

/// Whether two names are one property: the same path, or a bare name
/// and a qualified one ending in it.
fn same_property(a: &str, b: &str) -> bool {
    let last = |name: &str| name.rsplit('.').next().unwrap_or(name).to_string();
    a == b || (!a.contains('.') && a == last(b)) || (!b.contains('.') && b == last(a))
}

/// The names `body` steps as counts: each stepped outside every block an
/// `if` on that name selects.
fn stepped_in(body: &[crate::lex::Token]) -> Vec<String> {
    let mut out = Vec::new();
    for (i, token) in body.iter().enumerate() {
        let stepped = match token.text.as_str() {
            "++" | "--" => name_ending_at(body, i.wrapping_sub(1))
                .map(|(name, _)| name)
                .or_else(|| name_beginning_at(body, i + 1).map(|(name, _)| name)),
            "+=" | "-=" => name_ending_at(body, i.wrapping_sub(1)).map(|(name, _)| name),
            "=" => {
                // `x = x + …` / `a.x = a.x - …`: the same name on both sides, then a `+` or `-`.
                let Some((left, _)) = name_ending_at(body, i.wrapping_sub(1)) else {
                    continue;
                };
                let Some((right, after)) = name_beginning_at(body, i + 1) else {
                    continue;
                };
                let stepped = body
                    .get(after)
                    .is_some_and(|t| matches!(t.text.as_str(), "+" | "-"));
                (left == right && stepped).then_some(right)
            }
            _ => None,
        };
        if let Some(name) = stepped
            && !stage_step(body, i, &name)
            && !out.contains(&name)
        {
            out.push(name);
        }
    }
    out
}

/// Whether the step at `at` is a stage's: inside a block an `if` on `name`
/// (or the `if` chain its `else` continues) selects, with a named wait
/// read in that block before it — `Awaited.all`, `still` or `drawnSince`,
/// the one way a stage asks the product, whose terms are what it answered.
/// A block its counter's value alone selects, stepped on nothing the
/// product answered, counts firings.
fn stage_step(body: &[crate::lex::Token], at: usize, name: &str) -> bool {
    let Some(open) = selecting_block(body, at, name) else {
        return false;
    };
    (open + 1..at).any(|i| {
        body[i].text == "Awaited"
            && body.get(i + 1).is_some_and(|t| t.text == ".")
            && body
                .get(i + 2)
                .is_some_and(|t| matches!(t.text.as_str(), "all" | "still" | "drawnSince"))
            && body.get(i + 3).is_some_and(|t| t.text == "(")
    })
}

/// The `{` of the innermost block around `at` whose `if` (or the `if`
/// chain its `else` continues) reads `name` in its condition.
fn selecting_block(body: &[crate::lex::Token], at: usize, name: &str) -> Option<usize> {
    let mut depth = 0;
    let mut i = at;
    while i > 0 {
        i -= 1;
        match body[i].text.as_str() {
            "}" => depth += 1,
            "{" if depth > 0 => depth -= 1,
            "{" if condition_reads(body, i, name) => return Some(i),
            _ => {}
        }
    }
    None
}

/// Whether the block opening at `open` is selected by an `if` whose
/// condition reads `name` — directly, or through the `else` chain it
/// hangs on.
fn condition_reads(body: &[crate::lex::Token], open: usize, name: &str) -> bool {
    let mut before = open;
    loop {
        if before == 0 {
            return false;
        }
        before -= 1;
        match body[before].text.as_str() {
            ")" => {
                let Some(open_paren) = matching_paren(body, before) else {
                    return false;
                };
                if open_paren == 0 || body[open_paren - 1].text != "if" {
                    return false;
                }
                if body[open_paren + 1..before]
                    .iter()
                    .enumerate()
                    .any(|(k, t)| {
                        is_name(&t.text)
                            && name_ending_at(body, open_paren + 1 + k)
                                .is_some_and(|(read, _)| same_property(&read, name))
                    })
                {
                    return true;
                }
                // `else if (…)`: the chain above may select on the name too — read on from the `if`, so the
                // next step back lands on the `else`.
                if open_paren >= 2 && body[open_paren - 2].text == "else" {
                    before = open_paren - 1;
                    continue;
                }
                return false;
            }
            "else" => {
                // `} else {`: back over the block before it to its own `if`.
                if before == 0 || body[before - 1].text != "}" {
                    return false;
                }
                let mut depth = 0;
                let mut j = before - 1;
                loop {
                    match body[j].text.as_str() {
                        "}" => depth += 1,
                        "{" => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    if j == 0 {
                        return false;
                    }
                    j -= 1;
                }
                return condition_reads(body, j, name);
            }
            _ => return false,
        }
    }
}

/// The `(` matching the `)` at `close`.
fn matching_paren(body: &[crate::lex::Token], close: usize) -> Option<usize> {
    let mut depth = 0;
    let mut i = close;
    loop {
        match body[i].text.as_str() {
            ")" => depth += 1,
            "(" => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        if i == 0 {
            return None;
        }
        i -= 1;
    }
}

/// The lines on which one of `counters` is held against a number other
/// than zero.
fn compared_with_a_number(tokens: &[crate::lex::Token], counters: &[String]) -> Vec<usize> {
    const OPERATORS: [&str; 8] = ["===", "!==", "==", "!=", "<=", ">=", "<", ">"];
    let a_count =
        |text: &str| text.chars().all(|c| c.is_ascii_digit()) && text.chars().any(|c| c != '0');
    let counted =
        |name: Option<String>| name.is_some_and(|n| counters.iter().any(|c| same_property(c, &n)));
    let mut lines = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        if !OPERATORS.contains(&token.text.as_str()) {
            continue;
        }
        let left_name = name_ending_at(tokens, i.wrapping_sub(1)).map(|(name, _)| name);
        let right = name_beginning_at(tokens, i + 1).map(|(name, _)| name);
        let left_number = i.checked_sub(1).is_some_and(|j| a_count(&tokens[j].text));
        let right_number = tokens.get(i + 1).is_some_and(|t| a_count(&t.text));
        if ((counted(left_name) && right_number) || (counted(right) && left_number))
            && !lines.contains(&token.line)
        {
            lines.push(token.line);
        }
    }
    lines
}

/// One call of a name looked for, read off the whole code view rather
/// than one line, since a call may wrap across lines.
struct Call {
    /// The index of the name's first character.
    start: usize,
    /// The index of the `)`, absent for a call never closed.
    close: Option<usize>,
    /// The lines it opens and closes on.
    first: usize,
    last: usize,
    /// Its arguments, split at the commas of its own level.
    args: Vec<String>,
}

/// Whether `call`'s answer reaches nobody: it stands as a statement of
/// its own, with nothing but a receiver (`case.`) before it and nothing
/// but a `;` or a closing brace after it.
fn unread_call(chars: &[char], call: &Call) -> bool {
    let mut at = call.start;
    while at > 0 && (is_word(Some(chars[at - 1])) || chars[at - 1] == '.') {
        at -= 1;
    }
    if !opens_a_statement(chars, at) {
        return false;
    }
    // A call left open reads as one nobody closed around, either.
    let Some(close) = call.close else {
        return true;
    };
    let ends_the_line = chars[close + 1..]
        .iter()
        .take_while(|c| **c != '\n')
        .all(|c| c.is_whitespace() || matches!(c, ';' | '}'));
    // What continues an expression cannot begin a statement, so a line
    // the call ends may still be a line the next one reads.
    let continued = chars[close + 1..]
        .iter()
        .find(|c| !c.is_whitespace())
        .is_some_and(|c| matches!(c, '.' | '?' | ':' | '&' | '|' | '='));
    ends_the_line && !continued
}

/// Whether a statement begins at `at`: after a `;`, a brace, the start of
/// the file, or a line that ended on a value (QML supplies the `;`).
fn opens_a_statement(chars: &[char], at: usize) -> bool {
    let Some(before) = chars[..at].iter().rposition(|c| !c.is_whitespace()) else {
        return true;
    };
    matches!(chars[before], ';' | '{' | '}')
        || (chars[before + 1..at].contains(&'\n') && !crate::lex::opens_a_value(chars, at))
}

/// Every call of `name` in the code view. A call that is never closed is
/// the last one read: past an unclosed `(` nothing parses.
fn calls(code: &str, name: &str) -> Vec<Call> {
    let chars: Vec<char> = code.chars().collect();
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(hit) = code[from..].find(name) {
        let at = from + hit;
        from = at + name.len();
        if at > 0 && is_word(code[..at].chars().last()) {
            continue;
        }
        let start = code[..at].chars().count();
        let open = code[..from].chars().count() - 1;
        let closed = arguments(&chars, open);
        let ends_at = closed
            .as_ref()
            .map_or(chars.len().saturating_sub(1), |(close, _)| *close);
        let unclosed = closed.is_none();
        found.push(Call {
            start,
            close: closed.as_ref().map(|(close, _)| *close),
            first: line_of(&chars, open),
            last: line_of(&chars, ends_at),
            args: closed.map_or_else(Vec::new, |(_, args)| args),
        });
        if unclosed {
            break;
        }
    }
    found
}

fn is_word(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_alphanumeric() || c == '_')
}

/// The arguments between the `(` at `open` and its `)`, split at the
/// commas that are not inside a nested `()` / `[]` / `{}`; `None` when
/// the call is never closed.
fn arguments(chars: &[char], open: usize) -> Option<(usize, Vec<String>)> {
    let mut depth = 0usize;
    let mut args = vec![String::new()];
    for (i, c) in chars.iter().enumerate().skip(open) {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((i, args));
                }
            }
            ',' if depth == 1 => {
                args.push(String::new());
                continue;
            }
            _ => {}
        }
        if i > open {
            args.last_mut()?.push(*c);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{Kind, scan};

    fn found(text: &str) -> Vec<(usize, &'static str)> {
        let (findings, _) = scan("tst_x.qml", text, Kind::Test);
        findings.iter().map(|f| (f.line, f.rule)).collect()
    }

    /// The same, by the harness's rules.
    fn harness(text: &str) -> Vec<(usize, &'static str)> {
        let (findings, _) = scan("WindowActs.qml", text, Kind::Harness);
        findings.iter().map(|f| (f.line, f.rule)).collect()
    }

    #[test]
    fn a_wait_in_milliseconds_is_a_sleep_and_the_answered_waits_are_not() {
        let text = "\
wait(50)
waitForRendering(item)
verify(waitForRendering(item), \"painted\")
tryVerify(() => item.done)
";
        assert_eq!(found(text), vec![(1, "sleep"), (2, "ignored")]);
    }

    #[test]
    fn an_answered_wait_called_on_the_case_is_still_unread() {
        let text = "\
testCase.waitForRendering(item)
const ok = waitForRendering(item)
if (!waitForRendering(item)) fail(\"no paint\")
";
        assert_eq!(found(text), vec![(1, "ignored")]);
    }

    #[test]
    fn a_timeout_of_the_tests_own_is_a_deadline_and_the_runners_is_not() {
        let text = "\
tryCompare(card, \"opened\", true)
tryCompare(card, \"opened\", true, 1000)
tryVerify(() => root.n !== -1, 1000, \"reported, once the (double) window closes\")
tryVerify(() => root.n !== -1,
          \"a message with, a comma\")
tryVerify(() => root.n !== -1, undefined, \"the runner's own budget\")
";
        assert_eq!(found(text), vec![(2, "deadline"), (3, "deadline")]);
    }

    #[test]
    fn a_wrapped_answered_wait_is_read_across_the_lines_it_spans() {
        let text = "\
waitForRendering(
    item,
    500)
const painted =
    waitForRendering(item)
verify(waitForRendering(
    item), \"painted\")
if (!waitForItemPolished(
        item)) fail(\"no polish\")
";
        assert_eq!(
            found(text),
            vec![(1, "ignored")],
            "the answer is read where the expression around it stands, however it wrapped"
        );
    }

    #[test]
    fn a_marker_above_a_wrapped_wait_covers_the_whole_call() {
        let text = "\
// waits(timed): the paint is the product's own, and the bound is a floor
waitForRendering(
    item,
    500)
";
        let (findings, exceptions) = scan("tst_x.qml", text, Kind::Test);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(exceptions.len(), 1);
    }

    #[test]
    fn a_regex_literal_hides_nothing_behind_it() {
        let text = "\
const bare = /'/
wait(50)
";
        assert_eq!(
            found(text),
            vec![(2, "sleep")],
            "the quote in the pattern opens no string"
        );
    }

    #[test]
    fn a_marker_above_the_line_lets_it_stand() {
        let text = "\
// The double-click window is the product's own, read off the style hints.
// waits(timed): the window is Qt's, and the bound is a floor no load can break
tryVerify(() => root.n !== -1, Qt.styleHints.mouseDoubleClickInterval * 2)
";
        let (findings, exceptions) = scan("tst_x.qml", text, Kind::Test);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(exceptions.len(), 1);
    }

    #[test]
    fn the_harness_cadence_spells_no_span_and_a_timer_of_its_own_does() {
        let text = "\
SampleTimer {
    running: Harness.autoAct === \"tip\"
}
Timer {
    id: watchdog
    interval: Math.max(Harness.autoWatchdogMs, 1)
}
Timer {
}
NumberAnimation {
    duration: 12000
}
Timer
{
}
";
        assert_eq!(
            harness(text),
            vec![(4, "clock"), (8, "clock"), (11, "clock"), (13, "clock")],
            "the cadence has a name; a timer that spells none of its own still holds Qt's second, \
             and a brace on the next line builds the same object"
        );
    }

    #[test]
    fn the_harness_names_the_clock_it_keeps_and_not_the_runs_own() {
        let text = "\
Harness.report(\"perf_begin clock_ms=\" + PerfProbe.clockMs())
acts.at = Date.now()
Harness.report(\"wait=\" + (Date.now() - acts.at))
timer.waited += timer.interval
timer.done++
";
        assert_eq!(
            harness(text),
            vec![(2, "clock"), (3, "clock"), (4, "elapsed")],
            "the run's own clock is the harness's budget; a second origin and a kept count are not"
        );
    }

    #[test]
    fn the_qtest_rules_and_the_harness_rules_do_not_reach_into_each_other() {
        let beats = "Timer {\n    interval: 50\n}\n";
        assert_eq!(found(beats), vec![], "a QtTest file is judged on its calls");
        let calls = "tryCompare(card, \"opened\", true, 1000)\nwaitForRendering(item)\n";
        assert_eq!(
            harness(calls),
            vec![],
            "the harness has no runner to hold a budget for it"
        );
        assert_eq!(
            harness("wait(50)\n"),
            vec![(1, "sleep")],
            "a stretch of clock spent in place of an answer is neither one's to keep"
        );
    }

    #[test]
    fn a_marker_over_a_harness_beat_names_what_it_is_for() {
        let text = "\
// waits(ceiling): the run that stops answering is ended here and nowhere else
Timer {
    id: watchdog
    interval: Math.max(Harness.autoWatchdogMs, 1)
}
";
        let (findings, exceptions) = scan("WindowActs.qml", text, Kind::Harness);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(exceptions.len(), 1);
    }

    /// A verb counting the window's frames for itself is a wait of "two
    /// frames from here" written again; the count lives in `Awaited`.
    #[test]
    fn a_frame_count_of_a_verbs_own_is_named_and_the_keepers_are_not() {
        let text = "\
Item {
    property int framesSwapped: 0
    function laid() { return acts.framesSwapped >= wanted }
    function drawn() { return window.frameCounter > before }
}
";
        assert_eq!(
            harness(text),
            vec![(2, "frames"), (3, "frames"), (4, "frames")]
        );
        for keeper in ["Awaited.qml", "WindowHarness.qml", "AutoShotDriver.qml"] {
            let (findings, _) = scan(&format!("src/auto/{keeper}"), text, Kind::Harness);
            assert!(findings.is_empty(), "{keeper}: {findings:?}");
        }
    }

    /// A count that goes up once per beat, held against a number, is a
    /// wait of "so many beats from here"; the same count held against
    /// the beat it was asked in is a reading of order, and a count of
    /// presses made is no beat at all.
    #[test]
    fn a_beat_count_held_against_a_number_is_named() {
        let text = "\
SampleTimer {
    onTriggered: {
        acts.beats++
        if (acts.beats < 3)
            return
        if (acts.asked !== acts.beats - 1)
            stop()
        if (2 <= acts.beats || acts.beats === 0)
            acts.read()
    }
    function pressed() { item.click(); acts.presses++; return acts.presses < 2 }
}
";
        assert_eq!(harness(text), vec![(4, "beats"), (8, "beats")]);
    }

    /// The count is read by what it is, not by where it stands or how it
    /// is punctuated: behind a guard, with a `;`, as `+= 1` or `x = x + 1`,
    /// in a bare handler, and held against the number in a function of
    /// the file's own.
    #[test]
    fn a_beat_count_is_a_count_however_it_is_spelled_and_wherever_it_is_read() {
        let guarded = "\
SampleTimer {
    onTriggered: {
        if (!acts.ready)
            return;
        acts.beats++;
        if (acts.beats >= 3) acts.read()
    }
}
";
        assert_eq!(harness(guarded), vec![(6, "beats")]);
        let added = "\
SampleTimer {
    onTriggered: {
        acts.ticks += 1
        acts.turns = acts.turns + 1
    }
    function laid() { return acts.ticks > 2 && 4 !== acts.turns }
}
";
        assert_eq!(harness(added), vec![(6, "beats")]);
        let bare =
            "SampleTimer { onTriggered: acts.beats++ }\nfunction f() { return acts.beats === 2 }\n";
        assert_eq!(harness(bare), vec![(2, "beats")]);
    }

    /// A local of the handler's own does not outlive the beat; a count
    /// stepped outside a handler is no beat; and a handler that never
    /// closes is named, not passed.
    #[test]
    fn a_handlers_local_counts_nothing_and_an_unclosed_handler_is_named() {
        let local = "\
SampleTimer {
    onTriggered: {
        for (let i = 0; i < 320; i++)
            acts.probe(i)
        const [n, m] = acts.pair()
        if (n === 150)
            acts.read()
    }
}
";
        assert_eq!(harness(local), vec![]);
        let elsewhere = "\
Item {
    function pressed() { acts.presses++ }
    SampleTimer { onTriggered: { if (acts.presses < 2) return } }
}
";
        assert_eq!(harness(elsewhere), vec![]);
        let unclosed = "\
SampleTimer {
    onTriggered: {
        acts.beats++
        if (acts.beats < 3)
            return
";
        assert_eq!(harness(unclosed), vec![(2, "unread")]);
    }

    /// A stage counter advances inside the block its own value selects,
    /// on what the product answered there by name: no beat. And one
    /// object's `step` is not another's.
    #[test]
    fn a_stage_counter_and_another_objects_count_are_no_beats() {
        let stages = "\
SampleTimer {
    onTriggered: {
        if (acts.step === 0) {
            if (!Awaited.all(\"row\", { \"row\": item !== null }))
                return
            item.click()
            acts.step++
            return
        } else if (acts.step === 1) {
            if (!Awaited.all(\"card\", { \"card\": card.opened }))
                return
            acts.step += 1
        } else {
            if (!Awaited.still(\"laid\", card.y))
                return;
            acts.step = acts.step + 1
        }
        if (acts.step === 3)
            acts.finish()
    }
}
";
        assert_eq!(harness(stages), vec![]);
        let siblings = "\
Item {
    SampleTimer { onTriggered: { pressTimer.tries++ } }
    SampleTimer { onTriggered: { if (fileTimer.tries === 1) acts.read() } }
    function laid() { return pressTimer.tries > 2 }
}
";
        assert_eq!(harness(siblings), vec![(4, "beats")]);
    }

    /// A counter stepped on its own value alone counts firings, however
    /// the chain is laid out, and a guard on the counter grounds nothing.
    #[test]
    fn a_counter_stepped_on_its_own_value_alone_counts_firings() {
        let chain = "\
SampleTimer {
    onTriggered: {
        if (acts.step === 0) { acts.step++; return }
        else if (acts.step === 1) { acts.step++; return }
        if (acts.step === 2) acts.finish()
    }
}
";
        assert_eq!(
            harness(chain),
            vec![(4, "beats"), (5, "beats")],
            "zero is an edge, not a count"
        );
        let spread = "\
SampleTimer {
    onTriggered: {
        if (acts.step === 0) {
            acts.step++;
            return;
        }
        else if (acts.step === 1)
        {
            if (acts.step > 5) return
            acts.step += 1
        }
        if (acts.step === 2)
            acts.finish()
    }
}
";
        assert_eq!(harness(spread).len(), 3, "{:?}", harness(spread));
    }

    /// A chain's later branch is selected by the counter through the
    /// `else` it hangs on, whatever its own condition reads.
    #[test]
    fn a_stage_in_a_chains_later_branch_is_selected_through_the_else() {
        let chain = "\
SampleTimer {
    onTriggered: {
        if (acts.step === 0) {
            if (!Awaited.all(\"row\", { \"row\": item !== null }))
                return
            acts.step++
        } else if (!card.opened) {
            if (!Awaited.all(\"card\", { \"card\": card.asked }))
                return
            acts.step++
        } else {
            if (!Awaited.still(\"laid\", card.y))
                return
            acts.step++
        }
        if (acts.step === 3)
            acts.finish()
    }
}
";
        assert_eq!(harness(chain), vec![]);
    }
}
