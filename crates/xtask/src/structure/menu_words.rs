//! The words a right-click menu's rows and reasons travel in, one list on
//! each side of the bridge: core names every row (`menu::Row::word`) and
//! every reason a row greys for (`menu::Why::word`); the product's QML asks
//! for rows by those words (`MenuLines.has` / `why` / `live`, and the
//! functions that hand their argument on to one of them) and says each
//! reason in one sentence (`Words.menuWhy`).
//!
//! A reason with no sentence leaves the row core greys saying nothing — it
//! greys by core's reason, whatever its words come to
//! (`AppMenuItem.ownBlocked`) —, a row asked for by a word core has none of
//! never stands, and a reason compared with a word core never says never
//! matches. So the lists are held equal: every reason has its sentence and
//! every sentence its reason, and every word the QML asks or compares by is
//! core's.
//!
//! A menu is drawn off core's answer (`MenuLines`): each row of the type is
//! declared once, as an `AppMenuItem` naming its kind (`word: "<row>"`),
//! each card as an `AppMenu` naming its card (`Card::word`). So a kind is
//! core's, every row and card core has has a part to draw it, and what the
//! answer decides stays the answer's: a row of the type writes none of
//! `offered`, `blockedReason` and `doable` (a grey with no reason the type
//! gave), a card no `applies`, and no divider is declared above a row or
//! card of the type (the answer parts them where the group changes).
//!
//! No row, of the type or not, writes `enabled`: the row keeps it for
//! itself, off while the row is off show (`AppMenuItem.doable`) — that is
//! what keeps the keys and Qt's pick of a card's first row off a row the
//! menu does not show, and a row's own binding would put it back.
//!
//! The drawer warns of a row it has no part for in the words
//! `verify::gaps` reads a gap by (`MARK`), so a run says every gap it met.
//!
//! Comments are taken out before anything is read; the words are read off
//! the literals the code still holds.

use std::path::Path;

/// Where core names its rows, its cards and its reasons, each in one
/// `fn word` of its own `impl`.
const ROWS: &str = "crates/platitude-core/src/menu/row.rs";
const WHYS: &str = "crates/platitude-core/src/menu/line.rs";
/// The types a row and a card of the type are declared as, what the
/// drawing writes on them, and a divider.
const ROW_TYPE: &str = "AppMenuItem";
const CARD_TYPE: &str = "AppMenu";
const DRAWN_ROW: [&str; 3] = ["offered", "blockedReason", "doable"];
const DRAWN_CARD: &str = "applies";
/// What every row keeps for itself, of the type or not.
const ROWS_OWN: &str = "enabled";
const DIVIDER: &str = "AppMenuSeparator";
/// The product's QML, the file that says each reason, and the drawer, which
/// warns of a row it has no part for in the words `verify::gaps` reads.
const PRODUCT: &str = "crates/platitude-app/src/ui";
const WORDS: &str = "words/Words.qml";
const DRAWER: &str = "MenuLines.qml";
/// The function there that does.
const SENTENCES: &str = "menuWhy";
/// `MenuLines`' own questions, each taking a row's word.
const ASKS: [&str; 3] = ["has", "why", "live"];

/// One failure per word out of step, and how many product files were read.
pub(super) fn check(root: &Path) -> Result<(Vec<String>, usize), String> {
    let rows = core_words(root, ROWS, "Row")?;
    let cards = core_words(root, ROWS, "Card")?;
    let whys = core_words(root, WHYS, "Why")?;
    let product = root.join(PRODUCT);
    let mut paths = Vec::new();
    super::collect(&product, &mut paths)?;
    paths.sort();
    paths.retain(|path| path.extension().is_some_and(|e| e == "qml"));
    let mut files = Vec::new();
    for path in &paths {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        files.push((super::relative(&product, path), Code::of(&text)));
    }
    let mut failures = drawn_out_of_step(&files, &rows, &cards);
    failures.extend(rows_writing_their_own(&files));
    let mark = crate::verify::gaps::MARK;
    if !files
        .iter()
        .any(|(shown, code)| shown == DRAWER && code.warnings().iter().any(|said| said == mark))
    {
        failures.push(format!(
            "{PRODUCT}/{DRAWER}: no `console.warn(\"{mark}\" …)` — `verify::gaps` reads a row the drawer \
             has no part for by those words, so a gap said in any other would pass unseen"
        ));
    }
    failures.extend(sentences_out_of_step(&files, &whys)?);
    for (shown, code) in &files {
        for (line, word) in code.rows_asked() {
            if !rows.contains(&word) {
                failures.push(format!(
                    "{PRODUCT}/{shown}:{line}: asks the menu for the row `{word}`, which core \
                     has none of ({ROWS} `Row::word`)"
                ));
            }
        }
        for (line, word) in code.reasons_compared() {
            if !whys.contains(&word) {
                failures.push(format!(
                    "{PRODUCT}/{shown}:{line}: compares a row's reason with `{word}`, which \
                     core never gives ({WHYS} `Why::word`)"
                ));
            }
        }
    }
    Ok((failures, files.len()))
}

