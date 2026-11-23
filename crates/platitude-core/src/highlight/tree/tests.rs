//! What each grammar paints, character by character: one sample per
//! language under `samples/`, read whole the way the diff pane reads a
//! file ([`line_spans`]), against its snapshot. The snapshot is the
//! expectation — a crate update or a query change that moves a colour
//! shows as a diff to review, where `grammar`'s own tests name only the
//! cases someone thought of.
//!
//! A sample is `<language>.<extension>.sample`: the language is the
//! entry's name (`grammar::ENTRIES`), and the name without `.sample` is
//! the path the product resolves. The suffix keeps the tools that read
//! sources by extension (Rust, QML, Markdown) off the samples.

use std::path::Path;

use tree_sitter_highlight::Highlighter;

use super::super::theme::{ANNOTATION, COMMENT, CONSTANT, FUNCTION, KEYWORD, STRING, TYPE};
use super::{Lang, Rgb, Span, grammar, line_spans, reads};

/// The letter under each character, by デザイン規約 §シンタックスハイライト's
/// rows; plain text gets none.
const ROLES: [(u32, char); 7] = [
    (COMMENT, 'c'),
    (KEYWORD, 'k'),
    (TYPE, 't'),
    (FUNCTION, 'f'),
    (STRING, 's'),
    (CONSTANT, 'n'),
    (ANNOTATION, 'a'),
];

const LEGEND: &str = "Each sample line (`|`) over the role of each of its characters: \
                      c comment, k keyword, t type, f function, s string, n constant, \
                      a annotation, blank plain.";

fn role(color: Rgb) -> char {
    if color == grammar::PLAIN {
        return ' ';
    }
    ROLES
        .iter()
        .find(|(rgb, _)| Rgb::of(*rgb) == color)
        .map(|(_, letter)| *letter)
        .unwrap_or_else(|| panic!("{color:?} is no colour of the palette"))
}

/// The sample with a line of role letters under each line that has any.
fn rendered(text: &str, rows: &[Vec<Span>]) -> String {
    let mut out = String::new();
    for (line, spans) in text.lines().zip(rows) {
        out.push_str(&format!("| {line}\n"));
        let marks: String = spans
            .iter()
            .flat_map(|span| std::iter::repeat_n(role(span.color), span.len))
            .collect();
        let marks = marks.trim_end();
        if !marks.is_empty() {
            out.push_str(&format!("  {marks}\n"));
        }
    }
    out
}

/// Every sample as (the path the product resolves, its grammar, its text).
fn samples() -> Vec<(String, &'static Lang, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/highlight/tree/samples");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("the samples are in the tree")
        .map(|entry| {
            entry
                .expect("a listing entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    assert!(!names.is_empty(), "no samples under {}", dir.display());
    names
        .into_iter()
        .map(|name| {
            let path = name
                .strip_suffix(".sample")
                .unwrap_or_else(|| panic!("{name} is not a sample"));
            let (language, _) = path.split_once('.').expect("a sample names its language");
            let lang =
                grammar::for_path(path).unwrap_or_else(|| panic!("no grammar claims {path}"));
            assert_eq!(
                lang.config.language_name, language,
                "{path} resolves to another grammar"
            );
            let text = std::fs::read_to_string(dir.join(&name)).expect("a sample reads as text");
            // One byte, one column: the letters stand under their characters.
            assert!(
                text.is_ascii() && !text.contains('\t'),
                "{path} holds a tab or non-ASCII"
            );
            (path.to_string(), lang, text)
        })
        .collect()
}

/// Where the parse of `text` first breaks (row and column, from 1), if
/// it does.
fn first_break(lang: &Lang, text: &str) -> Option<(usize, usize)> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&lang.config.language).ok()?;
    let tree = parser.parse(text, None)?;
    let mut cursor = tree.walk();
    loop {
        let node = cursor.node();
        if node.is_error() || node.is_missing() {
            let at = node.start_position();
            return Some((at.row + 1, at.column + 1));
        }
        if node.has_error() && cursor.goto_first_child() {
            continue;
        }
        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() {
                return None;
            }
        }
    }
}

/// A sample the grammar does not read whole goes to the fallback lexer
/// in the product (`patch::patch_colors`), so its snapshot would show
/// colours nobody sees.
#[test]
fn every_sample_reads_without_a_parse_error() {
    let broken: Vec<String> = samples()
        .iter()
        .filter(|(_, lang, text)| !reads(lang, text))
        .map(|(path, lang, text)| format!("{path} at {:?}", first_break(lang, text)))
        .collect();
    assert!(broken.is_empty(), "samples with a parse error: {broken:#?}");
}

/// What a grammar paints is the expectation as it stands: a change to a
/// query, to `grammar::CAPTURES` or to a crate's version shows here as a
/// snapshot diff, to be read row by row before it is accepted.
#[test]
fn every_sample_paints_as_its_snapshot_says() {
    for (path, lang, text) in samples() {
        let rows = line_spans(lang, &mut Highlighter::new(), &text)
            .unwrap_or_else(|| panic!("{path} is refused by its grammar"));
        insta::with_settings!({ description => LEGEND, omit_expression => true }, {
            insta::assert_snapshot!(path, rendered(&text, &rows));
        });
    }
}
