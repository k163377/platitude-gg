//! The value cells of internal-docs/デザイン規約.md, as quotations of the
//! two QML singletons that hold the values.
//!
//! `Theme.qml` and `Metrics.qml` are where a token's value lives, and the
//! comment saying why it is that value lives beside it. The document says
//! which token a place uses and why that one —
//! everything a reader needs the Japanese for — and it prints the values
//! so that a § can be read without opening the source.
//!
//! Printing them is the machine's part of that. A value written in
//! two places drifts on the edit that only
//! remembers one of them, and the drift reads as correct in both: the
//! document states a number, the source states a number, and nothing
//! about either says which one anything is drawn with. So the value cells
//! are generated. `cargo xtask docs --sync` writes them from the source;
//! `cargo xtask docs` fails when what stands in the document is not what
//! the source says.
//!
//! Nothing else in the document is touched. Which tokens a table lists,
//! the order of its rows, the 用途 column and every word of prose are the
//! writer's, and a machine with an opinion on those would be answering a
//! question nobody asked it. What this owns is one cell per row.

use std::collections::BTreeMap;
use std::path::Path;

/// The QML singletons that hold the values, in the order they have to be
/// read: `Metrics` defines values as `Theme`'s, so `Theme` must already
/// be known when a `Metrics` line naming one is resolved.
const SOURCES: [&str; 2] = [
    "crates/platitude-app/src/ui/Theme.qml",
    "crates/platitude-app/src/ui/Metrics.qml",
];

/// The document whose value cells quote them.
pub(crate) const DOCUMENT: &str = "internal-docs/デザイン規約.md";

/// One token, as the source declares it.
struct Token {
    /// The right-hand side, as written — joined onto one line when the
    /// declaration runs over several.
    rhs: String,
    /// The token this one is declared as, when the right-hand side is a
    /// reference to another and nothing else (`laneW: Theme.iconLg`).
    /// The document prints the derivation beside the number, so that a
    /// reader sees a lane is an icon square without opening the source.
    via: Option<String>,
    /// What it resolves to: the right-hand side itself, or the literal of
    /// whatever it referred to.
    literal: String,
}

impl Token {
    /// How the document prints it, or `None` for a value that is not a
    /// literal at all — the font families are picked from what the
    /// machine running the app has installed, so there is no number to
    /// quote and the document says so in prose instead.
    fn cell(&self) -> Option<String> {
        if let Some(via) = &self.via {
            return Some(format!("{}(={via})", self.literal));
        }
        if let Some(text) = self.literal.strip_prefix('"') {
            return Some(format!("`{}`", text.trim_end_matches('"')));
        }
        if self.literal.starts_with('[') {
            return Some(self.literal.clone());
        }
        if self.literal.starts_with(|c: char| c.is_ascii_digit()) {
            return Some(self.literal.clone());
        }
        None
    }
}

/// What one run found, so that the caller can say it the way its own
/// stage says things.
pub(crate) struct Quoted {
    /// The cells that do not say what the source says, as sentences.
    pub findings: Vec<String>,
    /// The document with every value cell as the source has it, present
    /// only when that differs from what is on disk.
    pub rewritten: Option<String>,
    /// How many cells were read, so a run that matched nothing at all is
    /// not reported as a pass.
    pub cells: usize,
}

/// Read the sources, and hold the document's value cells to them.
pub(crate) fn quote(root: &Path) -> Result<Quoted, String> {
    let mut tokens: BTreeMap<String, Token> = BTreeMap::new();
    for source in SOURCES {
        let path = root.join(source);
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{source}: {e}"))?;
        declared(&text, &mut tokens);
    }
    resolve(&mut tokens);

    let path = root.join(DOCUMENT);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{DOCUMENT}: {e}"))?;
    Ok(hold(&text, &tokens))
}

