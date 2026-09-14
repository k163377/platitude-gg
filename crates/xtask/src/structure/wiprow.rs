//! Who may answer "is this window's own working-tree row on the graph",
//! counted by machine (.claude/rules/app-ui.md §作業コピーの行は全部 git の
//! all-zero id を着ている).
//!
//! **The id cannot answer it.** Every working copy's uncommitted work is
//! drawn as a row wearing git's all-zero id — that spelling means "there
//! is no object here", which is as true of a neighbour's row as of ours
//! (`GraphModel::carried_row_of`) — and the walk prepends this window's
//! only once the status has said there is something to commit. So a pass
//! that beat that status carries every copy's row and none of ours, and a
//! reader that takes a row out of the graph by index and asks the id
//! whether it is the working tree's is answered yes by somebody else's.
//! The graph answers it in one place instead (`GraphModel::wip_row`), and
//! which row a landing on a real commit measures against is another
//! (`newestCommitRow`).
//!
//! That question was spelled out by hand in several readers and one of
//! them was missed when the rule changed, which is the shape this exists
//! to stop: **a row taken from a numbered position is never asked whether
//! it is the working tree's.** A position is not a row anybody is on, so
//! the answer is being read as "ours is there", and it is not.
//!
//! **Asking it of a row somebody is on is untouched** — the row a click
//! landed on, the row a selection stood at, the row a reword names. There
//! the question is what kind of row this is, and a copy's row answers it
//! as truly as ours: neither is a commit to select.
//!
//! **The source is walked in the order it is written**, one character at
//! a time: a `{` opens a scope and a `}` closes it, wherever they are, so
//! a body written on one line reads exactly as the same body written on
//! five. Anything read line by line gets this wrong — a function whose
//! whole body is on its line opens no scope at all, and its neighbour's
//! parameter is then answered by a name from inside it.
//!
//! **It decides a spelling, not a program.** Two readings are refused:
//! the question asked of a numbered row in one expression, and the
//! question asked of a name whose innermost binding is given a numbered
//! row anywhere — on any path, since a name given one on a branch still
//! holds it there. **A name bound by anything this cannot read is
//! uncertain, and an uncertain name is never refused**: parameters,
//! arrows, loop variables and patterns all land there, and so does a word
//! one scope declares twice. Which row such a name means needs the
//! program, and what the landing does with it is what `wip-landing` and
//! `wip-landing-stopped` run against a real window (verify-ui スキル
//! §作業ツリー行への 2 つの着地). No branch is read here and no value is
//! followed.
//!
//! **A binder list this cannot read whole closes the body it opens to
//! names from outside.** A pattern or a default binds a name the list
//! does not spell plainly, so a reader inside is asking about that one —
//! and answering it with the word standing outside the body is the one
//! way a sound reader gets refused. The walk stops at such a scope and
//! says nothing, however the list is spaced.
//!
//! Read across both QML modules: the product's own and the harness's,
//! which asks the same question of the same graph.

use std::collections::BTreeMap;
use std::path::Path;

/// The QML this rule is read over — the product and the harness alike.
const TREES: &[&str] = &[
    "crates/platitude-app/src/ui",
    "crates/platitude-app/src/auto",
];
/// How a row is taken out of the graph at a numbered position.
const BY_INDEX: &str = "oidAt";
/// What must not be asked of it. **Only the working tree's question**: a
/// stash row carries a real object, so asking the id whether it names one
/// (`stashRefOf`) is answered by the row itself and by nothing else.
const OF_THE_ID: &[&str] = &["wipOid"];
/// The words that introduce a name rather than write to one.
const DECLARES: &[&str] = &["const", "let", "var"];
/// Where a failing line sends its reader.
const RULE: &str = ".claude/rules-refs/app-ui.md §作業コピーの行";

/// One failure per line that asks a row read by index what kind of row it
/// is, and how many QML files were read.
pub(super) fn check(root: &Path) -> Result<(Vec<String>, usize), String> {
    let mut failures = Vec::new();
    let mut read = 0;
    for tree in TREES {
        let dir = root.join(tree);
        let mut files = Vec::new();
        super::collect(&dir, &mut files)?;
        files.sort();
        files.retain(|path| path.extension().is_some_and(|e| e == "qml"));
        read += files.len();
        for path in &files {
            let shown = super::relative(&dir, path);
            let text =
                std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
            for (line, asked) in findings(&text) {
                failures.push(format!(
                    "{tree}/{shown}:{line}: asks `{asked}(` of a row taken out of the graph by \
                     index — every working copy's row wears the same all-zero id, so the answer is \
                     a neighbour's as readily as this window's. The graph says which row is ours \
                     (`GraphModel.wipRow`) and which is the newest real commit \
                     (`GraphModel.newestCommitRow`) ({RULE})"
                ));
            }
        }
    }
    Ok((failures, read))
}