/// Every reason core greys rows for against the sentences `Words.menuWhy`
/// says, both ways. No `Words.qml` read is a failure of its own: it would
/// hold no reason to a sentence at all.
fn sentences_out_of_step(files: &[(String, Code)], whys: &[String]) -> Result<Vec<String>, String> {
    let Some((_, code)) = files.iter().find(|(shown, _)| shown == WORDS) else {
        return Ok(vec![format!(
            "{PRODUCT}/{WORDS}: not read — the reasons core greys rows for are held to the \
             sentences `{SENTENCES}` says there, so none would be ({WHYS} `Why::word`)"
        )]);
    };
    let cases = code.cases_of(SENTENCES).ok_or_else(|| {
        format!("{PRODUCT}/{WORDS}: no `function {SENTENCES}` to read the sentences from")
    })?;
    let mut failures = Vec::new();
    for why in whys
        .iter()
        .filter(|why| !cases.iter().any(|(_, case)| case == *why))
    {
        failures.push(format!(
            "{PRODUCT}/{WORDS}: `{SENTENCES}` has no sentence for `{why}`, a reason core greys \
             rows for — a row greyed for it would say nothing of why ({WHYS} `Why::word`)"
        ));
    }
    for (line, case) in cases.iter().filter(|(_, case)| !whys.contains(case)) {
        failures.push(format!(
            "{PRODUCT}/{WORDS}:{line}: `{SENTENCES}` says `{case}`, a reason core never gives \
             ({WHYS} `Why::word`)"
        ));
    }
    Ok(failures)
}

/// What the drawn menus declare out of step with core, over every product
/// file: a kind core has none of, a row or card of core's with no part, a
/// row or card writing what the answer decides, a divider declared above
/// one of the type's (a part's own type counts: `BranchPushRow` is a row).
fn drawn_out_of_step(files: &[(String, Code)], rows: &[String], cards: &[String]) -> Vec<String> {
    let mut failures = Vec::new();
    let mut declared = Vec::new();
    let mut drawn_types = Vec::new();
    for (shown, code) in files {
        for kind in code.kinds() {
            let (known, what, writes) = if kind.card {
                (cards, "card", &[DRAWN_CARD][..])
            } else {
                (rows, "row", &DRAWN_ROW[..])
            };
            if !known.contains(&kind.word) {
                failures.push(format!(
                    "{PRODUCT}/{shown}:{}: names the {what} `{}`, which core has none of ({ROWS})",
                    kind.line, kind.word
                ));
            }
            for name in kind
                .bindings
                .iter()
                .filter(|name| writes.contains(&name.as_str()))
            {
                failures.push(format!(
                    "{PRODUCT}/{shown}:{}: the {what} `{}` writes `{name}`, which the answer decides \
                     (`MenuLines` draws it)",
                    kind.line, kind.word
                ));
            }
            if kind.root
                && let Some(stem) = shown
                    .rsplit('/')
                    .next()
                    .and_then(|name| name.strip_suffix(".qml"))
            {
                drawn_types.push(stem.to_string());
            }
            declared.push((kind.card, kind.word));
        }
    }
    for (shown, code) in files {
        for (line, sibling, drawn) in code.under_dividers() {
            if drawn || drawn_types.contains(&sibling) {
                failures.push(format!(
                    "{PRODUCT}/{shown}:{line}: a divider declared above `{sibling}`, which the type draws — \
                     the answer parts its rows where the group changes (`MenuLines`)"
                ));
            }
        }
    }
    for (card, words) in [(false, rows), (true, cards)] {
        for word in words
            .iter()
            .filter(|word| !declared.contains(&(card, (*word).clone())))
        {
            let (what, ty) = if card {
                ("card", CARD_TYPE)
            } else {
                ("row", ROW_TYPE)
            };
            failures.push(format!(
                "{ROWS}: core's {what} `{word}` has no part in {PRODUCT} — no menu could draw it \
                 (an `{ty}` naming `word: \"{word}\"`)"
            ));
        }
    }
    failures
}

