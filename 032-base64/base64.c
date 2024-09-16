#include <assert.h>
#include <errno.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/*
 * A rather naive base64 encoding impl.! More sophisticated stuff to come.
 */

static const size_t BUFSIZE = 4096;
static const uint8_t base64_chars[64] =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

static uint8_t base64_chars_reverse[255];

/* Returns the number of bytes treated. If done is true, that is the last
 * chunck of that to be encoded, and all n bytes should be handled, as well as
 * padding characters written if necessary.
 */
static size_t base64_encode_chunk(size_t n, uint8_t input[n], bool done, FILE *dst) {
  size_t chunks = n / 3, idx = 0;
  uint8_t output[BUFSIZE + BUFSIZE / 2];
  for (size_t c = 0; c < chunks; c++) {
    uint32_t b1 = input[c * 3 + 0] & 0xff, b2 = input[c * 3 + 1] & 0xff,
             b3 = input[c * 3 + 2] & 0xff;
    uint8_t c1 = base64_chars[(b1 >> 2) & 0x3f];
    uint8_t c2 = base64_chars[((b1 << 4) & 0x30) | ((b2 >> 4) & 0x0f)];
    uint8_t c3 = base64_chars[((b2 << 2) & 0x3c) | ((b3 >> 6) & 0x03)];
    uint8_t c4 = base64_chars[((b3 & 0x3f))];
    output[idx++] = c1;
    output[idx++] = c2;
    output[idx++] = c3;
    output[idx++] = c4;
  }

  size_t rem = n % 3;
  if (!done || rem == 0) {
    fwrite(output, sizeof(uint8_t), idx, dst);
    return chunks * 3;
  }

  if (rem == 1) {
    uint32_t b1 = input[chunks * 3 + 0] & 0xff;
    output[idx++] = base64_chars[(b1 >> 2) & 0x3f];
    output[idx++] = base64_chars[(b1 << 4) & 0x30];
    output[idx++] = '=';
    output[idx++] = '=';
  } else if (rem == 2) {
    uint32_t b1 = input[chunks * 3 + 0] & 0xff;
    uint32_t b2 = input[chunks * 3 + 1] & 0xff;
    output[idx++] = base64_chars[(b1 >> 2) & 0x3f];
    output[idx++] = base64_chars[((b1 << 4) & 0x30) | ((b2 >> 4) & 0x0f)];
    output[idx++] = base64_chars[(b2 << 2) & 0x3c];
    output[idx++] = '=';
  }

  fwrite(output, sizeof(uint8_t), idx, dst);
  return n;
}

static int encode(FILE *src, FILE *dst) {
  uint8_t input[BUFSIZE];
  size_t start_offset = 0;
  size_t n = -1ul;
  for (;;) {
    n = fread(&input[start_offset], sizeof(uint8_t), BUFSIZE - start_offset,
              src);
    if (n <= 0) {
      if (ferror(src)) {
        fprintf(stderr, "failed to read: %s\n", strerror(errno));
        return EXIT_FAILURE;
      }

      if (feof(src))
        break;
    }

    bool done = (n < BUFSIZE - start_offset) && feof(src);

    assert(n <= BUFSIZE - start_offset);
    /* done could be initialised with feof() if n < BUFSIZE - start_offset. */
    size_t m = base64_encode_chunk(n + start_offset, &input[0], done, dst);
    if (m == n + start_offset) {
      start_offset = 0;
    } else {
      start_offset = (n + start_offset) - m;
      memmove(&input[0], &input[m], start_offset);
    }
  }

  if (start_offset != 0)
    base64_encode_chunk(start_offset, &input[0], true, stdout);
  return EXIT_SUCCESS;
}

static size_t base64_decode_chunk(size_t n, uint8_t buf[], bool done, FILE *dst) {
  size_t chunks = n / 4;
  size_t rem = n - chunks * 4;
  uint8_t output[BUFSIZE];
  size_t opos = 0;
  for (size_t i = 0; i < chunks; i += 1) {
    uint8_t b1 = base64_chars_reverse[buf[i*4+0]];
    uint8_t b2 = base64_chars_reverse[buf[i*4+1]];
    uint8_t b3 = base64_chars_reverse[buf[i*4+2]];
    uint8_t b4 = base64_chars_reverse[buf[i*4+3]];
    // TODO: Allow whitespace?
    assert(((b1 != 0xff) & (b2 != 0xff) & (b3 != 0xff) & (b4 != 0xff)) &&
           "Expected only valid base64 chars.");

    output[opos++] = (b1 << 2) | ((b2 >> 4) & 0x3);
    output[opos++] = ((b2 << 4) & 0xf0) | ((b3 >> 2) & 0x0f);
    output[opos++] = ((b3 << 6) & 0xc0) | (b4 & 0x3f);
  }

  fwrite(&output[0], sizeof(uint8_t), opos, dst);
  return chunks * 4;
}

static int decode(FILE *src, FILE *dst) {
  // Initialise the reverse mapping.
  for (size_t i = 0; i < sizeof(base64_chars_reverse); i++)
    base64_chars_reverse[i] = 0xff;
  base64_chars_reverse['='] = 0x0;
  for (size_t i = 0; i < 64; i++)
    base64_chars_reverse[(size_t)base64_chars[i]] = i;

  uint8_t buf[BUFSIZE];
  size_t start_offset = 0;
  bool done = false;
  while (!done) {
    size_t n = fread(&buf[start_offset], sizeof(uint8_t),
                     sizeof(buf) - start_offset, src);
    if (n <= 0) {
      done = feof(src) != 0;
      if (ferror(src)) {
        fprintf(stderr, "failed to read: %s\n", strerror(errno));
        return EXIT_FAILURE;
      }
      n = 0;
    }

    size_t m = base64_decode_chunk(start_offset + n, &buf[0], done, dst);
    start_offset = (start_offset + n) - m;
    memmove(&buf[0], &buf[m], start_offset);
  }

  return EXIT_SUCCESS;
}

int main(int argc, const char *argv[]) {
  bool do_decode = argc == 2 && strcmp(argv[1], "--decode") == 0;
  if (argc > 1 && !do_decode) {
    fprintf(stderr,
            "%s: A base64 encoder. Reads (binary data) from stdin, "
            "writes (base64) to stdout.\n",
            argv[0]);
    return EXIT_FAILURE;
  }

  if (do_decode)
    return decode(stdin, stdout);
  else
    return encode(stdin, stdout);
}
