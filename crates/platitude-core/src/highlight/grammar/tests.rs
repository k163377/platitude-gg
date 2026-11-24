//! `grammar`'s tests, split out for length alone (structure.md §分割).

use tree_sitter::Query;
use tree_sitter_highlight::{HighlightEvent, Highlighter};

use super::*;

/// A grammar whose query the runtime refuses falls back silently, so
/// only a test can tell "shipped" from "shipped and never used".
#[test]
fn every_entry_builds_its_configuration() {
    for entry in &ENTRIES {
        assert!(
            lang_of(entry).is_some(),
            "grammar {} failed to build its configuration",
            entry.name
        );
    }
}

/// A comment and a literal in each language, by [`Entry::name`] —
/// enough that any query which paints at all paints something here. A
/// delta's holds only what its bases paint
/// (`a_delta_s_sample_is_its_bases_to_paint`).
const SAMPLES: [(&str, &str); 37] = [
    ("rust", "// note\nfn main() { let s = \"hi\"; }"),
    ("qml", "// note\nItem { function f() { return \"hi\" } }"),
    ("kotlin", "// note\nfun main() { val s = \"hi\" }"),
    ("markdown", "# Title\n\nsome text"),
    ("markdown_inline", "some `code` and a [link](x)"),
    ("toml", "# note\nkey = \"value\""),
    ("yaml", "# note\nkey: value"),
    ("json", "{\"key\": \"value\"}"),
    ("javascript", "// note\nconst s = \"hi\";"),
    ("typescript", "// note\nconst s = \"hi\";"),
    ("tsx", "// note\nconst a = <div className=\"x\">y</div>;"),
    ("python", "# note\ns = \"hi\""),
    ("java", "// note\nclass A { String s = \"hi\"; }"),
    ("c", "// note\nint main(void) { return 0; }"),
    ("cpp", "// note\nint main() { return 0; }"),
    ("c_sharp", "// note\nclass A { string s = \"hi\"; }"),
    ("go", "// note\npackage main"),
    ("ruby", "# note\ns = \"hi\""),
    ("php", "<?php\n// note\n$s = \"hi\";"),
    ("swift", "// note\nlet s = \"hi\""),
    ("html", "<!-- note -->\n<div class=\"a\">x</div>"),
    ("css", "/* note */\na { color: red; }"),
    ("bash", "# note\ns=\"hi\""),
    ("scala", "// note\nval s = \"hi\""),
    ("haskell", "-- note\nmain = putStrLn \"hi\""),
    ("lua", "-- note\nlocal s = \"hi\""),
    ("zig", "// note\nconst s = \"hi\";"),
    ("elixir", "# note\ns = \"hi\""),
    ("ocaml", "(* note *)\nlet s = \"hi\""),
    ("objc", "// note\nNSString *s = @\"hi\";"),
    ("nix", "# note\n{ s = \"hi\"; }"),
    ("sql", "-- note\nSELECT * FROM t;"),
    ("diff", "--- a/f\n+++ b/f\n@@ -1 +1 @@\n-a\n+b"),
    ("powershell", "# note\n$s = \"hi\""),
    ("properties", "# note\nkey=value"),
    ("cmake", "# note\nset(VAR \"hi\")"),
    ("xml", "<!-- note -->\n<a b=\"c\"/>"),
];

/// The sample the grammar called `name` answers for.
fn sample_of(name: &str) -> &'static str {
    SAMPLES
        .iter()
        .find(|(sample, _)| *sample == name)
        .map(|(_, text)| *text)
        .unwrap_or_else(|| panic!("grammar {name} has no sample to answer for"))
}

/// Whether `config` paints any of `text` in a colour of its own: a row
/// may paint plain (`character.special`), so only another colour tells
/// that the query paints.
fn paints(config: &HighlightConfiguration, text: &str) -> bool {
    let mut highlighter = Highlighter::new();
    // The callback through a closure: as a fn item it answers
    // `&'static`, which would lend the highlighter itself for
    // `'static` (`tree::line_spans` does the same).
    #[expect(
        clippy::redundant_closure,
        reason = "the fn item would lend the highlighter for 'static"
    )]
    let events = highlighter
        .highlight(config, text.as_bytes(), None, None, |name| injection(name))
        .expect("a grammar reads its own language");
    events.flatten().any(|event| {
        matches!(
            event,
            HighlightEvent::HighlightStart(h) if style(h.0).is_some_and(|c| c != PLAIN)
        )
    })
}

