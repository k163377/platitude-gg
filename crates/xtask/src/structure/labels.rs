//! Which label draws what, counted by machine (.claude/rules-refs/app-ui.md
//! 「入力は `PlainTextLabel`、固定値は素の `Label`」): the repository's
//! words — every string that crosses the bridge — are drawn by
//! `PlainTextLabel`, which pins plain text; a bare `Label` or `Text` draws
//! the app's words alone, and Qt's guess at a format (`Text.AutoText`)
//! never meets a name.
//!
//! Three shapes are counted, all in the product's own QML:
//!
//! * a bare `Label` / `Text` whose `text` is not made of the app's words —
//!   string literals, numbers, `Number(…)` (digits whatever it is handed),
//!   `qsTr(literal)` and its `.arg(…)`, `Words` and `Theme` and their
//!   functions, every call's arguments the app's words themselves (a
//!   sentence of the vocabulary's that embeds a bridge string, as
//!   `Words.lineEndings` does the file's extension, is input), joined by
//!   `+` or chosen by a ternary (whose condition may read anything);
//! * a `textFormat` written outside the files that draw markup whose
//!   pieces were escaped first (and the base itself): elsewhere the format
//!   is the base's to pin;
//! * input bound where no format can be named — `placeholderText`, which
//!   the style draws, and the `text` of the controls that leave their
//!   words to the style (`HoverToolButton`, `AppCheckBox`, a bare
//!   `ToolButton` or `CheckBox`). A box that shows input draws its own
//!   `placeholder` with a `PlainTextLabel`.
//!
//! Only a block's own body counts, a one-line `Label { text: … }`
//! included; comments and string literals are taken out before anything
//! is read (a literal's contents never matter: it is the app's words
//! whatever it says). Not read: a `text` reached through a `property
//! alias` and bound by the caller — the alias's own file shows no binding.

use std::path::Path;

/// The product's QML.
const PRODUCT: &str = "crates/platitude-app/src/ui";
/// The files that may write `textFormat:` — the base that pins plain text,
/// the parts that draw markup their callers escaped, and the owners of a
/// `LineRuler`, which read their rows' format.
const NAMING: &[&str] = &[
    "PlainTextLabel.qml",
    "CardText.qml",
    "LineText.qml",
    "AppMenuItem.qml",
    "DiffLineCell.qml",
    "DiffTextMetrics.qml",
    "DiffPane.qml",
    "CommandsPane.qml",
];
/// Qt's labels, which guess their format.
const BARE: &[&str] = &["Label", "Text"];
/// The controls whose words the style draws, so no format can be named.
const UNPINNABLE: &[&str] = &["HoverToolButton", "AppCheckBox", "ToolButton", "CheckBox"];
/// The property the style draws as a `Text` of its own.
const PLACEHOLDER: &str = "placeholderText";
/// Where a failing line sends its reader.
pub(crate) const RULE: &str =
    ".claude/rules-refs/app-ui.md「入力は `PlainTextLabel`、固定値は素の `Label`」";

pub(crate) struct Finding {
    pub(crate) line: usize,
    pub(crate) what: String,
}

/// One failure per line drawing input with a guessed format, and how many
/// product files were read.
pub(super) fn check(root: &Path) -> Result<(Vec<String>, usize), String> {
    let product = root.join(PRODUCT);
    let mut files = Vec::new();
    super::collect(&product, &mut files)?;
    files.sort();
    files.retain(|path| path.extension().is_some_and(|e| e == "qml"));
    let mut failures = Vec::new();
    for path in &files {
        let shown = super::relative(&product, path);
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        for finding in findings(&shown, &text) {
            failures.push(format!(
                "{PRODUCT}/{shown}:{}: {}",
                finding.line, finding.what
            ));
        }
    }
    Ok((failures, files.len()))
}

/// An object as its own body declares it.
struct Frame {
    name: String,
    line: usize,
    /// The brace depth of the lines directly inside it.
    inside: usize,
    text: Option<String>,
    format: bool,
}

