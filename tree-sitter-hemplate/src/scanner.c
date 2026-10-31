// Derived from tree-sitter-html's external scanner and modified for Hemplate.
// See ../THIRD_PARTY_LICENSES.md for the original MIT license notice.
#include "tag.h"
#include "tree_sitter/parser.h"

#include <wctype.h>

enum TokenType {
  START_TAG_NAME,
  SCRIPT_START_TAG_NAME,
  STYLE_START_TAG_NAME,
  END_TAG_NAME,
  ERRONEOUS_END_TAG_NAME,
  SELF_CLOSING_TAG_DELIMITER,
  IMPLICIT_END_TAG,
  RAW_TEXT,
  COMMENT,
  ESCAPED_EXPRESSION,
  RAW_EXPRESSION,
  INTERPOLATION_START,
  RAW_INTERPOLATION_START,
  RAW_TEXT_CHUNK,
  RAW_START_TAG_NAME,
};

typedef struct {
  Array(Tag) tags;
} Scanner;

#define MAX(a, b) ((a) > (b) ? (a) : (b))

static inline void advance(TSLexer *lexer) { lexer->advance(lexer, false); }

static inline void skip(TSLexer *lexer) { lexer->advance(lexer, true); }

static unsigned serialize(Scanner *scanner, char *buffer) {
  uint16_t tag_count =
      scanner->tags.size > UINT16_MAX ? UINT16_MAX : scanner->tags.size;
  uint16_t serialized_tag_count = 0;

  unsigned size = sizeof(tag_count);
  memcpy(&buffer[size], &tag_count, sizeof(tag_count));
  size += sizeof(tag_count);

  for (; serialized_tag_count < tag_count; serialized_tag_count++) {
    Tag tag = scanner->tags.contents[serialized_tag_count];
    if (tag.type == CUSTOM) {
      unsigned name_length = tag.custom_tag_name.size;
      if (name_length > UINT8_MAX) {
        name_length = UINT8_MAX;
      }
      if (size + 4 + name_length >= TREE_SITTER_SERIALIZATION_BUFFER_SIZE) {
        break;
      }
      buffer[size++] = (char)tag.type;
      buffer[size++] = (char)name_length;
      strncpy(&buffer[size], tag.custom_tag_name.contents, name_length);
      size += name_length;
    } else {
      if (size + 3 >= TREE_SITTER_SERIALIZATION_BUFFER_SIZE) {
        break;
      }
      buffer[size++] = (char)tag.type;
    }
    buffer[size++] = (char)tag.namespace;
    buffer[size++] = (char)tag.children;
  }

  memcpy(&buffer[0], &serialized_tag_count, sizeof(serialized_tag_count));
  return size;
}

static void deserialize(Scanner *scanner, const char *buffer, unsigned length) {
  for (unsigned i = 0; i < scanner->tags.size; i++) {
    tag_free(&scanner->tags.contents[i]);
  }
  array_clear(&scanner->tags);

  if (length > 0) {
    unsigned size = 0;
    uint16_t tag_count = 0;
    uint16_t serialized_tag_count = 0;

    memcpy(&serialized_tag_count, &buffer[size], sizeof(serialized_tag_count));
    size += sizeof(serialized_tag_count);

    memcpy(&tag_count, &buffer[size], sizeof(tag_count));
    size += sizeof(tag_count);

    array_reserve(&scanner->tags, tag_count);
    if (tag_count > 0) {
      unsigned iter = 0;
      for (iter = 0; iter < serialized_tag_count; iter++) {
        Tag tag = tag_new();
        tag.type = (TagType)buffer[size++];
        if (tag.type == CUSTOM) {
          uint16_t name_length = (uint8_t)buffer[size++];
          array_reserve(&tag.custom_tag_name, name_length);
          tag.custom_tag_name.size = name_length;
          memcpy(tag.custom_tag_name.contents, &buffer[size], name_length);
          size += name_length;
        }
        tag.namespace = (Namespace)buffer[size++];
        tag.children = (Namespace)buffer[size++];
        array_push(&scanner->tags, tag);
      }
      // add zero tags if we didn't read enough, this is because the
      // buffer had no more room but we held more tags.
      for (; iter < tag_count; iter++) {
        array_push(&scanner->tags, tag_new());
      }
    }
  }
}