/// Every row over the product's files that writes what the row keeps for
/// itself: an `AppMenuItem`, or a part that is one (a file whose own root
/// is an `AppMenuItem`, `BranchPushRow`), wherever it is declared.
fn rows_writing_their_own(files: &[(String, Code)]) -> Vec<String> {
    let parts: Vec<String> = files
        .iter()
        .filter(|(_, code)| code.root_type().as_deref() == Some(ROW_TYPE))
        .filter_map(|(shown, _)| {
            let stem = shown.rsplit('/').next()?.strip_suffix(".qml")?;
            Some(stem.to_string())
        })
        .collect();
    let mut failures = Vec::new();
    for (shown, code) in files {
        let row = |ty: &str| ty == ROW_TYPE || parts.iter().any(|part| part == ty);
        for (line, ty) in code.objects_binding(ROWS_OWN, row) {
            failures.push(format!(
                "{PRODUCT}/{shown}:{line}: a row (`{ty}`) writes `{ROWS_OWN}`, which the row keeps for \
                 itself — off while it is off show; a row that cannot be done now says `doable: false` \
                 (`AppMenuItem.doable`)"
            ));
        }
    }
    failures
}

/// A row or a card of the type a file declares: its word, the line its
/// object opens on, the bindings it makes itself, and whether it is the
/// file's own root (the file is then a part that is the row or the card).
struct Kind {
    card: bool,
    word: String,
    line: usize,
    bindings: Vec<String>,
    root: bool,
}

/// The words of the `fn word` of `owner`'s `impl` in core's file `name`:
/// the literal each arm of its match answers (`Self::Opening =>
/// "opening",`), read line by line down to the brace that closes the
/// function at its own indent — Rust's `'static` is no quote, so the QML's
/// blanking is not used here. The `impl` is one at the file's own level
/// (`impl Row {` in column 0, not a macro's), the first that has the
/// function: a type may have several.
fn core_words(root: &Path, name: &str, owner: &str) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(root.join(name)).map_err(|e| format!("{name}: {e}"))?;
    let opens = format!("impl {owner} {{");
    let all: Vec<&str> = text.lines().collect();
    let mut from = 0;
    let head = loop {
        let start = all[from..]
            .iter()
            .position(|line| *line == opens)
            .map(|at| from + at)
            .ok_or_else(|| {
                format!("{name}: no `fn word(self)` in `impl {owner}` to read the words from")
            })?;
        let end = all[start..]
            .iter()
            .position(|line| *line == "}")
            .map_or(all.len(), |at| start + at);
        if let Some(at) = all[start..end]
            .iter()
            .position(|line| line.contains("fn word(self)"))
        {
            break start + at;
        }
        from = end;
    };
    let indent = all[head].len() - all[head].trim_start().len();
    let mut words = Vec::new();
    for line in all.into_iter().skip(head + 1) {
        let code = line.trim_start();
        if code == "}" && line.len() - code.len() == indent {
            break;
        }
        if code.starts_with("//") {
            continue;
        }
        if let Some(at) = line.find("=> \"") {
            let rest = &line[at + 4..];
            if let Some(end) = rest.find('"') {
                words.push(rest[..end].to_string());
            }
        }
    }
    if words.is_empty() {
        return Err(format!("{name}: `fn word` answers no literal"));
    }
    Ok(words)
}

/// A file's text twice over, position for position: as written, and with
/// every comment and every literal's contents blanked — what is searched
/// is the code, what is read out is the literal.
struct Code {
    text: Vec<char>,
    bare: Vec<char>,
}

impl Code {
    fn of(text: &str) -> Self {
        let blanked = super::without_comments_and_strings(text);
        Self {
            text: text.chars().collect(),
            bare: blanked.chars().collect(),
        }
    }

    fn find(&self, needle: &str) -> Option<usize> {
        self.find_from(needle, 0)
    }

    /// Where `needle` next stands in the code, from `from` on.
    fn find_from(&self, needle: &str, from: usize) -> Option<usize> {
        let needle: Vec<char> = needle.chars().collect();
        (from..self.bare.len().saturating_sub(needle.len() - 1))
            .find(|&at| self.bare[at..].starts_with(&needle))
    }

