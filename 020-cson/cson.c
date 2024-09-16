#include "cson.h"

#include <stddef.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>
#include <stdio.h>
#include <string.h>
#include <assert.h>
#include <errno.h>

enum cson_token {
  TOK_INVALID,
  TOK_ERROR,
  TOK_EOF,
  TOK_ID,
  TOK_NULL,
  TOK_BOOLEAN,
  TOK_INTEGER,
  TOK_REAL,
  TOK_STRING,
  TOK_LPAREN,
  TOK_RPAREN,
  TOK_LBRACKET,
  TOK_RBRACKET,
  TOK_LBRACES,
  TOK_RBRACES,
  TOK_COMMA,
  TOK_COLON,
};

struct parser {
  uint32_t line, col;
  size_t pos;
  size_t size;
  char *input;
  const char *error;
  char buf[128];
};

static struct cson*
alloc_cson()
{
  return (struct cson*)calloc(1, sizeof(struct cson));
}

void
cson_free(struct cson *cson)
{
  if (cson->container && cson->container->type == CSON_MAP && cson->key_allocated)
    free((void*)cson->key);

  if (cson->prev) {
    assert(cson->container);
    cson_free(cson->prev);
  }

  switch (cson->type) {
  case CSON_ERROR:
    free((void*)cson->error);
    break;
  case CSON_STRING:
    if (cson->allocated)
      free((void*)cson->string);
    break;
  case CSON_ARRAY:
    for (struct cson *e = cson->array, *next; e != NULL; e = next) {
      next = e->next;
      e->prev = NULL;
      cson_free(e);
    }
    break;
  case CSON_MAP:
    for (struct cson *e = cson->map, *next; e != NULL; e = next) {
      next = e->next;
      e->prev = NULL;
      cson_free(e);
    }
    break;
  default:
    break;
  }
  free(cson);
}

static bool
skip_whitespace(struct parser *p)
{
  while (p->pos < p->size) {
    switch (p->input[p->pos]) {
    case ' ':
    case '\t':
    case '\v':
      p->pos += 1;
      p->col += 1;
      continue;
    case '\n':
      p->pos += 1;
      p->line += 1;
      p->col = 0;
      continue;
    default:
      return true;
    }
  }
  return false;
}

static bool
expect_char(struct parser *p, char expected)
{
  bool eof = !skip_whitespace(p);
  if (eof) {
    snprintf(p->buf, sizeof(p->buf) - 1,
             "cson: unexpected EOF, expected: '%c'", expected);
    p->error = strdup(p->buf);
    return false;
  }

  char c = p->input[p->pos++];
  if (c != expected) {
    snprintf(p->buf, sizeof(p->buf) - 1,
             "cson: unexpected '%c', expected: '%c'", c, expected);
    p->error = strdup(p->buf);
    return false;
  }

  p->col++;
  return true;
}

// Only works if the initial '"' was already consumed.
static bool
handle_string(struct parser *p, const char **str, size_t *str_len, bool *allocated)
{
  bool trivial = true;
  size_t off, len_estimate = 0;
  for (off = 0; p->pos + off < p->size; off++) {
    char c = p->input[p->pos + off];
    if (c == '"')
      break;
    if (c == '\\') {
      trivial = false;
      off += 1;
    }
    len_estimate += 1;
  }

  if (p->pos + off >= p->size) {
    snprintf(p->buf, sizeof(p->buf) - 1,
             "cson: unexpected EOF, expected closing '\"'");
    p->error = strdup(p->buf);
    return false;
  }

  if (trivial) {
    p->input[p->pos + len_estimate] = '\0';
    *str_len = len_estimate;
    *str = &p->input[p->pos];
    *allocated = false;
    p->pos += len_estimate + 1;
    p->col += len_estimate + 1;
    return true;
  }

  assert(0 && "TODO!");
  return false;
}

static inline bool
expect_string(struct parser *p, const char **str, size_t *str_len, bool *allocated)
{
  if (!expect_char(p, '"'))
    return false;

  return handle_string(p, str, str_len, allocated);
}

