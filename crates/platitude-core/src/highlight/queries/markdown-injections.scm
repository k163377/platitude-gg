;; tree-sitter-md's block injections (its
;; tree-sitter-markdown/queries/injections.scm, MIT), each keeping its
;; content node's children
;; (`injection.include-children`): the block grammar holds the inline
;; grammar's punctuation (`` ` ``, `[`, `<`, `{`) as anonymous children of
;; `inline`, `code_fence_content` and `html_block`, and
;; tree-sitter-highlight cuts every child out of an injection's text
;; unless told to keep them. A document's HTML blocks read as one HTML
;; text (`injection.combined`), so a `</details>` a blank line puts in a
;; block of its own still closes its element.

(fenced_code_block
  (info_string
    (language) @injection.language)
  (code_fence_content) @injection.content
  (#set! injection.include-children))

((html_block) @injection.content
  (#set! injection.language "html")
  (#set! injection.include-children)
  (#set! injection.combined))

(document
  .
  (section
    .
    (thematic_break)
    (_) @injection.content
    (thematic_break))
  (#set! injection.language "yaml")
  (#set! injection.include-children))

((minus_metadata) @injection.content
  (#set! injection.language "yaml")
  (#set! injection.include-children))

((plus_metadata) @injection.content
  (#set! injection.language "toml")
  (#set! injection.include-children))

((inline) @injection.content
  (#set! injection.language "markdown_inline")
  (#set! injection.include-children))
