//! The grammars the fast path knows: which languages, how their capture
//! names map onto the app's palette, and one lazily built configuration
//! per language. A path no entry claims falls back to the regex lexer.

use std::sync::OnceLock;

use tree_sitter::Language;
use tree_sitter_highlight::HighlightConfiguration;

use super::Rgb;
use super::theme::{
    self, ANNOTATION, COMMENT, CONSTANT, FUNCTION, KEYWORD, PLAIN_RGB, STRING, TYPE,
};

/// The colour a run gets when no capture below claims it — the same
/// `textPrimary` the regex lexer falls back to.
pub(super) const PLAIN: Rgb = PLAIN_RGB;

/// Capture name → colour: デザイン規約 §シンタックスハイライト's table in
/// tree-sitter's capture vocabulary (the values live in `theme`, shared
/// with `theme::PALETTE`).
///
/// Unlisted captures (operators, punctuation, variables) come out
/// [`PLAIN`]. tree-sitter resolves a capture to the listed name with the
/// most dot-separated parts that all appear among the capture's own, in
/// any position; a tie goes to the earlier row. So `keyword` covers
/// `keyword.control`, the narrow `constant.builtin` stands without the
/// broad `constant`, and `tag.attribute` takes `attribute`'s colour.
///
/// The rows speak the current generation of the editors' capture
/// vocabulary, and only the names a shipped query uses. A crate still
/// shipping the older one has its query renamed on the way in
/// ([`current_names`]), so no row stands for an older name. A row in
/// `theme::PLAIN` keeps a narrower name off a broader row's colour: swift's
/// wildcard `_` (`character.special`) is not a character literal.
const CAPTURES: [(&str, u32); 16] = [
    ("comment", COMMENT),
    ("keyword", KEYWORD),
    ("type", TYPE),
    ("constructor", TYPE),
    ("function", FUNCTION),
    ("string", STRING),
    ("character", STRING),
    ("character.special", theme::PLAIN),
    ("number", CONSTANT),
    ("boolean", CONSTANT),
    ("constant.builtin", CONSTANT),
    ("attribute", ANNOTATION),
    ("tag", KEYWORD),
    ("markup.heading", KEYWORD),
    ("markup.raw", STRING),
    ("markup.link", KEYWORD),
];

/// The older capture names some crates still ship, each with the name
/// nvim-treesitter renamed it to (nvim-treesitter#5831); a name that
/// starts with one (`method.call`) keeps its tail. `storageclass` and
/// `type.qualifier` go where the current crates put `static` and
/// `final`: `keyword.modifier`.
const RENAMED: [(&str, &str); 18] = [
    ("conditional", "keyword.conditional"),
    ("repeat", "keyword.repeat"),
    ("include", "keyword.import"),
    ("exception", "keyword.exception"),
    ("storageclass", "keyword.modifier"),
    ("type.qualifier", "keyword.modifier"),
    ("preproc", "keyword.directive"),
    ("method", "function.method"),
    ("field", "variable.member"),
    ("parameter", "variable.parameter"),
    ("float", "number.float"),
    ("namespace", "module"),
    ("text.title", "markup.heading"),
    ("text.literal", "markup.raw"),
    ("text.uri", "markup.link.url"),
    ("text.reference", "markup.link"),
    ("text.emphasis", "markup.italic"),
    ("text.strong", "markup.strong"),
];