static enum cson_token
next_token(struct cson *data, struct parser *p)
{
  if (!skip_whitespace(p))
    return TOK_EOF;

  data->line = p->line;
  data->col = p->col++;

  size_t len;
  switch (p->input[p->pos++]) {
  case ',': return TOK_COMMA;
  case ':': return TOK_COLON;
  case '(': return TOK_LPAREN;
  case ')': return TOK_RPAREN;
  case '[': return TOK_LBRACKET;
  case ']': return TOK_RBRACKET;
  case '{': return TOK_LBRACES;
  case '}': return TOK_RBRACES;

  case 'n': // "null" if the only valid sequence here...
    if (strncmp(p->input + p->pos, "ull", 3) == 0) {
      data->type = CSON_NULL;
      p->col += 3;
      p->pos += 3;
      return TOK_NULL;
    }
    goto unexpected;

  case 't': // "true"
    if (strncmp(p->input + p->pos, "rue", 3) == 0) {
      data->type = CSON_BOOLEAN;
      data->boolean = true;
      p->col += 3;
      p->pos += 3;
      return TOK_BOOLEAN;
    }
    goto unexpected;

  case 'f': // false
    if (strncmp(p->input + p->pos, "alse", 4) == 0) {
      data->type = CSON_BOOLEAN;
      data->line = p->line;
      data->col = p->col;
      data->boolean = false;
      p->col += 4;
      p->pos += 4;
      return TOK_BOOLEAN;
    }
    goto unexpected;

  case '0':
  case '1':
  case '2':
  case '3':
  case '4':
  case '5':
  case '6':
  case '7':
  case '8':
  case '9':
    // handle integers, TODO: floats
    p->buf[0] = p->input[p->pos - 1];
    len = 1;
    while (p->pos < p->size && len < sizeof(p->buf) - 1) {
      char c = p->input[p->pos];
      if (!('0' <= c && c <= '9'))
        break;
      if (c == '.' || c == 'e') {
        assert("TODO!" && 0);
        return TOK_ERROR;
      }
      p->col += 1;
      p->pos += 1;
      p->buf[len++] = c;
    }
    p->buf[len] = '\0';

    errno = 0;
    data->integer = strtoll(p->buf, NULL, 10);
    if (errno) {
      snprintf(p->buf, sizeof(p->buf) - 1,
               "cson: invalid integer literal: %s", strerror(errno));
      p->error = strdup(p->buf);
      return TOK_ERROR;
    }
    data->type = CSON_INTEGER;
    return TOK_INTEGER;

  case '"':
    if (!handle_string(p, &data->string, &data->size, &data->allocated))
      return TOK_ERROR;
    data->type = CSON_STRING;
    return TOK_STRING;

  unexpected:
  default:
    snprintf(p->buf, sizeof(p->buf) - 1,
             "cson: unexpected token: '%c'",
             p->input[p->pos - 1]);
    p->error = strdup(p->buf);
    return TOK_ERROR;
  }
}