    /// The braces of the first body opened after `from`.
    fn body_after(&self, from: usize) -> Option<(usize, usize)> {
        let open = (from..self.bare.len()).find(|&at| self.bare[at] == '{')?;
        self.closing(open, '{', '}').map(|close| (open, close))
    }

    /// The bracket closing the one opened at `open`.
    fn closing(&self, open: usize, opens: char, closes: char) -> Option<usize> {
        let mut depth = 0usize;
        for (at, &c) in self.bare.iter().enumerate().skip(open) {
            if c == opens {
                depth += 1;
            } else if c == closes {
                depth -= 1;
                if depth == 0 {
                    return Some(at);
                }
            }
        }
        None
    }

    /// The literal that stands next after `from`, past spaces only: where
    /// it opens, and what it holds.
    fn literal_after(&self, from: usize) -> Option<(usize, String)> {
        let at = (from..self.bare.len()).find(|&at| !self.bare[at].is_whitespace())?;
        let quote = self.bare[at];
        if !matches!(quote, '"' | '\'') {
            return None;
        }
        let end = (at + 1..self.bare.len()).find(|&end| self.bare[end] == quote)?;
        Some((at, self.text[at + 1..end].iter().collect()))
    }

    fn line_of(&self, at: usize) -> usize {
        1 + self.bare[..at].iter().filter(|&&c| c == '\n').count()
    }

    /// The identifier that ends right before `at`.
    fn identifier_before(&self, at: usize) -> Option<String> {
        let end = at;
        let start = (0..end)
            .rev()
            .take_while(|&i| self.bare[i].is_alphanumeric() || self.bare[i] == '_')
            .last()?;
        Some(self.bare[start..end].iter().collect())
    }

    /// The literals of each `case` of the function `name`'s body, with the
    /// line each stands on; `None` where the file has no such function.
    fn cases_of(&self, name: &str) -> Option<Vec<(usize, String)>> {
        let start = self.find(&format!("function {name}("))?;
        let (open, close) = self.body_after(start)?;
        let mut cases = Vec::new();
        let mut at = open;
        while let Some(case) = self.find_from("case ", at).filter(|&case| case < close) {
            if let Some((quote, word)) = self.literal_after(case + 4) {
                cases.push((self.line_of(quote), word));
            }
            at = case + 4;
        }
        Some(cases)
    }

    /// The functions that take a row's word: `MenuLines`' own, and every
    /// function declared here that hands its one argument straight on to
    /// one of them (`deleteWhy(row)` → `lines.why(row)`).
    fn row_takers(&self) -> Vec<String> {
        let mut takers: Vec<String> = ASKS.iter().map(|ask| (*ask).to_string()).collect();
        let mut at = 0;
        while let Some(function) = self.find_from("function ", at) {
            at = function + 9;
            let Some(paren) = self.find_from("(", at) else {
                break;
            };
            let name: String = self.bare[at..paren]
                .iter()
                .collect::<String>()
                .trim()
                .to_string();
            let Some(close) = self.closing(paren, '(', ')') else {
                continue;
            };
            let param: String = self.bare[paren + 1..close]
                .iter()
                .collect::<String>()
                .trim()
                .to_string();
            if param.is_empty() || param.contains(',') || ASKS.contains(&name.as_str()) {
                continue;
            }
            let Some((open, end)) = self.body_after(close) else {
                continue;
            };
            let body: String = self.bare[open..end].iter().collect();
            if ASKS
                .iter()
                .any(|ask| body.contains(&format!(".{ask}({param})")))
            {
                takers.push(name);
            }
        }
        takers
    }

    /// Every literal handed to a function that takes a row's word, with its
    /// line — both sides of a `?:` among them.
    fn rows_asked(&self) -> Vec<(usize, String)> {
        let takers = self.row_takers();
        let mut asked = Vec::new();
        for (paren, _) in self.bare.iter().enumerate().filter(|(_, c)| **c == '(') {
            let Some(name) = self.identifier_before(paren) else {
                continue;
            };
            let start = paren - name.chars().count();
            // A declaration names its parameter, not a row.
            let declared =
                start >= 9 && self.bare[start - 9..start].iter().collect::<String>() == "function ";
            if declared || !takers.contains(&name) {
                continue;
            }
            let Some(close) = self.closing(paren, '(', ')') else {
                continue;
            };
            let mut at = paren + 1;
            while let Some(quote) = (at..close).find(|&i| matches!(self.bare[i], '"' | '\'')) {
                if let Some((_, word)) = self.literal_after(quote) {
                    asked.push((self.line_of(quote), word.clone()));
                    at = quote + word.chars().count() + 2;
                } else {
                    break;
                }
            }
        }
        asked
    }