/// A query being accepted is not painting: a delta query (see
/// [`Entry::highlights`]) is colourless without its bases. Every entry
/// answers for its own sample, so a language added without its bases
/// fails here.
#[test]
fn every_grammar_colours_a_sample_of_its_language() {
    for entry in &ENTRIES {
        let lang = lang_of(entry).expect("the guard above builds every configuration");
        assert!(
            paints(&lang.config, sample_of(entry.name)),
            "grammar {} left its own sample uncoloured",
            entry.name
        );
    }
}

/// A delta's sample is its bases' to paint: built without them, the
/// entry leaves it uncoloured, so the test above fails for a delta
/// whose bases are left out.
#[test]
fn a_delta_s_sample_is_its_bases_to_paint() {
    for (name, bases) in [
        ("qml", 2),
        ("typescript", 1),
        ("tsx", 2),
        ("cpp", 1),
        ("objc", 1),
    ] {
        let entry = ENTRIES
            .iter()
            .find(|e| e.name == name)
            .unwrap_or_else(|| panic!("no grammar is called {name}"));
        let alone = Entry {
            matches: &[],
            name: entry.name,
            language: entry.language,
            highlights: &entry.highlights[bases..],
            injections: entry.injections,
            locals: entry.locals,
            cell: OnceLock::new(),
        };
        let config = configured(&alone).expect("a delta builds without its bases");
        assert!(
            !paints(&config, sample_of(name)),
            "{name}'s sample is painted without its bases"
        );
    }
}

/// Every language a path reaches answers for its colours in `tree`'s
/// role snapshots, through a sample named for the entry
/// (`<name>.<extension>.sample`).
#[test]
fn every_claimed_language_has_a_role_sample() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/highlight/tree/samples");
    for entry in ENTRIES.iter().filter(|e| !e.matches.is_empty()) {
        assert!(
            entry
                .matches
                .iter()
                .any(|m| dir.join(format!("{}.{m}.sample", entry.name)).is_file()),
            "grammar {} has no sample under {}",
            entry.name,
            dir.display()
        );
    }
}

/// The row `configure` gives a capture name: the listed name with the
/// most parts, all among the capture's own; a tie goes to the earlier.
fn row_of(capture: &str) -> Option<&'static str> {
    let parts: Vec<&str> = capture.split('.').collect();
    let mut best: Option<(&'static str, usize)> = None;
    for (name, _) in &CAPTURES {
        let row: Vec<&str> = name.split('.').collect();
        if row.iter().all(|p| parts.contains(p)) && best.is_none_or(|(_, n)| row.len() > n) {
            best = Some((name, row.len()));
        }
    }
    best.map(|(name, _)| name)
}

/// The names a shipped query uses with no row, each meant to read as
/// the capture around it: `escape` and the markup emphases, always
/// inside a run of their own colour; and the one-off names no editors'
/// theme knows (`delimiter`, `error`, powershell's `array` and
/// `assignvalue`).
const SURROUNDINGS: [&str; 8] = [
    "escape",
    "markup",
    "markup.italic",
    "markup.strong",
    "delimiter",
    "error",
    "array",
    "assignvalue",
];

/// Whether a capture name is no colour of its own: a private one
/// (`_name`) feeds a predicate, `local` and `injection` are
/// tree-sitter's, and the spell-check marks are turned off.
fn no_colour(name: &str) -> bool {
    name.starts_with('_')
        || name.starts_with("local.")
        || name.starts_with("injection.")
        || ["spell", "nospell"].contains(&name)
}

