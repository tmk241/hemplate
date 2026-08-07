;; hemplate highlights — aligned with grammar.js (2026-03-07)
;;
;; Capture group reference (nvim-treesitter standard):
;;   @tag / @tag.delimiter / @tag.attribute  — HTML structure
;;   @keyword / @keyword.conditional / @keyword.repeat  — hemplate directives
;;   @punctuation.special  — interpolation delimiters {+ +}
;;   @keyword.exception  — raw interpolation delimiters {+= =+}
;;   @variable / @variable.member / @function.call  — expressions
;;   @number / @string / @boolean / @operator  — literals & operators
;;   @character.special  — HTML entities
;;   @comment / @keyword.directive  — comments & doctype


;; === Tags ===

(tag_name) @tag

; Tag delimiters: < > </ />
(start_tag "<" @tag.delimiter)
(start_tag ">" @tag.delimiter)
(end_tag "</" @tag.delimiter)
(end_tag ">" @tag.delimiter)
(self_closing_element "<" @tag.delimiter)
(self_closing_element "/>" @tag.delimiter)


;; === Text ===

(text) @none @spell

; Semantic text in heading elements
(((start_tag (tag_name) @_tag) (text) @markup.heading)
  (#eq? @_tag "title"))
(((start_tag (tag_name) @_tag) (text) @markup.heading.1)
  (#eq? @_tag "h1"))
(((start_tag (tag_name) @_tag) (text) @markup.heading.2)
  (#eq? @_tag "h2"))
(((start_tag (tag_name) @_tag) (text) @markup.heading.3)
  (#eq? @_tag "h3"))
(((start_tag (tag_name) @_tag) (text) @markup.heading.4)
  (#eq? @_tag "h4"))
(((start_tag (tag_name) @_tag) (text) @markup.heading.5)
  (#eq? @_tag "h5"))
(((start_tag (tag_name) @_tag) (text) @markup.heading.6)
  (#eq? @_tag "h6"))

; Inline semantic elements
(((start_tag (tag_name) @_tag) (text) @markup.strong)
  (#any-of? @_tag "strong" "b"))
(((start_tag (tag_name) @_tag) (text) @markup.italic)
  (#any-of? @_tag "em" "i"))
(((start_tag (tag_name) @_tag) (text) @markup.strikethrough)
  (#any-of? @_tag "s" "del"))
(((start_tag (tag_name) @_tag) (text) @markup.underline)
  (#eq? @_tag "u"))
(((start_tag (tag_name) @_tag) (text) @markup.raw)
  (#any-of? @_tag "code" "kbd"))
(((start_tag (tag_name) @_tag) (text) @markup.link.label)
  (#eq? @_tag "a"))


;; === Attributes ===

(regular_attribute (attribute_name) @tag.attribute)
(quoted_value) @string
(unquoted_value) @string

; Quote delimiters
(double_quoted_value "\"" @string)
(single_quoted_value "'" @string)

; URL detection in href/src attributes
((regular_attribute
  (attribute_name) @_attr
  (attribute_value (quoted_value (double_quoted_value (text) @string.special.url))))
  (#any-of? @_attr "href" "src" "href=" "src=")
  (#set! @string.special.url url))


;; === Template Directives (h-* control flow) ===

; Generic fallback — all directives are keywords
(template_directive_name) @keyword

; Conditionals: h-if, h-else-if, h-else (token may include leading space and trailing =)
((template_directive_name) @keyword.conditional
  (#match? @keyword.conditional "h-if"))
((template_directive_name) @keyword.conditional
  (#match? @keyword.conditional "h-else"))

; Loops: h-for
((template_directive_name) @keyword.repeat
  (#match? @keyword.repeat "h-for"))

; Directive values
(template_directive (attribute_value) @string.special @nospell)


;; === Dynamic Attributes (+attr="expr") ===

; The + prefix and attribute name share the same highlight as interpolation delimiters
(dynamic_attribute "+" @punctuation.special)
(dynamic_attribute (attribute_name) @punctuation.special)
(dynamic_attribute (quoted_expression) @string.special @nospell)

; Expression quote delimiters
(double_quoted_expression "\"" @string.special)
(single_quoted_expression "'" @string.special)


;; === Interpolations ===

; Escaped interpolation {+ expr +} — delimiters inherit @punctuation.special
; @nospell prevents parent (text) @spell from underlining template syntax
(interpolation) @punctuation.special @nospell

; Raw interpolation {+= expr =+} — visually distinct (dangerous: no XSS escaping)
(raw_interpolation) @keyword.exception @nospell


;; === Expression highlighting (mirrors Rust tree-sitter captures) ===

; self → @variable.builtin (matches Rust's (self) @variable.builtin)
((identifier) @variable.builtin
  (#eq? @variable.builtin "self"))

; General identifiers → @variable
(identifier) @variable

; Field/property access → @variable.member (matches Rust's (field_identifier) @variable.member)
(member_expression
  object: (identifier) @variable
  "." @punctuation.delimiter
  property: (identifier) @variable.member)

; self in member object position → @variable.builtin
(member_expression
  object: ((identifier) @variable.builtin
    (#eq? @variable.builtin "self"))
  "." @punctuation.delimiter
  property: (identifier) @variable.member)

; Chained member access: obj.foo.bar
(member_expression
  object: (member_expression)
  "." @punctuation.delimiter
  property: (identifier) @variable.member)

; Method calls → callee property gets @function.call (matches Rust's field_expression in call_expression)
(call_expression
  callee: (identifier) @function.call
  "(" @punctuation.bracket
  ")" @punctuation.bracket)

(call_expression
  callee: (member_expression
    property: (identifier) @function.call)
  "(" @punctuation.bracket
  ")" @punctuation.bracket)

; Literals (match Rust: boolean/integer @number, string @string)
(number) @number
(string) @string
(boolean) @boolean

; Operators (match Rust: @operator)
(unary_operator) @operator

; Brackets (match Rust: @punctuation.bracket)
(array_expression "[" @punctuation.bracket)
(array_expression "]" @punctuation.bracket)
(tuple_expression "(" @punctuation.bracket)
(tuple_expression ")" @punctuation.bracket)
(parenthesized_expression "(" @punctuation.bracket)
(parenthesized_expression ")" @punctuation.bracket)
(subscript_expression "[" @punctuation.bracket)
(subscript_expression "]" @punctuation.bracket)


;; === Entities ===

(entity) @character.special


;; === Raw text (script/style content) ===

(script_element (raw_text) @none)
(style_element (raw_text) @none)


;; === Comments ===

(comment) @comment


;; === Doctype ===

(doctype) @keyword.directive


;; === Errors ===

(ERROR) @error
