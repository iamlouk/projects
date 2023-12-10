#ifndef CSON_H
#define CSON_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>
#include <stdio.h>
#include <string.h>

enum cson_type {
  CSON_INVALID, // 0
  CSON_ERROR,   // 1
  CSON_NULL,    // 2
  CSON_BOOLEAN, // 3
  CSON_INTEGER, // 4
  CSON_REAL,    // 5
  CSON_STRING,  // 6
  CSON_ARRAY,   // 7
  CSON_MAP,     // 8
  CSON_ROOT     // 9
};

struct cson {
  enum cson_type type;
  struct cson *next, *prev, *container;
  uint32_t line, col;
  bool allocated;
  size_t size;
  union {
    struct {
      const char *key;
      size_t key_size;
      bool key_allocated;
    };
    size_t idx;
  };
  union {
    const char *error;
    bool boolean;
    int64_t integer;
    double real;
    const char *string;
    struct cson *array;
    struct cson *map;
  };
};

void cson_free(struct cson *cson);

static inline bool 
cson_is_null(const struct cson *cson)
{
  return cson->type == CSON_NULL;
}

static inline bool
cson_get_bool(const struct cson *cson, bool *x)
{
  if (cson->type != CSON_BOOLEAN) return false;
  if (x) *x = cson->boolean;
  return true;
}

static inline bool
cson_get_integer(const struct cson *cson, int64_t *x)
{
  if (cson->type != CSON_INTEGER) return false;
  if (x) *x = cson->integer;
  return true;
}

static inline bool
cson_get_string(const struct cson *cson, const char **x)
{
  if (cson->type != CSON_STRING) return false;
  if (x) *x = cson->string;
  return true;
}

static inline bool
cson_get_array(const struct cson *cson, struct cson **x)
{
  if (cson->type != CSON_ARRAY) return false;
  if (x) *x = cson->array;
  return true;
}

static inline bool
cson_get_map(const struct cson *cson, struct cson **x)
{
  if (cson->type != CSON_MAP) return false;
  if (x) *x = cson->map;
  return true;
}

static inline bool
cson_array_next(const struct cson *cson, size_t *idx, const struct cson **x)
{
  if (!cson->container || cson->container->type != CSON_ARRAY || cson->next == NULL)
    return false;

  *idx = cson->next->idx;
  *x = cson->next;
  return true;
}

static inline bool
cson_map_next(const struct cson *cson, const char **key, const struct cson **x)
{
  if (!cson->container || cson->container->type != CSON_MAP || cson->next == NULL)
    return false;

  *key = cson->next->key;
  *x = cson->next;
  return true;
}

extern struct cson *cson_parse_file(const char *file_path, const char **freewhendone);

extern struct cson *cson_parse(char *data, int64_t size);

extern ssize_t cson_write(struct cson *cson, FILE *f);

static inline bool
cson_map_get_field(const struct cson *cson, const char *key, const struct cson **field)
{
  if (cson->type == CSON_MAP)
	return cson_map_get_field(cson->map, key, field);
  if (!cson->container || cson->container->type != CSON_MAP)
	return false;
  if (strcmp(cson->key, key) == 0)
	return *field = cson, true;

  return cson->next ? cson_map_get_field(cson->next, key, field) : false;
}

#endif /* CSON_H */