/// Every `readonly property` in one file, added to what is known.
///
/// A declaration whose right-hand side opens a bracket runs on until the
/// bracket closes (the lane colours take four lines, the font families
/// two), so the lines are joined before anything is read off them —
/// otherwise the array reads as unterminated and the value as missing.
fn declared(text: &str, out: &mut BTreeMap<String, Token>) {
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        let Some(rest) = line.trim_start().strip_prefix("readonly property ") else {
            continue;
        };
        let Some((_kind, rest)) = rest.split_once(' ') else {
            continue;
        };
        let Some((name, head)) = rest.split_once(": ") else {
            continue;
        };
        let mut rhs = head.trim().to_string();
        while depth(&rhs) > 0 {
            let Some(more) = lines.next() else { break };
            rhs.push(' ');
            rhs.push_str(more.trim());
        }
        out.insert(
            name.trim().to_string(),
            Token {
                literal: rhs.clone(),
                rhs,
                via: None,
            },
        );
    }
}

/// How many brackets a right-hand side has opened and not closed.
fn depth(rhs: &str) -> usize {
    let opened = rhs.matches('[').count();
    let closed = rhs.matches(']').count();
    opened.saturating_sub(closed)
}

/// Fill in what a reference resolves to, for the declarations that are
/// nothing but a reference to another token.
fn resolve(tokens: &mut BTreeMap<String, Token>) {
    let literals: BTreeMap<String, String> = tokens
        .iter()
        .map(|(name, token)| (name.clone(), token.rhs.clone()))
        .collect();
    for token in tokens.values_mut() {
        let referred = token.rhs.strip_prefix("Theme.").unwrap_or(&token.rhs);
        if referred.contains(|c: char| !c.is_alphanumeric() && c != '_') {
            continue;
        }
        if let Some(literal) = literals.get(referred) {
            token.via = Some(referred.to_string());
            token.literal = literal.clone();
        }
    }
}

/// Where a row writes the name of the token it is about.
enum Names {
    /// In a cell of its own. Several tokens that share a use may be
    /// folded into it with ` / `, and then the value cell folds their
    /// values the same way.
    Cell(usize),
    /// Inside the value cell, after the number (`` 640(`textWidth`) ``).
    /// The layout table is keyed by the measurement a screen opens at,
    /// because most of what it lists is the
    /// starting width of something a hand then drags; the rows that do
    /// name a token still quote it.
    AfterValue,
}

/// Which cell of a row names the token, and which cells hold its values.
struct Shape {
    /// Where the token's name stands.
    names: Names,
    /// The cell holding its value.
    value: usize,
    /// The cell holding the line height of the same token, for the one
    /// table that sets a type step's two numbers side by side. The token
    /// is the step's own name with `Line` on the end; a step that has no
    /// such token has no line height to print, and the document leaves
    /// the cell as it found it.
    line: Option<usize>,
}

/// The tables a value is quoted in, read off the header row so that a new
/// table joins by being written in one of the shapes already in use.
fn shape(cells: &[&str]) -> Option<Shape> {
    match cells {
        ["token" | "定数", "値", ..] => Some(Shape {
            names: Names::Cell(0),
            value: 1,
            line: None,
        }),
        ["token", "pixelSize", "lineHeight", ..] => Some(Shape {
            names: Names::Cell(0),
            value: 1,
            line: Some(2),
        }),
        ["画面", "token", "値", ..] => Some(Shape {
            names: Names::Cell(1),
            value: 2,
            line: None,
        }),
        ["値", ..] => Some(Shape {
            names: Names::AfterValue,
            value: 0,
            line: None,
        }),
        _ => None,
    }
}

