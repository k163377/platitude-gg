;; The language's own constants, which tree-sitter-c's query leaves out
;; (`true`, `false`) or names by the broad `constant` it also gives every
;; all-caps name (`NULL`, and the `nullptr` word tree-sitter-cpp's query
;; names inside it). Laid after the C query in every entry built on it.
;; Part of this repository, MIT like the rest of it.

(null) @constant.builtin
(null _ @constant.builtin)
[(true) (false)] @boolean
