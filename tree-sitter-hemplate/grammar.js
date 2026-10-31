/**
 * @file HTML extension grammar for tree-sitter with template extensions
 *
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

module.exports = grammar({
  name: "hemplate",

  externals: ($) => [
    $._start_tag_name,
    $._script_start_tag_name,
    $._style_start_tag_name,
    $._end_tag_name,
    $.erroneous_end_tag_name,
    "/>",
    $._implicit_end_tag,
    $.raw_text,
    $.comment,
    $._escaped_expression,
    $._raw_expression,
    $._interpolation_start,
    $._raw_interpolation_start,
    $._raw_text_chunk,
    $._raw_start_tag_name,
  ],

  rules: {
    document: ($) => seq(optional($.doctype), repeat(choice($._node, $.end_tag))),

    doctype: ($) => seq("<!", alias($._doctype, "doctype"), /[^>]+/, ">"),

    _doctype: (_) => /[Dd][Oo][Cc][Tt][Yy][Pp][Ee]/,

    // An entity can be named, numeric (decimal), or numeric (hexacecimal). The
    // longest entity name is 29 characters long, and the HTML spec says that
    // no more will ever be added.
    entity: (_) => /&(#([xX][0-9a-fA-F]{1,6}|[0-9]{1,5})|[A-Za-z]{1,30});?/,

    start_tag: ($) =>
      seq(
        "<",
        alias($._start_tag_name, $.tag_name),
        optional($.attributes),
        ">",
      ),

    script_start_tag: ($) =>
      seq(
        "<",
        alias($._script_start_tag_name, $.tag_name),
        optional($.attributes),
        ">",
      ),

    style_start_tag: ($) =>
      seq(
        "<",
        alias($._style_start_tag_name, $.tag_name),
        optional($.attributes),
        ">",
      ),

    end_tag: ($) => seq("</", alias($._end_tag_name, $.tag_name), ">"),

    script_element: ($) =>
      seq(
        alias($.script_start_tag, $.start_tag),
        optional($.raw_text),
        $.end_tag,
      ),

    style_element: ($) =>
      seq(
        alias($.style_start_tag, $.start_tag),
        optional($.raw_text),
        $.end_tag,
      ),

    _node: ($) => choice($.entity, $.comment, $.text, $.element),

    raw_start_tag: ($) => seq("<", alias($._raw_start_tag_name, $.tag_name), optional($.attributes), ">"),

    raw_content: ($) => repeat1(choice(alias($._raw_text_chunk, $.text_chunk), $.comment, $.interpolation, $.raw_interpolation)),

    // Only HTML-namespace raw-text/RCDATA elements reach this rule. Consumers
    // use the field instead of guessing the namespace from a tag's spelling.
    raw_element: ($) => seq(alias($.raw_start_tag, $.start_tag), optional(field("raw_content", alias($.raw_content, $.text))), choice($.end_tag, $._implicit_end_tag)),

    regular_element: ($) =>
      choice(
        seq($.start_tag, repeat1($._node), choice($.end_tag, $._implicit_end_tag)),
        seq($.start_tag, $.end_tag),
      ),

    void_element: ($) =>
      seq($.start_tag, $._implicit_end_tag),

    self_closing_element: ($) =>
      seq(
        "<",
        alias(choice($._start_tag_name, $._raw_start_tag_name), $.tag_name),
        optional($.attributes),
        "/>",
      ),

    element: ($) =>
      choice(
        $.regular_element,
        $.script_element,
        $.self_closing_element,
        $.style_element,
        $.void_element,
        alias($.raw_element, $.regular_element),
      ),

    // Keep ordinary attribute names permissive so custom HTML attributes pass through.
    attribute_name: () =>
      token(
        prec(-1, /[:@#?*.\[(]*[a-zA-Z0-9_$][a-zA-Z0-9_$.\-:|#\])]*/),
      ),

    attribute: ($) =>
      choice(
        prec(30, $.template_directive),
        $.dynamic_attribute,
        $.regular_attribute,
      ),

    attributes: ($) =>
      prec.right(repeat1(choice($.ignored_whitespace_segment, $.attribute))),

    template_directive_name: () =>
      token(prec(1, seq(
        "h-",
        choice("if", "else-if", "else", "for", "match", "case"),
      ))),

    // Prefer an entire assignment separator over a standalone whitespace token.
    // The equals sign remains mandatory; whitespace alone is never a value.
    _equals_token: () => token(prec(2, /[ \t\r\n\f]*=[ \t\r\n\f]*/)),

    attribute_value: ($) => seq(choice($.quoted_value, $.unquoted_value)),

    template_directive: ($) =>
      seq($.template_directive_name, optional(seq($._equals_token, $.attribute_value))),

    regular_attribute: ($) =>
      seq($.attribute_name, optional(seq($._equals_token, $.attribute_value))),

    dynamic_attribute: ($) => seq("+", $.attribute_name, $._equals_token, $.quoted_expression),

    hard_whitespace: ($) => prec.right(repeat1($.newline)),

    space: () => token(/[ ]/),
    tab: () => token(/\t/),

    _space_tab: ($) => choice($.space, $.tab),

    soft_whitespace: ($) => prec.right(repeat1($._space_tab)),

    _whitespace: ($) => choice($.soft_whitespace, $.hard_whitespace),

    whitespace_segment: ($) => prec.right(repeat1($._whitespace)),

    ignored_whitespace_segment: ($) =>
      prec.right(
        repeat1(choice($.ignored_soft_whitespace, $.ignored_hard_whitespace)),
      ),

    ignored_soft_whitespace: ($) =>
      alias($.soft_whitespace, "ignored_soft_whitespace"),

    ignored_hard_whitespace: ($) =>
      alias($.hard_whitespace, "ignored_hard_whitespace"),

    newline: () => token(/(\r\n|\r|\n|\u2028|\u2029)/),

    double_quoted_value: ($) => seq('"', optional($.text), '"'),
    single_quoted_value: ($) => seq("'", optional($.text), "'"),

    double_quoted_expression: ($) =>
      seq('"', alias($._double_quoted_expression_text, $.expression), '"'),
    single_quoted_expression: ($) =>
      seq("'", alias($._single_quoted_expression_text, $.expression), "'"),

    quoted_value: ($) => choice($.single_quoted_value, $.double_quoted_value),

    quoted_expression: ($) =>
      choice($.single_quoted_expression, $.double_quoted_expression),

    unquoted_value: () => /[^<>"'=`\s]+/,

    text: ($) => prec.right(repeat1($._text_segment)),

    text_chunk: () =>
      prec.right(
        repeat1(
          choice(
            token(/[^{}\s<>"'\t\r\n]+/),
            token(/["']/),
            token(/\{/),
            token(/\}/),
            token(/[<>]/),
          ),
        ),
      ),

    _text_segment: ($) =>
      choice($.raw_interpolation, $.interpolation, $.whitespace_segment, $.text_chunk),

    // The scanner emits an opening delimiter only when a complete
    // interpolation follows; otherwise `{` remains ordinary text.
    interpolation: ($) =>
      seq(
        alias($._interpolation_start, "{+"),
        $.interpolation_expression,
        token.immediate(/[ \t\r\n]*\+\}/),
      ),

    raw_interpolation: ($) =>
      seq(
        alias($._raw_interpolation_start, "{+="),
        alias($.raw_interpolation_expression, $.interpolation_expression),
        token.immediate(/[ \t\r\n]*=\+\}/),
      ),

    // An interpolation holds one opaque Rust expression. The external scanner
    // ends it at the first closing delimiter outside string/char literals and
    // balanced brackets; Rust's own parser checks the expression itself.
    interpolation_expression: ($) => $._escaped_expression,
    raw_interpolation_expression: ($) => $._raw_expression,

    // A dynamic attribute value may hold any Rust expression that does not
    // contain its enclosing quote character.
    _double_quoted_expression_text: () => /[^"]+/,
    _single_quoted_expression_text: () => /[^']+/,
  },
});
