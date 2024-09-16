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

/* Returns the number of bytes treated. If done is true, that is the last
 * chunck of that to be encoded, and all n bytes should be handled, as well as
 * padding characters written if necessary.
 */
static size_t base64_encode(size_t n, uint8_t input[n], bool done, FILE *dst) {
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

int main(int argc, const char *argv[]) {
  if (argc > 1) {
    fprintf(stderr,
            "%s: A base64 encoder. Reads (binary data) from stdin, "
            "writes (base64) to stdout.\n",
            argv[0]);
    return EXIT_FAILURE;
  }

  uint8_t input[BUFSIZE];
  size_t start_offset = 0;
  size_t n = -1ul;
  for (;;) {
    n = fread(&input[start_offset], sizeof(uint8_t), BUFSIZE - start_offset,
              stdin);
    if (n <= 0) {
      if (ferror(stdin)) {
        fprintf(stderr, "%s: failed to read from stdin: %s\n", argv[0],
                strerror(errno));
        return EXIT_FAILURE;
      }

      if (feof(stdin))
        break;
    }

    bool done = (n < BUFSIZE - start_offset) && feof(stdin);

    assert(n <= BUFSIZE - start_offset);
    /* done could be initialised with feof() if n < BUFSIZE - start_offset. */
    size_t m = base64_encode(n + start_offset, &input[0], done, stdout);
    if (m == n + start_offset) {
      start_offset = 0;
    } else {
      start_offset = (n + start_offset) - m;
      memmove(&input[0], &input[m], start_offset);
    }
  }

  if (start_offset != 0)
    base64_encode(start_offset, &input[0], true, stdout);

  return EXIT_SUCCESS;
}
