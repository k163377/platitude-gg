//! The syntax set and the palette it is highlighted against.

use std::sync::OnceLock;

use syntect::highlighting::{Color, StyleModifier, Theme, ThemeItem, ThemeSettings};
use syntect::parsing::{SyntaxReference, SyntaxSet};

/// The colour a run gets when no rule below claims it: the app's own
/// `textPrimary` (デザイン規約 §シンタックスハイライト).
pub(super) const PLAIN: u32 = 0xE2E8F0;

/// The same colour as [`PLAIN`], in the form the grammar path paints
/// with (`grammar::PLAIN` re-exports it).
pub(super) const PLAIN_RGB: super::Rgb = super::Rgb::of(PLAIN);

// The rest of デザイン規約 §シンタックスハイライト's table, one const per
// row, written once for both `PALETTE` (syntect scopes) and
// `grammar::CAPTURES` (tree-sitter captures): a conflicted file of a
// grammar language reads through syntect, so two copies drifting would
// give one file two colourings.

/// `textSecondary`, never dimmer — dimming says disabled
/// (デザイン規約 §シンタックスハイライト).
pub(super) const COMMENT: u32 = 0x94A3B8;

pub(super) const KEYWORD: u32 = 0x60A5FA;

pub(super) const TYPE: u32 = 0x7DD3FC;

pub(super) const FUNCTION: u32 = 0xA78BFA;

pub(super) const STRING: u32 = 0xFCD34D;

/// Numbers, and the constants a language names itself — `true`, `null`.
pub(super) const CONSTANT: u32 = 0xF0ABFC;

/// What is said *about* the code — annotations, attributes, macros.
pub(super) const ANNOTATION: u32 = 0xFDA4AF;

/// Scope → one of the colours above, in the app's palette. The
/// mapping is what this array owns; the values are the consts above,
/// which デザイン規約 §シンタックスハイライト's table quotes by hand
/// (`cargo xtask docs` does not check that table).
///
/// syntect takes the best-scoring selector, so `keyword` and
/// `keyword.operator` sit side by side and the narrower one wins.
///
/// A row naming one language's scope (`.cs`) answers where that syntax
/// gives a type word a keyword's scope, as C#'s does: `int` is the bare
/// `storage.type.cs` while `class`, `enum` and `var` take a narrower
/// name, and `=>` is a `storage.type.function` (`.lambda` in a lambda).
/// Only the `class`, `struct` and `interface` of a `where` constraint
/// share `int`'s scope, under the same outer scopes as a parameter's
/// type, and take the type colour. Where one scope carries both (C's `int` and
/// `struct`, Rust's `i32` and `let`), no row tells them apart, and the
/// type word stays in the keyword colour.
const PALETTE: [(&str, u32); 22] = [
    ("comment", COMMENT),
    ("punctuation.definition.comment", COMMENT),
    ("keyword", KEYWORD),
    ("storage", KEYWORD),
    // Operators and punctuation stay plain: with `=` and `.` in the keyword
    // colour a line reads as a row of lamps.
    ("keyword.operator", PLAIN),
    ("punctuation", PLAIN),
    ("entity.name.type", TYPE),
    ("entity.name.class", TYPE),
    ("entity.other.inherited-class", TYPE),
    ("support.type", TYPE),
    ("support.class", TYPE),
    ("entity.name.function", FUNCTION),
    ("support.function", FUNCTION),
    ("variable.function", FUNCTION),
    ("string", STRING),
    ("constant.numeric", CONSTANT),
    ("constant.language", CONSTANT),
    ("meta.annotation", ANNOTATION),
    ("variable.annotation", ANNOTATION),
    ("storage.type.cs", TYPE),
    ("storage.type.function.cs", PLAIN),
    ("storage.type.function.lambda.cs", PLAIN),
];

/// Whether the fallback set has a language for this path (the public
/// question, grammars included, is `patch::knows`).
pub(super) fn knows(path: &str) -> bool {
    syntax_for(&assets().syntaxes, path).is_some()
}

/// Whether the set has a syntax of its own for this path, past the
/// plain-text one, which colours nothing — whether a file that outgrew
/// its grammar is worth handing to the lexer (`patch::patch_colors`).
pub(super) fn reads(path: &str) -> bool {
    let syntaxes = &assets().syntaxes;
    syntax_for(syntaxes, path)
        .is_some_and(|syntax| syntax.name != syntaxes.find_syntax_plain_text().name)
}

pub(super) struct Assets {
    pub(super) syntaxes: SyntaxSet,
    pub(super) theme: Theme,
}

/// Loaded once, on the first diff that asks, inside that diff's
/// background hop.
pub(super) fn assets() -> &'static Assets {
    static ASSETS: OnceLock<Assets> = OnceLock::new();
    ASSETS.get_or_init(|| Assets {
        syntaxes: two_face::syntax::extra_newlines(),
        theme: palette(),
    })
}

