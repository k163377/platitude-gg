//! Values a sentence or an `Image` reads directly: byte sizes, data
//! URLs, the pieces of the line-ending notice, and how a rename says
//! where it came from.

use base64::Engine as _;

/// `data:` URL a QML `Image` loads directly — no temp files, no image
/// providers, and blob content works the same as working-tree content.
pub fn image_data_url(mime: &str, bytes: &[u8]) -> String {
    let mut out = format!("data:{mime};base64,");
    base64::engine::general_purpose::STANDARD.encode_string(bytes, &mut out);
    out
}

/// A line-ending notice taken apart into the pieces its sentence
/// (`Words.lineEndings`) needs.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EndingWords {
    /// `""` (nothing to say) / `flipped` / `mixed` / `new` / `first`.
    pub kind: String,
    /// The two endings in the order the sentence names them.
    pub from: String,
    pub to: String,
    pub lines: i32,
    /// How far the sample reached: `""` / `here` / `ext` / `repo`.
    pub scope: String,
    pub ext: String,
}

pub fn ending_words(notice: Option<&platitude_core::eol::Notice>) -> EndingWords {
    use platitude_core::eol::{Notice, Scope};
    let mut out = EndingWords::default();
    let Some(notice) = notice else { return out };
    let (kind, from, to) = match notice {
        Notice::Flipped { from, to } => ("flipped", *from, *to),
        Notice::Mixed {
            lines, added, file, ..
        } => {
            out.lines = i32::try_from(*lines).unwrap_or(i32::MAX);
            ("mixed", *added, *file)
        }
        Notice::NewFile { eol, baseline } => ("new", *eol, baseline.eol),
        Notice::FirstEnding { eol, baseline } => ("first", *eol, baseline.eol),
    };
    let baseline = match notice {
        Notice::NewFile { baseline, .. } | Notice::FirstEnding { baseline, .. } => Some(baseline),
        _ => None,
    };
    if let Some(baseline) = baseline {
        let (scope, ext) = match &baseline.scope {
            Scope::Here(ext) => ("here", ext.as_str()),
            Scope::Ext(ext) => ("ext", ext.as_str()),
            Scope::Repo => ("repo", ""),
        };
        out.scope = scope.to_string();
        out.ext = ext.to_string();
    }
    out.kind = kind.to_string();
    out.from = from.as_str().to_string();
    out.to = to.as_str().to_string();
    out
}

/// How a rename's source is written beside the new name.
///
/// The old path is cut back exactly as far as the new one is: a file that
/// moved inside its own directory shows two bare names, and one that came
/// from somewhere else keeps the path that says where. `cut` is how many
/// bytes of the new path its row does not have to spell — the folders
/// above it already do (a flat list passes 0, and both names stay whole).
pub fn rename_source<'a>(orig_path: &'a str, path: &str, cut: usize) -> &'a str {
    if cut == 0 || orig_path.is_empty() {
        return orig_path;
    }
    let Some(prefix) = path.get(..cut) else {
        return orig_path;
    };
    orig_path.strip_prefix(prefix).unwrap_or(orig_path)
}

/// Human-readable byte size ("67 B", "1.5 KB", "234 KB", "1.2 MB").
/// 1024-based; one decimal below ten.
pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = UNITS[0];
    for u in UNITS {
        value /= 1024.0;
        unit = u;
        if value < 1024.0 {
            break;
        }
    }
    if value < 10.0 {
        format!("{value:.1} {unit}")
    } else {
        format!("{value:.0} {unit}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rename_inside_one_directory_drops_the_prefix_both_names_share() {
        let path = "docs/a/new.txt";
        let cut = path.len() - "new.txt".len();
        assert_eq!(rename_source("docs/a/old.txt", path, cut), "old.txt");
    }

    #[test]
    fn a_rename_from_elsewhere_keeps_the_path_that_says_where() {
        let path = "docs/b/new.txt";
        let cut = path.len() - "new.txt".len();
        assert_eq!(rename_source("docs/a/old.txt", path, cut), "docs/a/old.txt");
    }

    #[test]
    fn a_flat_list_leaves_both_names_whole() {
        assert_eq!(
            rename_source("docs/a/old.txt", "docs/a/new.txt", 0),
            "docs/a/old.txt"
        );
    }

    #[test]
    fn a_cut_that_lands_inside_a_character_changes_nothing() {
        // The offset comes from the row's own name, so this cannot
        // happen — but a panic here would take the whole list down.
        let path = "文/new.txt";
        assert_eq!(rename_source("文/old.txt", path, 1), "文/old.txt");
    }

    #[test]
    fn image_data_urls_are_base64_with_the_mime_up_front() {
        assert_eq!(
            image_data_url("image/png", b"abc"),
            "data:image/png;base64,YWJj"
        );
        assert_eq!(image_data_url("image/gif", b""), "data:image/gif;base64,");
    }

    #[test]
    fn human_sizes_step_through_units() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(1023), "1023 B");
        assert_eq!(human_size(1024), "1.0 KB");
        assert_eq!(human_size(1536), "1.5 KB");
        assert_eq!(human_size(239_616), "234 KB");
        assert_eq!(human_size(1_258_291), "1.2 MB");
        assert_eq!(human_size(17 * 1024 * 1024 * 1024), "17 GB");
    }
}
