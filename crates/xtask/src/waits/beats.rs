//! The beats rule of the app's harness: a count of a handler's own
//! firings held against a number, read off the file's tokens ([`beats`]).

use super::Candidate;
use super::cond::guards;
use super::stmts::{Stmt, parse_handler};

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
/// inside a branch an `if` on that same name selects, reached on no way
/// but through a named wait answering yes (`Awaited.all` / `still` /
/// `drawnSince`, the one way a stage asks the product — `if (x === 1) {
/// if (!Awaited.all(…)) return; x++ }`). The handler is read as a tree of
/// statements (`parse_handler`), and the proof asks every way from the
/// branch's start to the step (`proved`), each `if` on the way read by
/// what its condition's truth or falsity implies of a wait (`Cond`: `||`
/// over `&&` over `!`, parentheses, the wait call, and operands it reads
/// nothing inside; a condition the grammar does not read whole implies
/// nothing — no part of it is picked out). A wait called and its answer
/// dropped, read into a name, standing on a way of its own (`if (y) if
/// (!Awaited…) return`, an `else if`, a loop's body), or one a `&&` or
/// `||` lets the step past, proves nothing, and a block its counter's
/// value alone selects, stepped on nothing the product answered, counts
/// firings: `if (x === 0) { x++ }` is one. A step inside a function or an
/// object an expression hands off runs on a way the proof cannot read,
/// and counts. A handler holding what the tree does not read — a
/// `switch`, `try`, `do`, `function`, `class`, `with`, a bracket never
/// closed — is `unread`. A name is the whole `.`-joined path,
/// so `a.step` is not `b.step`; a bare name is the same property as a
/// qualified one ending in it. Such a count held with `<`, `>`, `<=`,
/// `>=`, `===`, `!==`, `==` or `!=` against a number other than zero,
/// anywhere in the file, is named at that line — whatever the comparison
/// then decides, since a count of beats is a reading of the clock
/// wherever it is read. Held against another value (the beat it was
/// asked in, `asked !== beats - 1`) it is a reading of order, not time,
/// and zero is "none yet", an edge, not a count. The tokens are the
/// file's own (`crate::lex::tokens`), a literal one token among them: a
/// line closing on a string closes, as one closing on a name does. What
/// cannot be read is not passed.
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
        let Some(tree) = parse_handler(body) else {
            candidates.push(Candidate {
                first: tokens[open].line,
                last: tokens[open].line,
                shown: tokens[open].line,
                rule: "unread",
            });
            continue;
        };
        let locals = declared_in(body);
        for stepped in stepped_in(body, &tree) {
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

/// The names `body` steps as counts: each stepped — by one, by an
/// amount, or set to anything but zero (a reset, not a step), a
/// transition all the same — outside every block an `if` on that name
/// selects, or inside one with no wait letting the step run
/// (`stage_step`).
fn stepped_in(body: &[crate::lex::Token], tree: &[Stmt]) -> Vec<String> {
    let mut out = Vec::new();
    for (i, token) in body.iter().enumerate() {
        let stepped = match token.text.as_str() {
            "++" | "--" => name_ending_at(body, i.wrapping_sub(1))
                .map(|(name, _)| name)
                .or_else(|| name_beginning_at(body, i + 1).map(|(name, _)| name)),
            "+=" | "-=" => name_ending_at(body, i.wrapping_sub(1)).map(|(name, _)| name),
            "=" => {
                let Some((left, _)) = name_ending_at(body, i.wrapping_sub(1)) else {
                    continue;
                };
                // `x = 0` is a reset; anything else set — a number, a name, a choice between two — moves the
                // stage.
                let reset = body.get(i + 1).is_some_and(|t| t.text == "0")
                    && body
                        .get(i + 2)
                        .is_none_or(|t| t.line != body[i + 1].line || t.text == ";");
                (!reset).then_some(left)
            }
            _ => None,
        };
        if let Some(name) = stepped
            && !stage_step(body, tree, i, &name)
            && !out.contains(&name)
        {
            out.push(name);
        }
    }
    out
}

/// The statements holding `at`, outermost first.
fn ancestors(stmts: &[Stmt], at: usize) -> Vec<&Stmt> {
    let mut out = Vec::new();
    let mut level: &[Stmt] = stmts;
    loop {
        let Some(stmt) = level.iter().find(|s| s.holds(at)) else {
            return out;
        };
        out.push(stmt);
        level = match stmt {
            Stmt::Block { body, .. } => body,
            Stmt::If { then, els, .. } => {
                if then.holds(at) {
                    std::slice::from_ref(then.as_ref())
                } else if let Some(els) = els
                    && els.holds(at)
                {
                    std::slice::from_ref(els.as_ref())
                } else {
                    return out;
                }
            }
            Stmt::Loop { body, .. } => std::slice::from_ref(body.as_ref()),
            Stmt::Exit { .. } | Stmt::Plain { .. } => return out,
        };
    }
}

/// Whether the step at `at` is a stage's, proved: it lies in a branch an
/// `if` on `name` selects — the innermost `if` holding it whose condition
/// reads the name, through the `else` chain it may hang on — and every
/// way from that branch's start to the step passes a wait answering yes
/// (`proved`). A step inside a condition, or inside a function or an
/// object an expression hands off, runs on a way the proof cannot read.
fn stage_step(body: &[crate::lex::Token], tree: &[Stmt], at: usize, name: &str) -> bool {
    let chain = ancestors(tree, at);
    let Some(innermost) = chain.last() else {
        return false;
    };
    if matches!(innermost, Stmt::Plain { .. } | Stmt::Exit { .. })
        && inside_braces(body, innermost.span().0, at)
    {
        return false;
    }
    for stmt in chain.iter().rev() {
        if let Stmt::If {
            cond: (from, to),
            then,
            els,
            ..
        } = stmt
        {
            if *from <= at && at < *to {
                return false;
            }
            if !condition_reads(&body[*from..*to], name) {
                continue;
            }
            let branch: &Stmt = if then.holds(at) {
                then
            } else if let Some(els) = els
                && els.holds(at)
            {
                els
            } else {
                return false;
            };
            return proved_in(body, branch, at, false);
        }
    }
    false
}

/// Whether `at` lies inside a `{ … }` of the expression beginning at `from`.
fn inside_braces(body: &[crate::lex::Token], from: usize, at: usize) -> bool {
    let mut depth = 0i32;
    for token in &body[from..at] {
        match token.text.as_str() {
            "{" => depth += 1,
            "}" => depth -= 1,
            _ => {}
        }
    }
    depth > 0
}

/// Whether `cond` reads `name` (`same_property`).
fn condition_reads(cond: &[crate::lex::Token], name: &str) -> bool {
    (0..cond.len()).any(|k| {
        is_name(&cond[k].text)
            && name_ending_at(cond, k).is_some_and(|(read, _)| same_property(&read, name))
    })
}

/// Whether every way from the start of `stmt` to the step at `at` passes a
/// wait answering yes; `guarded` says whether the ways into `stmt` already
/// did. A wait's answer selects: inside the then-branch of a condition
/// whose truth implies it, and inside the else-branch of one whose
/// falsity does, it answered yes (`Guards`).
fn proved_in(body: &[crate::lex::Token], stmt: &Stmt, at: usize, guarded: bool) -> bool {
    match stmt {
        Stmt::Block { body: stmts, .. } => proved(body, stmts, at, guarded),
        Stmt::Loop { body: inner, .. } => proved_in(body, inner, at, guarded),
        Stmt::If {
            cond: (from, to),
            then,
            els,
            ..
        } => {
            if *from <= at && at < *to {
                return false;
            }
            let guards = guards(&body[*from..*to]);
            if then.holds(at) {
                proved_in(body, then, at, guarded || guards.then)
            } else if let Some(els) = els
                && els.holds(at)
            {
                proved_in(body, els, at, guarded || guards.els)
            } else {
                false
            }
        }
        Stmt::Exit { .. } | Stmt::Plain { .. } => guarded,
    }
}

/// The statements of one level in order: a statement before the step that
/// leaves the handler unless a wait answered yes — an `if` whose
/// then-branch always leaves and whose condition's falsity implies the
/// wait (`if (!Awaited.X(…) [|| …]) return …`), or one whose else-branch
/// always leaves and whose truth does (`if (Awaited.X(…) [&& …]) … else
/// return …`) — guards every way past it; the statement holding the step
/// is entered with what stands by then.
fn proved(body: &[crate::lex::Token], stmts: &[Stmt], at: usize, mut guarded: bool) -> bool {
    for stmt in stmts {
        if stmt.holds(at) {
            return proved_in(body, stmt, at, guarded);
        }
        if let Stmt::If {
            cond: (from, to),
            then,
            els,
            ..
        } = stmt
        {
            let guards = guards(&body[*from..*to]);
            if (guards.els && then.always_exits())
                || (guards.then && els.as_ref().is_some_and(|e| e.always_exits()))
            {
                guarded = true;
            }
        }
    }
    false
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

#[cfg(test)]
mod tests {
    /// The findings by the harness's rules, as `qml::scan` makes them.
    fn harness(text: &str) -> Vec<(usize, &'static str)> {
        let (findings, _) =
            super::super::qml::scan("WindowActs.qml", text, super::super::qml::Kind::Harness);
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

    /// A stage's step is proved by the wait's answer letting it run, and by
    /// nothing else: a wait called and its answer dropped, or read into a
    /// name, counts firings — as does a guard above the block, in a sibling
    /// block, behind an `else`, or one a `&&` lets the step past.
    #[test]
    fn a_step_is_proved_only_by_a_wait_whose_answer_lets_it_run() {
        let stage = |block: &str| {
            format!(
                "SampleTimer {{\n    onTriggered: {{\n        if (acts.step === 0) {{\n{block}\n            \
                 acts.step++\n        }}\n        if (acts.step === 1) acts.finish()\n    }}\n}}\n"
            )
        };
        let firing = |block: &str, why: &str| {
            assert_eq!(
                harness(&stage(block)),
                vec![(6 + block.lines().count(), "beats")],
                "{why}"
            );
        };
        let grounded = |block: &str, why: &str| {
            assert_eq!(harness(&stage(block)), vec![], "{why}");
        };
        firing(
            "            Awaited.all(\"row\", { \"row\": item !== null })",
            "the wait's answer dropped",
        );
        firing(
            "            const ok = Awaited.all(\"row\", { \"row\": item !== null })\n            if (!ok) return",
            "the wait read into a name",
        );
        firing(
            "            if (x && !Awaited.all(\"row\", { \"row\": item }))\n                return",
            "a `&&` lets the step past with the wait false",
        );
        firing(
            "            if (y) { if (!Awaited.all(\"row\", { \"row\": item })) return }",
            "a guard in a sibling block",
        );
        firing(
            "            if (y) z()\n            else if (!Awaited.all(\"row\", { \"row\": item })) return",
            "a guard behind an `else`",
        );
        firing(
            "            if (Awaited.all(\"row\", { \"row\": item }) || y) { z() }",
            "an `||` lets the then-block run with the wait false — and the step is outside it",
        );
        grounded(
            "            if (!Awaited.all(\"row\", { \"row\": item !== null }) || x)\n                return",
            "a guard that returns whenever the wait answers no",
        );
        grounded(
            "            if (!Awaited.still(\"laid\", card.y)) {\n                return\n            }",
            "the braced form",
        );
        grounded(
            "            if (!(Awaited.drawnSince(\"drawn\"))) return false",
            "the wait in parentheses, a value returned",
        );
        grounded(
            "            if (!Awaited.all(\"row\", { \"row\": item })) return\n            if (y) z()",
            "a guard at the block's head reaches every statement after it",
        );
        grounded(
            "            if (!Awaited.all(\"row\", { \"row\": item })) {\n                view.positionViewAtIndex(want)\n                return\n            }",
            "a braced guard that does something before it returns",
        );
        firing(
            "            if (!Awaited.all(\"row\", { \"row\": item })) {\n                if (late) return\n            }",
            "a braced guard that returns only sometimes",
        );
        // A stage set by choice, or to a name, moves like a step; a reset to zero does not.
        let chosen = "\
SampleTimer {
    onTriggered: {
        if (acts.step === 0) {
            acts.step = acts.away ? 1 : 3
        } else if (acts.step === 1) {
            if (!Awaited.all(\"gone\", { \"gone\": peek === \"\" }))
                return
            acts.strokes = 0
            acts.step = next
        }
        if (acts.step === 3 || acts.strokes > 2) acts.finish()
    }
}
";
        assert_eq!(harness(chosen), vec![(5, "beats"), (11, "beats")]);
    }

    /// A condition proves what its truth or falsity implies of a wait, read by the grammar — negation over a group,
    /// `&&` under `||` by precedence — and a condition the grammar does not read whole proves nothing: the `&&` or
    /// `||` around an operand it cannot read is not picked out.
    #[test]
    fn a_condition_proves_what_its_truth_implies_and_an_unread_one_nothing() {
        let stage = |block: &str| {
            format!(
                "SampleTimer {{\n    onTriggered: {{\n        if (acts.step === 0) {{\n{block}\n            \
                 acts.step++\n        }}\n        if (acts.step === 1) acts.finish()\n    }}\n}}\n"
            )
        };
        let firing = |block: &str, why: &str| {
            assert_eq!(
                harness(&stage(block)),
                vec![(6 + block.lines().count(), "beats")],
                "{why}"
            );
        };
        let grounded = |block: &str, why: &str| {
            assert_eq!(harness(&stage(block)), vec![], "{why}");
        };
        grounded(
            "            if (!(Awaited.all(\"row\", { \"row\": item }) && y))\n                return",
            "a negated `&&`: leaving on its truth, what follows has both parts true",
        );
        grounded(
            "            if (!(!Awaited.all(\"row\", { \"row\": item }) || y)) {\n                z()\n            } else return",
            "a negated `||`: the then-branch runs with both parts false, the wait's no among them",
        );
        grounded(
            "            if (y || !Awaited.all(\"row\", { \"row\": item }) || z && w)\n                return",
            "an `&&` under the `||`, by precedence: false means every disjunct false",
        );
        grounded(
            "            if ((Awaited.all(\"row\", { \"row\": item })))\n                z()\n            else\n                return",
            "parentheses around the wait, the else leaving",
        );
        firing(
            "            if (!Awaited.all(\"row\", { \"row\": item }) && y)\n                return",
            "an `&&` on the guard: it stays with the wait's no when y is false",
        );
        firing(
            "            if (!Awaited.all(\"row\", { \"row\": item }) || y ? z : w)\n                return",
            "a `?:` under the `||`: the condition is read whole or not at all",
        );
        firing(
            "            if (x = !Awaited.all(\"row\", { \"row\": item }))\n                return",
            "an assignment in the condition",
        );
        firing(
            "            if (Awaited.all(\"row\", { \"row\": item }) === true) {\n                z()\n            } else return",
            "the wait with something hung on it is an operand read for nothing",
        );
        firing(
            "            if (x === !Awaited.all(\"row\", { \"row\": item }))\n                return",
            "a `!` past an operand's start",
        );
        firing(
            "            if (!Awaited.all(\"row\", { \"row\": item }) || (y, z))\n                return",
            "a `,` inside a group is the group's; the group is an operand read for nothing — and this one is not the wait's",
        );
    }

    /// The then-block a wait selects grounds the step inside it; a guard above the selecting block grounds nothing
    /// in it — the stage asks for itself.
    #[test]
    fn a_then_block_a_wait_selects_grounds_its_step_and_a_guard_above_the_block_does_not() {
        let then = "\
SampleTimer {
    onTriggered: {
        if (acts.step === 0) {
            if (Awaited.all(\"row\", { \"row\": item }) && y) {
                z()
                acts.step++
            }
        }
        if (acts.step === 1) acts.finish()
    }
}
";
        assert_eq!(harness(then), vec![]);
        let above = "\
SampleTimer {
    onTriggered: {
        if (!Awaited.all(\"open\", { \"open\": page !== null }))
            return
        if (acts.step === 0) {
            item.click()
            acts.step++
        }
        if (acts.step === 1) acts.finish()
    }
}
";
        assert_eq!(harness(above), vec![(9, "beats")]);
    }

    /// The proof asks every way to the step: a guard on a way of its own — under another `if`, behind an
    /// `else if`, in a loop's body the step is not in — proves nothing, a wait whose no leaves by the else proves
    /// the then, and a step inside a function the handler hands off runs on a way the proof cannot read.
    #[test]
    fn a_guard_on_a_way_of_its_own_proves_nothing() {
        let stage = |block: &str| {
            format!(
                "SampleTimer {{\n    onTriggered: {{\n        if (acts.step === 0) {{\n{block}\n            \
                 acts.step++\n        }}\n        if (acts.step === 1) acts.finish()\n    }}\n}}\n"
            )
        };
        let firing = |block: &str, why: &str| {
            assert_eq!(
                harness(&stage(block)),
                vec![(6 + block.lines().count(), "beats")],
                "{why}"
            );
        };
        let grounded = |block: &str, why: &str| {
            assert_eq!(harness(&stage(block)), vec![], "{why}");
        };
        firing(
            "            if (y)\n                if (!Awaited.all(\"row\", { \"row\": item })) return",
            "a guard under another if, on one line each",
        );
        firing(
            "            for (let i = 0; i < 3; i++) {\n                if (!Awaited.all(\"row\", { \"row\": item })) return\n            }",
            "a guard in a loop's body the step is not in",
        );
        grounded(
            "            for (let i = 0; i < 3; i++) {\n                if (!Awaited.all(\"row\", { \"row\": item })) return\n                z(i)\n            }\n            if (!Awaited.all(\"laid\", { \"laid\": y }))\n                return",
            "the step after a guard of its own level",
        );
        grounded(
            "            if (Awaited.all(\"row\", { \"row\": item })) z()\n            else return",
            "a wait whose no leaves by the else guards what follows",
        );
        grounded(
            "            Harness.report(\"x a=\" + a\n                           + \" b=\" + b)\n            if (!Awaited.all(\"row\", { \"row\": item }))\n                return",
            "a report over two lines is one statement",
        );
        let handed_off = "\
SampleTimer {
    onTriggered: {
        if (acts.step === 0) {
            if (!Awaited.all(\"row\", { \"row\": item }))
                return
            item.grabToImage(function (res) {
                acts.step = 1
            })
        }
        if (acts.step === 1) acts.finish()
    }
}
";
        assert_eq!(harness(handed_off), vec![(10, "beats")]);
    }

    /// A literal closing a line closes the statement as a name or a number would: the guard on the next line is
    /// the step's own, not the tail of the `const` above it, whatever the literal — a string, a template, a regular
    /// expression — and whatever the operator before it.
    #[test]
    fn a_literal_closing_a_line_closes_its_statement() {
        let text = "\
SampleTimer {
    onTriggered: {
        const door = Harness.autoAct === \"ops-branch-pick\" ? \"branch\"
                   : Harness.autoAct === \"ops-worktree-pick\" ? \"worktree\" : \"repo\"
        if (acts.step === 0) {
            const eol = door === \"branch\" || door === \"worktree\"
            if (!Awaited.all(\"row\", { \"row\": item }))
                return
            acts.step++
            return
        }
        if (acts.step === 1) {
            const label = `door ${door}`
            if (!Awaited.all(\"laid\", { \"laid\": label !== \"\" }))
                return
            acts.step++
            return
        }
        if (acts.step === 2) {
            const lines = /\\r?\\n/
            if (!Awaited.all(\"split\", { \"split\": item.text.split(lines).length > 1 }))
                return
            acts.step++
        }
        if (acts.step === 3) acts.finish(door)
    }
}
";
        assert_eq!(harness(text), vec![]);
    }

    /// A handler holding what the tree does not read is `unread`, not passed.
    #[test]
    fn a_handler_the_proof_cannot_read_is_unread() {
        let switched = "\
SampleTimer {
    onTriggered: {
        switch (acts.step) {
        case 0:
            acts.step = 1
        }
    }
}
";
        assert_eq!(harness(switched), vec![(2, "unread")]);
        let dangling = "\
SampleTimer {
    onTriggered: {
        else acts.step = 1
    }
}
";
        assert_eq!(harness(dangling), vec![(2, "unread")]);
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