/// Every line that puts the working tree's question to a row read at a
/// numbered position, with the question it asked.
fn findings(text: &str) -> Vec<(usize, &'static str)> {
    Walk::over(&super::without_comments_and_strings(text)).findings()
}

/// What a scope knows about one name.
#[derive(Default)]
struct Knows {
    /// It is given a row read at a numbered position somewhere. **On any
    /// path** — a name given one inside an `if` holds it wherever that
    /// `if` was taken, and a reader after the block is asking about it
    /// there.
    numbered: bool,
    /// Something bound it that this cannot read: a parameter, an arrow's,
    /// a loop's, or a second declaration in the same scope. Never
    /// refused.
    uncertain: bool,
}

/// One scope's names, and whether what opened it could be read.
#[derive(Default)]
struct Scope {
    names: BTreeMap<String, Knows>,
    /// A binder list opened this scope that this could not read whole — a
    /// pattern, a default, a rest. **A name not bound here may then not
    /// be answered by one from outside it**, because what that list bound
    /// is exactly what could not be read: the walk stops here and says
    /// nothing.
    unreadable: bool,
}

/// The file read in the order it is written, carrying the scopes that are
/// open at each point.
struct Walk<'a> {
    code: &'a str,
    chars: Vec<(usize, char)>,
    at: usize,
    line: usize,
    /// Innermost last; the first is the file itself and is never popped.
    scopes: Vec<Scope>,
    /// Names owed to the next scope a `{` opens — a function's or an
    /// arrow's parameters, a loop's variable — and whether the list they
    /// came from was read whole.
    owed: Vec<String>,
    owed_unreadable: bool,
    found: Vec<(usize, &'static str)>,
}

impl<'a> Walk<'a> {
    fn over(code: &'a str) -> Self {
        Self {
            code,
            chars: code.char_indices().collect(),
            at: 0,
            line: 1,
            scopes: vec![Scope::default()],
            owed: Vec::new(),
            owed_unreadable: false,
            found: Vec::new(),
        }
    }

    fn findings(mut self) -> Vec<(usize, &'static str)> {
        while let Some(c) = self.peek(self.at) {
            match c {
                '\n' => {
                    self.line += 1;
                    self.at += 1;
                }
                '{' => {
                    let mut scope = Scope {
                        unreadable: std::mem::take(&mut self.owed_unreadable),
                        ..Scope::default()
                    };
                    for name in std::mem::take(&mut self.owed) {
                        scope.names.entry(name).or_default().uncertain = true;
                    }
                    self.scopes.push(scope);
                    self.at += 1;
                }
                '}' => {
                    if self.scopes.len() > 1 {
                        self.scopes.pop();
                    }
                    self.at += 1;
                }
                '(' => match self.an_arrow_at(self.at) {
                    Some(close) => self.skip_past(close),
                    None => self.at += 1,
                },
                c if starts_a_word(c) => self.word(),
                _ => self.at += 1,
            }
        }
        self.found
    }

    /// One identifier and what follows it decides what it was.
    fn word(&mut self) {
        let from = self.at;
        while self.peek(self.at).is_some_and(is_word) {
            self.at += 1;
        }
        let word = self.slice(from, self.at);
        let member = self.behind(from) == Some('.');
        let asked = OF_THE_ID
            .iter()
            .copied()
            .find(|of| *of == word)
            // A space between the name and its bracket changes nothing
            // about what is asked.
            .filter(|_| matches!(self.non_space(self.at), Some((_, '('))));
        match word {
            // The names a function binds are its body's, and the body is
            // whatever `{` comes next — on this line or five below.
            "function" if !member => {
                if let Some((bound, close)) = self.binders_after(self.at) {
                    self.owe(bound);
                    self.skip_past(close);
                }
            }
            // A loop or a catch binds only names its head spells out, and
            // every one of those is taken, so the body it opens is read
            // like any other. It may have no brace at all, so the scope
            // standing here is told as well.
            "for" | "catch" if !member => {
                if let Some((bound, close)) = self.binders_after(self.at) {
                    for name in &bound.names {
                        self.uncertain(name);
                    }
                    self.owed.extend(bound.names);
                    self.skip_past(close);
                }
            }
            // QML gives a property its value with `:`, not `=`.
            "property" if !member => self.a_property(),
            _ => match asked {
                Some(asked) => self.asked(asked),
                None => self.a_name(word, from, member),
            },
        }
    }

