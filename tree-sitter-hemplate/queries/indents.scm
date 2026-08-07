;0
(regular_element) @indent.begin
;1
(style_element) @indent.begin
;2
(script_element) @indent.begin
;3
(end_tag) @indent.branch

;4
(regular_element
  (start_tag
    (tag_name))
  (text
    (whitespace_segment
      (hard_whitespace)))
  (end_tag)  @indent.dedent)

;5
(style_element
  (start_tag
    (tag_name))
  (raw_text) @_raw_text
  (end_tag) @indent.dedent
  (#match? @_raw_text "\n"))

;6
(script_element
  (start_tag
    (tag_name))
  (raw_text) @_raw_text
  (end_tag) @indent.dedent
  (#match? @_raw_text "\n"))


;7
; Target attributes for alignment
(attributes) @indent.align
(#set! indent.open_delimiter "<")
(#set! indent.close_delimiter ">")

;8
; Target individual attributes for alignment
(attribute) @indent.align

;9
(start_tag
  (">") @indent.branch)

;10
(doctype) @indent.zero

;11
(regular_element
  (start_tag
    (tag_name) @_tag_name)
  (_) @indent.auto
  (end_tag)
  (#match? @_tag_name "(pre|code|textarea)"))