static String scan_tag_name(TSLexer *lexer) {
  String tag_name = array_new();
  while (iswalnum(lexer->lookahead) || lexer->lookahead == '-' ||
         lexer->lookahead == ':') {
    array_push(&tag_name, towupper(lexer->lookahead));
    advance(lexer);
  }
  return tag_name;
}

static bool scan_comment(TSLexer *lexer) {
  if (lexer->lookahead != '-') {
    return false;
  }
  advance(lexer);
  if (lexer->lookahead != '-') {
    return false;
  }
  advance(lexer);

  unsigned dashes = 0;
  while (lexer->lookahead) {
    switch (lexer->lookahead) {
    case '-':
      ++dashes;
      break;
    case '>':
      if (dashes >= 2) {
        lexer->result_symbol = COMMENT;
        advance(lexer);
        lexer->mark_end(lexer);
        return true;
      }
      // fallthrough: '>' without enough dashes resets counter
      /* fall through */
    default:
      dashes = 0;
    }
    advance(lexer);
  }
  return false;
}

static bool tag_named(const Tag *tag, const char *name) {
  if (tag->type != CUSTOM) {
    for (unsigned i = 0; i < sizeof(TAG_TYPES_BY_TAG_NAME) / sizeof(TagMapEntry); i++) {
      if (TAG_TYPES_BY_TAG_NAME[i].tag_type == tag->type) {
        return strcmp(TAG_TYPES_BY_TAG_NAME[i].tag_name, name) == 0;
      }
    }
    return false;
  }
  return tag->custom_tag_name.size == strlen(name) &&
         memcmp(tag->custom_tag_name.contents, name, strlen(name)) == 0;
}

static bool raw_chunk_tag(const Tag *tag) {
  return tag->type == TITLE || tag->type == TEXTAREA || tag->type == IFRAME ||
         tag_named(tag, "XMP") ||
         tag_named(tag, "NOEMBED") || tag_named(tag, "NOFRAMES") ||
         tag_named(tag, "PLAINTEXT");
}

static bool tag_boundary(int32_t c) {
  return c == '>' || c == '/' || c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\f';
}

/// HTML raw-text/RCDATA boundaries, including script escaped and double-escaped
/// states. Matching a prefix alone (e.g. </stylex>) is not an end tag.
static bool scan_raw_content(Scanner *scanner, TSLexer *lexer, bool chunk) {
  if (scanner->tags.size == 0) return false;
  Tag *tag = array_back(&scanner->tags);
  if (tag->namespace != NS_HTML || (chunk && !raw_chunk_tag(tag))) return false;
  bool rcdata = tag->type == TITLE || tag->type == TEXTAREA;
  bool plaintext = tag_named(tag, "PLAINTEXT");
  unsigned state = 0; // script data, escaped, double escaped
  unsigned dashes = 0;
  bool consumed = false;
  lexer->mark_end(lexer);
  while (!lexer->eof(lexer)) {
    if (rcdata && lexer->lookahead == '{') {
      // Leave openers to the interpolation scanner, including malformed ones
      // that the AST validator must diagnose rather than silently render.
      advance(lexer);
      if (lexer->lookahead == '+') break;
      consumed = true;
      lexer->mark_end(lexer);
      continue;
    }
    if (!plaintext && lexer->lookahead == '<') {
      advance(lexer);
      bool closing = lexer->lookahead == '/';
      if (closing) advance(lexer);
      char name[32];
      unsigned size = 0;
      while ((lexer->lookahead >= 'a' && lexer->lookahead <= 'z') ||
             (lexer->lookahead >= 'A' && lexer->lookahead <= 'Z')) {
        if (size + 1 < sizeof(name)) name[size++] = (char)towupper(lexer->lookahead);
        advance(lexer);
      }
      name[size] = '\0';
      if (closing && tag_named(tag, name) && tag_boundary(lexer->lookahead)) {
        if (tag->type == SCRIPT && state == 2) {
          state = 1;
        } else {
          break; // mark_end still precedes '<'
        }
      } else if (tag->type == SCRIPT && !closing && strcmp(name, "SCRIPT") == 0 &&
                 tag_boundary(lexer->lookahead) && state == 1) {
        state = 2;
      } else if (tag->type == SCRIPT && lexer->lookahead == '!') {
        advance(lexer);
        if (lexer->lookahead == '-') {
          advance(lexer);
          if (lexer->lookahead == '-') {
            advance(lexer);
            state = 1;
          }
        }
      }
      dashes = 0;
    } else {
      if (tag->type == SCRIPT && lexer->lookahead == '>' && dashes >= 2) state = 0;
      dashes = lexer->lookahead == '-' ? dashes + 1 : 0;
      advance(lexer);
    }
    consumed = true;
    lexer->mark_end(lexer);
  }
  lexer->result_symbol = chunk ? RAW_TEXT_CHUNK : RAW_TEXT;
  return consumed;
}