    /// Every literal a row's reason is compared with (`lines.why(row) ===
    /// "held"`, `why !== "tag-unnamed"`), with its line; the empty one, a
    /// live row, is no reason.
    fn reasons_compared(&self) -> Vec<(usize, String)> {
        let mut compared = Vec::new();
        for op in ["===", "!=="] {
            let mut at = 0;
            while let Some(found) = self.find_from(op, at) {
                at = found + 3;
                if !self.reason_before(found) {
                    continue;
                }
                if let Some((quote, word)) = self.literal_after(at)
                    && !word.is_empty()
                {
                    compared.push((self.line_of(quote), word));
                }
            }
        }
        compared
    }

    /// Whether what stands left of `at` is a row's reason: a call of `why`
    /// (`….why(row)`), or a name that is `why` itself.
    fn reason_before(&self, at: usize) -> bool {
        let Some(end) = (0..at).rev().find(|&i| !self.bare[i].is_whitespace()) else {
            return false;
        };
        if self.bare[end] == ')' {
            let mut depth = 0usize;
            let Some(open) = (0..=end).rev().find(|&i| {
                match self.bare[i] {
                    ')' => depth += 1,
                    '(' => depth -= 1,
                    _ => {}
                }
                depth == 0
            }) else {
                return false;
            };
            return self.identifier_before(open).as_deref() == Some("why");
        }
        self.identifier_before(end + 1).as_deref() == Some("why")
    }

    /// The type an object's open brace at `open` declares (`AppMenuItem {`);
    /// `None` for a brace that opens no object (a function's body, `({`).
    fn type_before(&self, open: usize) -> Option<String> {
        let end = (0..open).rev().find(|&i| !self.bare[i].is_whitespace())? + 1;
        self.identifier_before(end)
            .filter(|name| name.starts_with(|c: char| c.is_ascii_uppercase()))
    }

    /// The literal opening each `console.warn(…)` in the code.
    fn warnings(&self) -> Vec<String> {
        let call = "console.warn(";
        let mut said = Vec::new();
        let mut at = 0;
        while let Some(found) = self.find_from(call, at) {
            at = found + call.len();
            if let Some((_, words)) = self.literal_after(at) {
                said.push(words);
            }
        }
        said
    }

    /// The type the file's own root object declares.
    fn root_type(&self) -> Option<String> {
        let first = (0..self.bare.len()).find(|&i| self.bare[i] == '{')?;
        self.type_before(first)
    }

    /// Every object of a type `wanted` takes that binds `name` itself: the
    /// line of the binding, and the object's type.
    fn objects_binding(&self, name: &str, wanted: impl Fn(&str) -> bool) -> Vec<(usize, String)> {
        let mut found = Vec::new();
        for open in (0..self.bare.len()).filter(|&i| self.bare[i] == '{') {
            let Some(ty) = self.type_before(open).filter(|ty| wanted(ty)) else {
                continue;
            };
            let Some(close) = self.closing(open, '{', '}') else {
                continue;
            };
            for (_, value) in self
                .bindings(open, close)
                .into_iter()
                .filter(|(bound, _)| bound == name)
            {
                found.push((self.line_of(value), ty.clone()));
            }
        }
        found
    }

    /// The bindings the object braced `open`..`close` makes itself, not its
    /// children: each name opening a line at the body's own depth that a
    /// `:` follows, with where its value starts. A declaration
    /// (`property string word: ""`) opens with another word, so it is none.
    fn bindings(&self, open: usize, close: usize) -> Vec<(String, usize)> {
        let word = |c: char| c.is_alphanumeric() || c == '_';
        let mut found = Vec::new();
        let mut depth = 0usize;
        let mut line_start = true;
        let mut at = open + 1;
        while at < close {
            let c = self.bare[at];
            if line_start && !c.is_whitespace() {
                line_start = false;
                if depth == 0 && (c.is_alphabetic() || c == '_') {
                    let end = (at..close).find(|&i| !word(self.bare[i])).unwrap_or(close);
                    let colon = (end..close).find(|&i| self.bare[i] != ' ');
                    if let Some(colon) = colon.filter(|&i| self.bare[i] == ':') {
                        found.push((self.bare[at..end].iter().collect(), colon + 1));
                    }
                    at = end;
                    continue;
                }
            }
            match c {
                '{' | '(' | '[' => depth += 1,
                '}' | ')' | ']' => depth = depth.saturating_sub(1),
                '\n' => line_start = true,
                _ => {}
            }
            at += 1;
        }
        found
    }