/// A name a crate adds reaches neither the palette nor this list by
/// itself: with no row, it is no highlight, and it takes the node
/// from any earlier pattern's colour (see [`CAPTURES`]).
#[test]
fn every_capture_name_has_a_row_or_reads_as_its_surroundings() {
    let mut used = Vec::new();
    for entry in &ENTRIES {
        let language = (entry.language)();
        let query = Query::new(&language, &highlights_of(entry)).expect("every entry builds");
        for name in query.capture_names() {
            if no_colour(name) {
                continue;
            }
            if row_of(name).is_none() {
                assert!(
                    SURROUNDINGS.contains(name),
                    "{}'s @{name} has no row, and is not one that reads as its surroundings",
                    entry.name
                );
                used.push((*name).to_owned());
            }
        }
    }
    for name in SURROUNDINGS {
        assert!(
            used.iter().any(|u| u == name),
            "no shipped query uses @{name} any more"
        );
    }
}

/// The colour the grammar called `name` paints the `nth` occurrence
/// of `needle` in `text` — one colour across all of it.
fn colour_of_nth(name: &str, text: &str, needle: &str, nth: usize) -> Rgb {
    let entry = ENTRIES
        .iter()
        .find(|e| e.name == name)
        .unwrap_or_else(|| panic!("no grammar is called {name}"));
    let lang = lang_of(entry).expect("every entry builds its configuration");
    let (at, _) = text
        .match_indices(needle)
        .nth(nth)
        .expect("the needle is in the text");
    let mut highlighter = Highlighter::new();
    let events = highlighter
        .highlight(&lang.config, text.as_bytes(), None, None, |name| {
            injection(name)
        })
        .expect("a grammar reads its own language");
    let mut stack: Vec<usize> = Vec::new();
    let mut seen: Vec<Rgb> = Vec::new();
    for event in events {
        match event.expect("a highlight runs to the end of its text") {
            HighlightEvent::HighlightStart(h) => stack.push(h.0),
            HighlightEvent::HighlightEnd => {
                stack.pop();
            }
            HighlightEvent::Source { start, end } if start < at + needle.len() && at < end => {
                seen.push(stack.last().and_then(|&i| style(i)).unwrap_or(PLAIN));
            }
            HighlightEvent::Source { .. } => {}
        }
    }
    seen.dedup();
    assert_eq!(seen.len(), 1, "{needle:?} in {name} is painted in {seen:?}");
    seen[0]
}

fn colour_of(name: &str, text: &str, needle: &str) -> Rgb {
    colour_of_nth(name, text, needle, 0)
}

fn assert_colours(cases: &[(&str, &str, &str, u32)]) {
    for &(name, text, needle, rgb) in cases {
        assert_eq!(
            colour_of(name, text, needle),
            Rgb::of(rgb),
            "{needle:?} in {name}: {text:?}"
        );
    }
}

/// What only a layered language's own query names keeps its colour:
/// a base's catch-all (`(identifier) @variable`) after it would take
/// the node back (see [`Entry::highlights`]).
#[test]
fn a_layered_language_keeps_what_its_own_query_names() {
    let jsx = "const a = <div className=\"x\">y</div>;";
    let qml = "Item {\n    signal clicked()\n}";
    let objc = "@implementation Foo\n- (void)run { [self doThing]; }\n@end";
    assert_colours(&[
        ("cpp", "void Foo::bar() {}", "bar", FUNCTION),
        ("cpp", "int main() { std::move(x); }", "move", FUNCTION),
        ("cpp", "class A { void run(); };", "run", FUNCTION),
        ("javascript", jsx, "div", KEYWORD),
        ("javascript", jsx, "className", ANNOTATION),
        ("typescript", "const MAX = 1;", "MAX", TYPE),
        ("tsx", jsx, "div", KEYWORD),
        ("tsx", jsx, "className", ANNOTATION),
        ("qml", qml, "clicked", FUNCTION),
        ("objc", objc, "Foo", TYPE),
        ("objc", objc, "doThing", FUNCTION),
    ]);
}

