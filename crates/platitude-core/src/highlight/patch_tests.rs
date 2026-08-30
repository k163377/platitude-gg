//! `patch`'s tests, split out for length alone (structure.md §分割).

use super::super::testkit::patches;
use super::*;
/// One changed line of Rust between two context lines, in the
/// ordinary unified form.
const RUST: &str = "\
diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,3 @@
 fn main() {
-    let x = 1;
+    let x = 2;
 }
";

#[test]
fn context_lines_are_taken_apart() {
    let colors = colors(&patches(RUST), None);
    // ` fn main() {` is the first line of the only hunk.
    let context = colors.line(0, 0, 0);
    assert!(
        context.spans.len() > 1,
        "a line with a keyword in it is not one colour: {context:?}"
    );
    let total: usize = context.spans.iter().map(|s| s.len).sum();
    assert_eq!(total, "fn main() {".len(), "runs cover the whole line");
}

#[test]
fn changed_lines_are_taken_apart_too() {
    // Both sides wear the theme (デザイン規約 §シンタックスハイライト): what names a
    // changed line is its background and weight, not a missing colour.
    let colors = colors(&patches(RUST), None);
    assert!(colors.line(0, 0, 1).spans.len() > 1, "deleted line");
    assert!(colors.line(0, 0, 2).spans.len() > 1, "added line");
}

#[test]
fn a_patch_of_pure_additions_is_coloured() {
    // A new file is all additions and has no other side at all.
    let patch = "\
diff --git a/src/new.rs b/src/new.rs
--- /dev/null
+++ b/src/new.rs
@@ -0,0 +1,2 @@
+fn main() {
+}
";
    let colors = colors(&patches(patch), None);
    assert!(colors.line(0, 0, 0).spans.len() > 1);
}