/// Walk the document, holding every value cell of every token table to
/// what the source says.
fn hold(text: &str, tokens: &BTreeMap<String, Token>) -> Quoted {
    let mut findings = Vec::new();
    let mut out: Vec<String> = Vec::new();
    let mut cells = 0usize;
    let mut open: Option<Shape> = None;

    for (at, line) in text.lines().enumerate() {
        if !line.trim_start().starts_with('|') {
            open = None;
            out.push(line.to_string());
            continue;
        }
        let row = split(line);
        if open.is_none() {
            open = shape(&row.iter().map(|c| c.text.trim()).collect::<Vec<_>>());
            out.push(line.to_string());
            continue;
        }
        let Some(form) = &open else {
            out.push(line.to_string());
            continue;
        };
        if row.iter().all(|cell| is_rule(cell.text)) {
            out.push(line.to_string());
            continue;
        }
        let held = match &form.names {
            Names::Cell(at) => folded(row.get(*at).map(|cell| cell.text)),
            Names::AfterValue => {
                named_after(row.get(form.value).map(|cell| cell.text)).map(|name| vec![name])
            }
        };
        let Some(names) = held else {
            out.push(line.to_string());
            continue;
        };

        let mut edits: Vec<(usize, String)> = Vec::new();
        let mut wanted = |column: usize, suffix: &str, findings: &mut Vec<String>| {
            let mut parts = Vec::new();
            for name in &names {
                let looked = format!("{name}{suffix}");
                match tokens.get(&looked) {
                    Some(token) => match token.cell() {
                        Some(text) => parts.push(text),
                        None => return,
                    },
                    None if suffix.is_empty() => {
                        findings.push(format!(
                            "line {}: `{name}` is not declared in Theme.qml or Metrics.qml — the \
                             document quotes a token the source does not have",
                            at + 1
                        ));
                        return;
                    }
                    None => return,
                }
            }
            let mut text = parts.join(" / ");
            if matches!(form.names, Names::AfterValue) {
                text = format!("{text}(`{}`)", names[0]);
            }
            edits.push((column, text));
        };
        wanted(form.value, "", &mut findings);
        if let Some(line_at) = form.line {
            wanted(line_at, "Line", &mut findings);
        }

        let mut written = line.to_string();
        for (index, text) in edits.into_iter().rev() {
            let Some(cell) = row.get(index) else { continue };
            cells += 1;
            if cell.text.trim() == text {
                continue;
            }
            findings.push(format!(
                "line {}: the document quotes {} where the source says {} — `cargo xtask docs \
                 --sync` writes the source's value",
                at + 1,
                cell.text.trim(),
                text
            ));
            written.replace_range(cell.from..cell.to, &format!(" {text} "));
        }
        out.push(written);
    }

    let mut rebuilt = out.join("\n");
    if text.ends_with('\n') {
        rebuilt.push('\n');
    }
    Quoted {
        findings,
        rewritten: (rebuilt != text).then_some(rebuilt),
        cells,
    }
}

/// One cell of a table row, and where it sits in the line — the cell is
/// rewritten in place so that everything after it, `|` in a code span
/// included, is left exactly as the writer set it.
struct Cell<'a> {
    text: &'a str,
    from: usize,
    to: usize,
}

/// The cells of a row: what stands between its pipes, in order.
fn split(line: &str) -> Vec<Cell<'_>> {
    let bars: Vec<usize> = line.match_indices('|').map(|(at, _)| at).collect();
    bars.windows(2)
        .map(|pair| Cell {
            text: &line[pair[0] + 1..pair[1]],
            from: pair[0] + 1,
            to: pair[1],
        })
        .collect()
}

/// Whether a cell is one of the dashes under a header.
fn is_rule(text: &str) -> bool {
    let bare = text.trim();
    !bare.is_empty() && bare.chars().all(|c| c == '-' || c == ':')
}

/// The token names a cell holds, or `None` when it holds anything else.
/// A row may fold several tokens that share a use into one line, and then
/// the value cell folds their values the same way (`iconXs / iconSm / …`
/// against `10 / 12 / …`), so both sides are read as lists throughout.
fn folded(text: Option<&str>) -> Option<Vec<String>> {
    let bare = text?.trim();
    if bare.is_empty() {
        return None;
    }
    let mut names = Vec::new();
    for part in bare.split(" / ") {
        let name = part.trim().strip_prefix('`')?.strip_suffix('`')?;
        if name.is_empty() || name.contains(|c: char| !c.is_alphanumeric() && c != '_') {
            return None;
        }
        names.push(name.to_string());
    }
    Some(names)
}

