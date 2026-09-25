//! The command lines in this tree's documents and sources, held to the
//! catalogue the modules declare (`crate::command`). The rule is
//! .claude/rules-refs/structure.md §コマンドの正本.

use std::path::{Path, PathBuf};

use crate::command::{Command, Permission};

/// The markdown this reads — wider than the torn-block count's
/// (`super::ROOTS`). What is left out and why: rules-refs/structure.md
/// 「どのファイルを読むか」.
const ROOTS: [&str; 4] = [
    "internal-docs",
    ".claude/rules",
    ".claude/rules-refs",
    ".claude/skills",
];
const LOOSE: [&str; 2] = ["CLAUDE.md", "AGENTS.md"];

/// The pages `cargo xtask` with no argument prints.
const USAGE: [&str; 3] = [
    "crates/xtask/src/usage.rs",
    "crates/xtask/src/usage/demo_repo.rs",
    "crates/xtask/src/usage/verify_ui.rs",
];

/// Where the verbs are dispatched, the other list that has to agree with
/// the catalogue.
///
/// Spelled in pieces, as `gate::graph::complaints` does: a whole path in a
/// string reads to the gate's graph as this file reading the crate root,
/// which that check refuses.
const DISPATCH: (&str, &str, &str) = ("xtask", "src", "main.rs");

fn dispatch_path() -> String {
    let (package, dir, name) = DISPATCH;
    format!("crates/{package}/{dir}/{name}")
}

pub(crate) const ROSTER: &str = "crates/xtask/command-ids.txt";

/// Built at run time, because it names the commands that write and
/// read it.
fn roster_header() -> String {
    format!(
        "\
# Every declared command id, written by `{}` and read by `{}`.
# A tripwire, not a catalogue: no command text, and the rule it serves is
# .claude/rules-refs/structure.md §コマンドの正本.
",
        crate::docs::SYNC.line(),
        crate::docs::CHECK.line()
    )
}

const MARK_OPEN: &str = "<!--cmd:";
const MARK_CALL: &str = "<!--call:";
const MARK_CLOSE: &str = "-->";

/// How much of a command a span writes.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Shape {
    /// The whole line, escape and runner included.
    Line,
    /// The call alone, for a section that has said it abbreviates.
    Call,
}

impl Shape {
    fn mark(self) -> &'static str {
        match self {
            Self::Line => MARK_OPEN,
            Self::Call => MARK_CALL,
        }
    }

    fn body(self, command: &Command) -> String {
        match self {
            Self::Line => command.line(),
            Self::Call => command.call.to_string(),
        }
    }
}

pub(crate) struct Held {
    /// What a person has to answer.
    pub(crate) findings: Vec<String>,
    /// What `--sync` answers by writing.
    pub(crate) writable: Vec<String>,
    /// Files whose generated text differs from what stands on disk.
    pub(crate) rewritten: Vec<(PathBuf, String)>,
    /// Counted for the pass line, where a run that matched nothing shows 0.
    pub(crate) spans: usize,
    pub(crate) mentions: usize,
}

/// Hold every place that writes a command line to the catalogue.
pub(crate) fn hold(root: &Path) -> Result<Held, String> {
    let catalogue = crate::commands::all();
    let mut held = Held {
        findings: Vec::new(),
        writable: Vec::new(),
        rewritten: Vec::new(),
        spans: 0,
        mentions: 0,
    };
    for id in crate::commands::repeated() {
        held.findings.push(format!(
            "{ROSTER}: {id} is declared twice — an id names one operation, and a reference to \
             a repeated one means either of them"
        ));
    }
    for file in documents(root)? {
        let text = read(&file)?;
        let named = name_of(root, &file);
        let read = markdown(&text, &catalogue, &named, &mut held);
        if read != text {
            held.rewritten.push((file, read));
        }
    }
    usage(root, &catalogue, &mut held)?;
    dispatch(root, &catalogue, &mut held)?;
    sources(root, &catalogue, &mut held)?;
    roster(root, &catalogue, &mut held)?;
    Ok(held)
}

fn documents(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut found = Vec::new();
    for dir in ROOTS {
        super::under(&root.join(dir), "md", &mut found)?;
    }
    found.extend(LOOSE.iter().map(|name| root.join(name)));
    found.sort();
    Ok(found)
}