static void pop_tag(Scanner *scanner) {
  Tag popped_tag = array_pop(&scanner->tags);
  tag_free(&popped_tag);
}

static bool optional_end(const Tag *tag) {
  if (tag->namespace != NS_HTML) return false;
  switch (tag->type) {
    case HTML: case HEAD: case BODY: case P: case LI: case DT: case DD:
    case RB: case RT: case RP: case RTC: case OPTION: case OPTGROUP:
    case COLGROUP: case THEAD: case TBODY: case TFOOT: case TR: case TD: case TH:
      return true;
    default:
      return tag_named(tag, "PLAINTEXT");
  }
}

static bool scan_implicit_end_tag(Scanner *scanner, TSLexer *lexer) {
  Tag *parent = scanner->tags.size == 0 ? NULL : array_back(&scanner->tags);

  bool is_closing_tag = false;
  if (lexer->lookahead == '/') {
    is_closing_tag = true;
    advance(lexer);
  } else {
    if (parent && parent->namespace == NS_HTML && tag_is_void(parent)) {
      pop_tag(scanner);
      lexer->result_symbol = IMPLICIT_END_TAG;
      return true;
    }
  }

  String tag_name = scan_tag_name(lexer);
  if (tag_name.size == 0 && !lexer->eof(lexer)) {
    array_delete(&tag_name);
    return false;
  }

  Tag next_tag = tag_for_name(tag_name);

  if (is_closing_tag) {
    // The tag correctly closes the topmost element on the stack
    if (scanner->tags.size > 0 &&
        tag_eq(array_back(&scanner->tags), &next_tag)) {
      tag_free(&next_tag);
      return false;
    }

    // Otherwise, dig deeper and queue implicit end tags (to be nice in
    // the case of malformed HTML)
    for (unsigned i = scanner->tags.size; i > 0; i--) {
      if (tag_eq(&scanner->tags.contents[i - 1], &next_tag) && parent &&
          optional_end(parent) &&
          !(parent->type == P && (next_tag.type == A || next_tag.type == AUDIO ||
            next_tag.type == DEL || next_tag.type == INS || next_tag.type == MAP ||
            next_tag.type == NOSCRIPT || next_tag.type == VIDEO))) {
        pop_tag(scanner);
        lexer->result_symbol = IMPLICIT_END_TAG;
        tag_free(&next_tag);
        return true;
      }
    }
    if (parent && optional_end(parent) &&
        (next_tag.type == HTML || next_tag.type == HEAD || next_tag.type == BODY)) {
      pop_tag(scanner);
      lexer->result_symbol = IMPLICIT_END_TAG;
      tag_free(&next_tag);
      return true;
    }
  } else if (parent && ((!lexer->eof(lexer) && parent->namespace == NS_HTML &&
                        !tag_can_contain(parent, &next_tag)) ||
                        (optional_end(parent) && lexer->eof(lexer)))) {
    pop_tag(scanner);
    lexer->result_symbol = IMPLICIT_END_TAG;
    tag_free(&next_tag);
    return true;
  }

  tag_free(&next_tag);
  return false;
}