/// The token a value cell names after its own number, for a row written
/// `` 640(`textWidth`) ``. A cell that names none — most of the layout
/// table, where the number is a pane's starting width and belongs to the
/// component that owns the pane — is left where it stands.
fn named_after(text: Option<&str>) -> Option<String> {
    let (_, rest) = text?.trim().split_once("(`")?;
    let name = rest.strip_suffix("`)")?;
    if name.is_empty() || name.contains(|c: char| !c.is_alphanumeric() && c != '_') {
        return None;
    }
    Some(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sources() -> BTreeMap<String, Token> {
        let mut tokens = BTreeMap::new();
        declared(
            "\
readonly property color bgBase: \"#020617\"
readonly property int iconLg: 20
readonly property int fontSm: 12
readonly property int fontSmLine: 16
readonly property int fontCode: 13
readonly property int laneW: Theme.iconLg
readonly property var laneDash: [1, 1]
readonly property string uiFamily: _pickFamily(fontFamilyUi)
",
            &mut tokens,
        );
        resolve(&mut tokens);
        tokens
    }

    fn run(doc: &str) -> Quoted {
        hold(doc, &sources())
    }

    #[test]
    fn a_colour_is_quoted_in_code_ticks() {
        let found = run("| token | 値 |\n|---|---|\n| `bgBase` | `#FFFFFF` |\n");
        assert_eq!(found.findings.len(), 1, "{:?}", found.findings);
        assert!(
            found
                .rewritten
                .unwrap()
                .contains("| `bgBase` | `#020617` |")
        );
    }

    #[test]
    fn a_cell_that_already_says_what_the_source_says_is_left_alone() {
        let found = run("| token | 値 |\n|---|---|\n| `bgBase` | `#020617` |\n");
        assert!(found.findings.is_empty(), "{:?}", found.findings);
        assert!(found.rewritten.is_none());
        assert_eq!(found.cells, 1);
    }

    #[test]
    fn a_value_declared_as_another_token_prints_the_derivation() {
        let found = run("| 定数 | 値 |\n|---|---|\n| `laneW` | 20 |\n");
        assert!(
            found
                .rewritten
                .unwrap()
                .contains("| `laneW` | 20(=iconLg) |")
        );
    }

    #[test]
    fn a_folded_row_is_read_as_a_list_on_both_sides() {
        let found = run("| token | 値 |\n|---|---|\n| `iconLg` / `fontSm` | 1 / 2 |\n");
        assert!(
            found
                .rewritten
                .unwrap()
                .contains("| `iconLg` / `fontSm` | 20 / 12 |")
        );
    }

    #[test]
    fn the_type_table_takes_its_line_height_from_the_step_s_own_token() {
        let found = run(
            "| token | pixelSize | lineHeight | 用途 |\n|---|---|---|---|\n| `fontSm` | 1 | 2 | 説明 |\n",
        );
        assert!(
            found
                .rewritten
                .unwrap()
                .contains("| `fontSm` | 12 | 16 | 説明 |")
        );
    }

    #[test]
    fn a_step_with_no_line_height_token_keeps_the_cell_it_had() {
        let found = run(
            "| token | pixelSize | lineHeight | 用途 |\n|---|---|---|---|\n| `fontCode` | 13 | — | 説明 |\n",
        );
        assert!(found.findings.is_empty(), "{:?}", found.findings);
        assert!(found.rewritten.is_none());
    }

    #[test]
    fn a_token_the_source_does_not_have_is_named() {
        let found = run("| token | 値 |\n|---|---|\n| `gone` | 1 |\n");
        assert_eq!(found.findings.len(), 1, "{:?}", found.findings);
        assert!(found.findings[0].contains("`gone` is not declared"));
        assert!(found.rewritten.is_none());
    }

    #[test]
    fn a_finding_carries_the_line_the_row_stands_on_not_the_column() {
        let found = run("\n\n\n| token | 値 | 用途 |\n|---|---|---|\n| `gone` | 1 | 説明 |\n");
        assert_eq!(found.findings.len(), 1, "{:?}", found.findings);
        assert!(
            found.findings[0].starts_with("line 6:"),
            "{:?}",
            found.findings
        );
    }

    #[test]
    fn the_layout_table_quotes_the_token_a_row_names_after_its_number() {
        let found = run("| 値 | 用途 |\n|---|---|\n| 9(`iconLg`) | 面の列 |\n");
        assert!(
            found
                .rewritten
                .unwrap()
                .contains("| 20(`iconLg`) | 面の列 |")
        );
    }

    #[test]
    fn a_layout_row_that_names_no_token_is_left_where_it_stands() {
        let doc = "| 値 | 用途 |\n|---|---|\n| 260(min 180) | サイドバー幅 |\n| 1440×900 | 初期ウィンドウ |\n";
        let found = run(doc);
        assert!(found.findings.is_empty(), "{:?}", found.findings);
        assert!(found.rewritten.is_none());
    }

    #[test]
    fn a_value_that_is_not_a_literal_is_left_to_the_prose() {
        let found = run("| token | 値 |\n|---|---|\n| `uiFamily` | OS ごと |\n");
        assert!(found.findings.is_empty(), "{:?}", found.findings);
        assert!(found.rewritten.is_none());
    }

    #[test]
    fn the_screen_column_table_finds_the_token_in_its_second_cell() {
        let found = run(
            "| 画面 | token | 値 | 用途 |\n|---|---|---|---|\n| 設定画面 | `iconLg` | 9 | 列 |\n",
        );
        assert!(
            found
                .rewritten
                .unwrap()
                .contains("| 設定画面 | `iconLg` | 20 | 列 |")
        );
    }

    #[test]
    fn a_table_that_is_not_a_token_table_is_not_touched() {
        let doc = "| 役割 | 段 |\n|---|---|\n| 読ませる本体 | `fontSm` |\n";
        let found = run(doc);
        assert!(found.findings.is_empty(), "{:?}", found.findings);
        assert!(found.rewritten.is_none());
    }

    #[test]
    fn a_row_whose_first_cell_is_prose_is_left_where_it_stands() {
        let found = run("| token | 値 |\n|---|---|\n| その diff の最大行番号 | 1 |\n");
        assert!(found.findings.is_empty(), "{:?}", found.findings);
        assert!(found.rewritten.is_none());
    }

    #[test]
    fn a_pipe_later_in_the_row_survives_the_rewrite() {
        let found = run("| token | 値 | 用途 |\n|---|---|---|\n| `bgBase` | `#FFF` | `a | b` |\n");
        assert!(found.rewritten.unwrap().contains("| `#020617` | `a | b` |"));
    }

    #[test]
    fn a_blank_line_closes_the_table_so_the_next_one_reads_its_own_header() {
        let doc = "| token | 値 |\n|---|---|\n| `bgBase` | `#020617` |\n\n| 役割 | 段 |\n|---|---|\n| `iconLg` | 9 |\n";
        let found = run(doc);
        assert!(found.findings.is_empty(), "{:?}", found.findings);
        assert!(found.rewritten.is_none());
    }

    #[test]
    fn an_array_is_quoted_as_the_source_writes_it() {
        let found = run("| 定数 | 値 |\n|---|---|\n| `laneDash` | 1 on / 1 off |\n");
        assert!(found.rewritten.unwrap().contains("| `laneDash` | [1, 1] |"));
    }

    #[test]
    fn a_declaration_that_runs_over_several_lines_is_read_whole() {
        let mut tokens = BTreeMap::new();
        declared(
            "readonly property var graphLane: [\n    \"#56B4E9\", \"#E69F00\",\n    \"#009E73\"\n]\nreadonly property int after: 3\n",
            &mut tokens,
        );
        assert_eq!(
            tokens["graphLane"].rhs,
            "[ \"#56B4E9\", \"#E69F00\", \"#009E73\" ]"
        );
        assert_eq!(tokens["after"].rhs, "3");
    }
}