/// Everything one file draws against the rule, in line order.
pub(crate) fn findings(file_name: &str, text: &str) -> Vec<Finding> {
    let code = super::without_comments_and_strings(text);
    let naming = NAMING.iter().any(|name| file_name.ends_with(name));
    let lines: Vec<&str> = code.lines().collect();
    let mut stack: Vec<Frame> = Vec::new();
    let mut found = Vec::new();
    let mut depth: usize = 0;
    for (index, line) in lines.iter().enumerate() {
        let number = index + 1;
        let before = depth;
        let opens = line.matches('{').count();
        let closes = line.matches('}').count();
        depth = (depth + opens).saturating_sub(closes);
        if let Some(name) = object_opener(line) {
            let mut frame = Frame {
                name,
                line: number,
                inside: before + 1,
                text: None,
                format: false,
            };
            // A body written on the opener's line (`Label { text: row.name }`).
            if let Some(open) = line.find('{') {
                let body = line[open + 1..]
                    .rsplit_once('}')
                    .map_or(&line[open + 1..], |(b, _)| b);
                for part in body.split(';') {
                    match property(part) {
                        Some((name, value)) if name == "text" => frame.text = Some(value),
                        Some((name, _)) if name == "textFormat" => frame.format = true,
                        _ => {}
                    }
                }
            }
            stack.push(frame);
        } else if let Some((name, _)) = property(line) {
            if name == "textFormat" && !naming {
                found.push(Finding {
                    line: number,
                    what: format!(
                        "names `textFormat` — the format is `PlainTextLabel`'s to pin; only the \
                         parts that draw markup their callers escaped name one ({RULE})"
                    ),
                });
            }
            if name == PLACEHOLDER {
                let expr = gather(&lines, index);
                if !app_words(&expr) {
                    found.push(Finding {
                        line: number,
                        what: format!(
                            "binds input to `{PLACEHOLDER}` (`{expr}`), which the style draws as a \
                             `Text` guessing its format — a box that shows input draws its own \
                             `placeholder` with a `PlainTextLabel` ({RULE})"
                        ),
                    });
                }
            }
            if let Some(frame) = stack.last_mut()
                && frame.inside == before
            {
                match name.as_str() {
                    "text" if frame.text.is_none() => frame.text = Some(gather(&lines, index)),
                    "textFormat" => frame.format = true,
                    _ => {}
                }
            }
        }
        while stack.last().is_some_and(|frame| depth < frame.inside) {
            let Some(frame) = stack.pop() else { break };
            found.extend(judge(frame));
        }
    }
    found.sort_by_key(|finding| finding.line);
    found
}

/// What a closed object draws against the rule.
fn judge(frame: Frame) -> Option<Finding> {
    let bare = BARE.contains(&frame.name.as_str());
    let unpinnable = UNPINNABLE.contains(&frame.name.as_str());
    if !(bare || unpinnable) || (bare && frame.format) {
        return None;
    }
    let expr = frame.text?;
    if app_words(&expr) {
        return None;
    }
    let what = if bare {
        format!(
            "a bare `{}` drawing input (`text: {expr}`) — the repository's words are drawn by \
             `PlainTextLabel`, which pins plain text; a bare label draws the app's words alone: \
             literals, `qsTr(literal)`, `Words` ({RULE})",
            frame.name
        )
    } else {
        format!(
            "binds input to the `text` of a `{}` (`{expr}`), whose words the style draws with a \
             format it guesses — give it the app's words, or draw the input with a \
             `PlainTextLabel` of its own ({RULE})",
            frame.name
        )
    };
    Some(Finding {
        line: frame.line,
        what,
    })
}

