#ifndef CSON_H
#define CSON_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>
#include <stdio.h>

enum cson_type {
  CSON_INVALID,
  CSON_ERROR,
  CSON_NULL,
  CSON_BOOLEAN,
  CSON_INTEGER,
  CSON_REAL,
  CSON_STRING,
  CSON_ARRAY,
  CSON_MAP,
  CSON_ROOT
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
cson_is_null(struct cson *cson)
{
  return cson->type == CSON_NULL;
}

static inline bool
cson_get_bool(struct cson *cson, bool *x)
{
  if (cson->type != CSON_BOOLEAN) return false;
  if (x) *x = cson->boolean;
  return true;
}

static inline bool
cson_get_integer(struct cson *cson, int64_t *x)
{
  if (cson->type != CSON_INTEGER) return false;
  if (x) *x = cson->integer;
  return true;
}

static inline bool
cson_get_array(struct cson *cson, struct cson **x)
{
  if (cson->type != CSON_ARRAY) return false;
  if (x) *x = cson->array;
  return true;
}

static inline bool
cson_get_map(struct cson *cson, struct cson **x)
{
  if (cson->type != CSON_MAP) return false;
  if (x) *x = cson->array;
  return true;
}

static inline bool
cson_array_next(struct cson *cson, size_t *idx, struct cson **x)
{
  if (!cson->container || cson->container->type == CSON_ARRAY || cson->next == NULL)
    return false;

  *idx = cson->next->idx;
  *x = cson->next;
  return true;
}

static inline bool
cson_map_next(struct cson *cson, const char **key, struct cson **x)
{
  if (!cson->container || cson->container->type == CSON_MAP || cson->next == NULL)
    return false;

  *key = cson->next->key;
  *x = cson->next;
  return true;
}



extern struct cson *cson_parse(char *data, int64_t size);

extern ssize_t cson_write(struct cson *cson, FILE *f);

#endif /* CSON_H */
