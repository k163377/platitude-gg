# Modified copy of tree-sitter-highlight 0.27.2

This directory is `tree-sitter-highlight` 0.27.2 as published on crates.io (upstream commit
3fb7be49145acfb07e33b61ed5689e2b6771b2e9, `crates/highlight` of
https://github.com/tree-sitter/tree-sitter), licensed MIT (`LICENSE`).

**Modified by platitude-gg on 2026-11-24.** The only change is `platitude-layer-order.diff`
(`src/highlight.rs`, apply with `patch -p1` to the published crate); every other file of the
published crate is byte for byte as published. Added beside them: this file, the diff, and
`UPSTREAM-REPORT.md`.

What the change does: `HighlightIterLayer::sort_key` orders the layers by their next event.
Where the next events of two layers are both starts at one byte, the shallower layer's now
comes first; published, the deeper layer's — the injected language's — did. Ends at one byte
keep the deeper layer's first, as published.

Why: a host capture that begins at the byte an injected language's capture begins at opened
inside it, and `HighlightEvent::HighlightEnd` names no highlight, so the end of the injected
capture closed the host's: the host's colour stopped there and the injected one ran on to the
end of the host capture. In platitude-gg's diff pane a Markdown heading that opens with inline
code or a link lost the heading colour after it. The highlight tests of the grammars
platitude-gg ships give the same verdicts either way.

Back to the published crate when a tree-sitter-highlight release opens the outer of two
captures that begin at one byte first, however it orders them: then
`highlight::grammar::tests::an_injected_capture_at_a_host_capture_s_first_byte_nests_inside_it`
passes against it with this directory gone. Remove the directory and the `[patch.crates-io]`
entry in the workspace `Cargo.toml`, and run it on Windows and Linux.
