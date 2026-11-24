# Report for tree-sitter (not sent)

**Title:** highlight: a host capture that begins at an injected capture's first byte nests inside it

tree-sitter-highlight 0.27.2. `HighlightIterLayer::sort_key` keys a layer's next event by
`(offset, is_start, depth)`, its local `depth` being `-(self.depth as isize)`, and the layers
are kept in ascending order of the key, so of two starts at one byte the deeper layer's — the
injected language's — comes first. The host's capture then opens inside the injected one.
`HighlightEvent::HighlightEnd` names no highlight, so a consumer that keeps a stack (the CLI's
renderers do) pops the host's capture at the end of the injected one: the host's highlight
stops there, and the injected capture's runs on to the end of the host capture.

**Reproduction.** tree-sitter-md 0.5.4's block and inline grammars with their own
`highlights.scm` and `injections.scm`, the injection names resolved to them and to
tree-sitter-bash 0.25's own configuration, with `none` among the recognised highlight names
(the editors' name for no highlight, which the block query lays on a fence's code):

````markdown
```sh
echo hello world
```
````

The bash layer's `(command_name) @function` and the block layer's
`(code_fence_content) @none` begin at one byte. `echo`'s start comes first, so `none` opens
inside it, and `echo`'s `HighlightEnd` pops `none`: ` hello world`, which the bash query
leaves uncaptured, is painted as the function until the fence ends.

The same order nests a heading inside its own inline content once the inline injection keeps
its content's children (`(#set! injection.include-children)`; without it tree-sitter-highlight
cuts the code span's backticks and a link's brackets out of the inline grammar's text). In
`` # `code` rest ``, the code span (`text.literal`) and its opening delimiter begin at the
first byte of the heading's `(inline) @text.title` and open first; the delimiter's
`HighlightEnd` pops the heading, and ` rest` is painted as code.

**Expected.** The outer capture opens first: ` hello world` under `none`, ` rest` under
`text.title`.

**A fix that works downstream.** Starts at one byte sort shallow-first and ends keep
deep-first: in `sort_key`, `Some((start, true, -depth))` and `Some((i, true, -depth))` (the
layer's own depth, ascending), the ends' `Some((end, false, depth))` and
`Some((j, false, depth))` as they are, and the comment above it to match. The highlight tests
of 23 grammars (2965 assertions, judged as `tree-sitter test` judges them) give the same
verdicts with it as without.
