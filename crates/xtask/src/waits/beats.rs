//! The beats rule of the app's harness: a count of a handler's own
//! firings held against a number, read off the file's tokens ([`beats`]).

use super::Candidate;

/// The beat counts of a harness file held against a number, as
/// candidates: a wait of "so many beats from here", which is the
/// sampler's interval times a count — a clock.
///
/// What is read: a count is a name an `onTriggered:` handler steps
/// anywhere in its body — `x++`, `++x`, `x--`, `x += …`, `x -= …`,
/// `x = x + …`, `x = x - …`, with or without a `;` — that is not a `let` /
/// `const` / `var` of the handler's own (a local does not outlive the
/// beat, so it counts nothing across beats; a `for` index is one). No
/// stepped name is a stage: a run's stages are its operations'
/// (`auto/StageTimer.qml`), and a sampler that keeps one of its own is
/// named by the stages rule (`super::stages`). A name is the whole
/// `.`-joined path, so `a.step` is not `b.step`; a bare name is the same
/// property as a qualified one ending in it. Such a count held with `<`,
/// `>`, `<=`, `>=`, `===`, `!==`, `==` or `!=` against a number other than
/// zero, anywhere in the file, is named at that line — whatever the
/// comparison then decides, since a count of beats is a reading of the
/// clock wherever it is read. Held against another value (the beat it was
/// asked in, `asked !== beats - 1`) it is a reading of order, not time,
/// and zero is "none yet", an edge, not a count. The tokens are the
/// file's own (`crate::lex::tokens`), a literal one token among them: a
/// line closing on a string closes, as one closing on a name does. A
/// handler whose brace never closes is `unread`: what cannot be read is
/// not passed.
pub(super) fn beats(text: &str) -> Vec<Candidate> {
    let tokens = crate::lex::tokens(text, crate::lex::Lang::Qml);
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

/// The `.`-joined name ending at `last`, if a name ends there.
fn name_ending_at(tokens: &[crate::lex::Token], last: usize) -> Option<String> {
    if !tokens.get(last).is_some_and(|t| is_name(&t.text)) {
        return None;
    }
    let mut first = last;
    while first >= 2 && tokens[first - 1].text == "." && is_name(&tokens[first - 2].text) {
        first -= 2;
    }
    Some(joined(&tokens[first..=last]))
}

/// The `.`-joined name beginning at `first`, if a name begins there.
fn name_beginning_at(tokens: &[crate::lex::Token], first: usize) -> Option<String> {
    if !tokens.get(first).is_some_and(|t| is_name(&t.text)) {
        return None;
    }
    let mut last = first;
    while tokens.get(last + 1).is_some_and(|t| t.text == ".")
        && tokens.get(last + 2).is_some_and(|t| is_name(&t.text))
    {
        last += 2;
    }
    Some(joined(&tokens[first..=last]))
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

/// The names `body` steps: by one, by an amount, or set to itself plus
/// or minus something.
fn stepped_in(body: &[crate::lex::Token]) -> Vec<String> {
    let mut out = Vec::new();
    for (i, token) in body.iter().enumerate() {
        let stepped = match token.text.as_str() {
            "++" | "--" => {
                name_ending_at(body, i.wrapping_sub(1)).or_else(|| name_beginning_at(body, i + 1))
            }
            "+=" | "-=" => name_ending_at(body, i.wrapping_sub(1)),
            "=" => name_ending_at(body, i.wrapping_sub(1)).filter(|left| {
                name_beginning_at(body, i + 1).is_some_and(|right| same_property(left, &right))
                    && body
                        .get(i + 1 + 2 * left.matches('.').count() + 1)
                        .is_some_and(|t| matches!(t.text.as_str(), "+" | "-"))
            }),
            _ => None,
        };
        if let Some(name) = stepped
            && !out.contains(&name)
        {
            out.push(name);
        }
    }
    out
}

/// The lines comparing one of `counters` with a number other than zero.
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
        let left_name = name_ending_at(tokens, i.wrapping_sub(1));
        let right = name_beginning_at(tokens, i + 1);
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

#[cfg(test)]
mod tests {
    /// The findings by the harness's rules, as `qml::scan` makes them, in
    /// a file that keeps plain samplers (the stages rule's own findings are
    /// its tests').
    fn harness(text: &str) -> Vec<(usize, &'static str)> {
        let (findings, _) = super::super::qml::scan(
            "src/auto/PagePerfDriver.qml",
            text,
            super::super::qml::Kind::Harness,
        );
        findings.iter().map(|f| (f.line, f.rule)).collect()
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

    /// A counter that selects its own branches is a count like any other:
    /// a stage kept in a counter is no stage (the operation keeps them),
    /// whatever waits stand before its steps. One object's count is not
    /// another's, and zero is an edge, not a count.
    #[test]
    fn a_counter_selecting_its_own_branches_is_a_count_and_another_objects_is_not() {
        let stages = "\
SampleTimer {
    onTriggered: {
        if (acts.step === 0) {
            if (!Awaited.all(\"row\", { \"row\": item !== null }))
                return
            item.click()
            acts.step++
            return
        }
        if (acts.step === 1)
            acts.finish()
    }
}
";
        assert_eq!(harness(stages), vec![(10, "beats")]);
        let siblings = "\
Item {
    SampleTimer { onTriggered: { pressTimer.tries++ } }
    SampleTimer { onTriggered: { if (fileTimer.tries === 1) acts.read() } }
    function laid() { return pressTimer.tries > 2 }
}
";
        assert_eq!(harness(siblings), vec![(4, "beats")]);
    }

    /// The statement a step stands in is the line's own: a line closing on
    /// a literal — a string, a template, a regular expression — closes,
    /// and the name on the next line is no part of it.
    #[test]
    fn a_literal_closing_a_line_closes_its_statement() {
        let text = "\
SampleTimer {
    onTriggered: {
        const door = Harness.autoAct === \"ops-branch-pick\" ? \"branch\" : \"repo\"
        const label = `door ${door}`
        const lines = /\\r?\\n/
        acts.mark = door
        acts.text = label
    }
}
";
        assert_eq!(harness(text), vec![]);
    }
}