/// The app's palette as a syntect theme; published themes are rejected:
/// each paints ordinary text a step darker than `textPrimary` (base16-ocean,
/// Catppuccin Mocha, Dracula, Monokai Extended alike), so the code reads as
/// an accessory of the UI (user decision — ask before changing). So syntect
/// goes without its `default-themes` feature.
fn palette() -> Theme {
    // Opaque: the palette is foreground only.
    let color = |rgb: u32| {
        let super::Rgb { r, g, b } = super::Rgb::of(rgb);
        Color { r, g, b, a: 0xff }
    };
    Theme {
        name: Some("platitude".to_string()),
        settings: ThemeSettings {
            foreground: Some(color(PLAIN)),
            ..ThemeSettings::default()
        },
        scopes: PALETTE
            .iter()
            .filter_map(|(selector, rgb)| {
                Some(ThemeItem {
                    scope: selector.parse().ok()?,
                    style: StyleModifier {
                        foreground: Some(color(*rgb)),
                        background: None,
                        font_style: None,
                    },
                })
            })
            .collect(),
        ..Theme::default()
    }
}

/// The language for a path, or `None` for a file the set does not know.
pub(super) fn syntax_for<'a>(syntaxes: &'a SyntaxSet, path: &str) -> Option<&'a SyntaxReference> {
    let name = path.rsplit('/').next().unwrap_or(path);
    // Extension first, then the whole name: Sublime's definitions list
    // `Makefile` and `Dockerfile` among their extensions, and a dotfile
    // (`.gitignore`) reads as an extension already.
    let ext = name.rsplit_once('.').map_or(name, |(_, e)| e);
    syntaxes
        .find_syntax_by_extension(ext)
        .or_else(|| syntaxes.find_syntax_by_extension(name))
}

#[cfg(test)]
mod tests {
    use super::super::colors;
    use super::super::testkit::{at, patches};
    #[test]
    fn kotlin_is_in_the_set() {
        let patch = "\
diff --git a/A.kt b/A.kt
--- a/A.kt
+++ b/A.kt
@@ -1,2 +1,2 @@
 fun a(): Int = 1
-fun b(): Int = 2
+fun b(): Int = 3
";
        let colors = colors(&patches(patch), None);
        assert!(
            colors.line(at(0, 0, 0)).spans.len() > 1,
            "Kotlin is one of the languages two-face adds to Sublime's own set"
        );
    }

    /// A C# file its grammar cannot read reads through the lexer, which
    /// paints type words and `=>` as the grammar would (`PALETTE`'s `.cs`
    /// rows), and declaration words still as keywords.
    #[test]
    fn csharp_through_the_lexer_paints_types_as_its_grammar_does() {
        use super::super::{Rgb, grammar, tree};
        let source = "\
class Counter
{
    int Count(string s) => s.Length;
    Func<int, int> twice = x => x * 2;
    void Run() { int x = ; }
}
";
        let lang = grammar::for_path("A.cs").expect("C# has a grammar");
        assert!(
            !tree::reads(lang, source),
            "the source must send the file to the lexer"
        );
        let patch = "\
diff --git a/A.cs b/A.cs
--- a/A.cs
+++ b/A.cs
@@ -1,6 +1,6 @@
 class Counter
 {
-    int Count(string s) => s.Size;
+    int Count(string s) => s.Length;
     Func<int, int> twice = x => x * 2;
     void Run() { int x = ; }
 }
";
        let colors = colors(&patches(patch), Some(source));
        let colour_of = |line: usize, text: &str, needle: &str| {
            let start = text.find(needle).expect("the needle is on the line");
            let mut end = 0;
            colors
                .line(at(0, 0, line))
                .spans
                .iter()
                .find(|span| {
                    end += span.len;
                    start < end
                })
                .map(|span| span.color)
        };
        let added = "    int Count(string s) => s.Length;";
        assert_eq!(colour_of(3, added, "int"), Some(Rgb::of(super::TYPE)));
        assert_eq!(colour_of(3, added, "string"), Some(Rgb::of(super::TYPE)));
        assert_eq!(colour_of(3, added, "=>"), Some(Rgb::of(super::PLAIN)));
        let lambda = "    Func<int, int> twice = x => x * 2;";
        assert_eq!(colour_of(4, lambda, "=>"), Some(Rgb::of(super::PLAIN)));
        assert_eq!(
            colour_of(0, "class Counter", "class"),
            Some(Rgb::of(super::KEYWORD))
        );
    }

    #[test]
    fn a_language_nobody_knows_gets_no_colour() {
        let patch = "\
diff --git a/notes.qqq b/notes.qqq
--- a/notes.qqq
+++ b/notes.qqq
@@ -1,2 +1,2 @@
 zero
-one
+two
";
        assert!(colors(&patches(patch), None).is_empty());
    }
}