/// `query` with every capture in the older vocabulary ([`RENAMED`])
/// written in the current one, the predicates' references to it with it.
/// Strings (objc's `"@try"`) and comments are copied as they stand. A
/// crate that moves to the current names leaves this nothing to do.
fn current_names(query: &str) -> String {
    let mut out = String::with_capacity(query.len());
    let mut chars = query.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        match c {
            '"' => {
                out.push(c);
                while let Some((_, s)) = chars.next() {
                    out.push(s);
                    match s {
                        '\\' => {
                            if let Some((_, e)) = chars.next() {
                                out.push(e);
                            }
                        }
                        '"' => break,
                        _ => {}
                    }
                }
            }
            ';' => {
                out.push(c);
                while let Some(&(_, s)) = chars.peek() {
                    if s == '\n' {
                        break;
                    }
                    out.push(s);
                    chars.next();
                }
            }
            '@' => {
                let start = at + 1;
                let mut end = start;
                while let Some(&(i, s)) = chars.peek() {
                    if !(s.is_ascii_alphanumeric() || s == '_' || s == '.') {
                        break;
                    }
                    end = i + s.len_utf8();
                    chars.next();
                }
                let name = &query[start..end];
                let renamed = RENAMED.iter().find_map(|(old, new)| {
                    let tail = name.strip_prefix(old)?;
                    (tail.is_empty() || tail.starts_with('.')).then(|| format!("{new}{tail}"))
                });
                out.push('@');
                out.push_str(renamed.as_deref().unwrap_or(name));
            }
            _ => out.push(c),
        }
    }
    out
}

/// The colour behind one highlight index — the same order
/// [`HighlightConfiguration::configure`] was given.
pub(super) fn style(index: usize) -> Option<Rgb> {
    let (_, rgb) = CAPTURES.get(index)?;
    Some(Rgb::of(*rgb))
}

/// One language the fast path can read.
pub(super) struct Lang {
    pub(super) config: HighlightConfiguration,
}

struct Entry {
    /// Extensions — and whole file names (`CMakeLists.txt`) — this
    /// language claims, lower-case.
    matches: &'static [&'static str],
    /// The name injections address this language by (```rust fences,
    /// markdown's own inline grammar).
    name: &'static str,
    build: fn() -> Option<HighlightConfiguration>,
    cell: OnceLock<Option<Lang>>,
}

/// Builds one configuration, or `None` for a query the runtime
/// refuses — that language then reads through the fallback lexer.
fn configured(
    language: Language,
    name: &'static str,
    highlights: &str,
    injections: &str,
) -> Option<HighlightConfiguration> {
    // `CAPTURES` speaks one vocabulary.
    let highlights = current_names(highlights);
    let mut config =
        HighlightConfiguration::new(language, name, &highlights, injections, "").ok()?;
    // tree-sitter-highlight evaluates the text predicates (`#eq?`,
    // `#match?`, `#any-of?`) and the property ones, and ignores the rest
    // — editors' own, `#lua-match?` and `#has-ancestor?` — so a pattern
    // carrying one would claim every node of its shape.
    for pattern in 0..config.query.pattern_count() {
        if !config.query.general_predicates(pattern).is_empty() {
            config.query.disable_pattern(pattern);
        }
    }
    // Spell-check marks share a node with its colour (`@comment @spell`),
    // and the later capture on a node is the one painted.
    for mark in ["spell", "nospell"] {
        config.query.disable_capture(mark);
    }
    let names: Vec<&str> = CAPTURES.iter().map(|(n, _)| *n).collect();
    config.configure(&names);
    Some(config)
}

/// Several highlight queries as one, for crates that ship theirs as a
/// delta over another language's (qmljs over JavaScript's, objc over
/// C's) and colour almost nothing alone. The base language goes first,
/// as tree-sitter-cpp's and tree-sitter-qmljs's own manifests list
/// theirs: where two patterns claim one node, tree-sitter-highlight
/// paints the later, so a base's catch-all (`(identifier) @variable`)
/// put last would take back every node the specific language names.
///
/// Five ship so: qmljs, typescript, tsx, cpp and objc. Only objc says so
/// (`; inherits:`), so a search of the queries does not find the rest;
/// what tells is a sample painting something besides `PLAIN` — a language
/// added takes a `SAMPLES` line (`every_grammar_colours_a_sample_of_its_language`).
/// javascript layers its own JSX delta too, which no such sample misses
/// (the base alone paints): there the tell is the crate's second
/// highlight constant (`JSX_HIGHLIGHT_QUERY`).
///
/// Where a crate's query names a node otherwise than デザイン規約
/// §シンタックスハイライト would (C++'s casts as the template calls they
/// parse as, SQL's NULL as a type word, every Haskell variable as a type),
/// the crate's naming stands. A query of this repository's own laid last
/// adds only what a crate's query leaves out (C#'s `extension`, C's
/// `true` and `false`) or names by the broad `constant` alone, which
/// `CAPTURES` does not take for it also marks every all-caps name (C's
/// `NULL`, Elixir's and OCaml's booleans). Laid last, such a pattern also
/// takes the node where the crate names it otherwise in one context, so
/// the query names those contexts again in the crate's own words
/// (Elixir's `@doc false`).
fn layered(queries: &[&str]) -> String {
    queries.join("\n")
}