    /// A bare name: written to, bound by an arrow, or just read.
    fn a_name(&mut self, word: &str, from: usize, member: bool) {
        let Some((at, c)) = self.non_space(self.at) else {
            return;
        };
        if c == '=' && self.next_is(at, '>') {
            // `oid => …`: the one-parameter arrow, whose body may be an
            // expression with no brace of its own.
            self.uncertain(word);
            self.owed.push(word.to_string());
            return;
        }
        if c != '=' || matches!(self.peek(at + 1), Some('=')) || member {
            return;
        }
        let declares = DECLARES.contains(&self.word_behind(from).as_str());
        let value = self.value_from(at + 1).to_string();
        self.give(word.to_string(), declares, at_a_number(&value));
    }

    /// `property <kind> <name>: <value>`, read only from the keyword so
    /// that a ternary's `:` and an object's are left alone.
    fn a_property(&mut self) {
        let Some((_, after_kind)) = self.word_at(self.at) else {
            return;
        };
        let Some((from, after_name)) = self.word_at(after_kind) else {
            return;
        };
        let name = self.slice(from, after_name);
        let numbered = match self.non_space(after_name) {
            Some((at, ':')) => at_a_number(self.value_from(at + 1)),
            // A property with no value of its own holds whatever QML
            // gives it, which is not a row read by number.
            _ => false,
        };
        self.give(name.to_string(), true, numbered);
    }

    /// The working tree's question, put to whatever the call is handed.
    fn asked(&mut self, word: &'static str) {
        let Some((open, _)) = self.non_space(self.at) else {
            return;
        };
        let inside = self.argument(open);
        if at_a_number(inside) || self.stands_for_a_numbered_row(inside) {
            self.found.push((self.line, word));
        }
    }

    /// Whether `(` at `open` closes on an `=>`, so the names inside it
    /// are that arrow's parameters. Answers where it closed, for the walk
    /// to step over: a list is not a body, and a pattern's braces are not
    /// blocks.
    fn an_arrow_at(&mut self, open: usize) -> Option<usize> {
        let close = self.closing(open)?;
        if !matches!(self.non_space(close + 1), Some((at, '=')) if self.next_is(at, '>')) {
            return None;
        }
        let bound = names_in(self.slice(open + 1, close));
        for name in &bound.names {
            self.uncertain(name);
        }
        self.owe(bound);
        Some(close)
    }

    /// Steps the walk past `close`, counting the lines it goes over.
    fn skip_past(&mut self, close: usize) {
        while self.at <= close {
            if self.peek(self.at) == Some('\n') {
                self.line += 1;
            }
            self.at += 1;
        }
    }

    /// Puts `name` in the scope the walk is standing in, or writes to the
    /// innermost one that already has it.
    fn give(&mut self, name: String, declares: bool, numbered: bool) {
        if declares {
            if let Some(scope) = self.scopes.last_mut() {
                match scope.names.get_mut(&name) {
                    // One scope, two declarations of a word: which one a
                    // reader means is not this rule's to say.
                    Some(knows) => knows.uncertain = true,
                    None => {
                        scope.names.insert(
                            name,
                            Knows {
                                numbered,
                                uncertain: false,
                            },
                        );
                    }
                }
            }
            return;
        }
        for scope in self.scopes.iter_mut().rev() {
            if let Some(knows) = scope.names.get_mut(&name) {
                knows.numbered |= numbered;
                return;
            }
            // A write cannot reach past a list this could not read
            // either: the name it means may well be one of that list's.
            if scope.unreadable {
                break;
            }
        }
        if let Some(scope) = self.scopes.last_mut() {
            scope.names.insert(
                name,
                Knows {
                    numbered,
                    uncertain: false,
                },
            );
        }
    }

