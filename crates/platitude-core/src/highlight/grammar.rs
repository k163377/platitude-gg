//! The grammars the fast path knows: which languages, how their capture
//! names map onto the app's palette, and one lazily built configuration
//! per language. A path no entry claims falls back to the regex lexer.

use std::sync::OnceLock;

use tree_sitter::Language;
use tree_sitter_highlight::HighlightConfiguration;

use super::Rgb;
use super::theme::{ANNOTATION, COMMENT, CONSTANT, FUNCTION, KEYWORD, PLAIN_RGB, STRING, TYPE};

/// The colour a run gets when no capture below claims it — the same
/// `textPrimary` the regex lexer falls back to.
pub(super) const PLAIN: Rgb = PLAIN_RGB;

/// Capture name → colour, デザイン規約 §シンタックスハイライト's table said in
/// tree-sitter's capture vocabulary. The colours themselves live in
/// `theme`, which the syntect mirror (`theme::PALETTE`) reads too, so
/// the two vocabularies cannot disagree about a value.
///
/// Only coloured names are listed: a capture nothing here matches —
/// operators, punctuation, variables — comes out [`PLAIN`]. tree-sitter
/// resolves a capture to the **longest listed dot-prefix**, so `keyword`
/// covers `keyword.control` and the narrow `constant.builtin` stands
/// beside the unlisted broad `constant`.
const CAPTURES: [(&str, u32); 20] = [
    ("comment", COMMENT),
    ("keyword", KEYWORD),
    ("storage", KEYWORD),
    ("type", TYPE),
    ("constructor", TYPE),
    ("function", FUNCTION),
    ("method", FUNCTION),
    ("string", STRING),
    ("number", CONSTANT),
    ("float", CONSTANT),
    ("boolean", CONSTANT),
    ("constant.builtin", CONSTANT),
    ("constant.numeric", CONSTANT),
    ("attribute", ANNOTATION),
    ("decorator", ANNOTATION),
    ("tag", KEYWORD),
    ("text.title", KEYWORD),
    ("text.literal", STRING),
    ("text.uri", KEYWORD),
    ("text.reference", TYPE),
];

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

/// Builds one configuration, or `None` for a query the runtime refuses
/// — that language then reads through the fallback lexer, never as an
/// error.
fn configured(
    language: Language,
    name: &'static str,
    highlights: &str,
    injections: &str,
) -> Option<HighlightConfiguration> {
    let mut config =
        HighlightConfiguration::new(language, name, highlights, injections, "").ok()?;
    let names: Vec<&str> = CAPTURES.iter().map(|(n, _)| *n).collect();
    config.configure(&names);
    Some(config)
}