macro_rules! entry {
    ($matches:expr, $name:literal, $build:expr) => {
        Entry {
            matches: $matches,
            name: $name,
            build: $build,
            cell: OnceLock::new(),
        }
    };
}

#[rustfmt::skip]
static ENTRIES: [Entry; 37] = [
    entry!(&["rs"], "rust", || configured(tree_sitter_rust::LANGUAGE.into(), "rust", tree_sitter_rust::HIGHLIGHTS_QUERY, tree_sitter_rust::INJECTIONS_QUERY)),
    entry!(&["qml"], "qml", || configured(tree_sitter_qmljs::LANGUAGE.into(), "qml", &layered(&[tree_sitter_javascript::HIGHLIGHT_QUERY, tree_sitter_typescript::HIGHLIGHTS_QUERY, tree_sitter_qmljs::HIGHLIGHTS_QUERY]), "")),
    entry!(&["kt", "kts"], "kotlin", || configured(tree_sitter_kotlin_ng::LANGUAGE.into(), "kotlin", include_str!("queries/kotlin-highlights.scm"), "")),
    entry!(&["md", "markdown"], "markdown", || configured(tree_sitter_md::LANGUAGE.into(), "markdown", tree_sitter_md::HIGHLIGHT_QUERY_BLOCK, include_str!("queries/markdown-injections.scm"))),
    entry!(&[], "markdown_inline", || configured(tree_sitter_md::INLINE_LANGUAGE.into(), "markdown_inline", tree_sitter_md::HIGHLIGHT_QUERY_INLINE, include_str!("queries/markdown_inline-injections.scm"))),
    entry!(&["toml"], "toml", || configured(tree_sitter_toml_ng::LANGUAGE.into(), "toml", tree_sitter_toml_ng::HIGHLIGHTS_QUERY, "")),
    entry!(&["yaml", "yml"], "yaml", || configured(tree_sitter_yaml::LANGUAGE.into(), "yaml", tree_sitter_yaml::HIGHLIGHTS_QUERY, "")),
    entry!(&["json", "jsonc"], "json", || configured(tree_sitter_json::LANGUAGE.into(), "json", tree_sitter_json::HIGHLIGHTS_QUERY, "")),
    entry!(&["js", "mjs", "cjs", "jsx"], "javascript", || configured(tree_sitter_javascript::LANGUAGE.into(), "javascript", &layered(&[tree_sitter_javascript::HIGHLIGHT_QUERY, tree_sitter_javascript::JSX_HIGHLIGHT_QUERY]), tree_sitter_javascript::INJECTIONS_QUERY)),
    entry!(&["ts", "mts", "cts"], "typescript", || configured(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(), "typescript", &layered(&[tree_sitter_javascript::HIGHLIGHT_QUERY, tree_sitter_typescript::HIGHLIGHTS_QUERY]), "")),
    entry!(&["tsx"], "tsx", || configured(tree_sitter_typescript::LANGUAGE_TSX.into(), "tsx", &layered(&[tree_sitter_javascript::HIGHLIGHT_QUERY, tree_sitter_javascript::JSX_HIGHLIGHT_QUERY, tree_sitter_typescript::HIGHLIGHTS_QUERY]), "")),
    entry!(&["py", "pyi"], "python", || configured(tree_sitter_python::LANGUAGE.into(), "python", tree_sitter_python::HIGHLIGHTS_QUERY, "")),
    entry!(&["java"], "java", || configured(tree_sitter_java::LANGUAGE.into(), "java", tree_sitter_java::HIGHLIGHTS_QUERY, "")),
    entry!(&["c", "h"], "c", || configured(tree_sitter_c::LANGUAGE.into(), "c", &layered(&[tree_sitter_c::HIGHLIGHT_QUERY, include_str!("queries/c-highlights.scm")]), "")),
    entry!(&["cpp", "cc", "cxx", "hpp", "hh", "hxx"], "cpp", || configured(tree_sitter_cpp::LANGUAGE.into(), "cpp", &layered(&[tree_sitter_c::HIGHLIGHT_QUERY, tree_sitter_cpp::HIGHLIGHT_QUERY, include_str!("queries/c-highlights.scm")]), "")),
    entry!(&["cs"], "c_sharp", || configured(tree_sitter_c_sharp::LANGUAGE.into(), "c_sharp", &layered(&[tree_sitter_c_sharp::HIGHLIGHTS_QUERY, include_str!("queries/c_sharp-highlights.scm")]), "")),
    entry!(&["go"], "go", || configured(tree_sitter_go::LANGUAGE.into(), "go", tree_sitter_go::HIGHLIGHTS_QUERY, "")),
    entry!(&["rb"], "ruby", || configured(tree_sitter_ruby::LANGUAGE.into(), "ruby", tree_sitter_ruby::HIGHLIGHTS_QUERY, "")),
    entry!(&["php"], "php", || configured(tree_sitter_php::LANGUAGE_PHP.into(), "php", tree_sitter_php::HIGHLIGHTS_QUERY, &layered(&[tree_sitter_php::INJECTIONS_QUERY, include_str!("queries/php-injections-text.scm")]))),
    entry!(&["swift"], "swift", || configured(tree_sitter_swift::LANGUAGE.into(), "swift", tree_sitter_swift::HIGHLIGHTS_QUERY, tree_sitter_swift::INJECTIONS_QUERY)),
    entry!(&["html", "htm"], "html", || configured(tree_sitter_html::LANGUAGE.into(), "html", tree_sitter_html::HIGHLIGHTS_QUERY, tree_sitter_html::INJECTIONS_QUERY)),
    entry!(&["css"], "css", || configured(tree_sitter_css::LANGUAGE.into(), "css", tree_sitter_css::HIGHLIGHTS_QUERY, "")),
    entry!(&["sh", "bash", "zsh"], "bash", || configured(tree_sitter_bash::LANGUAGE.into(), "bash", tree_sitter_bash::HIGHLIGHT_QUERY, "")),
    entry!(&["scala", "sbt"], "scala", || configured(tree_sitter_scala::LANGUAGE.into(), "scala", tree_sitter_scala::HIGHLIGHTS_QUERY, "")),
    entry!(&["hs"], "haskell", || configured(tree_sitter_haskell::LANGUAGE.into(), "haskell", tree_sitter_haskell::HIGHLIGHTS_QUERY, tree_sitter_haskell::INJECTIONS_QUERY)),
    entry!(&["lua"], "lua", || configured(tree_sitter_lua::LANGUAGE.into(), "lua", tree_sitter_lua::HIGHLIGHTS_QUERY, tree_sitter_lua::INJECTIONS_QUERY)),
    entry!(&["zig"], "zig", || configured(tree_sitter_zig::LANGUAGE.into(), "zig", tree_sitter_zig::HIGHLIGHTS_QUERY, tree_sitter_zig::INJECTIONS_QUERY)),
    entry!(&["ex", "exs"], "elixir", || configured(tree_sitter_elixir::LANGUAGE.into(), "elixir", &layered(&[tree_sitter_elixir::HIGHLIGHTS_QUERY, include_str!("queries/elixir-highlights.scm")]), tree_sitter_elixir::INJECTIONS_QUERY)),
    // `.mli` stays with the fallback lexer: the crate's one highlight
    // query names nodes the interface grammar does not have.
    entry!(&["ml"], "ocaml", || configured(tree_sitter_ocaml::LANGUAGE_OCAML.into(), "ocaml", &layered(&[tree_sitter_ocaml::HIGHLIGHTS_QUERY, include_str!("queries/ocaml-highlights.scm")]), "")),
    entry!(&["m", "mm"], "objc", || configured(tree_sitter_objc::LANGUAGE.into(), "objc", &layered(&[tree_sitter_c::HIGHLIGHT_QUERY, tree_sitter_objc::HIGHLIGHTS_QUERY, include_str!("queries/c-highlights.scm")]), tree_sitter_objc::INJECTIONS_QUERY)),
    entry!(&["nix"], "nix", || configured(tree_sitter_nix::LANGUAGE.into(), "nix", tree_sitter_nix::HIGHLIGHTS_QUERY, tree_sitter_nix::INJECTIONS_QUERY)),
    entry!(&["sql"], "sql", || configured(tree_sitter_sequel::LANGUAGE.into(), "sql", tree_sitter_sequel::HIGHLIGHTS_QUERY, "")),
    entry!(&["diff", "patch"], "diff", || configured(tree_sitter_diff::LANGUAGE.into(), "diff", tree_sitter_diff::HIGHLIGHTS_QUERY, "")),
    entry!(&["ps1", "psm1", "psd1"], "powershell", || configured(tree_sitter_powershell::LANGUAGE.into(), "powershell", tree_sitter_powershell::HIGHLIGHTS_QUERY, "")),
    entry!(&["properties"], "properties", || configured(tree_sitter_properties::LANGUAGE.into(), "properties", tree_sitter_properties::HIGHLIGHTS_QUERY, "")),
    entry!(&["cmake", "cmakelists.txt"], "cmake", || configured(tree_sitter_cmake::LANGUAGE.into(), "cmake", tree_sitter_cmake::HIGHLIGHTS_QUERY, tree_sitter_cmake::INJECTIONS_QUERY)),
    entry!(&["xml", "svg", "xsd", "plist"], "xml", || configured(tree_sitter_xml::LANGUAGE_XML.into(), "xml", tree_sitter_xml::XML_HIGHLIGHT_QUERY, "")),
];