    /// The literal a binding of the object braced `open`..`close` holds:
    /// `Some("push")` for `word: "push"`, `None` where it makes no such
    /// binding or binds something else.
    fn literal_binding(&self, open: usize, close: usize, name: &str) -> Option<String> {
        let bindings = self.bindings(open, close);
        let (_, value) = bindings.iter().find(|(bound, _)| bound == name)?;
        self.literal_after(*value)
            .map(|(_, word)| word)
            .filter(|word| !word.is_empty())
    }

    /// Every row and card of the type the file declares.
    fn kinds(&self) -> Vec<Kind> {
        let first = (0..self.bare.len()).find(|&i| self.bare[i] == '{');
        let mut kinds = Vec::new();
        for open in (0..self.bare.len()).filter(|&i| self.bare[i] == '{') {
            let Some(ty) = self
                .type_before(open)
                .filter(|ty| ty == ROW_TYPE || ty == CARD_TYPE)
            else {
                continue;
            };
            let Some(close) = self.closing(open, '{', '}') else {
                continue;
            };
            let Some(word) = self.literal_binding(open, close, "word") else {
                continue;
            };
            kinds.push(Kind {
                card: ty == CARD_TYPE,
                word,
                line: self.line_of(open),
                bindings: self
                    .bindings(open, close)
                    .into_iter()
                    .map(|(name, _)| name)
                    .collect(),
                root: first == Some(open),
            });
        }
        kinds
    }

    /// Each object declared right under a divider (`AppMenuSeparator {}`):
    /// the divider's line, the object's type, and whether it names a kind of
    /// the type itself.
    fn under_dividers(&self) -> Vec<(usize, String, bool)> {
        let word = |c: char| c.is_alphanumeric() || c == '_';
        let past_space =
            |from: usize| (from..self.bare.len()).find(|&i| !self.bare[i].is_whitespace());
        let mut under = Vec::new();
        let mut at = 0;
        while let Some(found) = self.find_from(DIVIDER, at) {
            at = found + DIVIDER.len();
            if found > 0 && word(self.bare[found - 1]) {
                continue;
            }
            let Some(open) = past_space(at).filter(|&i| self.bare[i] == '{') else {
                continue;
            };
            let Some(next) = self
                .closing(open, '{', '}')
                .and_then(|close| past_space(close + 1))
            else {
                continue;
            };
            let end = (next..self.bare.len())
                .find(|&i| !word(self.bare[i]))
                .unwrap_or(self.bare.len());
            let Some(brace) = past_space(end).filter(|&i| self.bare[i] == '{') else {
                continue;
            };
            let Some(close) = self.closing(brace, '{', '}') else {
                continue;
            };
            let ty: String = self.bare[next..end].iter().collect();
            let names = (ty == ROW_TYPE || ty == CARD_TYPE)
                && self.literal_binding(brace, close, "word").is_some();
            under.push((self.line_of(found), ty, names));
        }
        under
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Code, WORDS, core_words, drawn_out_of_step, rows_writing_their_own, sentences_out_of_step,
    };

    /// Read off the first `impl` of the type at the file's own level that
    /// has `fn word` — not a macro's, nor another type's coming first.
    #[test]
    fn a_word_is_read_off_every_arm_of_fn_word() {
        let root = crate::yard::Yard::new("menu-words");
        let text = "macro_rules! whys {\n    () => {\n        impl Why {\n            pub const ALL: u8 = 0;\n        \
                    }\n    };\n}\n\nimpl Why {\n    pub fn other(self) -> u8 {\n        0\n    }\n}\n\nimpl Gone {\n    \
                    pub fn word(self) -> &'static str {\n        match self {\n            Self::A => \"gone\",\n        \
                    }\n    }\n}\n\nimpl Why {\n    /// The word.\n    pub fn word(self) -> &'static str {\n        \
                    match self {\n            Self::Opening => \"opening\",\n            // Self::Gone => \
                    \"gone\",\n            Self::KeptBy(HeldBy::Rebase) => \"rebasing\",\n        }\n    \
                    }\n\n    pub fn other(self) -> &'static str {\n        \"not-a-word\"\n    }\n}\n";
        std::fs::write(root.join("line.rs"), text).expect("the file written");
        let words = core_words(&root, "line.rs", "Why");
        assert_eq!(
            words,
            Ok(vec!["opening".to_string(), "rebasing".to_string()])
        );
    }