/// The type an object declaration on this line opens (`Label {`,
/// `contentItem: Label {`): the identifier before its first brace, when
/// it starts with a capital.
fn object_opener(line: &str) -> Option<String> {
    let at = line.find('{')?;
    let head = line[..at].trim_end();
    let start = head
        .rfind(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.'))
        .map_or(0, |i| i + 1);
    let name = &head[start..];
    let last = name.rsplit('.').next()?;
    last.chars()
        .next()
        .is_some_and(char::is_uppercase)
        .then(|| last.to_string())
}

/// A property binding on this line: its name and the first line of its
/// value. Not an object opener, a handler body or a declaration.
fn property(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim_start();
    let end = trimmed
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.'))
        .unwrap_or(trimmed.len());
    if end == 0 {
        return None;
    }
    let name = &trimmed[..end];
    let rest = trimmed[end..].trim_start();
    let value = rest.strip_prefix(':')?;
    if value.starts_with(':') || object_opener(line).is_some() {
        return None;
    }
    Some((name.to_string(), value.trim().to_string()))
}

/// The whole binding that starts on `lines[start]`, over the lines that
/// continue it — one that starts with an operator, or follows a line that
/// ends with one.
fn gather(lines: &[&str], start: usize) -> String {
    let Some((_, first)) = property(lines[start]) else {
        return String::new();
    };
    let mut expr = first;
    let mut prev = lines[start].to_string();
    for line in &lines[start + 1..] {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('}') || object_opener(line).is_some() {
            break;
        }
        let continues_prev = prev.trim_end().ends_with(['?', ':', '+', '(', ','])
            || prev.trim_end().ends_with("&&")
            || prev.trim_end().ends_with("||");
        let continues_here = trimmed.starts_with(['?', ':', '+', '.'])
            || trimmed.starts_with("&&")
            || trimmed.starts_with("||");
        if !continues_here && (!continues_prev || property(line).is_some()) {
            break;
        }
        expr.push(' ');
        expr.push_str(trimmed);
        prev = (*line).to_string();
    }
    expr
}

// ---- the app's words -------------------------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone)]
enum Token {
    Str,
    Num,
    Ident(String),
    Op(String),
}

/// The expression as tokens, or nothing where a character is not the
/// grammar's (read as input).
fn tokenize(expr: &str) -> Option<Vec<Token>> {
    let chars: Vec<char> = expr.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == '"' || c == '\'' || c == '`' {
            i += 1;
            while i < chars.len() && chars[i] != c {
                i += if chars[i] == '\\' { 2 } else { 1 };
            }
            i += 1;
            tokens.push(Token::Str);
        } else if c.is_ascii_digit() {
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            tokens.push(Token::Num);
        } else if c.is_alphabetic() || c == '_' || c == '$' {
            let start = i;
            while i < chars.len()
                && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '$')
            {
                i += 1;
            }
            tokens.push(Token::Ident(chars[start..i].iter().collect()));
        } else {
            let rest: String = chars[i..chars.len().min(i + 3)].iter().collect();
            let op = ["===", "!==", "==", "!=", "<=", ">=", "&&", "||"]
                .into_iter()
                .find(|op| rest.starts_with(op))
                .map(str::to_string)
                .or_else(|| "+-*/%<>!?:.(),[]".contains(c).then(|| c.to_string()))?;
            i += op.len();
            tokens.push(Token::Op(op));
        }
    }
    Some(tokens)
}

/// Whether an expression is made of the app's words alone.
fn app_words(expr: &str) -> bool {
    let Some(tokens) = tokenize(expr) else {
        return false;
    };
    if tokens.is_empty() {
        return false;
    }
    let mut parser = Parser { tokens, at: 0 };
    let words = parser.ternary();
    words && parser.at == parser.tokens.len()
}

/// Recursive descent over the JS subset bindings are written in; every
/// node answers "the app's words alone?".
struct Parser {
    tokens: Vec<Token>,
    at: usize,
}

#[derive(PartialEq, Eq)]
enum Head {
    Literal,
    QsTr,
    Words,
    Other,
}

const LEVELS: &[&[&str]] = &[
    &["||"],
    &["&&"],
    &["===", "!==", "==", "!="],
    &["<", ">", "<=", ">="],
    &["+", "-"],
    &["*", "/", "%"],
];

impl Parser {
    fn peek(&self, ahead: usize) -> Option<&Token> {
        self.tokens.get(self.at + ahead)
    }

