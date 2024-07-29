#pragma once

#include <assert.h>
#include <errno.h>
#include <stddef.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

struct rbuffer {
  size_t cap;
  size_t size;
  size_t widx, ridx;
  uint8_t data[];
};

static inline struct rbuffer *
rbuffer_create(size_t cap) {
  struct rbuffer *b = malloc(sizeof(struct rbuffer) + cap);
  b->cap = cap;
  b->widx = 0;
  b->ridx = 0;
  b->size = 0;
  return b;
}

static inline void
rbuffer_destroy(struct rbuffer *b) { free(b); }

static inline bool
rbuffer_is_empty(const struct rbuffer *b) {
  assert(b->ridx < b->cap && b->widx < b->cap);
  assert(b->size == (b->ridx < b->widx
      ? (b->widx - b->ridx) : (b->ridx - b->widx)));
  return b->size == 0;
}

static inline bool
rbuffer_is_full(const struct rbuffer *b) {
  assert(b->ridx < b->cap && b->widx < b->cap);
  assert(b->size == (b->ridx < b->widx
      ? (b->widx - b->ridx) : (b->ridx - b->widx)));
  return b->size == b->cap;
}

static inline ssize_t
rbuffer_put(struct rbuffer *b, uint8_t c) {
  if (rbuffer_is_full(b))
    return 0;

  b->data[b->widx] = c;
  b->widx = (b->widx + 1) % b->cap;
  b->size += 1;
  return 1;
}

static inline ssize_t
rbuffer_get(struct rbuffer *b, uint8_t *c) {
  if (rbuffer_is_empty(b))
    return 0;

  *c = b->data[b->ridx];
  b->ridx = (b->ridx + 1) % b->cap;
  b->size -= 1;
  return 1;
}

/* This function writes to the buffer and reads from fd. */
static inline ssize_t
rbuffer_read_from_fd(struct rbuffer *b, int fd) {
  errno = 0;
  if (rbuffer_is_full(b))
    return 0;

#if 1
  if (rbuffer_is_empty(b)) {
    b->ridx = 0;
    b->widx = 0;
  }
#endif

  if (b->widx <= b->ridx) {
    size_t max = rbuffer_is_empty(b) ? b->cap : b->ridx - b->widx;
    ssize_t n = read(fd, &b->data[b->widx], max);
    if (n <= 0)
      return n;

    b->size += n;
    b->widx += n;
    assert(b->widx <= b->cap);
    b->widx = b->widx % b->cap;
    return n;
  }

  if (b->widx >= b->ridx) {
    size_t max = b->cap - b->widx;
    ssize_t n = read(fd, &b->data[b->widx], max);
    if (n <= 0)
      return n;

    b->size += n;
    b->widx += n;
    assert(b->widx <= b->widx);
    b->widx = b->widx % b->cap;
    return n;
  }

  assert(false && "?");
}

/* This function reads from the buffer and writes to the fd. */
static inline ssize_t
rbuffer_write_to_fd(struct rbuffer *b, int fd) {
  errno = 0;
  if (rbuffer_is_empty(b))
    return 0;

#if 1
  if (rbuffer_is_empty(b)) {
    b->ridx = 0;
    b->widx = 0;
  }
#endif

  if (b->ridx <= b->widx) {
    size_t max = rbuffer_is_full(b) ? b->cap : b->widx - b->ridx;
    ssize_t n = write(fd, &b->data[b->ridx], max);
    if (n <= 0)
      return n;

    b->size -= n;
    b->ridx += n;
    assert(b->ridx <= b->cap);
    b->ridx = b->ridx % b->cap;
    return n;
  }

  if (b->ridx >= b->widx) {
    size_t max = b->cap - b->ridx;
    ssize_t n = write(fd, &b->data[b->ridx], max);
    if (n <= 0)
      return n;

    b->size -= n;
    b->ridx += n;
    assert(b->ridx <= b->widx);
    b->ridx = b->ridx % b->cap;
    return n;
  }

  assert(false && "?");
}

/* Without consuming it, return the length (or something negative if not avil.)
 * of the next line in this buffer, and write a pointer to it to buf. The pointer
 * will be invalidated.
 */
static inline ssize_t
rbuffer_get_line(struct rbuffer *b, const char **line) {
  *line = NULL;
  if (rbuffer_is_empty(b)) {
    return -1;
  }

  ssize_t len = 0;
  uint8_t *start = &b->data[b->ridx];
  uint8_t *end = b->ridx < b->widx ? &b->data[b->widx] : &b->data[b->cap];
  while (start + len < end) {
    if (start[len] == '\n') {
      *line = (char*)start;
      return len + 1;
    }
    len += 1;
  }

  return -1;
}