static bool
parse(struct cson *data, struct parser *p)
{
  struct cson *prev = NULL;
  enum cson_token tok = next_token(data, p);
  switch (tok) {
  case TOK_NULL:
    return true;
  case TOK_BOOLEAN:
    return true;
  case TOK_INTEGER:
    return true;
  case TOK_STRING:
    return true;
  case TOK_LBRACKET:
    data->type = CSON_ARRAY;
    for (size_t len = 0; true; len++) {
      struct cson *e = alloc_cson();
      e->container = data;
      e->idx = len;
      e->prev = prev;
      if (prev) prev->next = e;
      else data->array = e;

      if (!parse(e, p)) {
	cson_free(e);
	return false;
      }

      if (!skip_whitespace(p)) {
	cson_free(e);
	goto unexpected_eof;
      }

      char c = p->input[p->pos++];
      if (c == ',') {
	prev = e;
	continue;
      } else if (c == ']') {
	data->size = len;
	return true;
      } else {
	cson_free(e);
	tok = TOK_INVALID;
	goto unexpected_tok;
      }
    }
    return false; /* unreachable! */

  case TOK_LBRACES:
    data->type = CSON_MAP;
    for (size_t len = 0; true; len++) {
      struct cson *e = alloc_cson();
      e->container = data;
      if (!expect_string(p, &e->key, &e->key_size, &e->key_allocated)) {
	cson_free(e);
	if (prev)
	  cson_free(prev);
	return false;
      }
      e->prev = prev;
      if (prev) prev->next = e;
      else data->array = e;

      if (!expect_char(p, ':')) {
	free((void*)e->key);
	cson_free(e);
	tok = TOK_INVALID;
	goto unexpected_tok;
      }

      if (!parse(e, p)) {
	cson_free(e);
	return false;
      }

      if (!skip_whitespace(p)) {
	cson_free(e);
	goto unexpected_eof;
      }

      char c = p->input[p->pos++];
      if (c == ',') {
	prev = e;
	continue;
      } else if (c == '}') {
	data->size = len;
	return true;
      } else {
	cson_free(e);
	tok = TOK_INVALID;
	goto unexpected_tok;
      }
    }
    return false; /* unreachable! */

  unexpected_eof:
  case TOK_EOF:
    snprintf(p->buf, sizeof(p->buf) - 1,
             "cson: unexpected EOF");
    p->error = strdup(p->buf);
    return false;

  case TOK_ERROR:
    return false;
  unexpected_tok:
  default:
    snprintf(p->buf, sizeof(p->buf) - 1,
             "cson: unexpected token (code: %d)", (int) tok);
    p->error = strdup(p->buf);
    return false;
  }
}

extern struct cson*
cson_parse(char *data, int64_t size)
{
  struct parser parser = {
    .line = 1,
    .col = 0,
    .pos = 0,
    .size = size < 0 ? strlen(data) : (size_t)size,
    .input = data,
    .error = NULL,
  };

  struct cson *root = alloc_cson();
  root->container = NULL;
  root->line = 1;
  root->col = 0;
  if (parse(root, &parser))
    return root;

  root->type = CSON_ERROR;
  root->error = parser.error;
  return root;
}

static ssize_t
write_string(const char *str, size_t len, FILE *f)
{
  // TODO: Handle more escape sequences...
  ssize_t n = 0;
  n += fputc('"', f);
  for (size_t i = 0; i < len; i++) {
    char c = str[i];
    switch (c) {
    case '\n': n += fputs("\\n", f); break;
    case '\t': n += fputs("\\t", f); break;
    case '\\': n += fputc('\\', f); break;
    case '"': n += fputs("\\\"", f); break;
    default:
      n += fputc(c, f);
      break;
    }
  }
  n += fputc('"', f);
  return n;
}

extern ssize_t
cson_write(struct cson *cson, FILE *f)
{
  ssize_t n = 0;
  switch (cson->type) {
  case CSON_NULL:
    return fprintf(f, "null");
  case CSON_BOOLEAN:
    return fprintf(f, cson->boolean ? "true" : "false");
  case CSON_INTEGER:
    return fprintf(f, "%ld", cson->integer);
  case CSON_STRING:
    return write_string(cson->string, cson->size, f);
  case CSON_ARRAY:
    n = fprintf(f, "[");
    for (struct cson *e = cson->array; e != NULL; e = e->next) {
      n += cson_write(e, f);
      if (e->next)
	n += fprintf(f, ",");
    }
    return n + fprintf(f, "]");
  case CSON_MAP:
    n = fprintf(f, "{");
    for (struct cson *e = cson->map; e != NULL; e = e->next) {
      n += write_string(e->key, e->key_size, f);
      n += fprintf(f, ":");
      n += cson_write(e, f);
      if (e->next)
	n += fprintf(f, ",");
    }
    return n + fprintf(f, "}");
  default:
    assert(0);
    return -1;
  }
}

