;; The language's own constants, which tree-sitter-objc reads as plain
;; identifiers and its query leaves unnamed (`YES` and `NO`, `nil` and
;; `Nil`), named as C's `true`, `false` and `NULL` are. Laid last in the
;; objc entry. Part of this repository, MIT like the rest of it.

((identifier) @boolean (#any-of? @boolean "YES" "NO"))
((identifier) @constant.builtin (#any-of? @constant.builtin "nil" "Nil"))
