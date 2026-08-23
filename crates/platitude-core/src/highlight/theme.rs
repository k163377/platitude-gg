//! The syntax set and the palette it is highlighted against.

use std::sync::OnceLock;

use syntect::highlighting::{Color, StyleModifier, Theme, ThemeItem, ThemeSettings};
use syntect::parsing::{SyntaxReference, SyntaxSet};

/// The colour a run gets when no rule below claims it — identifiers,
/// parameters, punctuation. **The app's own `textPrimary`**: code is the
/// thing on this screen worth reading, and anything dimmer than the
/// window's own words reads as though it were not
/// (デザイン規約 §シンタックスハイライト).
const PLAIN: u32 = 0xE2E8F0;

/// The same colour as [`PLAIN`], in the form the grammar path paints
/// with (`grammar::PLAIN` re-exports it).
pub(super) const PLAIN_RGB: super::Rgb = super::Rgb::of(PLAIN);

// The rest of デザイン規約 §シンタックスハイライト's table, one const per
// row of it. Two arrays name these — `PALETTE` in syntect's scopes,
// `grammar::CAPTURES` in tree-sitter's captures — and the two
// vocabularies differ by design; the values do not, so they are written
// once here and referred to from both. Written twice they would drift
// silently, and one file would then wear two colourings: a path a
// grammar claims still reads through syntect when it is conflicted
// (`patch::patch_colors` takes the combined branch first).

/// Context, not content — the colour the app gives every secondary
/// word. Emphatically not `textMuted`: dimming text is how this app
/// says "disabled" (規約 §無効), and a comment is not disabled.
pub(super) const COMMENT: u32 = 0x94A3B8;

/// The words that make it a language.
pub(super) const KEYWORD: u32 = 0x60A5FA;

/// What things are.
pub(super) const TYPE: u32 = 0x7DD3FC;

/// What things do.
pub(super) const FUNCTION: u32 = 0xA78BFA;

/// What is written down literally.
pub(super) const STRING: u32 = 0xFCD34D;

/// Numbers, and the constants a language names itself — `true`, `null`.
pub(super) const CONSTANT: u32 = 0xF0ABFC;

/// What is said *about* the code — annotations, attributes, macros.
pub(super) const ANNOTATION: u32 = 0xFDA4AF;

/// Scope → one of the colours above, in the app's palette rather than a
/// theme's. The mapping is what this array owns; デザイン規約's table
/// owns the values (same rule as `Theme.qml`).
///
/// syntect scores selectors and takes the best match, so a broad name
/// sits safely beside a narrow one — `keyword` and `keyword.operator`
/// both belong here and the narrower one wins where it applies.
const PALETTE: [(&str, u32); 19] = [
    ("comment", COMMENT),
    ("punctuation.definition.comment", COMMENT),
    ("keyword", KEYWORD),
    ("storage", KEYWORD),
    // Keywords, but not their operators: `=` and `+` in accent blue
    // turns every line into a row of lights.
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
];

/// Whether the fallback set has a language for this path (the public
/// question, grammars included, is `patch::knows`).
pub(super) fn knows(path: &str) -> bool {
    syntax_for(&assets().syntaxes, path).is_some()
}

pub(super) struct Assets {
    pub(super) syntaxes: SyntaxSet,
    pub(super) theme: Theme,
}

/// Loaded once, on the first diff that asks — never at startup. A window
/// that is opened and closed without a file being read pays none of it,
/// and the load is inside the same background hop as the diff itself.
pub(super) fn assets() -> &'static Assets {
    static ASSETS: OnceLock<Assets> = OnceLock::new();
    ASSETS.get_or_init(|| Assets {
        syntaxes: two_face::syntax::extra_newlines(),
        theme: palette(),
    })
}

/// The app's palette as something syntect can highlight against. Built
/// rather than loaded: every published theme is drawn for its own ground
/// and its own idea of how loud code should be, and next to this window's
/// words all of them read as though the code were the caption
/// (2026-08-13 実測 — base16-ocean, Catppuccin Mocha, Dracula, Monokai
/// were all tried against the real thing).
fn palette() -> Theme {
    // Through `super::Rgb::of`, the one place `0xRRGGBB` is taken
    // apart, and then into syntect's own colour — opaque, since the
    // palette is foreground only.
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
    use super::super::testkit::patches;
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
            colors.line(0, 0, 0).spans.len() > 1,
            "Kotlin is one of the languages two-face adds to Sublime's own set"
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
