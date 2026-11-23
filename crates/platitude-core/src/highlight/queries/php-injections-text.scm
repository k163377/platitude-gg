;; tree-sitter-php's queries/injections-text.scm (MIT), which its
;; manifest lists beside queries/injections.scm for the `php` grammar but
;; the crate exports no constant for: the HTML around `<?php … ?>`.

((text) @injection.content
  (#set! injection.language "html")
  (#set! injection.combined))