static bool scan_interpolation_expression(TSLexer *lexer, bool raw, bool mark);

// Read only the attributes that affect foreign-content integration. The caller
// marks the end of the tag-name token first; this lookahead consumes no syntax.
// Keep dynamic/character-reference encodings ambiguous, as codegen does.
typedef struct {
  Namespace annotation;
  bool font_breakout;
} IntegrationAttributes;

static IntegrationAttributes integration_attributes(TSLexer *lexer) {
  IntegrationAttributes attrs = {NS_MATH_ANNOTATION, false};
  bool encoding_seen = false;
  while (!lexer->eof(lexer)) {
    while (iswspace(lexer->lookahead)) advance(lexer);
    if (lexer->lookahead == '>' || lexer->lookahead == '/') break;
    bool dynamic = lexer->lookahead == '+';
    if (dynamic) advance(lexer);
    char name[32];
    unsigned size = 0;
    while (lexer->lookahead && !iswspace(lexer->lookahead) &&
           lexer->lookahead != '=' && lexer->lookahead != '>' && lexer->lookahead != '/') {
      if (size + 1 < sizeof(name)) {
        name[size++] = lexer->lookahead < 128 ? (char)towupper(lexer->lookahead) : '?';
      }
      advance(lexer);
    }
    name[size] = '\0';
    if (!size) break;
    if (!strcmp(name, "COLOR") || !strcmp(name, "FACE") || !strcmp(name, "SIZE")) {
      attrs.font_breakout = true;
    }
    while (iswspace(lexer->lookahead)) advance(lexer);
    char value[64];
    size = 0;
    bool ambiguous = dynamic;
    bool overflow = false;
    if (lexer->lookahead == '=') {
      advance(lexer);
      while (iswspace(lexer->lookahead)) advance(lexer);
      int32_t quote = lexer->lookahead == '\'' || lexer->lookahead == '"' ? lexer->lookahead : 0;
      if (quote) advance(lexer);
      while (lexer->lookahead && (quote ? lexer->lookahead != quote :
             !iswspace(lexer->lookahead) && lexer->lookahead != '>')) {
        if (lexer->lookahead == '&') ambiguous = true;
        if (quote && lexer->lookahead == '{') {
          // A Rust string inside interpolation can contain the enclosing HTML
          // quote and fake attribute syntax. It must not end this attribute.
          ambiguous = true;
          advance(lexer);
          if (lexer->lookahead == '+') {
            advance(lexer);
            bool raw = lexer->lookahead == '=';
            if (raw) advance(lexer);
            if (!scan_interpolation_expression(lexer, raw, false)) return attrs;
            advance(lexer); // closing '}'
          }
          continue;
        }
        if (size + 1 < sizeof(value)) {
          value[size++] = lexer->lookahead < 128 ? (char)towupper(lexer->lookahead) : '?';
        } else overflow = true;
        advance(lexer);
      }
      if (quote && lexer->lookahead == quote) advance(lexer);
    }
    value[size] = '\0';
    if (!encoding_seen && !strcmp(name, "ENCODING")) {
      encoding_seen = true;
      if (ambiguous) attrs.annotation = NS_AMBIGUOUS;
      else if (!overflow && (!strcmp(value, "TEXT/HTML") ||
                             !strcmp(value, "APPLICATION/XHTML+XML"))) attrs.annotation = NS_HTML;
    }
  }
  return attrs;
}