fn read(file: &Path) -> Result<String, String> {
    std::fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))
}

fn name_of(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/")
}

/// One document: every span held to the catalogue, and the text as
/// `--sync` would write it.
fn markdown(text: &str, catalogue: &[&'static Command], named: &str, held: &mut Held) -> String {
    let mut out = String::with_capacity(text.len());
    let mut fenced = false;
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            push(&mut out, line, text);
            continue;
        }
        if fenced {
            fence_line(line, catalogue, named, number, held);
            push(&mut out, line, text);
            continue;
        }
        push(
            &mut out,
            &prose_line(line, catalogue, named, number, held),
            text,
        );
    }
    out
}

fn push(out: &mut String, line: &str, text: &str) {
    out.push_str(line);
    if out.len() < text.len() || text.ends_with('\n') {
        out.push('\n');
    }
}

/// A line inside a fence: read for its verbs, kept as it is.
fn fence_line(
    line: &str,
    catalogue: &[&'static Command],
    named: &str,
    number: usize,
    held: &mut Held,
) {
    if let Reading::Managed(command, _) = reading(line, catalogue) {
        held.findings.push(format!(
            "{named}:{number}: this fenced line is {}, written out by hand — a marked span \
             outside the fence is what a reader can be given and a rename can reach",
            command.id
        ));
    }
    // Word by word: taken whole, a compound line reads as no command.
    for verb in called_verbs(line) {
        unknown(verb, catalogue, named, number, held);
    }
    escapes(line, named, number, held);
}

/// Every verb a line calls the runner with, wherever it stands.
fn called_verbs(line: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut rest = line;
    while let Some(at) = rest.find("cargo xtask ") {
        let after = &rest[at + "cargo xtask ".len()..];
        if let Some(word) = after.split_whitespace().next()
            && !word.starts_with('<')
            && !word.starts_with('…')
            && !word.starts_with("...")
        {
            found.push(word);
        }
        rest = after;
    }
    found
}

/// A line of prose: the spans in it, held and rewritten.
fn prose_line(
    line: &str,
    catalogue: &[&'static Command],
    named: &str,
    number: usize,
    held: &mut Held,
) -> String {
    escapes(line, named, number, held);
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(open) = rest.find('`') {
        let (before, after) = rest.split_at(open);
        let Some(close) = after[1..].find('`') else {
            out.push_str(rest);
            return out;
        };
        let span = &after[1..=close];
        out.push_str(before);
        let marked = marker(before);
        let body = span_body(span, marked, catalogue, named, number, held);
        out.push('`');
        out.push_str(&body);
        out.push('`');
        rest = &after[close + 2..];
    }
    out.push_str(rest);
    out
}

/// The id a span is marked with, and how much of the command it asks
/// for — when the text right in front of it is a marker and nothing else
/// stands between.
fn marker(before: &str) -> Option<(Shape, &str)> {
    let head = before.strip_suffix(MARK_CLOSE)?;
    if let Some(at) = head.rfind(MARK_OPEN) {
        return Some((Shape::Line, &head[at + MARK_OPEN.len()..]));
    }
    let at = head.rfind(MARK_CALL)?;
    Some((Shape::Call, &head[at + MARK_CALL.len()..]))
}

/// One span, answered for: a marked one is the catalogue's to write, an
/// unmarked one has to be a mention or an example.
fn span_body(
    span: &str,
    marked: Option<(Shape, &str)>,
    catalogue: &[&'static Command],
    named: &str,
    number: usize,
    held: &mut Held,
) -> String {
    let Some((shape, id)) = marked else {
        match reading(span, catalogue) {
            Reading::Managed(command, shape) => held.findings.push(format!(
                "{named}:{number}: `{span}` is {}, written out by hand — put the marker in \
                 front of it ({}{}{MARK_CLOSE}) and `{}` will write the span",
                command.id,
                shape.mark(),
                command.id,
                crate::docs::SYNC.line()
            )),
            Reading::Mention(verb) => {
                held.mentions += 1;
                unknown(verb, catalogue, named, number, held);
            }
            Reading::Example(verb) => unknown(verb, catalogue, named, number, held),
            Reading::None => {}
        }
        return span.to_string();
    };
    held.spans += 1;
    let Some(command) = catalogue.iter().find(|command| command.id == id) else {
        held.findings.push(format!(
            "{named}:{number}: {}{id}{MARK_CLOSE} names no command — no module declares that \
             id ({ROSTER} lists the ones that are declared)",
            shape.mark()
        ));
        return span.to_string();
    };
    if shape == Shape::Call
        && let Permission::Escape(flag) = command.permission
    {
        held.findings.push(format!(
            "{named}:{number}: {id} runs on {flag}=1 in front of it, so the abbreviated form \
             would drop the half that gates it — write it with {MARK_OPEN}{id}{MARK_CLOSE}"
        ));
        return span.to_string();
    }
    let line = shape.body(command);
    if span != line {
        held.writable.push(format!(
            "{named}:{number}: {id} reads `{span}` here and `{line}` where it is declared — \
             `{}` writes the span, so the one to change is the module",
            crate::docs::SYNC.line()
        ));
    }
    line
}

fn unknown(
    verb: &str,
    catalogue: &[&'static Command],
    named: &str,
    number: usize,
    held: &mut Held,
) {
    if catalogue.iter().any(|command| command.verb() == verb) {
        return;
    }
    held.findings.push(format!(
        "{named}:{number}: `cargo xtask {verb}` names no verb this runner dispatches — either \
         the spelling moved and this reference did not, or the verb is new and the module that \
         runs it declares nothing"
    ));
}

/// A `PGG_ALLOW_*` in a document has to be one the hook reads: the escape
/// is the permission, and one spelled wrong gates nothing.
fn escapes(line: &str, named: &str, number: usize, held: &mut Held) {
    for word in line.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
        if !word.starts_with("PGG_ALLOW_") {
            continue;
        }
        if crate::hook::approval::APPROVAL_FLAGS.contains(&word) {
            continue;
        }
        held.findings.push(format!(
            "{named}:{number}: {word} is not an escape the pre-shell hook reads ({}) — an \
             instruction carrying it is gated by nothing",
            crate::hook::approval::APPROVAL_FLAGS.join(", ")
        ));
    }
}

/// What a piece of text says (rules-refs/structure.md「文書側の記法」).
enum Reading<'a> {
    None,
    /// Exactly `cargo xtask <verb>`: the verb named in prose.
    Mention(&'a str),
    /// Some entry's whole line, or its whole call.
    Managed(&'static Command, Shape),
    /// A call no entry declares.
    Example(&'a str),
}

fn reading<'a>(text: &'a str, catalogue: &[&'static Command]) -> Reading<'a> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let assignments = words.iter().take_while(|word| is_assignment(word)).count();
    let rest = &words[assignments..];
    let whole = words.join(" ");
    if rest.first() != Some(&"cargo") || rest.get(1) != Some(&"xtask") {
        return abbreviated(&whole, words.len(), assignments, catalogue);
    }
    let call = &rest[2..];
    let Some(verb) = call.first() else {
        return Reading::None;
    };
    // A placeholder where the verb stands is the runner being named, not
    // a call: `cargo xtask <command>` is the shape of the usage page.
    if verb.starts_with('<') || verb.starts_with('…') || verb.starts_with("...") {
        return Reading::None;
    }
    if let Some(command) = catalogue.iter().find(|command| command.line() == whole) {
        if assignments == 0 && call.len() == 1 {
            return Reading::Mention(verb);
        }
        return Reading::Managed(command, Shape::Line);
    }
    if assignments == 0 && call.len() == 1 {
        return Reading::Mention(verb);
    }
    Reading::Example(verb)
}

/// A span with no runner in front of it (rules-refs/structure.md
/// 「文書側の記法」).
fn abbreviated<'a>(
    whole: &str,
    words: usize,
    assignments: usize,
    catalogue: &[&'static Command],
) -> Reading<'a> {
    if words < 2 || assignments > 0 {
        return Reading::None;
    }
    match catalogue.iter().find(|command| {
        command.call == whole && !matches!(command.permission, Permission::Escape(_))
    }) {
        Some(command) => Reading::Managed(command, Shape::Call),
        None => Reading::None,
    }
}

fn is_assignment(word: &str) -> bool {
    let Some((name, _)) = word.split_once('=') else {
        return false;
    };
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// The usage page and the catalogue describe the same verbs, or one of
/// them is out of date and a reader cannot tell which.
fn usage(root: &Path, catalogue: &[&'static Command], held: &mut Held) -> Result<(), String> {
    let mut printed: Vec<(String, String)> = Vec::new();
    for page in USAGE {
        printed.extend(blocks(&read(&root.join(page))?));
    }
    held.findings.extend(described(&printed, catalogue));
    Ok(())
}

fn blocks(text: &str) -> Vec<(String, String)> {
    let mut printed: Vec<(String, String)> = Vec::new();
    let mut current: Option<(String, String)> = None;
    for line in text.lines() {
        if let Some(header) = header(line) {
            if let Some(done) = current.take() {
                printed.push(done);
            }
            current = Some((header.to_string(), line.to_string()));
        } else if let Some((_, block)) = current.as_mut() {
            block.push('\n');
            block.push_str(line);
        }
    }
    printed.extend(current);
    printed
}

/// What the page and the catalogue disagree about.
fn described(printed: &[(String, String)], catalogue: &[&'static Command]) -> Vec<String> {
    let mut out = Vec::new();
    for (verb, _) in printed {
        if !catalogue.iter().any(|command| command.verb() == verb) {
            out.push(format!(
                "{}: the page describes `{verb}`, which no module declares — a verb a reader \
                 is told about and the catalogue does not carry is a verb nothing holds",
                USAGE[0]
            ));
        }
    }
    for command in catalogue {
        let verb = command.verb();
        let Some((_, block)) = printed.iter().find(|(named, _)| named == verb) else {
            out.push(format!(
                "{}: nothing on the page describes `{verb}`, declared by {} — the page is \
                 what `cargo xtask` with no argument hands a reader",
                USAGE[0], command.id
            ));
            continue;
        };
        for option in command.options() {
            if !block.contains(option) {
                out.push(format!(
                    "{}: {} is run with {option} and the page's `{verb}` never names it",
                    USAGE[0], command.id
                ));
            }
        }
    }
    out
}

/// A header on the usage page: two spaces and a verb, at the start of a
/// line or right after the opening quote of the const that holds the
/// page.
fn header(line: &str) -> Option<&str> {
    let text = match line.split_once(": &str = \"") {
        Some((_, rest)) => rest,
        None => line,
    };
    let rest = text.strip_prefix("  ")?;
    if rest.starts_with(' ') {
        return None;
    }
    let verb = rest.split_whitespace().next()?;
    verb.chars()
        .all(|c| c.is_ascii_lowercase() || c == '-')
        .then_some(verb)
}

/// The verbs `main.rs` dispatches and the verbs the catalogue declares
/// are the same set, or a command was added to one and not the other.
fn dispatch(root: &Path, catalogue: &[&'static Command], held: &mut Held) -> Result<(), String> {
    let path = dispatch_path();
    let text = read(&root.join(&path))?;
    held.findings.extend(dispatched(&text, catalogue, &path));
    Ok(())
}

fn dispatched(text: &str, catalogue: &[&'static Command], path: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut verbs: Vec<&str> = Vec::new();
    for line in text.lines() {
        let Some((_, rest)) = line.split_once("Some(\"") else {
            continue;
        };
        let Some((verb, _)) = rest.split_once('"') else {
            continue;
        };
        verbs.push(verb);
    }
    for verb in &verbs {
        if !catalogue.iter().any(|command| command.verb() == *verb) {
            out.push(format!(
                "{path}: `{verb}` is dispatched and no module declares it — a command that \
                 runs and is written down nowhere is the copy this check exists to stop"
            ));
        }
    }
    let mut declared: Vec<&str> = catalogue.iter().map(|command| command.verb()).collect();
    declared.sort_unstable();
    declared.dedup();
    for verb in declared {
        if !verbs.contains(&verb) {
            out.push(format!(
                "{path}: the catalogue declares `{verb}` and nothing dispatches it — either \
                 the verb was renamed here alone, or the declaration outlived it"
            ));
        }
    }
    out
}

/// The runner's own sources (why these: rules-refs/structure.md
/// 「どのファイルを読むか」). The cut for test code is the first
/// `#[cfg(test)]`, which is where this tree puts them.
fn sources(root: &Path, catalogue: &[&'static Command], held: &mut Held) -> Result<(), String> {
    let mut files = Vec::new();
    super::under(&root.join("crates/xtask/src"), "rs", &mut files)?;
    files.sort();
    for file in files {
        let text = read(&file)?;
        let named = name_of(root, &file);
        for (index, line) in text.lines().enumerate() {
            if line.trim_start().starts_with("#[cfg(test)]") {
                break;
            }
            if line.trim_start().starts_with("//") {
                continue;
            }
            for span in quoted(line) {
                let Reading::Managed(command, Shape::Line) = reading(span, catalogue) else {
                    continue;
                };
                held.findings.push(format!(
                    "{named}:{}: `{span}` is {}, written out by hand — `{}.line()` is the \
                     same sentence with one place to edit",
                    index + 1,
                    command.id,
                    command.id
                ));
            }
        }
    }
    Ok(())
}

/// The backquoted runs in one line of source: a sentence writes a
/// command between backquotes, which is what makes it findable.
fn quoted(line: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut rest = line;
    while let Some(open) = rest.find('`') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('`') else {
            break;
        };
        found.push(&after[..close]);
        rest = &after[close + 1..];
    }
    found
}

fn roster(root: &Path, catalogue: &[&'static Command], held: &mut Held) -> Result<(), String> {
    let mut ids: Vec<&str> = catalogue.iter().map(|command| command.id).collect();
    ids.sort_unstable();
    let written = format!("{}\n{}\n", roster_header(), ids.join("\n"));
    let path = root.join(ROSTER);
    let standing = std::fs::read_to_string(&path).unwrap_or_default();
    if standing == written {
        return Ok(());
    }
    held.writable.push(drifted(&standing, &ids));
    held.rewritten.push((path, written));
    Ok(())
}

/// What the roster says about a catalogue it no longer matches. An id it
/// carries and nothing declares is the loud half: every reference in the
/// tree was written with that id, and none of them moved.
fn drifted(standing: &str, ids: &[&str]) -> String {
    let gone: Vec<&str> = standing
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .filter(|line| !ids.contains(line))
        .collect();
    if gone.is_empty() {
        return format!(
            "{ROSTER}: the roster does not carry every declared id — `{}` writes it, and the \
             diff is the record of what was added",
            crate::docs::SYNC.line()
        );
    }
    format!(
        "{ROSTER}: {} is on the roster and no module declares it any more. An id is what every \
         reference to an operation is written with, so retiring or renaming one is a change of \
         its own — move every reference in the documents, the skills, the sources, \
         .claude/settings.json and the hooks, then `{}`",
        gone.join(", "),
        crate::docs::SYNC.line()
    )
}

#[cfg(test)]
mod tests {
    use super::{
        Held, PathBuf, Reading, Shape, blocks, described, dispatched, drifted, header, markdown,
        marker, reading,
    };
    use crate::command::{Command, Permission, Where};

    static LAND: Command = Command {
        id: "sample.land",
        call: "land <branch>",
        purpose: "sample",
        run_in: Where::Seat,
        needs: &[],
        permission: Permission::Permit,
    };

    static LAUNCH: Command = Command {
        id: "sample.launch",
        call: "launch",
        purpose: "sample",
        run_in: Where::Seat,
        needs: &[],
        permission: Permission::Escape("PGG_ALLOW_GUI"),
    };

    fn catalogue() -> Vec<&'static Command> {
        vec![&LAND, &LAUNCH]
    }

    fn say(text: &str) -> &'static str {
        match reading(text, &catalogue()) {
            Reading::None => "none",
            Reading::Mention(_) => "mention",
            Reading::Managed(_, Shape::Line) => "managed",
            Reading::Managed(_, Shape::Call) => "managed call",
            Reading::Example(_) => "example",
        }
    }

    /// The three shapes, told apart by what stands after the verb.
    #[test]
    fn a_bare_verb_is_a_mention_and_a_declared_call_is_managed() {
        assert_eq!(say("cargo xtask land"), "mention");
        assert_eq!(say("cargo xtask land <branch>"), "managed");
        assert_eq!(say("cargo xtask land --onto main"), "example");
        assert_eq!(say("git status"), "none");
        assert_eq!(say("cargo xtask <command>"), "none");
    }

    /// The flag is the permission, so it is the half held in step.
    #[test]
    fn an_escape_in_front_makes_even_a_bare_verb_managed() {
        assert_eq!(say("PGG_ALLOW_GUI=1 cargo xtask launch"), "managed");
        assert_eq!(say("cargo xtask launch"), "mention");
        assert_eq!(say("PGG_ALLOW_GUI=0 cargo xtask launch"), "example");
    }

    #[test]
    fn extra_spacing_does_not_change_what_a_span_says() {
        assert_eq!(say("cargo  xtask   land <branch>"), "managed");
    }

    #[test]
    fn a_marker_is_read_only_when_it_stands_against_the_span() {
        assert_eq!(
            marker("see <!--cmd:seat.take-->"),
            Some((Shape::Line, "seat.take"))
        );
        assert_eq!(
            marker("see <!--call:seat.take-->"),
            Some((Shape::Call, "seat.take"))
        );
        assert_eq!(marker("<!--cmd:a.b--> "), None);
        assert_eq!(marker("no marker here"), None);
    }

    #[test]
    fn the_abbreviated_form_writes_the_call_and_refuses_an_escaped_command() {
        assert_eq!(say("land <branch>"), "managed call");
        assert_eq!(say("launch"), "none");
        assert_eq!(say("git status --short"), "none");
        assert_eq!(say("land"), "none");

        let held = held_over("the daily tier is <!--call:sample.land-->`land --onto main`\n");
        assert_eq!(held.writable.len(), 1, "{:?}", held.writable);
        assert_eq!(
            rewritten(&held),
            "the daily tier is <!--call:sample.land-->`land <branch>`\n"
        );

        let escaped = held_over("<!--call:sample.launch-->`launch`\n");
        assert_eq!(escaped.findings.len(), 1, "{:?}", escaped.findings);
        assert!(
            escaped.findings[0].contains("drop the half that gates it"),
            "{:?}",
            escaped.findings
        );
        assert_eq!(rewritten(&escaped), "<!--call:sample.launch-->`launch`\n");
    }

    #[test]
    fn the_usage_headers_are_the_two_space_verbs() {
        assert_eq!(header("  land [<branch>]"), Some("land"));
        assert_eq!(
            header("const TAIL: &str = \"  shipped [--no-build]"),
            Some("shipped")
        );
        assert_eq!(header("      options:"), None);
        assert_eq!(header("  Not a verb"), None);
    }

    fn held_over(text: &str) -> Held {
        let mut held = Held {
            findings: Vec::new(),
            writable: Vec::new(),
            rewritten: Vec::new(),
            spans: 0,
            mentions: 0,
        };
        let read = markdown(text, &catalogue(), "sample.md", &mut held);
        held.rewritten.push((PathBuf::from("sample.md"), read));
        held
    }

    fn rewritten(held: &Held) -> &str {
        held.rewritten
            .first()
            .map(|(_, text)| text.as_str())
            .unwrap_or_default()
    }

    #[test]
    fn a_stale_span_is_written_from_the_declaration() {
        let held = held_over("run <!--cmd:sample.land-->`cargo xtask land --onto main` here\n");
        assert_eq!(held.writable.len(), 1, "{:?}", held.writable);
        assert!(
            held.writable[0].contains("sample.land"),
            "{:?}",
            held.writable
        );
        assert_eq!(
            rewritten(&held),
            "run <!--cmd:sample.land-->`cargo xtask land <branch>` here\n"
        );
    }

    /// Byte for byte, which is what makes the write idempotent.
    #[test]
    fn a_span_that_already_quotes_the_declaration_is_untouched() {
        let text = "run <!--cmd:sample.land-->`cargo xtask land <branch>` here\n";
        let held = held_over(text);
        assert!(held.writable.is_empty(), "{:?}", held.writable);
        assert!(held.findings.is_empty(), "{:?}", held.findings);
        assert_eq!(rewritten(&held), text);
        assert_eq!(held.spans, 1);
    }

    #[test]
    fn a_marker_naming_no_command_is_a_finding_a_sync_cannot_answer() {
        let held = held_over("<!--cmd:sample.gone-->`cargo xtask land <branch>`\n");
        assert!(held.writable.is_empty(), "{:?}", held.writable);
        assert_eq!(held.findings.len(), 1, "{:?}", held.findings);
        assert!(
            held.findings[0].contains("names no command"),
            "{:?}",
            held.findings
        );
    }

    #[test]
    fn a_managed_line_written_by_hand_is_named_with_the_marker_to_use() {
        let held = held_over("just run `PGG_ALLOW_GUI=1 cargo xtask launch` when asked\n");
        assert_eq!(held.findings.len(), 1, "{:?}", held.findings);
        assert!(
            held.findings[0].contains("<!--cmd:sample.launch-->"),
            "{:?}",
            held.findings
        );
    }

    /// The mention shape has no body to rewrite, so this is all that
    /// holds it.
    #[test]
    fn a_mention_of_a_verb_nobody_dispatches_is_a_finding() {
        let held = held_over("`cargo xtask land` is the one way; `cargo xtask sail` is not\n");
        assert_eq!(held.findings.len(), 1, "{:?}", held.findings);
        assert!(
            held.findings[0].contains("`cargo xtask sail`"),
            "{:?}",
            held.findings
        );
        assert_eq!(held.mentions, 2);
    }

    #[test]
    fn an_escape_the_hook_does_not_read_is_a_finding() {
        let held = held_over("run it with PGG_ALLOW_EVERYTHING=1 in front\n");
        assert_eq!(held.findings.len(), 1, "{:?}", held.findings);
        assert!(
            held.findings[0].contains("PGG_ALLOW_EVERYTHING"),
            "{:?}",
            held.findings
        );
        assert!(
            held_over("with PGG_ALLOW_GUI=1 in front\n")
                .findings
                .is_empty()
        );
    }

    #[test]
    fn a_fenced_block_keeps_its_text_and_still_answers_for_its_verbs() {
        let text = "```sh\nPGG_ALLOW_REBASE=1 git rebase main && cargo xtask sail\n```\n";
        let held = held_over(text);
        assert_eq!(rewritten(&held), text);
        assert_eq!(held.findings.len(), 1, "{:?}", held.findings);
        assert!(held.findings[0].contains("sail"), "{:?}", held.findings);
    }

    #[test]
    fn a_verb_on_one_list_and_not_the_other_is_named_both_ways() {
        let both = "        Some(\"land\") => land::run(&args[1..]),\n\
                            Some(\"launch\") => gui::launch(&args[1..]),\n";
        assert!(dispatched(both, &catalogue(), "main.rs").is_empty());

        let extra = format!("{both}        Some(\"sail\") => sail::run(&args[1..]),\n");
        let found = dispatched(&extra, &catalogue(), "main.rs");
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("no module declares it"), "{found:?}");

        let short = "        Some(\"land\") => land::run(&args[1..]),\n";
        let found = dispatched(short, &catalogue(), "main.rs");
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("nothing dispatches it"), "{found:?}");
    }

    #[test]
    fn the_page_answers_for_every_verb_and_every_option() {
        let page = "  land [<branch>]\n      Put a branch on main.\n\n  launch [--no-build]\n      A real window.\n";
        assert!(described(&blocks(page), &catalogue()).is_empty());

        let silent = "  land [<branch>]\n      Put a branch on main.\n";
        let found = described(&blocks(silent), &catalogue());
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].contains("nothing on the page describes `launch`"),
            "{found:?}"
        );
    }

    #[test]
    fn an_id_the_roster_carries_and_nobody_declares_is_said_loudly() {
        let standing = "# header\n\nsample.land\nsample.retired\n";
        let said = drifted(standing, &["sample.land", "sample.launch"]);
        assert!(said.contains("sample.retired"), "{said}");
        assert!(said.contains("a change of its own"), "{said}");

        let growing = drifted(
            "# header\n\nsample.land\n",
            &["sample.land", "sample.launch"],
        );
        assert!(
            growing.contains("does not carry every declared id"),
            "{growing}"
        );
    }
}
