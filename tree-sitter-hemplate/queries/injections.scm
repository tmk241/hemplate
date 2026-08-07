;; hemplate injections — language injection for embedded content

;; Script content → JavaScript
((script_element (raw_text) @injection.content)
  (#set! injection.language "javascript"))

;; Style content → CSS
((style_element (raw_text) @injection.content)
  (#set! injection.language "css"))

;; Interpolation expressions → Rust (expression-level; may not parse fully)
((interpolation (interpolation_expression) @injection.content)
  (#set! injection.language "rust"))

((raw_interpolation (interpolation_expression) @injection.content)
  (#set! injection.language "rust"))
