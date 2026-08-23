;; Hand-written for the tree-sitter-grammars Kotlin parser (the
;; tree-sitter-kotlin-ng crate ships no queries), against its own
;; node-types.json. Part of this repository, MIT like the rest of it.

(line_comment) @comment
(block_comment) @comment

(string_literal) @string
(multiline_string_literal) @string
(character_literal) @string

(number_literal) @number
(float_literal) @number

(annotation) @attribute

(user_type (identifier) @type)
(class_declaration (identifier) @type)

(function_declaration (identifier) @function)
(call_expression (identifier) @function)

[
  "abstract" "actual" "as" "by" "catch" "class" "companion" "const"
  "constructor" "crossinline" "data" "do" "dynamic" "else" "enum"
  "expect" "external" "final" "finally" "for" "fun" "if" "import" "in"
  "infix" "init" "inline" "inner" "interface" "internal" "is" "lateinit"
  "noinline" "object" "open" "operator" "out" "override" "package"
  "private" "protected" "public" "return" "sealed" "super" "suspend"
  "tailrec" "this" "throw" "try" "typealias" "val" "vararg" "var" "when"
  "where" "while"
] @keyword