static bool foreign_breakout(const Tag *tag, bool font_breakout) {
  switch (tag->type) {
    case B: case BLOCKQUOTE: case BODY: case BR: case CODE: case DD: case DIV:
    case DL: case DT: case EM: case EMBED: case H1: case H2: case H3: case H4:
    case H5: case H6: case HEAD: case HR: case I: case IMG: case LI: case MENU:
    case META: case OL: case P: case PRE: case RUBY: case S: case SMALL: case SPAN:
    case STRONG: case SUB: case SUP: case TABLE: case U: case UL: case VAR:
      return true;
    default:
      return tag_named(tag, "BIG") || tag_named(tag, "CENTER") ||
             tag_named(tag, "LISTING") || tag_named(tag, "NOBR") ||
             tag_named(tag, "STRIKE") || tag_named(tag, "TT") ||
             (tag_named(tag, "FONT") && font_breakout);
  }
}

static Namespace element_namespace(Scanner *scanner, const Tag *tag, bool font_breakout) {
  Namespace parent = scanner->tags.size ? array_back(&scanner->tags)->children : NS_HTML;
  if (parent == NS_MATH_TEXT) {
    parent = tag_named(tag, "MGLYPH") || tag_named(tag, "MALIGNMARK") ? NS_MATH : NS_HTML;
  } else if (parent == NS_MATH_ANNOTATION) {
    if (tag->type == SVG) return NS_SVG;
    parent = NS_MATH;
  }
  if (parent != NS_HTML && parent != NS_AMBIGUOUS && foreign_breakout(tag, font_breakout)) {
    // HTML5 pops foreign ancestors at a breakout. Retain the source stack for
    // matching authored end tags, but remember that subsequent content is HTML.
    for (unsigned i = scanner->tags.size; i > 0; i--) {
      Tag *ancestor = &scanner->tags.contents[i - 1];
      if (ancestor->namespace == NS_HTML || ancestor->children == NS_HTML ||
          ancestor->children == NS_MATH_TEXT) break;
      ancestor->children = NS_HTML;
    }
    parent = NS_HTML;
  }
  if (parent == NS_HTML || parent == NS_AMBIGUOUS) {
    if (tag->type == SVG) return NS_SVG;
    if (tag->type == MATH) return NS_MATH;
  }
  return parent;
}

static Namespace children_namespace(const Tag *tag, IntegrationAttributes attrs) {
  if (tag->namespace == NS_SVG && (tag_named(tag, "FOREIGNOBJECT") ||
      tag_named(tag, "DESC") || tag->type == TITLE)) return NS_HTML;
  if (tag->namespace == NS_MATH) {
    if (tag_named(tag, "MI") || tag_named(tag, "MO") || tag_named(tag, "MN") ||
        tag_named(tag, "MS") || tag_named(tag, "MTEXT")) return NS_MATH_TEXT;
    if (tag_named(tag, "ANNOTATION-XML")) return attrs.annotation;
  }
  return tag->namespace;
}

static bool scan_start_tag_name(Scanner *scanner, TSLexer *lexer) {
  String tag_name = scan_tag_name(lexer);
  if (tag_name.size == 0) {
    array_delete(&tag_name);
    return false;
  }

  Tag tag = tag_for_name(tag_name);
  lexer->mark_end(lexer);
  IntegrationAttributes attrs = {NS_MATH_ANNOTATION, false};
  if (tag_named(&tag, "ANNOTATION-XML") || tag_named(&tag, "FONT")) {
    attrs = integration_attributes(lexer);
  }
  tag.namespace = element_namespace(scanner, &tag, attrs.font_breakout);
  tag.children = children_namespace(&tag, attrs);
  array_push(&scanner->tags, tag);
  if (tag.namespace != NS_HTML) {
    lexer->result_symbol = START_TAG_NAME;
  } else if (tag.type == SCRIPT) {
    lexer->result_symbol = SCRIPT_START_TAG_NAME;
  } else if (tag.type == STYLE) {
    lexer->result_symbol = STYLE_START_TAG_NAME;
  } else {
    lexer->result_symbol = raw_chunk_tag(&tag) ? RAW_START_TAG_NAME : START_TAG_NAME;
  }
  return true;
}