    /// Marks a name as bound by something this cannot read, in the scope
    /// standing here.
    fn uncertain(&mut self, name: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.names.entry(name.to_string()).or_default().uncertain = true;
        }
    }

    /// Hands a binder list to the scope the next `{` opens, carrying
    /// whether it could be read whole.
    fn owe(&mut self, bound: Binders) {
        self.owed.extend(bound.names);
        self.owed_unreadable |= !bound.readable;
    }

    /// Whether the name stands for a row read at a numbered position —
    /// the innermost binding of it answers, and an uncertain one is no
    /// answer.
    fn stands_for_a_numbered_row(&self, word: &str) -> bool {
        for scope in self.scopes.iter().rev() {
            if let Some(knows) = scope.names.get(word) {
                return knows.numbered && !knows.uncertain;
            }
            // **The walk stops at a list it could not read.** What such a
            // list bound is exactly what could not be read, so a name it
            // does not plainly hold is not the outer one either.
            if scope.unreadable {
                return false;
            }
        }
        false
    }

    // Reading, none of it moving the walk.

    fn peek(&self, at: usize) -> Option<char> {
        self.chars.get(at).map(|(_, c)| *c)
    }

    fn next_is(&self, at: usize, c: char) -> bool {
        self.peek(at + 1) == Some(c)
    }

    fn behind(&self, at: usize) -> Option<char> {
        at.checked_sub(1).and_then(|before| self.peek(before))
    }

    fn byte(&self, at: usize) -> usize {
        self.chars.get(at).map_or(self.code.len(), |(b, _)| *b)
    }

    fn slice(&self, from: usize, to: usize) -> &'a str {
        self.code.get(self.byte(from)..self.byte(to)).unwrap_or("")
    }

    /// The first character from `at` that is not a space, with where it
    /// is. Newlines count as space: a call's bracket may be on the line
    /// below.
    fn non_space(&self, mut at: usize) -> Option<(usize, char)> {
        while let Some(c) = self.peek(at) {
            if !c.is_whitespace() {
                return Some((at, c));
            }
            at += 1;
        }
        None
    }

    /// The identifier that starts at or after `at`, as (start, end).
    fn word_at(&self, at: usize) -> Option<(usize, usize)> {
        let (from, c) = self.non_space(at)?;
        if !starts_a_word(c) {
            return None;
        }
        let mut to = from;
        while self.peek(to).is_some_and(is_word) {
            to += 1;
        }
        Some((from, to))
    }

    /// The identifier written just before `at`, for telling a declaration
    /// from a write.
    fn word_behind(&self, at: usize) -> String {
        let mut end = at;
        while end > 0 && self.peek(end - 1).is_some_and(char::is_whitespace) {
            end -= 1;
        }
        let mut from = end;
        while from > 0 && self.peek(from - 1).is_some_and(is_word) {
            from -= 1;
        }
        self.slice(from, end).to_string()
    }

    /// Where the `(` at or after `at` closes.
    fn closing(&self, at: usize) -> Option<usize> {
        let (open, '(') = self.non_space(at)? else {
            return None;
        };
        let mut depth = 0usize;
        let mut walk = open;
        while let Some(c) = self.peek(walk) {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(walk);
                    }
                }
                _ => {}
            }
            walk += 1;
        }
        None
    }

    /// What the call opening at or after `at` is handed.
    fn argument(&self, at: usize) -> &'a str {
        match (self.non_space(at), self.closing(at)) {
            (Some((open, '(')), Some(close)) => self.slice(open + 1, close).trim(),
            _ => "",
        }
    }

    /// The plain names inside the bracket that opens at or after `at` —
    /// a parameter list, or a loop's head.
    fn binders_after(&self, at: usize) -> Option<(Binders, usize)> {
        // `function name(…)` puts a name between the two.
        let from = self.word_at(at).map_or(at, |(_, to)| to);
        let (open, '(') = self.non_space(from)? else {
            return None;
        };
        let close = self.closing(open)?;
        Some((names_in(self.slice(open + 1, close)), close))
    }

    /// What a name is given: from `at` to the end of the statement, or to
    /// the comma that starts the next name's. **Read to the end of the
    /// line instead and one name is given another's value**, which is how
    /// a reader of a selection's row came to look numbered.
    fn value_from(&self, from: usize) -> &'a str {
        let mut depth = 0usize;
        let mut at = from;
        while let Some(c) = self.peek(at) {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' if depth == 0 => break,
                ')' | ']' | '}' => depth -= 1,
                ';' | ',' if depth == 0 => break,
                '\n' if depth == 0 => {
                    let so_far = self.slice(from, at);
                    let so_far = so_far.trim();
                    if !so_far.is_empty() && !ends_owing_the_rest(so_far) {
                        break;
                    }
                }
                _ => {}
            }
            at += 1;
        }
        self.slice(from, at).trim()
    }
}

fn starts_a_word(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == '$'
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

/// The names a binder list holds, and whether every one of them was a
/// plain word.
struct Binders {
    names: Vec<String>,
    /// False where an entry had more shape than a word — a pattern, a
    /// default, a rest. **What such an entry binds cannot be read**, and
    /// the names taken out of it are only the words it mentions.
    readable: bool,
}

/// The names in a comma-separated binder list. **An entry this cannot
/// read whole gives up every word in it** rather than nothing: a pattern
/// binds one of them, and which one is what could not be read. Read per
/// entry rather than per word, so `oid=fallback` and `oid = fallback`
/// answer the same.
fn names_in(list: &str) -> Binders {
    let mut names = Vec::new();
    let mut readable = true;
    for entry in list.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        if entry.starts_with(starts_a_word) && entry.chars().all(is_word) {
            names.push(entry.to_string());
            continue;
        }
        readable = false;
        names.extend(words_in(entry));
    }
    Binders { names, readable }
}

