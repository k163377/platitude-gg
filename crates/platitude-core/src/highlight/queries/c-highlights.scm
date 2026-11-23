;; The language's own constants, which tree-sitter-c's query leaves out
;; (`true`, `false`) or names by the broad `constant` it also gives every
;; all-caps name (`NULL`). Laid after the C query in every entry built on
;; it. Part of this repository, MIT like the rest of it.

(null) @constant.builtin
[(true) (false)] @boolean