/// Several highlight queries as one. Some crates ship theirs as a
/// *delta* over another language's — qmljs over JavaScript's, objc over
/// C's — and alone those colour almost nothing. The specific language
/// goes first: where two patterns claim one node, tree-sitter keeps the
/// earlier.
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
    entry!(&["qml"], "qml", || configured(tree_sitter_qmljs::LANGUAGE.into(), "qml", &layered(&[tree_sitter_qmljs::HIGHLIGHTS_QUERY, tree_sitter_typescript::HIGHLIGHTS_QUERY, tree_sitter_javascript::HIGHLIGHT_QUERY]), "")),
    entry!(&["kt", "kts"], "kotlin", || configured(tree_sitter_kotlin_ng::LANGUAGE.into(), "kotlin", include_str!("queries/kotlin-highlights.scm"), "")),
    entry!(&["md", "markdown"], "markdown", || configured(tree_sitter_md::LANGUAGE.into(), "markdown", tree_sitter_md::HIGHLIGHT_QUERY_BLOCK, tree_sitter_md::INJECTION_QUERY_BLOCK)),
    entry!(&[], "markdown_inline", || configured(tree_sitter_md::INLINE_LANGUAGE.into(), "markdown_inline", tree_sitter_md::HIGHLIGHT_QUERY_INLINE, tree_sitter_md::INJECTION_QUERY_INLINE)),
    entry!(&["toml"], "toml", || configured(tree_sitter_toml_ng::LANGUAGE.into(), "toml", tree_sitter_toml_ng::HIGHLIGHTS_QUERY, "")),
    entry!(&["yaml", "yml"], "yaml", || configured(tree_sitter_yaml::LANGUAGE.into(), "yaml", tree_sitter_yaml::HIGHLIGHTS_QUERY, "")),
    entry!(&["json", "jsonc"], "json", || configured(tree_sitter_json::LANGUAGE.into(), "json", tree_sitter_json::HIGHLIGHTS_QUERY, "")),
    entry!(&["js", "mjs", "cjs", "jsx"], "javascript", || configured(tree_sitter_javascript::LANGUAGE.into(), "javascript", tree_sitter_javascript::HIGHLIGHT_QUERY, tree_sitter_javascript::INJECTIONS_QUERY)),
    entry!(&["ts", "mts", "cts"], "typescript", || configured(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(), "typescript", &layered(&[tree_sitter_typescript::HIGHLIGHTS_QUERY, tree_sitter_javascript::HIGHLIGHT_QUERY]), "")),
    entry!(&["tsx"], "tsx", || configured(tree_sitter_typescript::LANGUAGE_TSX.into(), "tsx", &layered(&[tree_sitter_typescript::HIGHLIGHTS_QUERY, tree_sitter_javascript::JSX_HIGHLIGHT_QUERY, tree_sitter_javascript::HIGHLIGHT_QUERY]), "")),
    entry!(&["py", "pyi"], "python", || configured(tree_sitter_python::LANGUAGE.into(), "python", tree_sitter_python::HIGHLIGHTS_QUERY, "")),
    entry!(&["java"], "java", || configured(tree_sitter_java::LANGUAGE.into(), "java", tree_sitter_java::HIGHLIGHTS_QUERY, "")),
    entry!(&["c", "h"], "c", || configured(tree_sitter_c::LANGUAGE.into(), "c", tree_sitter_c::HIGHLIGHT_QUERY, "")),
    entry!(&["cpp", "cc", "cxx", "hpp", "hh", "hxx"], "cpp", || configured(tree_sitter_cpp::LANGUAGE.into(), "cpp", &layered(&[tree_sitter_cpp::HIGHLIGHT_QUERY, tree_sitter_c::HIGHLIGHT_QUERY]), "")),
    entry!(&["cs"], "c_sharp", || configured(tree_sitter_c_sharp::LANGUAGE.into(), "c_sharp", tree_sitter_c_sharp::HIGHLIGHTS_QUERY, "")),
    entry!(&["go"], "go", || configured(tree_sitter_go::LANGUAGE.into(), "go", tree_sitter_go::HIGHLIGHTS_QUERY, "")),
    entry!(&["rb"], "ruby", || configured(tree_sitter_ruby::LANGUAGE.into(), "ruby", tree_sitter_ruby::HIGHLIGHTS_QUERY, "")),
    entry!(&["php"], "php", || configured(tree_sitter_php::LANGUAGE_PHP.into(), "php", tree_sitter_php::HIGHLIGHTS_QUERY, tree_sitter_php::INJECTIONS_QUERY)),
    entry!(&["swift"], "swift", || configured(tree_sitter_swift::LANGUAGE.into(), "swift", tree_sitter_swift::HIGHLIGHTS_QUERY, tree_sitter_swift::INJECTIONS_QUERY)),
    entry!(&["html", "htm"], "html", || configured(tree_sitter_html::LANGUAGE.into(), "html", tree_sitter_html::HIGHLIGHTS_QUERY, tree_sitter_html::INJECTIONS_QUERY)),
    entry!(&["css"], "css", || configured(tree_sitter_css::LANGUAGE.into(), "css", tree_sitter_css::HIGHLIGHTS_QUERY, "")),
    entry!(&["sh", "bash", "zsh"], "bash", || configured(tree_sitter_bash::LANGUAGE.into(), "bash", tree_sitter_bash::HIGHLIGHT_QUERY, "")),
    entry!(&["scala", "sbt"], "scala", || configured(tree_sitter_scala::LANGUAGE.into(), "scala", tree_sitter_scala::HIGHLIGHTS_QUERY, "")),
    entry!(&["hs"], "haskell", || configured(tree_sitter_haskell::LANGUAGE.into(), "haskell", tree_sitter_haskell::HIGHLIGHTS_QUERY, tree_sitter_haskell::INJECTIONS_QUERY)),
    entry!(&["lua"], "lua", || configured(tree_sitter_lua::LANGUAGE.into(), "lua", tree_sitter_lua::HIGHLIGHTS_QUERY, tree_sitter_lua::INJECTIONS_QUERY)),
    entry!(&["zig"], "zig", || configured(tree_sitter_zig::LANGUAGE.into(), "zig", tree_sitter_zig::HIGHLIGHTS_QUERY, tree_sitter_zig::INJECTIONS_QUERY)),
    entry!(&["ex", "exs"], "elixir", || configured(tree_sitter_elixir::LANGUAGE.into(), "elixir", tree_sitter_elixir::HIGHLIGHTS_QUERY, tree_sitter_elixir::INJECTIONS_QUERY)),
    // `.mli` stays with the fallback lexer: the crate's one highlight
    // query names nodes the interface grammar does not have.
    entry!(&["ml"], "ocaml", || configured(tree_sitter_ocaml::LANGUAGE_OCAML.into(), "ocaml", tree_sitter_ocaml::HIGHLIGHTS_QUERY, "")),
    entry!(&["m", "mm"], "objc", || configured(tree_sitter_objc::LANGUAGE.into(), "objc", &layered(&[tree_sitter_objc::HIGHLIGHTS_QUERY, tree_sitter_c::HIGHLIGHT_QUERY]), tree_sitter_objc::INJECTIONS_QUERY)),
    entry!(&["nix"], "nix", || configured(tree_sitter_nix::LANGUAGE.into(), "nix", tree_sitter_nix::HIGHLIGHTS_QUERY, tree_sitter_nix::INJECTIONS_QUERY)),
    entry!(&["sql"], "sql", || configured(tree_sitter_sequel::LANGUAGE.into(), "sql", tree_sitter_sequel::HIGHLIGHTS_QUERY, "")),
    entry!(&["diff", "patch"], "diff", || configured(tree_sitter_diff::LANGUAGE.into(), "diff", tree_sitter_diff::HIGHLIGHTS_QUERY, "")),
    entry!(&["ps1", "psm1", "psd1"], "powershell", || configured(tree_sitter_powershell::LANGUAGE.into(), "powershell", tree_sitter_powershell::HIGHLIGHTS_QUERY, "")),
    entry!(&["properties"], "properties", || configured(tree_sitter_properties::LANGUAGE.into(), "properties", tree_sitter_properties::HIGHLIGHTS_QUERY, "")),
    entry!(&["cmake", "cmakelists.txt"], "cmake", || configured(tree_sitter_cmake::LANGUAGE.into(), "cmake", tree_sitter_cmake::HIGHLIGHTS_QUERY, tree_sitter_cmake::INJECTIONS_QUERY)),
    entry!(&["xml", "svg", "xsd", "plist"], "xml", || configured(tree_sitter_xml::LANGUAGE_XML.into(), "xml", tree_sitter_xml::XML_HIGHLIGHT_QUERY, "")),
];

