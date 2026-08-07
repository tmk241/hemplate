const PREC = {
  INTERPOLATION: 40,
  UNARY: 12,
  CALL: 11,
  MEMBER: 10,
  // Binary operators
  OR: 1,
  AND: 2,
  EQUALITY: 3,
  RELATIONAL: 4,
  ADDITIVE: 5,
  MULTIPLICATIVE: 6,
};

/**
 * @file HTML extension grammar for tree-sitter with template extensions
 *
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check
//
// Helper: make an operator token that includes any surrounding spaces in one leaf

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
  ],

  rules: {
    document: ($) => seq(optional($.doctype), repeat($._node)),

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
    //erroneous_end_tag: ($) => seq("</", $.erroneous_end_tag_name, ">"),

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

    regular_element: ($) =>
      seq($.start_tag, repeat($._node), choice($.end_tag)),

    void_element: ($) =>
      seq($.start_tag, $._implicit_end_tag),

    self_closing_element: ($) =>
      seq(
        "<",
        alias($._start_tag_name, $.tag_name),
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
      ),

    // Keep ordinary attribute names permissive so custom HTML attributes pass through.
    attribute_name: () =>
      token(
        prec(-1, seq(/[:@#?*.\[(]*[a-zA-Z_$][a-zA-Z0-9_$.\-:|#\])]*/, optional(choice(" = ", "=")))),
      ),

    // Then use it in your attribute rule
    attribute: ($) =>
      choice(
        prec(30, $.template_directive),
        $.dynamic_attribute,
        $.regular_attribute,
      ),

    flag_attribute: ($) => $.attribute_name,

    attributes: ($) =>
      prec.right(repeat1(choice($.ignored_whitespace_segment, $.attribute))),

    template_directive_name: () =>
      token(prec(1, seq(
        "h-",
        choice("if", "else-if", "else", "for", "match", "case"),
        optional(choice(" = ", "=")),
      ))),

    // Define a token specifically for equals
    _equals_token: () => token.immediate(choice(" = ", "=")),

    attribute_value: ($) => seq(choice($.quoted_value, $.unquoted_value)),

    template_directive: ($) =>
      seq($.template_directive_name, optional($.attribute_value)),

    regular_attribute: ($) =>
      seq($.attribute_name, optional($.attribute_value)),

    dynamic_attribute: ($) => seq("+", $.attribute_name, $.quoted_expression),

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

    double_quoted_expression: ($) => seq('"', $.expression, '"'),
    single_quoted_expression: ($) => seq("'", $.expression, "'"),

    quoted_value: ($) => choice($.single_quoted_value, $.double_quoted_value),

    quoted_expression: ($) =>
      choice($.single_quoted_expression, $.double_quoted_expression),

    unquoted_value: () => /[^<>"'=`\s]+/,

    //text: ($) => repeat1($.text_segment),
    text: ($) => prec.right(repeat1($._text_segment)),

    //text_chunk: () => token(prec(-1, /[^\s{}<>"]+|["']|\{|\}|[{}<>]/)),
    //text_chunk: () =>
    //token(choice(/[^{}<>\t\r\n"']+/, /["']/, /\{/, /\}/, /[<>]/)),

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

    interpolation: ($) =>
      seq(
        token.immediate(seq(token("{+"), optional(" "))),
        $.interpolation_expression,
        token.immediate(seq(optional(" "), token("+}"))),
      ),

    raw_interpolation: ($) =>
      seq(
        token.immediate(seq(token("{+="), optional(" "))),
        $.interpolation_expression,
        token.immediate(seq(optional(" "), token("=+}"))),
      ),

    interpolation_expression: ($) => $.expression,

    array_expression: ($) =>
      seq(
        "[",
        optional(
          seq(
            field("element", $.expression),
            repeat(seq(",", optional(" "), field("element", $.expression))),
            optional(","),
          ),
        ),
        "]",
      ),

    tuple_expression: ($) =>
      seq(
        "(",
        field("item", $.expression),
        ",",
        repeat(seq(field("item", $.expression), optional(","), optional(" "))),
        ")",
      ),

    expression: ($) =>
      choice(
        $.identifier,
        $.literal,
        $.array_expression,
        $.tuple_expression,
        $.member_expression,
        $.call_expression,
        $.binary_expression,
        $.unary_expression,
        $.parenthesized_expression,
        $.subscript_expression,
      ),

    identifier: () => /[a-zA-Z_:][a-zA-Z0-9_:<>!]*/,

    literal: ($) => choice($.number, $.string, $.boolean),

    number: () =>
      /\d+(\.\d+)?(u8|i8|u16|i16|u32|i32|u64|i64|u128|i128|isize|usize|f32|f64)?/,

    string: () =>
      prec.dynamic(
        20,
        choice(token(seq("'", /[^']*/, "'")), token(seq('"', /[^"]*/, '"'))),
      ),

    boolean: () => choice("true", "false"),

    member_expression: ($) =>
      prec.left(
        PREC.MEMBER,
        seq(
          field(
            "object",
            choice(
              $.identifier,
              $.literal,
              $.member_expression,
              $.call_expression,
              $.subscript_expression,
            ),
          ),
          ".",
          field("property", $.identifier),
        ),
      ),

    call_expression: ($) =>
      prec.left(
        PREC.CALL,
        seq(
          field("callee", choice($.identifier, $.member_expression)),
          "(",
          optional(
            seq(
              field("arguments", $.expression),
              repeat(seq(",", optional(" "), field("arguments", $.expression))),
            ),
          ),
          ")",
        ),
      ),

    binary_expression: ($) => {
      const table = [
        ["+", PREC.ADDITIVE],
        ["-", PREC.ADDITIVE],
        ["*", PREC.MULTIPLICATIVE],
        ["/", PREC.MULTIPLICATIVE],
        ["%", PREC.MULTIPLICATIVE],
        ["==", PREC.EQUALITY],
        ["!=", PREC.EQUALITY],
        [">", PREC.RELATIONAL],
        ["<", PREC.RELATIONAL],
        [">=", PREC.RELATIONAL],
        ["<=", PREC.RELATIONAL],
        ["&&", PREC.AND],
        ["||", PREC.OR],
      ];

      return choice(
        ...table.map(([operator, precedence]) => {
          return prec.left(
            precedence,
            seq(
              field("left", $.expression),
              token.immediate(seq(optional(" "), operator, optional(" "))),
              field("right", $.expression),
            ),
          );
        }),
      );
    },

    unary_expression: ($) =>
      prec.right(
        PREC.UNARY,
        seq(
          field("operator", $.unary_operator),
          field(
            "argument",
            choice(
              $.identifier,
              $.literal,
              $.member_expression,
              $.call_expression,
              $.array_expression,
              $.tuple_expression,
              $.parenthesized_expression,
            ),
          ),
        ),
      ),

    unary_operator: () => choice("!", "-", "&", "*"),

    parenthesized_expression: ($) => seq("(", $.expression, ")"),

    subscript_expression: ($) =>
      prec.left(
        PREC.MEMBER,
        seq(
          field(
            "object",
            choice($.identifier, $.member_expression, $.call_expression),
          ),
          "[",
          field(
            "index",
            choice(
              $.identifier,
              $.literal,
              $.member_expression,
              $.call_expression,
            ),
          ),
          "]",
        ),
      ),
  },
});