    fn drawn(files: &[(&str, &str)], rows: &[&str], cards: &[&str]) -> Vec<String> {
        let files: Vec<(String, Code)> = files
            .iter()
            .map(|(name, text)| ((*name).to_string(), Code::of(text)))
            .collect();
        let words = |words: &[&str]| {
            words
                .iter()
                .map(|word| (*word).to_string())
                .collect::<Vec<_>>()
        };
        drawn_out_of_step(&files, &words(rows), &words(cards))
    }

    /// A row of the type is drawn off the answer: it writes nothing the
    /// answer decides, and no divider is declared above it — one above what
    /// stands outside the type is the menu's own.
    #[test]
    fn a_drawn_row_leaves_what_the_answer_decides_to_it() {
        let menu = "AppMenu {\n    AppMenuItem {\n        word: \"push\"\n        offered: true\n        \
                    doable: false\n        onTriggered: push()\n    }\n    AppMenuSeparator {}\n    AppMenuItem {\n        \
                    word: \"pull\"\n    }\n    AppMenuSeparator {}\n    GitHubMenu {}\n    AppMenu {\n        \
                    word: \"tag\"\n        applies: false\n    }\n}\n";
        assert_eq!(
            drawn(&[("Menu.qml", menu)], &["push", "pull"], &["tag"]),
            [
                "crates/platitude-app/src/ui/Menu.qml:2: the row `push` writes `offered`, which the answer \
                 decides (`MenuLines` draws it)",
                "crates/platitude-app/src/ui/Menu.qml:2: the row `push` writes `doable`, which the answer \
                 decides (`MenuLines` draws it)",
                "crates/platitude-app/src/ui/Menu.qml:14: the card `tag` writes `applies`, which the answer \
                 decides (`MenuLines` draws it)",
                "crates/platitude-app/src/ui/Menu.qml:8: a divider declared above `AppMenuItem`, which the type \
                 draws — the answer parts its rows where the group changes (`MenuLines`)",
            ]
        );
    }

    /// A part that is a row of the type (`BranchPushRow`) is one where it is
    /// declared; a word on anything but a row or a card (a chip's) is no
    /// kind; and every row core has needs a part, a kind core lacks none.
    #[test]
    fn every_row_has_a_part_and_every_part_a_row() {
        let part = "import QtQuick\n\n// A row.\nAppMenuItem {\n    word: \"push\"\n}\n";
        let menu = "AppMenu {\n    AppMenuSeparator {}\n    BranchPushRow {}\n    CodeChip {\n        word: \"edit\"\n    \
                    }\n    AppMenuItem {\n        word: \"pusj\"\n    }\n}\n";
        assert_eq!(
            drawn(
                &[("BranchPushRow.qml", part), ("Menu.qml", menu)],
                &["push", "pull"],
                &[]
            ),
            [
                "crates/platitude-app/src/ui/Menu.qml:7: names the row `pusj`, which core has none of \
                 (crates/platitude-core/src/menu/row.rs)",
                "crates/platitude-app/src/ui/Menu.qml:2: a divider declared above `BranchPushRow`, which the type \
                 draws — the answer parts its rows where the group changes (`MenuLines`)",
                "crates/platitude-core/src/menu/row.rs: core's row `pull` has no part in crates/platitude-app/src/ui \
                 — no menu could draw it (an `AppMenuItem` naming `word: \"pull\"`)",
            ]
        );
    }

    /// A row keeps `enabled` for itself: an `AppMenuItem` or a part that is
    /// one writing it is out of step, of the type or not; what stands inside
    /// a row (its press handler) is no row, and `doable` is the row's to say.
    #[test]
    fn no_row_writes_what_the_row_keeps() {
        let part = "import QtQuick\n\n// A row.\nAppMenuItem {\n    word: \"push\"\n    MouseArea {\n        \
                    enabled: false\n    }\n}\n";
        let menu = "AppMenu {\n    BranchPushRow {\n        enabled: false\n    }\n    AppMenuItem {\n        \
                    text: \"Copy\"\n        enabled: menu.can\n    }\n    AppMenuItem {\n        doable: menu.can\n    \
                    }\n    Button {\n        enabled: false\n    }\n}\n";
        let files: Vec<(String, Code)> = [("BranchPushRow.qml", part), ("Menu.qml", menu)]
            .iter()
            .map(|(name, text)| ((*name).to_string(), Code::of(text)))
            .collect();
        assert_eq!(
            rows_writing_their_own(&files),
            [
                "crates/platitude-app/src/ui/Menu.qml:3: a row (`BranchPushRow`) writes `enabled`, which the row \
                 keeps for itself — off while it is off show; a row that cannot be done now says `doable: false` \
                 (`AppMenuItem.doable`)",
                "crates/platitude-app/src/ui/Menu.qml:7: a row (`AppMenuItem`) writes `enabled`, which the row \
                 keeps for itself — off while it is off show; a row that cannot be done now says `doable: false` \
                 (`AppMenuItem.doable`)",
            ]
        );
    }