    fn take(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.at).cloned();
        if token.is_some() {
            self.at += 1;
        }
        token
    }

    fn accept(&mut self, op: &str) -> bool {
        if self.peek(0) == Some(&Token::Op(op.to_string())) {
            self.at += 1;
            return true;
        }
        false
    }

    fn ternary(&mut self) -> bool {
        let left = self.binary(0);
        if self.accept("?") {
            let a = self.ternary();
            self.accept(":");
            let b = self.ternary();
            return a && b;
        }
        left
    }

    fn binary(&mut self, level: usize) -> bool {
        let Some(ops) = LEVELS.get(level) else {
            return self.unary();
        };
        let mut left = self.binary(level + 1);
        while let Some(Token::Op(op)) = self.peek(0).cloned()
            && ops.contains(&op.as_str())
        {
            self.at += 1;
            let right = self.binary(level + 1);
            left = op == "+" && left && right;
        }
        left
    }

    fn unary(&mut self) -> bool {
        if self.accept("!") || self.accept("-") {
            self.unary();
            return false;
        }
        self.postfix()
    }

    /// `( … )` once the `(` is taken: whether every argument is the
    /// app's words.
    fn args(&mut self) -> bool {
        let mut words = true;
        if self.accept(")") {
            return words;
        }
        loop {
            if self.peek(0).is_none() {
                return false;
            }
            words = self.ternary() && words;
            if self.accept(",") {
                continue;
            }
            self.accept(")");
            return words;
        }
    }

    fn postfix(&mut self) -> bool {
        let (mut words, head) = match self.peek(0).cloned() {
            Some(Token::Str | Token::Num) => {
                self.at += 1;
                (true, Head::Literal)
            }
            Some(Token::Ident(name))
                if name == "qsTr" && self.peek(1) == Some(&Token::Op("(".into())) =>
            {
                self.at += 2;
                let literal = self.peek(0) == Some(&Token::Str);
                let inner = self.args();
                (literal && inner, Head::QsTr)
            }
            // A number, whatever it is handed: digits are the app's words.
            Some(Token::Ident(name))
                if name == "Number" && self.peek(1) == Some(&Token::Op("(".into())) =>
            {
                self.at += 2;
                self.args();
                (true, Head::Literal)
            }
            // The vocabulary and the tokens: the app's own, and so is a sentence of theirs made of the
            // app's words alone.
            Some(Token::Ident(name)) if name == "Words" || name == "Theme" => {
                self.at += 1;
                (true, Head::Words)
            }
            Some(Token::Op(op)) if op == "(" => {
                self.at += 1;
                let inner = self.ternary();
                self.accept(")");
                (inner, Head::Literal)
            }
            Some(_) => {
                self.at += 1;
                (false, Head::Other)
            }
            None => return false,
        };
        loop {
            if self.accept(".") {
                let member = match self.take() {
                    Some(Token::Ident(name)) => name,
                    _ => return false,
                };
                if self.accept("(") {
                    let inner = self.args();
                    words = match head {
                        Head::QsTr if member == "arg" => words && inner,
                        Head::Words => words && inner,
                        _ => false,
                    };
                } else {
                    words = head == Head::Words && words;
                }
            } else if self.accept("[") {
                self.ternary();
                self.accept("]");
                words = false;
            } else if self.accept("(") {
                self.args();
                words = false;
            } else {
                return words;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{app_words, findings};

    #[test]
    fn the_apps_words_are_literals_qstr_and_words() {
        for expr in [
            "\"\"",
            "qsTr(\"Settings\")",
            "Words.appName",
            "Words.stampDay(Number(authorRow.details.authorTime))",
            "Words.loadFurther(Number(band.step))",
            "Words.modeLine(\"100644\", \"100755\")",
            "\"(\" + Number(toggle.count) + \")\"",
            "\"(\" + qsTr(\"x\") + \")\"",
            "glyph.mark === \"case\" ? \"Aa\" : \"W\"",
            "planPane.waiting ? qsTr(\"Rebasing…\") : qsTr(\"Rebase\")",
            "qsTr(\"%1 of %2\").arg(1).arg(\"two\")",
            "42",
            "Theme.textMuted",
        ] {
            assert!(app_words(expr), "{expr}");
        }
    }

    #[test]
    fn anything_read_off_a_property_is_input() {
        for expr in [
            "row.name",
            "\"(\" + toggle.count + \")\"",
            "qsTr(\"Waiting for %1\").arg(bucketHeader.worktree.mergeTool)",
            "stateGroup.stateWorktree !== null ? stateGroup.stateWorktree.opText : \"\"",
            "\"n\".repeat(Theme.nameMaxChars)",
            "planRow.oid_hex.substring(0, 8)",
            "row.host.words.ago(row.entry.at, row.now)",
            "Words.lineEndings(kind, from, to, lines, scope, notices.diffModel.endingExt)",
            "Words.stampDay(authorRow.details.authorTime)",
            "Words.loadFurther(band.step)",
            "String(AppBackend.gitConcurrencyDefault)",
            "qsTr(text)",
            "",
        ] {
            assert!(!app_words(expr), "{expr}");
        }
    }

    #[test]
    fn a_bare_label_drawing_input_is_found_and_a_pinned_one_is_not() {
        let qml = "Item {\n    Label {\n        text: row.name\n    }\n    PlainTextLabel {\n        \
                   text: row.name\n    }\n    Label {\n        text: qsTr(\"Settings\")\n    }\n}\n";
        let found = findings("Pane.qml", qml);
        assert_eq!(
            found.len(),
            1,
            "{:?}",
            found.iter().map(|f| &f.what).collect::<Vec<_>>()
        );
        assert_eq!(found[0].line, 2);
    }

    #[test]
    fn a_binding_over_several_lines_is_read_whole() {
        let qml = "Item {\n    Label {\n        text: bar.searching\n              ? qsTr(\"%1 / %2\").arg(bar.atMatch)\
                   \n              : \"\"\n        color: \"red\"\n    }\n}\n";
        assert_eq!(findings("Pane.qml", qml).len(), 1);
        let qml = "Item {\n    Label {\n        text: bar.searching\n              ? qsTr(\"Yes\")\n              : \
                   qsTr(\"No\")\n    }\n}\n";
        assert!(findings("Pane.qml", qml).is_empty());
    }

    #[test]
    fn a_format_is_named_only_where_markup_is_drawn() {
        let qml = "Item {\n    Label {\n        text: row.name\n        textFormat: Text.PlainText\n    }\n}\n";
        let found = findings("Pane.qml", qml);
        assert_eq!(found.len(), 1);
        assert!(
            found[0].what.contains("names `textFormat`"),
            "{}",
            found[0].what
        );
        assert!(findings("CardText.qml", qml).is_empty());
    }

    #[test]
    fn input_where_no_format_can_be_named_is_found() {
        let qml = "Item {\n    SlimField {\n        placeholderText: box.standing\n    }\n    HoverToolButton {\n        \
                   text: qsTr(\"Clear\")\n    }\n    AppCheckBox {\n        text: row.word\n    }\n}\n";
        let found = findings("Pane.qml", qml);
        let lines: Vec<usize> = found.iter().map(|f| f.line).collect();
        assert_eq!(
            lines,
            vec![3, 8],
            "{:?}",
            found.iter().map(|f| &f.what).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_literal_keeps_its_say_whatever_it_spells() {
        let qml = "Item {\n    Label {\n        text: \"<b>bold</b> {\"\n    }\n}\n";
        assert!(findings("Pane.qml", qml).is_empty());
    }

    #[test]
    fn a_bare_text_and_a_one_line_label_are_read_too() {
        let qml = "Item {\n    Text {\n        text: row.name\n    }\n    Label { id: x; text: row.name }\n    \
                   Label { text: qsTr(\"Filter\") }\n    ToolButton {\n        text: row.name\n    }\n}\n";
        let lines: Vec<usize> = findings("Pane.qml", qml).iter().map(|f| f.line).collect();
        assert_eq!(lines, vec![2, 5, 7]);
    }

    #[test]
    fn the_apps_words_pass_where_no_format_can_be_named() {
        let qml = "Item {\n    SlimField {\n        placeholderText: qsTr(\"Filter\")\n    }\n    HoverToolButton {\n   \
                   text: Words.loadFurther(Number(band.step))\n    }\n}\n";
        assert!(findings("Pane.qml", qml).is_empty());
    }

    #[test]
    fn an_arg_on_the_next_line_belongs_to_the_binding() {
        let qml = "Item {\n    Label {\n        text: qsTr(\"— %1 of %2\")\n              .arg(card.worktree.opStep)\n  \
                   .arg(card.worktree.opSteps)\n        color: \"red\"\n    }\n}\n";
        assert_eq!(findings("Pane.qml", qml).len(), 1);
        let qml = "Item {\n    Label {\n        text: qsTr(\"— %1 of %2\")\n              .arg(1)\n              .arg(2)\
                   \n    }\n}\n";
        assert!(findings("Pane.qml", qml).is_empty());
    }
}
