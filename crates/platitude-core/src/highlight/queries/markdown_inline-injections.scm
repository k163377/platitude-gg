;; tree-sitter-md's inline injections (its
;; tree-sitter-markdown-inline/queries/injections.scm, MIT), keeping the
;; content node's children: an `html_tag` holds its `<`, `/` and `>` as
;; anonymous children (see `markdown-injections.scm`). A paragraph's tags
;; read as one HTML text (`injection.combined`): an end tag read alone is
;; an error to the HTML grammar and takes no colour.

((html_tag) @injection.content
  (#set! injection.language "html")
  (#set! injection.include-children)
  (#set! injection.combined))

((latex_block) @injection.content
  (#set! injection.language "latex")
  (#set! injection.include-children))