/// A file reads in the order it was written for: backwards for the
/// earlier pattern, its catch-alls first where they trail, as written
/// for the later pattern; a pattern left out is gone.
#[test]
fn a_highlights_file_reads_in_the_order_it_was_written_for() {
    let file = "(use_list (self) @keyword)\n(self) @variable.builtin\n(identifier) @variable";
    let read = |highlights: Highlights| highlights.read();
    assert_eq!(
        read(first_wins(file)),
        "(identifier) @variable\n(self) @variable.builtin\n(use_list (self) @keyword)"
    );
    assert_eq!(
        read(catch_alls_after(file)),
        "(self) @variable.builtin\n(identifier) @variable\n(use_list (self) @keyword)"
    );
    assert_eq!(
        read(last_wins(file).without(&["(self)   @variable.builtin"])),
        "(use_list (self) @keyword)\n(identifier) @variable"
    );
    assert_eq!(read(last_wins(file)), file);
}

/// A file ordered for the earlier pattern paints as its author meant:
/// go's calls and declarations take the function colour its trailing
/// catch-alls (`(identifier) @variable`, `(field_identifier) @property`)
/// would take back, and nix's builtins theirs.
#[test]
fn a_file_ordered_for_the_earlier_pattern_paints_as_written() {
    let go = "func main() { fmt.Println(len(xs)) }";
    assert_colours(&[
        ("go", go, "main", FUNCTION),
        ("go", go, "Println", FUNCTION),
        ("go", go, "len", FUNCTION),
        (
            "go",
            "func (p *Point) Area() int { return 0 }",
            "Area",
            FUNCTION,
        ),
        ("nix", "{ y = import ./a.nix; }", "import", FUNCTION),
    ]);
}

/// A catch-all left after the patterns it backs reads first: rust's
/// `self` is a keyword in a `use` path, where `(self) @variable.builtin`
/// after it would take it back.
#[test]
fn a_trailing_catch_all_backs_the_patterns_before_it() {
    assert_colours(&[("rust", "use std::io::{self, Read};", "self", KEYWORD)]);
}

/// A pattern left out of a crate's file is in that file, once: a crate
/// that drops or rewrites it leaves the entry naming nothing.
#[test]
fn every_left_out_pattern_is_in_its_file() {
    for entry in &ENTRIES {
        for file in entry.highlights {
            for left in file.without {
                let found = patterns(file.query)
                    .into_iter()
                    .filter(|pattern| same_pattern(pattern, left))
                    .count();
                assert_eq!(
                    found, 1,
                    "{} leaves out {left:?}, found {found} times",
                    entry.name
                );
            }
        }
    }
}

/// The patterns tree-sitter reads in `query`, each without the spacing
/// and comments its span runs on over, up to the next pattern.
fn tree_sitter_s_patterns<'q>(language: &Language, query: &'q str) -> Vec<&'q str> {
    let compiled = Query::new(language, query).expect("the query builds");
    (0..compiled.pattern_count())
        .map(|i| {
            let span = &query[compiled.start_byte_for_pattern(i)..compiled.end_byte_for_pattern(i)];
            without_trailing_comments(span)
        })
        .collect()
}