    #[test]
    fn a_warning_is_read_off_the_literal_it_opens_with() {
        let text = "Item {\n    function draw() {\n        // console.warn(\"commented\")\n        \
                    console.warn(\"MenuLines: a row with no part: \" + missed.join(\", \"))\n        \
                    console.warn(missed)\n    }\n}\n";
        assert_eq!(
            Code::of(text).warnings(),
            ["MenuLines: a row with no part: ".to_string()]
        );
    }

    #[test]
    fn a_case_is_read_off_the_function_it_stands_in() {
        let text = "QtObject {\n    function menuWhy(why, names) {\n        switch (why) {\n        \
                    case \"opening\": return qsTr(\"Wait\")\n        // case \"gone\": no\n        \
                    case \"held\": return x\n        default: return \"\"\n        }\n    }\n    \
                    function other(kind) {\n        switch (kind) { case \"far\": return 1 }\n    }\n}\n";
        let cases = Code::of(text)
            .cases_of("menuWhy")
            .expect("the function is there");
        assert_eq!(cases, [(4, "opening".to_string()), (6, "held".to_string())]);
    }

    /// The sentences are held both ways, and a tree where `Words.qml` is not
    /// read holds none — which is a failure, not a pass.
    #[test]
    fn every_reason_is_held_to_a_sentence_and_no_words_file_is_red() {
        let words = "QtObject {\n    function menuWhy(why, names) {\n        switch (why) {\n        \
                     case \"opening\": return qsTr(\"Wait\")\n        case \"gone\": return x\n        \
                     }\n    }\n}\n";
        let whys = ["opening".to_string(), "held".to_string()];
        let files = [(WORDS.to_string(), Code::of(words))];
        let failures = sentences_out_of_step(&files, &whys).expect("the function is there");
        assert_eq!(failures.len(), 2, "{failures:?}");
        assert!(
            failures[0].contains("no sentence for `held`"),
            "{failures:?}"
        );
        assert!(failures[1].contains("says `gone`"), "{failures:?}");

        let elsewhere = [("Other.qml".to_string(), Code::of(words))];
        let failures = sentences_out_of_step(&elsewhere, &whys).expect("nothing to read");
        assert_eq!(failures.len(), 1, "{failures:?}");
        assert!(failures[0].contains("not read"), "{failures:?}");
    }

    #[test]
    fn a_row_is_read_off_every_call_that_takes_one() {
        let text = "Item {\n    offered: lines.has(\"merge\")\n    \
                    blockedReason: card.deleteWhy(branch ? \"delete-branch\" : \"push\")\n    \
                    function deleteWhy(row) {\n        return Words.menuWhy(lines.why(row))\n    }\n    \
                    function pick(row) { return list.at(row) }\n    \
                    text: other.pick(\"not-a-row\")\n    // lines.has(\"commented\")\n}\n";
        let asked = Code::of(text).rows_asked();
        assert_eq!(
            asked,
            [
                (2, "merge".to_string()),
                (3, "delete-branch".to_string()),
                (3, "push".to_string()),
            ]
        );
    }

    #[test]
    fn a_reason_is_read_off_what_a_why_is_compared_with() {
        let text = "Item {\n    a: lines.why(\"pull\") === \"held\" ? x : \"\"\n    \
                    b: why !== \"tag-unnamed\"\n    c: anywhy === \"not-a-reason\"\n    \
                    d: lines.why(\"pull\") === \"\"\n    e: lines.has(\"pull\") === \"nope\"\n}\n";
        let compared = Code::of(text).reasons_compared();
        assert_eq!(
            compared,
            [(2, "held".to_string()), (3, "tag-unnamed".to_string())]
        );
    }
}