static bool scan_end_tag_name(Scanner *scanner, TSLexer *lexer) {
  String tag_name = scan_tag_name(lexer);

  if (tag_name.size == 0) {
    array_delete(&tag_name);
    return false;
  }

  Tag tag = tag_for_name(tag_name);
  if (scanner->tags.size > 0 && tag_eq(array_back(&scanner->tags), &tag)) {
    pop_tag(scanner);
    lexer->result_symbol = END_TAG_NAME;
  } else if (scanner->tags.size == 0 &&
             (tag.type == HTML || tag.type == HEAD || tag.type == BODY)) {
    // Opening html/head/body tags can be omitted in HTML documents.
    lexer->result_symbol = END_TAG_NAME;
  } else {
    lexer->result_symbol = ERRONEOUS_END_TAG_NAME;
  }

  tag_free(&tag);
  return true;
}

static bool scan_self_closing_tag_delimiter(Scanner *scanner, TSLexer *lexer) {
  advance(lexer);
  if (lexer->lookahead == '>') {
    advance(lexer);
    if (scanner->tags.size > 0) {
      pop_tag(scanner);
      lexer->result_symbol = SELF_CLOSING_TAG_DELIMITER;
    }
    return true;
  }
  return false;
}

// Consume one Rust string literal starting at its opening quote; `hashes` is
// the number of `#` of a raw string (or -1 for an ordinary escaped string).
static bool scan_rust_string(TSLexer *lexer, int hashes) {
  advance(lexer);
  for (;;) {
    if (lexer->eof(lexer)) {
      return false;
    }
    int32_t c = lexer->lookahead;
    advance(lexer);
    if (hashes < 0 && c == '\\') {
      if (lexer->eof(lexer)) {
        return false;
      }
      advance(lexer);
    } else if (c == '"') {
      int closing = 0;
      while (closing < hashes && lexer->lookahead == '#') {
        advance(lexer);
        closing++;
      }
      if (hashes < 0 || closing == hashes) {
        return true;
      }
    }
  }
}

