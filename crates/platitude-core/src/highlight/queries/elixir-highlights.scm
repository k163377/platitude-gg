;; The language's own constants, which tree-sitter-elixir's query names
;; by the broad `constant`. Laid last, the first pattern would also take
;; the two places that query names them otherwise, so those are named
;; again below in the query's own names: `@doc false` (`@moduledoc`,
;; `@typedoc`) is documentation, and `@true` or `@nil` an attribute. Part
;; of this repository, MIT like the rest of it.

[(boolean) (nil)] @constant.builtin

(unary_operator
  operator: "@"
  operand: [(boolean) (nil)] @attribute)

; The predicate's capture carries the query's own name too: a later
; pattern's capture of a node replaces the earlier one, coloured or not.
(unary_operator
  operand: (call
    target: (identifier) @comment.doc.__attribute__
    (arguments
      (boolean) @comment.doc))
  (#any-of? @comment.doc.__attribute__ "moduledoc" "typedoc" "doc"))