/// Every word in a piece this could not read whole.
fn words_in(piece: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut word = String::new();
    for c in piece.chars() {
        if word.is_empty() && starts_a_word(c) || !word.is_empty() && is_word(c) {
            word.push(c);
        } else if !word.is_empty() {
            out.push(std::mem::take(&mut word));
        }
    }
    if !word.is_empty() {
        out.push(word);
    }
    out
}

/// Whether a line cannot have ended the statement it is in: it stops on
/// an operator, so what it is about is on the line after it.
fn ends_owing_the_rest(line: &str) -> bool {
    const OWING: &[&str] = &["=", "&&", "||", "?", ":", ",", "+", "."];
    OWING.iter().any(|end| line.ends_with(end)) && !line.ends_with("==")
}

/// Whether an expression reads a row at a position spelled out as a
/// number — `graphModel.oidAt(0)`, and not `oidAt(row)`.
fn at_a_number(expression: &str) -> bool {
    let mut rest = expression;
    while let Some((before, after)) = rest.split_once(BY_INDEX) {
        rest = after;
        // `myOidAt(0)` is not the reader this names, and a space between
        // the name and its bracket changes nothing about what is read.
        if before.ends_with(is_word) {
            continue;
        }
        let Some(inside) = after.trim_start().strip_prefix('(') else {
            continue;
        };
        if up_to_the_closing_bracket(inside).parse::<i64>().is_ok() {
            return true;
        }
    }
    false
}

/// What a call is handed, read from just past its opening bracket to the
/// one that closes it.
fn up_to_the_closing_bracket(after: &str) -> &str {
    let mut depth = 0usize;
    for (at, c) in after.char_indices() {
        match c {
            '(' => depth += 1,
            ')' if depth == 0 => return after[..at].trim(),
            ')' => depth -= 1,
            _ => {}
        }
    }
    after.trim()
}

#[cfg(test)]
mod tests {
    use super::{findings, is_word, starts_a_word};