/// The grammar for a path, `None` where no entry claims it. An entry
/// matches by extension, or — for the entries that carry a dot
/// themselves (`cmakelists.txt`) — by the whole file name. A name with
/// no extension matches nothing: a wrapper script called `go` is not Go.
pub(super) fn for_path(path: &str) -> Option<&'static Lang> {
    entry_for(path).and_then(lang_of)
}

/// Whether an entry claims this path, without building anything.
pub(super) fn claims(path: &str) -> bool {
    entry_for(path).is_some()
}

fn entry_for(path: &str) -> Option<&'static Entry> {
    // git's paths use `/` alone; the fallback set splits the same way.
    let name = path.rsplit('/').next().unwrap_or(path);
    let ext = name.rsplit_once('.').map(|(_, e)| e);
    ENTRIES.iter().find(|e| {
        e.matches.iter().any(|m| {
            ext.is_some_and(|ext| m.eq_ignore_ascii_case(ext))
                || (m.contains('.') && m.eq_ignore_ascii_case(name))
        })
    })
}

/// The configuration an injection names — markdown's inline grammar,
/// the language of a fenced code block. A name is a language's own
/// (`javascript`) or one of the extensions it claims (`js`), in any case,
/// as a fence's info string names it. Unknown names read as plain text,
/// which is what an unhighlightable fence should look like.
pub(super) fn injection(name: &str) -> Option<&'static HighlightConfiguration> {
    ENTRIES
        .iter()
        .find(|e| {
            e.name.eq_ignore_ascii_case(name)
                || e.matches.iter().any(|m| m.eq_ignore_ascii_case(name))
        })
        .and_then(lang_of)
        .map(|lang| &lang.config)
}