// A Hemplate interpolation holds one opaque Rust expression. It ends at the
// first closing delimiter (`+}`, or `=+}` for raw output) that lies outside
// Rust comments, string/char literals and balanced brackets. Trailing whitespace
// before the delimiter is not part of the expression.
static bool scan_interpolation_expression(TSLexer *lexer, bool raw, bool mark) {
  unsigned depth = 0;
  bool any = false;
  for (;;) {
    if (lexer->eof(lexer)) {
      return false;
    }
    int32_t c = lexer->lookahead;
    if (iswspace(c)) {
      advance(lexer);
      continue;
    }
    if (depth == 0 && ((!raw && c == '+') || (raw && c == '='))) {
      advance(lexer);
      if (raw) {
        if (lexer->lookahead == '+') {
          advance(lexer);
          if (lexer->lookahead == '}') {
            break;
          }
        }
      } else if (lexer->lookahead == '}') {
        break;
      }
      // Not a delimiter: the consumed characters belong to the expression.
      any = true;
      if (mark) {
        lexer->mark_end(lexer);
      }
      continue;
    }
    if (c == '/') {
      advance(lexer);
      if (lexer->lookahead == '/') {
        // A line comment owns everything through its newline, including any
        // interpolation delimiters or brackets in the comment text.
        while (!lexer->eof(lexer) && lexer->lookahead != '\n' &&
               lexer->lookahead != '\r') {
          advance(lexer);
        }
      } else if (lexer->lookahead == '*') {
        advance(lexer);
        unsigned comments = 1;
        while (comments > 0) {
          if (lexer->eof(lexer)) {
            return false;
          }
          int32_t previous = lexer->lookahead;
          advance(lexer);
          if (previous == '/' && lexer->lookahead == '*') {
            advance(lexer);
            comments++;
          } else if (previous == '*' && lexer->lookahead == '/') {
            advance(lexer);
            comments--;
          }
        }
      }
    } else if (c == '"') {
      if (!scan_rust_string(lexer, -1)) {
        return false;
      }
    } else if (c == 'r') {
      advance(lexer);
      int hashes = 0;
      while (lexer->lookahead == '#') {
        advance(lexer);
        hashes++;
      }
      if (lexer->lookahead == '"' && !scan_rust_string(lexer, hashes)) {
        return false;
      }
    } else if (c == '\'') {
      // A char literal, or a lifetime/label whose quote is not closed.
      advance(lexer);
      if (lexer->lookahead == '\\') {
        advance(lexer);
        if (lexer->eof(lexer)) {
          return false;
        }
        // The escaped character itself may be a quote (`'\''`); it is
        // not the closing quote. Longer Unicode/hex escapes follow it.
        advance(lexer);
        for (unsigned i = 0; i < 12 && !lexer->eof(lexer) && lexer->lookahead != '\''; i++) {
          advance(lexer);
        }
        if (lexer->lookahead == '\'') {
          advance(lexer);
        }
      } else if (!lexer->eof(lexer) && lexer->lookahead != '\'') {
        advance(lexer);
        if (lexer->lookahead == '\'') {
          advance(lexer);
        }
      }
    } else {
      if (c == '(' || c == '[' || c == '{') {
        depth++;
      } else if ((c == ')' || c == ']' || c == '}') && depth > 0) {
        depth--;
      }
      advance(lexer);
    }
    any = true;
    if (mark) {
      lexer->mark_end(lexer);
    }
  }
  if (!any) {
    return false;
  }
  lexer->result_symbol = raw ? RAW_EXPRESSION : ESCAPED_EXPRESSION;
  return true;
}

// `{+` (or `{+=`) with one optional space opens an interpolation only when a
// complete expression and closing delimiter follow. Look ahead without
// consuming: a lone `{+` stays literal text.
static bool scan_interpolation_start(TSLexer *lexer, const bool *valid_symbols) {
  advance(lexer);
  if (lexer->lookahead != '+') {
    return false;
  }
  advance(lexer);
  bool raw = lexer->lookahead == '=';
  if (raw) {
    advance(lexer);
  }
  if (!valid_symbols[raw ? RAW_INTERPOLATION_START : INTERPOLATION_START]) {
    return false;
  }
  if (lexer->lookahead == ' ') {
    advance(lexer);
  }
  lexer->mark_end(lexer);
  while (iswspace(lexer->lookahead)) {
    advance(lexer);
  }
  if (!scan_interpolation_expression(lexer, raw, false)) {
    return false;
  }
  lexer->result_symbol = raw ? RAW_INTERPOLATION_START : INTERPOLATION_START;
  return true;
}

