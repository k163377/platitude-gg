;; tree-sitter-javascript's queries/highlights-params.scm (MIT), which its
;; manifest lists after highlights.scm and highlights-jsx.scm but the
;; crate exports no constant for: a parameter is a parameter, not the
;; constructor its capital would make it.

(formal_parameters
  [
    (identifier) @variable.parameter
    (array_pattern
      (identifier) @variable.parameter)
    (object_pattern
      [
        (pair_pattern value: (identifier) @variable.parameter)
        (shorthand_property_identifier_pattern) @variable.parameter
      ])
  ]
)
