;; Laid over tree-sitter-c-sharp's own highlights (`grammar::layered`):
;; tokens its grammar reads that its query leaves out. Part of this
;; repository, MIT like the rest of it.
;;
;; Names tokens an older grammar does not have: with one, the whole query
;; is refused and C# reads through the fallback lexer.

;; C# 14's extension block. The crate's query names its contextual
;; neighbours (`record`, `init`) but not this one.
"extension" @keyword