    /// The shape this exists to refuse, where it is read.
    const DIRECT: &str = "if (!GitFacts.wipOid(graphModel.oidAt(0)))\n";
    /// And through the name it was put in.
    const THROUGH_A_NAME: &str =
        "const oidHex = graphModel.oidAt(0)\nif (GitFacts.wipOid(oidHex))\n";
    /// A row somebody is on, which is nobody's business but the reader's.
    const SOMEBODY_IS_ON: &str = "const oidHex = graphModel.oidAt(page.selectedRow)\n\
                                  if (!GitFacts.wipOid(oidHex))\n";
    /// A whole body on its line, and the same body on five.
    const BODY_ON_ONE_LINE: &str = "function top() { const oid = graphModel.oidAt(0); return oid }\n\
                                    function isWip(oid) { return GitFacts.wipOid(oid) }\n";
    const BODY_ON_FIVE: &str = "function top() {\n\
                                \x20   const oid = graphModel.oidAt(0)\n\
                                \x20   return oid\n\
                                }\n\
                                function isWip(oid) {\n\
                                \x20   return GitFacts.wipOid(oid)\n\
                                }\n";
    /// The refused reading, laid out both ways.
    const REFUSED_ON_ONE_LINE: &str =
        "function only() { const oid = graphModel.oidAt(0); if (GitFacts.wipOid(oid)) return }\n";
    const REFUSED_ON_FIVE: &str = "function only() {\n\
                                   \x20   const oid = graphModel.oidAt(0)\n\
                                   \x20   if (GitFacts.wipOid(oid))\n\
                                   \x20       return\n\
                                   }\n";
    /// Two names given rows in one block, laid out both ways.
    const TWO_IN_A_BLOCK: &str = "if (deep) { const selected = graphModel.oidAt(page.selectedRow); \
                                  const top = graphModel.oidAt(0); \
                                  if (GitFacts.wipOid(selected)) return }\n";
    const TWO_IN_A_BLOCK_ON_FIVE: &str = "if (deep) {\n\
                                          \x20   const selected = graphModel.oidAt(page.selectedRow)\n\
                                          \x20   const top = graphModel.oidAt(0)\n\
                                          \x20   if (GitFacts.wipOid(selected))\n\
                                          \x20       return\n\
                                          }\n";
    /// A name given the numbered row on one path only.
    const ON_ONE_PATH: &str = "function only() {\n\
                               \x20   let oid = graphModel.oidAt(0)\n\
                               \x20   if (page.selectedRow >= 0) {\n\
                               \x20       oid = graphModel.oidAt(page.selectedRow)\n\
                               \x20   }\n\
                               \x20   if (GitFacts.wipOid(oid))\n\
                               \x20       return\n\
                               }\n";
    /// One word, two functions.
    const TWO_FUNCTIONS: &str = "function first() {\n\
                                 \x20   const oidHex = graphModel.oidAt(0)\n\
                                 \x20   if (GitFacts.wipOid(oidHex))\n\
                                 \x20       return\n\
                                 }\n\
                                 function second() {\n\
                                 \x20   const oidHex = graphModel.oidAt(page.selectedRow)\n\
                                 \x20   if (!GitFacts.wipOid(oidHex))\n\
                                 \x20       return\n\
                                 }\n";
    /// One word, a block that declares it again, and the body after it.
    const SHADOWED: &str = "function only() {\n\
                            \x20   const oidHex = graphModel.oidAt(0)\n\
                            \x20   if (oidHex !== \"\") {\n\
                            \x20       const oidHex = graphModel.oidAt(page.selectedRow)\n\
                            \x20       if (GitFacts.wipOid(oidHex))\n\
                            \x20           return\n\
                            \x20   }\n\
                            \x20   if (GitFacts.wipOid(oidHex))\n\
                            \x20       return\n\
                            }\n";
    /// An arrow's own word, with a body that is an expression.
    const ARROW: &str = "function only() {\n\
                         \x20   const oid = graphModel.oidAt(0)\n\
                         \x20   rows.forEach(oid => GitFacts.wipOid(oid))\n\
                         }\n";
    const ARROW_LIST: &str = "function only() {\n\
                              \x20   const oid = graphModel.oidAt(0)\n\
                              \x20   rows.forEach((oid, at) => GitFacts.wipOid(oid))\n\
                              }\n";
    /// A loop's own word.
    const LOOP: &str = "function only() {\n\
                        \x20   const oid = graphModel.oidAt(0)\n\
                        \x20   for (const oid of rows) {\n\
                        \x20       if (GitFacts.wipOid(oid))\n\
                        \x20           return\n\
                        \x20   }\n\
                        }\n";
    /// A parameter list with more shape than a word, and the body that
    /// declared the same name outside it.
    const PATTERN_PARAMETER: &str = "function outer() {\n\
                                     \x20   const oid = graphModel.oidAt(0)\n\
                                     \x20   function inner({oid}) { return GitFacts.wipOid(oid) }\n\
                                     }\n";
    const PATTERN_PARAMETER_AND_THE_WORD_OUTSIDE: &str = "function outer() {\n\
         \x20   const oid = graphModel.oidAt(0)\n\
         \x20   function inner({oid}) { return GitFacts.wipOid(oid) }\n\
         \x20   if (GitFacts.wipOid(oid))\n\
         \x20       return\n\
         }\n";
    /// A default, written tight and written apart.
    const DEFAULT_PARAMETER: &str = "function outer() {\n\
                                     \x20   const oid = graphModel.oidAt(0)\n\
                                     \x20   function inner(oid=fallback) { return GitFacts.wipOid(oid) }\n\
                                     }\n";
    /// A property, given its value with `:`.
    const PROPERTY: &str = "readonly property string topOid: graphModel.oidAt(0)\n\
                            if (GitFacts.wipOid(topOid))\n";
    /// A call and a value, each wrapped over two lines.
    const WRAPPED_CALL: &str = "if (!GitFacts.wipOid(\n        graphModel.oidAt(0)))\n    return\n";
    const WRAPPED_VALUE: &str =
        "const oidHex =\n    graphModel.oidAt(0)\nif (GitFacts.wipOid(oidHex))\n";

    /// Every shape above, for the properties that hold over all of them.
    const EVERY_SHAPE: &[&str] = &[
        DIRECT,
        THROUGH_A_NAME,
        SOMEBODY_IS_ON,
        BODY_ON_ONE_LINE,
        BODY_ON_FIVE,
        REFUSED_ON_ONE_LINE,
        REFUSED_ON_FIVE,
        TWO_IN_A_BLOCK,
        TWO_IN_A_BLOCK_ON_FIVE,
        ON_ONE_PATH,
        TWO_FUNCTIONS,
        SHADOWED,
        ARROW,
        ARROW_LIST,
        LOOP,
        PATTERN_PARAMETER,
        PATTERN_PARAMETER_AND_THE_WORD_OUTSIDE,
        DEFAULT_PARAMETER,
        PROPERTY,
        WRAPPED_CALL,
        WRAPPED_VALUE,
    ];