static bool scan(Scanner *scanner, TSLexer *lexer, const bool *valid_symbols) {
  bool literal_raw = scanner->tags.size > 0 && array_back(&scanner->tags)->namespace == NS_HTML &&
                     raw_chunk_tag(array_back(&scanner->tags)) &&
                     array_back(&scanner->tags)->type != TITLE && array_back(&scanner->tags)->type != TEXTAREA;
  if (lexer->lookahead == '{' && !literal_raw &&
      (valid_symbols[INTERPOLATION_START] || valid_symbols[RAW_INTERPOLATION_START])) {
    return scan_interpolation_start(lexer, valid_symbols);
  }
  // Exactly one expression kind is valid inside an interpolation; both are
  // valid only during error recovery, where the scanner must not guess.
  if (valid_symbols[ESCAPED_EXPRESSION] != valid_symbols[RAW_EXPRESSION]) {
    while (iswspace(lexer->lookahead)) {
      skip(lexer);
    }
    return scan_interpolation_expression(lexer, valid_symbols[RAW_EXPRESSION], true);
  }
  if (valid_symbols[RAW_TEXT] && !valid_symbols[START_TAG_NAME] &&
      !valid_symbols[END_TAG_NAME]) {
    return scan_raw_content(scanner, lexer, false);
  }

  if (valid_symbols[RAW_TEXT_CHUNK] && scanner->tags.size > 0 &&
      array_back(&scanner->tags)->namespace == NS_HTML && raw_chunk_tag(array_back(&scanner->tags))) {
    return scan_raw_content(scanner, lexer, true);
  }

  // Skip whitespace *only* when we're not scanning tag names or self-closing
  // delimiters
  bool scanning_tag = valid_symbols[START_TAG_NAME] || valid_symbols[RAW_START_TAG_NAME] ||
                      valid_symbols[END_TAG_NAME] ||
                      valid_symbols[SELF_CLOSING_TAG_DELIMITER] ||
                      valid_symbols[IMPLICIT_END_TAG];

  if (!scanning_tag) {
    while (iswspace(lexer->lookahead)) {
      skip(lexer);
    }
  }

  // Void elements close immediately after their start tag — emit
  // _implicit_end_tag as a zero-width token before any content is consumed.
  if (valid_symbols[IMPLICIT_END_TAG] && scanner->tags.size > 0 &&
      array_back(&scanner->tags)->namespace == NS_HTML && tag_is_void(array_back(&scanner->tags))) {
    pop_tag(scanner);
    lexer->result_symbol = IMPLICIT_END_TAG;
    return true;
  }

  switch (lexer->lookahead) {
  case '<':
    lexer->mark_end(lexer);
    advance(lexer);

    if (lexer->lookahead == '!') {
      advance(lexer);
      return scan_comment(lexer);
    }

    if (valid_symbols[IMPLICIT_END_TAG]) {
      return scan_implicit_end_tag(scanner, lexer);
    }
    break;

  case '\0':
    if (valid_symbols[IMPLICIT_END_TAG]) {
      return scan_implicit_end_tag(scanner, lexer);
    }
    break;

  case '/':
    if (valid_symbols[SELF_CLOSING_TAG_DELIMITER]) {
      return scan_self_closing_tag_delimiter(scanner, lexer);
    }
    break;

  default:
    if ((valid_symbols[START_TAG_NAME] || valid_symbols[RAW_START_TAG_NAME] || valid_symbols[END_TAG_NAME]) &&
        !valid_symbols[RAW_TEXT]) {
      return (valid_symbols[START_TAG_NAME] || valid_symbols[RAW_START_TAG_NAME]) ? scan_start_tag_name(scanner, lexer)
                                           : scan_end_tag_name(scanner, lexer);
    }
  }

  return false;
}

void *tree_sitter_hemplate_external_scanner_create() {
  Scanner *scanner = (Scanner *)ts_calloc(1, sizeof(Scanner));
  return scanner;
}

bool tree_sitter_hemplate_external_scanner_scan(void *payload, TSLexer *lexer,
                                                const bool *valid_symbols) {
  Scanner *scanner = (Scanner *)payload;
  return scan(scanner, lexer, valid_symbols);
}

unsigned tree_sitter_hemplate_external_scanner_serialize(void *payload,
                                                         char *buffer) {
  Scanner *scanner = (Scanner *)payload;
  return serialize(scanner, buffer);
}

void tree_sitter_hemplate_external_scanner_deserialize(void *payload,
                                                       const char *buffer,
                                                       unsigned length) {
  Scanner *scanner = (Scanner *)payload;
  deserialize(scanner, buffer, length);
}

void tree_sitter_hemplate_external_scanner_destroy(void *payload) {
  Scanner *scanner = (Scanner *)payload;
  for (unsigned i = 0; i < scanner->tags.size; i++) {
    tag_free(&scanner->tags.contents[i]);
  }
  array_delete(&scanner->tags);
  ts_free(scanner);
}