/// `span` up to its last character that is neither spacing nor in a
/// comment.
fn without_trailing_comments(span: &str) -> &str {
    let mut end = 0;
    let mut chars = span.char_indices();
    while let Some((at, c)) = chars.next() {
        match c {
            ';' => {
                for (_, c) in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '"' => {
                end = at + 1;
                while let Some((at, c)) = chars.next() {
                    end = at + c.len_utf8();
                    match c {
                        '\\' => {
                            if let Some((at, escaped)) = chars.next() {
                                end = at + escaped.len_utf8();
                            }
                        }
                        '"' => break,
                        _ => {}
                    }
                }
            }
            c if c.is_whitespace() => {}
            c => end = at + c.len_utf8(),
        }
    }
    &span[..end]
}

/// Every highlights file splits into the patterns tree-sitter reads
/// in it, and a query of every shape the splitting meets does too.
#[test]
fn a_query_splits_into_the_patterns_tree_sitter_reads() {
    for entry in &ENTRIES {
        let language = (entry.language)();
        for file in entry.highlights {
            assert_eq!(
                patterns(file.query),
                tree_sitter_s_patterns(&language, file.query),
                "{}",
                entry.name
            );
        }
    }
    let shapes = "; a comment with \"quotes\" and (parens)\n\
                  \"(\" @punctuation.bracket\n\
                  [\")\" \";\"] @punctuation ; and one after\n\
                  (line_comment)+ @comment\n\
                  (_) @x.y-z\n\
                  ((identifier) @constant (#match? @constant \"^[A-Z(]\"))\n\
                  (call_expression function: (identifier) @function)";
    let rust: Language = tree_sitter_rust::LANGUAGE.into();
    assert_eq!(patterns(shapes), tree_sitter_s_patterns(&rust, shapes));
    assert_eq!(patterns(shapes).len(), 6);
}

/// Haskell's names take the colours of what they are: a function its
/// own, a variable plain.
#[test]
fn a_haskell_variable_is_no_type() {
    let hs = "greet :: String -> String\ngreet name = \"hi \" ++ name\n";
    assert_colours(&[
        ("haskell", hs, "greet", FUNCTION),
        ("haskell", hs, "String", TYPE),
        ("haskell", hs, "name", theme::PLAIN),
    ]);
}

/// A reference to a local takes its definition's highlight: ruby's
/// local variable is no method call, a parameter reads alike where it
/// is defined and where it is called (QML's `require`, JavaScript's
/// callback).
#[test]
fn a_local_reference_takes_its_definition_s_colour() {
    let ruby = "def f\n  count = 1\n  puts count\nend\n";
    let qml = "Item {\n    function load(require) { return require(\"x\") }\n}\n";
    let js = "function run(callback) { return callback(); }";
    for (name, text, needle) in [
        ("ruby", ruby, "count"),
        ("qml", qml, "require"),
        ("javascript", js, "callback"),
    ] {
        let defined = colour_of_nth(name, text, needle, 0);
        assert_eq!(defined, Rgb::of(theme::PLAIN), "{needle:?} in {name}");
        assert_eq!(
            colour_of_nth(name, text, needle, 1),
            defined,
            "{needle:?} in {name} where it is used"
        );
    }
}

/// A name interpolated into a string reads as the name, not the string.
#[test]
fn an_interpolated_name_reads_as_itself() {
    assert_colours(&[
        (
            "javascript",
            "const s = `hi ${name}!`;",
            "name",
            theme::PLAIN,
        ),
        ("python", "s = f\"hi {name}!\"", "name", theme::PLAIN),
    ]);
}

/// What a query marks `@none` reads as plain text, not as the capture
/// around it: a fenced block's code where its own language names nothing,
/// scala's interpolated name that is no local.
#[test]
fn a_none_capture_reads_as_plain_text() {
    assert_colours(&[
        (
            "markdown",
            "```rust\nlet s = 1;\n```\n",
            "s =",
            theme::PLAIN,
        ),
        (
            "scala",
            "object A { def f = s\"hi $name\" }",
            "name",
            theme::PLAIN,
        ),
    ]);
}

/// A host's capture and an injected language's that begin at one byte
/// nest host outside: a heading that opens with inline code or a link
/// keeps its colour past it, and a fenced block's first token keeps its
/// own (vendor/tree-sitter-highlight/PLATITUDE-MODIFIED.md).
#[test]
fn an_injected_capture_at_a_host_capture_s_first_byte_nests_inside_it() {
    assert_colours(&[
        ("markdown", "# `code` rest\n", "rest", KEYWORD),
        (
            "markdown",
            "## [1.2.0](https://example.com) (2024)\n",
            "2024",
            KEYWORD,
        ),
        ("markdown", "```rust\nlet s = 1;\n```\n", "let", KEYWORD),
    ]);
}

/// A pattern whose predicate tree-sitter-highlight does not evaluate
/// claims nothing, rather than every node of its shape: objc's
/// `#has-ancestor?` property would otherwise take every identifier.
#[test]
fn a_predicate_the_highlighter_skips_claims_nothing() {
    assert_colours(&[("objc", "void f() { NSLog(@\"hi\"); }", "NSLog", FUNCTION)]);
}

/// Of the property predicates, tree-sitter-highlight evaluates
/// `#is-not? local` alone: a pattern carrying any other claims nothing,
/// as one carrying a general predicate does.
#[test]
fn a_property_predicate_the_highlighter_skips_claims_nothing() {
    const UNREAD: &[Highlights] = &[
        last_wins("((identifier) @keyword (#is? local))"),
        last_wins("((field_identifier) @keyword (#is-not? test))"),
    ];
    let entry = Entry {
        matches: &[],
        name: "rust",
        language: || tree_sitter_rust::LANGUAGE.into(),
        highlights: UNREAD,
        injections: &[],
        locals: &[],
        cell: OnceLock::new(),
    };
    let config = configured(&entry).expect("the query builds");
    assert!(!paints(&config, "fn f() { x.y }"));
}

/// A combined injection is matched from a copy of the injections that
/// [`configured`] cannot turn a pattern off in, so none may carry a
/// predicate tree-sitter-highlight leaves unevaluated.
#[test]
fn every_combined_injection_carries_only_what_the_highlighter_reads() {
    for entry in &ENTRIES {
        let language = (entry.language)();
        let injections = with_regex_matches(&entry.injections.join("\n"));
        let query = Query::new(&language, &injections).expect("every entry builds");
        for pattern in 0..query.pattern_count() {
            let combined = query
                .property_settings(pattern)
                .iter()
                .any(|setting| &*setting.key == "injection.combined");
            if combined {
                assert!(
                    query.general_predicates(pattern).is_empty()
                        && query.property_predicates(pattern).is_empty(),
                    "{}'s combined injection at byte {} carries a predicate left unevaluated",
                    entry.name,
                    query.start_byte_for_pattern(pattern)
                );
            }
        }
    }
}

/// A Lua pattern reads as the regular expression of the same meaning;
/// one with no such expression here gives none.
#[test]
fn a_lua_pattern_reads_as_the_regular_expression_of_its_meaning() {
    let read = |lua: &str| lua_as_regex(lua);
    assert_eq!(
        read("^[A-Z_][a-zA-Z0-9_]*").as_deref(),
        Some("^[A-Z_][a-zA-Z0-9_]*")
    );
    assert_eq!(
        read("^[%u@][%u%d_]+$").as_deref(),
        Some("^[A-Z@][A-Z0-9_]+$")
    );
    assert_eq!(read("^#!/").as_deref(), Some("^\\#!/"));
    assert_eq!(
        read("^%.?[%w_-]+$").as_deref(),
        Some("^\\.?[A-Za-z0-9_\\-]+$")
    );
    // A `-` after an escaped member is one of the set's members, not
    // the start of a range: `[%+-/]` is `+`, `-` and `/`.
    assert_eq!(read("[%+-/]").as_deref(), Some("[+\\-/]"));
    assert_eq!(read("a-b.").as_deref(), Some("a*?b(?s:.)"));
    assert_eq!(read("%S+"), Some("[^\\t\\n\\x0B\\x0C\\r ]+".to_string()));
    assert_eq!(read("x*$"), Some("x*$".to_string()));
    assert_eq!(read("a$b^"), Some("a\\$b\\^".to_string()));
    assert_eq!(read("%b()"), None);
    assert_eq!(read("[%A]"), None);
    assert_eq!(read("%1"), None);
}

/// A `#lua-match?` becomes the `#match?` of its regular expression; one
/// with none stays as it is, for its pattern to be turned off.
#[test]
fn a_lua_match_becomes_a_match_where_its_pattern_has_a_regular_expression() {
    let query = "((identifier) @type (#lua-match? @type \"^%u\"))\n\
                 ((x) @y (#not-lua-match? @y \"%b()\"))";
    assert_eq!(
        with_regex_matches(query),
        "((identifier) @type (#match? @type \"^[A-Z]\"))\n\
         ((x) @y (#not-lua-match? @y \"%b()\"))"
    );
    assert_colours(&[("zig", "const p = Point.init();", "Point", TYPE)]);
}

/// A spell-check mark beside a colour on one node (`@comment @spell`)
/// leaves the node its colour.
#[test]
fn a_spell_check_mark_leaves_a_comment_its_colour() {
    assert_colours(&[
        ("sql", "-- note\nSELECT 1;", "note", COMMENT),
        ("swift", "// note\nlet s = 1", "note", COMMENT),
        ("haskell", "-- note\nmain = 1", "note", COMMENT),
        ("cmake", "# note\nset(VAR 1)", "note", COMMENT),
        ("zig", "// note\nconst s = 1;", "note", COMMENT),
    ]);
}

/// What a crate's grammar reads and its query leaves out takes its
/// colour from the query of this repository's own laid over it, last.
#[test]
fn this_repository_s_query_adds_what_a_crate_s_leaves_out() {
    let cs =
        "static class T\n{\n    extension(string s)\n    {\n        public int N => 1;\n    }\n}";
    assert_colours(&[("c_sharp", cs, "extension", KEYWORD)]);
}

/// A crate in the older capture vocabulary paints as one in the
/// current: its keywords (`@conditional`, `@include`, `@storageclass`,
/// `@type.qualifier`) reach the table renamed ([`current_names`]).
#[test]
fn a_capture_in_the_older_vocabulary_takes_its_colour() {
    let objc =
        "#include <stdio.h>\nstatic volatile int n;\nvoid f(void) { @try { } @catch (id e) { } }\n";
    let scala = "sealed trait A { def f(x: Boolean) = if (x) 1 else 2 }";
    let sql = "SELECT CASE WHEN x THEN 1 ELSE 2 END FROM t;";
    assert_colours(&[
        ("scala", scala, "else", KEYWORD),
        ("scala", scala, "sealed", KEYWORD),
        ("lua", "while x do end", "while", KEYWORD),
        ("sql", sql, "CASE", KEYWORD),
        ("objc", objc, "#include", KEYWORD),
        ("objc", objc, "volatile", KEYWORD),
        ("objc", objc, "@try", KEYWORD),
    ]);
}

/// The current vocabulary's characters and markup have rows of their
/// own: crates on it name a `'a'` `@character` and CDATA `@markup.raw`.
#[test]
fn a_character_and_raw_markup_take_the_string_colour() {
    assert_colours(&[
        ("haskell", "c = 'a'", "'a'", STRING),
        ("zig", "const c = 'a';", "'a'", STRING),
        ("xml", "<a><![CDATA[x < y]]></a>", "x < y", STRING),
    ]);
}

/// A narrower name keeps off a broader row's colour where it means
/// another thing: swift's wildcard `_` (`@character.special`) is no
/// character literal.
#[test]
fn swift_s_wildcard_is_plain() {
    assert_colours(&[("swift", "switch x { case _: break }", "_", theme::PLAIN)]);
}

/// The renaming touches captures alone: a string or a comment that
/// spells one, and a name that only begins like one, stand as they
/// are; a predicate's reference follows its capture.
#[test]
fn an_older_capture_name_is_written_in_the_current_vocabulary() {
    let older = "(if) @conditional\n(call method: (_) @method.call)\n\
                 ((x) @include (#any-of? @include \"#include\"))\n\
                 \"@try\" @exception ; @repeat\n(y) @conditionally\n";
    let current = "(if) @keyword.conditional\n(call method: (_) @function.method.call)\n\
                   ((x) @keyword.import (#any-of? @keyword.import \"#include\"))\n\
                   \"@try\" @keyword.exception ; @repeat\n(y) @conditionally\n";
    assert_eq!(current_names(older), current);
}

#[test]
fn paths_resolve_by_extension_and_by_whole_name() {
    assert!(claims("src/main.rs"));
    assert!(claims("ui/DetailsPane.qml"));
    assert!(claims("A.kt"));
    assert!(claims("tools/CMakeLists.txt"));
    assert!(claims("デザイン規約.md"));
    assert!(!claims("notes.qqq"));
    assert!(!claims("scripts/go"));
    // Groovy's crate ships no queries, so gradle scripts read
    // through the fallback lexer.
    assert!(!claims("build.gradle"));
}

#[test]
fn an_injection_names_a_language_by_its_name_or_extension_in_any_case() {
    let named = |name: &str| injection(name).map(|config| config.language_name.as_str());
    assert_eq!(named("javascript"), Some("javascript"));
    assert_eq!(named("js"), Some("javascript"));
    assert_eq!(named("Rust"), Some("rust"));
    assert_eq!(named("yml"), Some("yaml"));
    assert_eq!(named("SQL"), Some("sql"));
    assert_eq!(named("markdown_inline"), Some("markdown_inline"));
    assert_eq!(named("console"), None);
}