    /// The shape this exists to refuse, in both spellings: the question
    /// asked of a numbered row where it is read, and asked of the name it
    /// was put in.
    #[test]
    fn a_row_at_a_numbered_position_may_not_be_asked_the_working_tree_s_question() {
        assert_eq!(findings(DIRECT).len(), 1, "asked in one expression");
        assert_eq!(findings(THROUGH_A_NAME).len(), 1, "asked through a name");
    }

    /// The row a click landed on, a selection stood at or a reword names
    /// is a row somebody is on: what is asked there is what kind of row it
    /// is, and a copy's row answers that as truly as this window's —
    /// neither is a commit to select.
    #[test]
    fn a_row_somebody_is_on_is_left_alone() {
        assert!(findings("if (GitFacts.wipOid(oidHex))\n").is_empty());
        assert!(findings(SOMEBODY_IS_ON).is_empty());
        assert!(findings("if (GitFacts.wipOid(graphModel.oidAt(row)))\n").is_empty());
    }

    /// Comments spell the forbidden shape out while explaining it.
    #[test]
    fn a_comment_is_not_a_call() {
        assert!(findings("// never GitFacts.wipOid(graphModel.oidAt(0))\n").is_empty());
    }

    /// **A body written on its own line is a body.** Two functions, one
    /// holding a numbered row and the next taking a parameter of the same
    /// name, read the same however they are laid out — anything that
    /// waited for a line to end deeper than it began answers the second
    /// with the first's name.
    #[test]
    fn a_body_reads_the_same_on_one_line_as_on_five() {
        assert!(findings(BODY_ON_ONE_LINE).is_empty(), "one line");
        assert!(findings(BODY_ON_FIVE).is_empty(), "five lines");
    }

    /// And the reading that is refused is refused either way round.
    #[test]
    fn the_refused_reading_reads_the_same_on_one_line_as_on_five() {
        assert_eq!(
            findings(REFUSED_ON_ONE_LINE),
            vec![(1, "wipOid")],
            "one line"
        );
        assert_eq!(findings(REFUSED_ON_FIVE), vec![(3, "wipOid")], "five lines");
    }

    /// **Two names given rows in one block are two names**, whether the
    /// block is a line or five. Read to the end of the line, the first is
    /// handed the second's value and a sound reader is refused for it.
    #[test]
    fn two_names_in_a_block_read_the_same_on_one_line_as_on_five() {
        assert!(findings(TWO_IN_A_BLOCK).is_empty(), "one line");
        assert!(findings(TWO_IN_A_BLOCK_ON_FIVE).is_empty(), "five lines");
        let asked = TWO_IN_A_BLOCK.replace("wipOid(selected)", "wipOid(top)");
        assert_eq!(findings(&asked), vec![(1, "wipOid")], "the other name");
    }

    /// **A name given the numbered row on one path still holds it there.**
    /// The branch that leaves it alone is the branch the question is
    /// asked of it on, so an analysis that took the last write for the
    /// only one would call this sound.
    #[test]
    fn a_name_given_the_numbered_row_on_any_path_is_asked_about_it() {
        assert_eq!(findings(ON_ONE_PATH), vec![(6, "wipOid")]);
    }

    /// **The same word in two functions is two words**, and a block that
    /// declares it again is a third: the inner reader is asking about its
    /// own, and the one after that block about the outer.
    #[test]
    fn a_word_is_the_innermost_one_that_bound_it() {
        assert_eq!(findings(TWO_FUNCTIONS), vec![(3, "wipOid")]);
        assert_eq!(findings(SHADOWED), vec![(8, "wipOid")]);
    }

    /// Everything else that binds a name is read as binding it and
    /// nothing more: an arrow's parameter with no brace of its own, and a
    /// loop's variable.
    #[test]
    fn a_name_bound_by_what_this_cannot_read_is_never_refused() {
        assert!(findings(ARROW).is_empty(), "an arrow's own");
        assert!(findings(ARROW_LIST).is_empty(), "an arrow's list");
        assert!(findings(LOOP).is_empty(), "a loop's own");
    }

    /// **A parameter this cannot read whole is still a parameter.** The
    /// pattern binds a name the list does not spell plainly, so the
    /// reader inside the body is asking about that one — and answering it
    /// with the word standing outside the body refuses a sound reader.
    #[test]
    fn a_parameter_this_cannot_read_is_never_the_word_outside() {
        assert!(findings(PATTERN_PARAMETER).is_empty());
        assert_eq!(
            findings(PATTERN_PARAMETER_AND_THE_WORD_OUTSIDE),
            vec![(4, "wipOid")],
            "the body that declared it"
        );
    }