fn lang_of(entry: &'static Entry) -> Option<&'static Lang> {
    entry
        .cell
        .get_or_init(|| (entry.build)().map(|config| Lang { config }))
        .as_ref()
}

#[cfg(test)]
mod tests {
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
    /// enough that any query which paints at all paints something here.
    const SAMPLES: [(&str, &str); 37] = [
        ("rust", "// note\nfn main() { let s = \"hi\"; }"),
        ("qml", "// note\nItem { property string s: \"hi\" }"),
        ("kotlin", "// note\nfun main() { val s = \"hi\" }"),
        ("markdown", "# Title\n\nsome text"),
        ("markdown_inline", "some `code` and a [link](x)"),
        ("toml", "# note\nkey = \"value\""),
        ("yaml", "# note\nkey: value"),
        ("json", "{\"key\": \"value\"}"),
        ("javascript", "// note\nconst s = \"hi\";"),
        ("typescript", "// note\nconst s: string = \"hi\";"),
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

    /// A query being accepted is not painting: a delta query (see
    /// [`layered`]) is colourless without its bases. Every entry answers
    /// for its own sample, so a language added without its bases fails
    /// here.
    #[test]
    fn every_grammar_colours_a_sample_of_its_language() {
        for entry in &ENTRIES {
            let (_, sample) = SAMPLES
                .iter()
                .find(|(name, _)| *name == entry.name)
                .unwrap_or_else(|| panic!("grammar {} has no sample to answer for", entry.name));
            let lang = lang_of(entry).expect("the guard above builds every configuration");
            let mut highlighter = Highlighter::new();
            // The callback through a closure: as a fn item it answers
            // `&'static`, which would lend the highlighter itself for
            // `'static` (`tree::line_spans` does the same).
            let events = highlighter
                .highlight(&lang.config, sample.as_bytes(), None, None, |name| {
                    injection(name)
                })
                .expect("a grammar reads its own language");
            // A row may paint plain (`character.special`): only another
            // colour tells that the query paints.
            assert!(
                events.flatten().any(|event| matches!(
                    event,
                    HighlightEvent::HighlightStart(h) if style(h.0).is_some_and(|c| c != PLAIN)
                )),
                "grammar {} left its own sample uncoloured",
                entry.name
            );
        }
    }

    /// Every language a path reaches answers for its colours in `tree`'s
    /// role snapshots, through a sample named for the entry
    /// (`<name>.<extension>.sample`).
    #[test]
    fn every_claimed_language_has_a_role_sample() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/highlight/tree/samples");
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

    /// The colour the grammar called `name` paints `needle`'s first
    /// occurrence in `text` — one colour across all of it.
    fn colour_of(name: &str, text: &str, needle: &str) -> Rgb {
        let entry = ENTRIES
            .iter()
            .find(|e| e.name == name)
            .unwrap_or_else(|| panic!("no grammar is called {name}"));
        let lang = lang_of(entry).expect("every entry builds its configuration");
        let at = text.find(needle).expect("the needle is in the text");
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
    /// the node back (see [`layered`]).
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

    /// A pattern whose predicate tree-sitter-highlight does not evaluate
    /// claims nothing, rather than every node of its shape: objc's
    /// `#has-ancestor?` property and zig's `#lua-match?` constant would
    /// otherwise take every identifier.
    #[test]
    fn a_predicate_the_highlighter_skips_claims_nothing() {
        assert_colours(&[
            ("objc", "void f() { NSLog(@\"hi\"); }", "NSLog", FUNCTION),
            ("zig", "const Point = struct { x: i32 };", "Point", TYPE),
        ]);
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
        let cs = "static class T\n{\n    extension(string s)\n    {\n        public int N => 1;\n    }\n}";
        assert_colours(&[("c_sharp", cs, "extension", KEYWORD)]);
    }

    /// A crate in the older capture vocabulary paints as one in the
    /// current: its keywords (`@conditional`, `@include`, `@storageclass`,
    /// `@type.qualifier`) reach the table renamed ([`current_names`]).
    #[test]
    fn a_capture_in_the_older_vocabulary_takes_its_colour() {
        let objc = "#include <stdio.h>\nstatic volatile int n;\nvoid f(void) { @try { } @catch (id e) { } }\n";
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
}