#[test]
fn a_deletion_does_not_bleed_into_the_added_lines() {
    // The deleted line opens a comment. Fed through one shared walk —
    // the old reading — everything after it came out painted as the
    // inside of a comment that is not in the file; the fork keeps the
    // sides apart.
    let patch = "\
diff --git a/src/a.rs b/src/a.rs
--- a/src/a.rs
+++ b/src/a.rs
@@ -1,3 +1,3 @@
 fn a() {}
-/* opened and never closed
+let x = 1;
 fn b() {}
";
    let colors = colors(&patches(patch), None);
    assert_eq!(
        colors.line(0, 0, 1).spans.len(),
        1,
        "the deleted line itself is the start of a comment"
    );
    assert!(
        colors.line(0, 0, 2).spans.len() > 1,
        "the added line is code, not the inside of what the deletion opened: {:?}",
        colors.line(0, 0, 2)
    );
    assert!(
        colors.line(0, 0, 3).spans.len() > 1,
        "and neither is the context after it"
    );
}

/// A hunk in the middle of a block comment: read from the top of the
/// file its context line is a comment, read cold it is code. The line
/// the diff shows is the same line either way — only the walk that
/// reached it differs.
const INSIDE_A_COMMENT: &str = "\
diff --git a/src/a.rs b/src/a.rs
--- a/src/a.rs
+++ b/src/a.rs
@@ -2,2 +2,2 @@
 let x = 1;
-let y = 2;
+let y = 3;
";
const COMMENTED_OUT: &str = "/* an old idea:\nlet x = 1;\nlet y = 3;\n*/\nfn main() {}\n";

#[test]
fn the_file_is_walked_into_the_hunk() {
    let colors = colors(&patches(INSIDE_A_COMMENT), Some(COMMENTED_OUT));
    let line = colors.line(0, 0, 0);
    assert_eq!(
        line.spans.len(),
        1,
        "line 2 of that file is inside a comment, whatever it says: {line:?}"
    );
}

#[test]
fn without_the_file_a_hunk_starts_cold() {
    // The same patch, and the difference is the whole point of
    // fetching the file: cold, the lexer has no idea it is inside
    // anything and reads a statement.
    let colors = colors(&patches(INSIDE_A_COMMENT), None);
    assert!(colors.line(0, 0, 0).spans.len() > 1);
}

#[test]
fn a_file_that_disagrees_with_the_hunk_is_not_used() {
    // Saved between the diff and the read: line 2 is not what the
    // hunk says is there, so walking it would place the reading
    // somewhere the diff never was.
    let elsewhere = "/* an old idea:\nsomething else entirely\n*/\nfn main() {}\n";
    let colors = colors(&patches(INSIDE_A_COMMENT), Some(elsewhere));
    assert!(
        colors.line(0, 0, 0).spans.len() > 1,
        "fell back to the cold read rather than trusting the wrong file"
    );
}

#[test]
fn the_walk_carries_across_hunks() {
    // A comment opened above the first hunk is still open in the
    // second: the state the second hunk starts from came down the
    // whole file, changed lines included.
    let patch = "\
diff --git a/src/a.rs b/src/a.rs
--- a/src/a.rs
+++ b/src/a.rs
@@ -1,3 +1,3 @@
 /* opened here
-one
+two
@@ -6,3 +6,3 @@
 still inside
-three
+four
";
    let source = "/* opened here\ntwo\nmid\nmid\nmid\nstill inside\nfour\nmid\n*/\n";
    let colors = colors(&patches(patch), Some(source));
    let first = colors.line(0, 0, 0);
    let second = colors.line(0, 1, 0);
    assert_eq!(second.spans.len(), 1, "one run, the comment's: {second:?}");
    assert_eq!(
        first.spans.first().map(|s| s.color),
        second.spans.first().map(|s| s.color),
        "the same comment colour the region opened with"
    );
}

#[test]
fn the_budget_bounds_what_one_patch_may_spend() {
    // A rewrite that keeps one context line at each edge. Colouring
    // the far one would mean lexing everything between them, so past
    // the budget it goes out plain instead.
    let mut patch = String::from("diff --git a/big.rs b/big.rs\n--- a/big.rs\n+++ b/big.rs\n");
    let n = 30usize;
    patch.push_str(&format!("@@ -1,{0} +1,{0} @@\n", n + 2));
    patch.push_str(" fn top() {}\n");
    for i in 0..n {
        patch.push_str(&format!("-let a{i} = 1;\n"));
    }
    for i in 0..n {
        patch.push_str(&format!("+let a{i} = 2;\n"));
    }
    patch.push_str(" fn bottom() {}\n");
    let parsed = patches(&patch);
    let assets = assets();
    let syntax = syntax_for(&assets.syntaxes, "big.rs").expect("Rust is in the set");
    let out = unified_colors(assets, &parsed[0], None, syntax, 20, None);
    let first = out[0].first().expect("the top context line");
    assert!(!first.spans.is_empty(), "the top line is within budget");
    let last = out[0].last().expect("the bottom context line");
    assert!(
        last.spans.is_empty(),
        "the bottom line is past the budget and goes out plain: {last:?}"
    );
}

#[test]
fn only_a_costly_diff_is_deep() {
    // Three rows at the top of a file are answered inside one pass.
    assert!(!deep(&patches(RUST)));
    // A hunk two thousand lines down a fallback-lexer language is a
    // walk somebody would wait for, so it is worth a quick pass first.
    let far = "\
diff --git a/big.groovy b/big.groovy
--- a/big.groovy
+++ b/big.groovy
@@ -2000,3 +2000,3 @@
 def main() {
-    def x = 1
+    def x = 2
";
    assert!(deep(&patches(far)));
    // The same depth in a language a grammar claims is not deep at all:
    // the whole side parses in milliseconds, so nothing is worth
    // sending ahead of the full answer.
    let far_rs = far.replace("big.groovy", "big.rs");
    assert!(!deep(&patches(&far_rs)));
}

#[test]
fn the_quick_pass_answers_now() {
    let quick = colors_quick(&patches(RUST));
    assert!(quick.line(0, 0, 0).spans.len() > 1);
}

#[test]
fn a_cached_reading_answers_the_same_colours() {
    // A fallback-lexer language (no grammar claims `.groovy`), with a
    // hunk deep enough that the walk to it crosses checkpoints. The
    // whole head of the file is one block comment, so a resumed walk
    // that lost its place would colour the hunk as code from nowhere.
    let mut source = String::from("/* opened\n");
    for i in 0..600 {
        source.push_str(&format!("filler {i}\n"));
    }
    source.push_str("*/\ndef main() {}\ndef tail = 1\n");
    let patch = "\
diff --git a/big.groovy b/big.groovy
--- a/big.groovy
+++ b/big.groovy
@@ -603,2 +603,2 @@
 def main() {}
-def old = 0
+def tail = 1
";
    let parsed = patches(patch);
    let (first, cache) = colors_cached(&parsed, Some(&source), None);
    assert!(
        cache.as_ref().is_some_and(|c| !c.states.is_empty()),
        "the walk crossed checkpoints and remembered them"
    );
    let (second, _) = colors_cached(&parsed, Some(&source), cache);
    assert_eq!(first, second, "a resumed walk answers what a full one did");
    assert!(
        first.line(0, 0, 0).spans.len() > 1,
        "line 603 is past the comment and reads as code: {:?}",
        first.line(0, 0, 0)
    );
}

#[test]
fn a_cache_over_different_text_is_not_used() {
    // The same commented-out shape as INSIDE_A_COMMENT, in a
    // fallback-lexer language so the cache is exercised at all.
    let patch = "\
diff --git a/src/a.groovy b/src/a.groovy
--- a/src/a.groovy
+++ b/src/a.groovy
@@ -2,2 +2,2 @@
 def x = 1
-def y = 2
+def y = 3
";
    let parsed = patches(patch);
    let commented = "/* an old idea:\ndef x = 1\ndef y = 3\n*/\ndef main() {}\n";
    // States remembered down the commented-out text…
    let (_, stale) = colors_cached(&parsed, Some(commented), None);
    // …must not colour a reading over text that says otherwise.
    let other = "def zero() {}\ndef x = 1\ndef y = 3\ndef b() {}\n";
    let (fresh, _) = colors_cached(&parsed, Some(other), None);
    let (with_stale, _) = colors_cached(&parsed, Some(other), stale);
    assert_eq!(fresh, with_stale);
    assert!(
        fresh.line(0, 0, 0).spans.len() > 1,
        "in this file the context line is code, not comment"
    );
}

#[test]
fn markdown_reads_through_its_grammar() {
    let patch = "\
diff --git a/notes.md b/notes.md
--- a/notes.md
+++ b/notes.md
@@ -1,2 +1,2 @@
 # A title
-old body
+new body
";
    let colors = colors(&patches(patch), None);
    let title = colors.line(0, 0, 0);
    assert!(
        title
            .spans
            .iter()
            .any(|s| s.color != super::super::grammar::PLAIN),
        "the heading wears a colour of its own: {title:?}"
    );
}

#[test]
fn qml_agrees_with_itself_wherever_the_hunk_is() {
    // The problem the regex walk existed for (measured): QML's
    // `readonly property` reads as storage keywords at file scope and
    // as plain identifiers inside an `Item {}`. A grammar parses the
    // whole file, so a hunk anywhere reads in its real context.
    let source = "\
Item {
    readonly property int a: 1
    readonly property int b: 2
}
";
    let patch = "\
diff --git a/A.qml b/A.qml
--- a/A.qml
+++ b/A.qml
@@ -2,2 +2,2 @@
 readonly property int a: 1
-readonly property int b: 3
+readonly property int b: 2
";
    let colors = colors(&patches(patch), Some(source));
    assert!(
        colors.line(0, 0, 0).spans.len() > 1,
        "a QML row is taken apart: {:?}",
        colors.line(0, 0, 0)
    );
}

#[test]
fn qml_colours_its_javascript_body_not_just_its_own_words() {
    // The qmljs crate's query is a delta over the JS/TS base queries:
    // alone it knows `property` and friends, and a real file came out
    // plain end to end (DetailsPane.qml at 8382160e).
    let patch = "\
diff --git a/A.qml b/A.qml
--- a/A.qml
+++ b/A.qml
@@ -1,3 +1,3 @@
 // a note
-Text { text: \"old\" }
+Text { text: \"new\" }
";
    let colors = colors(&patches(patch), None);
    let note = colors.line(0, 0, 0);
    assert!(
        note.spans
            .iter()
            .any(|s| s.color != super::super::grammar::PLAIN),
        "a comment wears the comment colour: {note:?}"
    );
    let added = colors.line(0, 0, 2);
    assert!(
        added
            .spans
            .iter()
            .any(|s| s.color != super::super::grammar::PLAIN),
        "a string literal wears the string colour: {added:?}"
    );
}

#[test]
fn a_file_its_grammar_has_not_learned_reads_through_the_lexer() {
    // Kotlin's `when` guards (`is Field if it.static ->`) postdate the
    // pinned grammar. The parse breaks, and past the break the
    // recovery left the added and context rows plain while deletions
    // kept their colours through the fragment (measured on a real
    // compiler file: rows 161..285 were one ERROR node with not a
    // capture inside). A source the grammar cannot parse reads through
    // the regex lexer instead, which reads line by line and does not
    // care what the whole of it means.
    let source = "\
// note
class Loader {
    val ready = items.any {
        when (it) {
            is Field if it.static && !it.late -> true
            else -> false
        }
    }

    fun label(): String {
        // a comment far below the guard
        return \"hi\"
    }
}
";
    let patch = "\
diff --git a/A.kt b/A.kt
--- a/A.kt
+++ b/A.kt
@@ -10,4 +10,4 @@
     fun label(): String {
         // a comment far below the guard
-        return \"lo\"
+        return \"hi\"
     }
";
    let colors = colors(&patches(patch), Some(source));
    let comment = colors.line(0, 0, 1);
    assert!(
        comment
            .spans
            .iter()
            .any(|s| s.color != super::super::grammar::PLAIN),
        "a comment below the unlearned syntax wears the comment colour: {comment:?}"
    );
    let added = colors.line(0, 0, 3);
    assert!(
        added
            .spans
            .iter()
            .any(|s| s.color != super::super::grammar::PLAIN),
        "an added row is taken apart too: {added:?}"
    );
}

#[test]
fn without_a_real_fallback_a_broken_file_keeps_its_grammar() {
    // A `.plist` is the XML grammar's, and syntect's set has nothing
    // for the extension — `syntax_for` answers `None`. However badly
    // the parse of one goes, the grammar's partial answer beats the
    // nothing the lexer road would say here.
    let source = "\
<!-- note -->
<plist>
  <key>one</
</plist>
";
    let patch = "\
diff --git a/p.plist b/p.plist
--- a/p.plist
+++ b/p.plist
@@ -1,3 +1,3 @@
 <!-- note -->
-<plist version=\"0\">
+<plist>
";
    let colors = colors(&patches(patch), Some(source));
    let note = colors.line(0, 0, 0);
    assert!(
        note.spans
            .iter()
            .any(|s| s.color != super::super::grammar::PLAIN),
        "the comment still wears what the grammar could read: {note:?}"
    );
}

#[test]
fn binary_patches_are_left_alone() {
    let patch = "\
diff --git a/logo.png b/logo.png
--- a/logo.png
+++ b/logo.png
Binary files a/logo.png and b/logo.png differ
";
    assert!(colors(&patches(patch), None).is_empty());
}