/// The grammar for a path, `None` where no entry claims it (the caller
/// then falls back to the regex lexer). An entry matches by extension,
/// or — for the entries that carry a dot themselves (`cmakelists.txt`)
/// — by the whole file name. A name with no extension matches nothing:
/// a wrapper script that happens to be called `go` is not Go.
pub(super) fn for_path(path: &str) -> Option<&'static Lang> {
    entry_for(path).and_then(lang_of)
}

/// Whether an entry claims this path — without building anything.
/// Asked wherever only the fork matters: the quick-pass decision, and
/// whether a file is worth fetching at all.
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
/// the language of a fenced code block. Unknown names read as plain
/// text, which is what an unhighlightable fence should look like.
pub(super) fn injection(name: &str) -> Option<&'static HighlightConfiguration> {
    ENTRIES
        .iter()
        .find(|e| e.name == name)
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
    /// little enough to read, enough that any query which paints at all
    /// paints something here.
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

    /// The guard above says a query was accepted, which is not the same
    /// as a query that paints: several upstream crates ship their
    /// highlights as a *delta* over another language's — colourless on
    /// their own (observed: qmljs, typescript, tsx, cpp and objc all
    /// did, and a QML diff came out plain end to end). Every entry
    /// answers for its own sample here, so a language added without its
    /// bases fails in this crate rather than on screen.
    #[test]
    fn every_grammar_colours_a_sample_of_its_language() {
        for entry in &ENTRIES {
            let (_, sample) = SAMPLES
                .iter()
                .find(|(name, _)| *name == entry.name)
                .unwrap_or_else(|| panic!("grammar {} has no sample to answer for", entry.name));
            let lang = lang_of(entry).expect("the guard above builds every configuration");
            let mut highlighter = Highlighter::new();
            // The callback through a closure, not by name: as a fn item
            // it answers `&'static`, which would lend the highlighter
            // itself for `'static` (`tree::line_spans` does the same).
            let events = highlighter
                .highlight(&lang.config, sample.as_bytes(), None, |name| {
                    injection(name)
                })
                .expect("a grammar reads its own language");
            // `configure` was given the palette's names and no others, so
            // any highlight at all is one of its colours.
            assert!(
                events
                    .flatten()
                    .any(|event| matches!(event, HighlightEvent::HighlightStart(_))),
                "grammar {} left its own sample uncoloured",
                entry.name
            );
        }
    }

    #[test]
    fn paths_resolve_by_extension_and_by_whole_name() {
        assert!(claims("src/main.rs"));
        assert!(claims("ui/DetailsPane.qml"));
        assert!(claims("A.kt"));
        assert!(claims("tools/CMakeLists.txt"));
        assert!(claims("デザイン規約.md"));
        assert!(!claims("notes.qqq"));
        // A name with no extension is nobody's: a wrapper script that
        // happens to be called `go` is not Go.
        assert!(!claims("scripts/go"));
        // Groovy's crate ships no queries, so gradle scripts read
        // through the fallback lexer.
        assert!(!claims("build.gradle"));
    }
}