    /// And how a list is spaced is not what it binds: a default reads the
    /// same written tight or apart.
    #[test]
    fn a_default_parameter_reads_the_same_spaced_or_not() {
        let spaced = DEFAULT_PARAMETER.replace("oid=fallback", "oid = fallback");
        assert!(findings(DEFAULT_PARAMETER).is_empty(), "tight");
        assert!(findings(&spaced).is_empty(), "spaced");
    }

    /// A property is given its value with `:`, and the question asked of
    /// one that holds a numbered row is the same question.
    #[test]
    fn a_property_given_the_numbered_row_is_asked_about_it() {
        assert_eq!(findings(PROPERTY), vec![(2, "wipOid")]);
        let plain = "property string oidHex\nif (GitFacts.wipOid(oidHex))\n";
        assert!(findings(plain).is_empty(), "no value of its own");
    }

    /// **Neither half of the question has to stay on one line.** A call
    /// wrapped across two is the same call, reported where it opened, and
    /// a value wrapped after its `=` is the same value.
    #[test]
    fn a_call_wrapped_over_two_lines_is_the_same_call() {
        assert_eq!(findings(WRAPPED_CALL), vec![(1, "wipOid")]);
        assert_eq!(findings(WRAPPED_VALUE), vec![(3, "wipOid")]);
    }

    /// **Nothing this decides is decided by a space or a line break.**
    /// The rule is about which row a reader asks about, and no part of
    /// that is spelled by whitespace — yet every defect found in it so
    /// far was one of the two leaking into the answer (a list read per
    /// word instead of per entry, a body that opened no scope because its
    /// line ended where it began, a call whose bracket had to touch its
    /// name). So it is checked as a property over every shape above
    /// rather than one example at a time, which is what reading the code
    /// kept failing to catch.
    #[test]
    fn nothing_here_is_decided_by_a_space_or_a_line_break() {
        for source in EVERY_SHAPE {
            let plain = findings(source).len();
            assert_eq!(
                findings(&spaced_out(source)).len(),
                plain,
                "spaced out: {source}"
            );
            assert_eq!(
                findings(&on_one_line(source)).len(),
                plain,
                "on one line: {source}"
            );
            assert_eq!(
                findings(&spaced_out(&on_one_line(source))).len(),
                plain,
                "both: {source}"
            );
        }
    }

    /// The same source with a space either side of every bracket, comma
    /// and lone `=`. None of that changes what any of it binds or reads.
    fn spaced_out(code: &str) -> String {
        let chars: Vec<char> = code.chars().collect();
        let mut out = String::new();
        for (at, c) in chars.iter().enumerate() {
            let before = at.checked_sub(1).and_then(|before| chars.get(before));
            let after = chars.get(at + 1);
            // `==`, `!=`, `<=`, `>=` and `=>` are one token each.
            let lone = *c == '='
                && !matches!(before, Some('=' | '!' | '<' | '>'))
                && !matches!(after, Some('=' | '>'));
            if "(){}[],".contains(*c) || lone {
                out.push(' ');
                out.push(*c);
                out.push(' ');
            } else {
                out.push(*c);
            }
        }
        out
    }

    /// The same source with every line break taken out, a `;` standing in
    /// where one ended a statement.
    fn on_one_line(code: &str) -> String {
        let mut out = String::new();
        for line in code.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if !out.is_empty() {
                if !out.ends_with(['{', '}', ';', '(', '[', ','])
                    && !super::ends_owing_the_rest(&out)
                {
                    out.push(';');
                }
                out.push(' ');
            }
            out.push_str(line);
        }
        out.push('\n');
        out
    }

    /// What the two shapes above are read for: a word is a word however
    /// it is written, and the transforms must not invent or swallow one.
    #[test]
    fn the_shapes_this_is_read_in_keep_every_word() {
        for source in EVERY_SHAPE {
            let words = |text: &str| {
                let mut out: Vec<String> = Vec::new();
                let mut word = String::new();
                for c in text.chars() {
                    if word.is_empty() && starts_a_word(c) || !word.is_empty() && is_word(c) {
                        word.push(c);
                    } else if !word.is_empty() {
                        out.push(std::mem::take(&mut word));
                    }
                }
                if !word.is_empty() {
                    out.push(word);
                }
                out
            };
            assert_eq!(
                words(&spaced_out(source)),
                words(source),
                "spaced: {source}"
            );
            assert_eq!(
                words(&on_one_line(source)),
                words(source),
                "one line: {source}"
            );
        }
    }
}
