#include <arm_sve.h>
#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>

#define PRINT_VEC(typ, s) \
    __attribute__((unused)) \
    static void print_sv##typ(FILE* f, const char *name, sv##typ v) { \
        size_t N = svlen(v); \
        fprintf(f, "%s = [", name); \
        for (size_t i = 0; i < N; i += 1) { \
            fprintf(f, i == 0 ? "%#04lx" : ", %#04lx", \
                    (uint64_t)svclastb(svwhilele_b##s(0ul, i), (typ)-1u, v)); \
        } \
        fprintf(f, "]\n"); \
    }

PRINT_VEC(uint8_t, 8);

__attribute__((unused))
static void print_svbool(FILE *f, const char *name, svbool_t b) {
    print_svuint8_t(f, name, svsel(b, svdup_u8(1), svdup_u8(0)));
}

static size_t base64_encode(size_t N, const uint8_t input[N],
                            uint8_t output[restrict static N + (N / 2)]) {
  // TODO: Implement padding. Masking automatically zeroes, it should not be
  // that hard to append '=' afterwards.
  assert(N % 3 == 0 && "Padding unimplemented!");
  const size_t VL = svcntb(), M = (N / 3) * 4;
  assert(VL % 4 == 0 && "WTF?");

  // 3 input bytes become one base64 ascii byte.
  uint8_t reorder1_buf[VL], reorder2_buf[VL];
  uint8_t lshift1_buf[VL], rshift2_buf[VL];
  uint8_t mask1_buf[VL], mask2_buf[VL];
  // TODO: This is kind of a repeating sequence, construct it more efficiently?
  // Or: Be VL-specific, not agnostic, and store these sequences in .text/as const?
  for (size_t i = 0, j = 0; i < VL; i += 4, j += 3) {
    reorder1_buf[i + 0] = 0xff;
    reorder1_buf[i + 1] = j + 0;
    reorder1_buf[i + 2] = j + 1;
    reorder1_buf[i + 3] = j + 2;

    reorder2_buf[i + 0] = j + 0;
    reorder2_buf[i + 1] = j + 1;
    reorder2_buf[i + 2] = j + 2;
    reorder2_buf[i + 3] = 0xff;

    lshift1_buf[i + 0] = 0;
    lshift1_buf[i + 1] = 4;
    lshift1_buf[i + 2] = 2;
    lshift1_buf[i + 3] = 0;

    rshift2_buf[i + 0] = 2;
    rshift2_buf[i + 1] = 4;
    rshift2_buf[i + 2] = 6;
    rshift2_buf[i + 3] = 0;

    mask1_buf[i + 0] = 0x00;
    mask1_buf[i + 1] = 0x30;
    mask1_buf[i + 2] = 0x3c;
    mask1_buf[i + 3] = 0x3f;

    mask2_buf[i + 0] = 0x3f;
    mask2_buf[i + 1] = 0x0f;
    mask2_buf[i + 2] = 0x03;
    mask2_buf[i + 3] = 0x00;
  }

  svbool_t pt = svptrue_b8();
  svuint8_t reorder1 = svld1(pt, &reorder1_buf[0]),
            reorder2 = svld1(pt, &reorder2_buf[0]),
            lshift1 = svld1(pt, &lshift1_buf[0]),
            rshift2 = svld1(pt, &rshift2_buf[0]),
            mask1 = svld1(pt, &mask1_buf[0]),
            mask2 = svld1(pt, &mask2_buf[0]);

  for (size_t i = 0, j = 0; i < N; i += (VL / 4) * 3, j += VL) {
    svbool_t lm = svwhilelt_b8(i, N),
             sm = svwhilelt_b8(j, M);

    // Basically a vector version of the core logic in ../032-base64/base64.c
    svuint8_t raw = svld1(lm, &input[i]),
              v1 = svtbl(raw, reorder1),
              v2 = svtbl(raw, reorder2);
    v1 = svand_u8_x(pt, svlsl_u8_x(pt, v1, lshift1), mask1);
    v2 = svand_u8_x(pt, svlsr_u8_x(pt, v2, rshift2), mask2);
    svuint8_t res = svorr_u8_x(pt, v1, v2);
    assert(!svptest_any(pt, svcmpge_n_u8(pt, res, 64)));


    // print_svuint8_t(stderr, "raw", raw);
    // print_svuint8_t(stderr, "res", res);
#if 0
    static const uint8_t base64_chars[64] =
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    // SVE has no 8-bit offset gathers, so the equiv. of this is not
    // directly vectorizeable. Four 32-bit offset gathers could be used.
    uint8_t buf[VL];
    svst1(sm, &buf[0], res);
    for (size_t k = 0; k < VL && j + k < M; k++)
      output[j + k] = base64_chars[buf[k]];
    continue;
#endif

    // Map the characters to what they will be in the encoding charset.
    // TODO: Improve the boolean arith. in here.
    svbool_t uppercase = svcmplt_n_u8(sm, res, 26),
             not_uppercase = svnot_b_z(sm, uppercase),
             lowercase = svcmplt_n_u8(not_uppercase, res, 26 + 26),
             not_letter = svnot_b_z(not_uppercase, lowercase),
             digit = svcmplt_n_u8(not_letter, res, 26 + 26 + 10),
             plus = svcmpeq_n_u8(sm, res, 26 + 26 + 10 + 1),
             slash = svcmpeq_n_u8(sm, res, 26 + 26 + 10 + 2);

    svuint8_t encoded = res;
    encoded = svadd_n_u8_m(uppercase, encoded, 'A');
    encoded = svadd_n_u8_m(lowercase, encoded, 'a' - 26);
    encoded = svadd_n_u8_m(digit, encoded, '0' - 26 - 26);
    encoded = svsel(plus, svdup_u8('+'), encoded);
    encoded = svsel(slash, svdup_u8('/'), encoded);
    svst1(sm, &output[j], encoded);
  }

  return M;
}

#ifdef TEST
int main(int argc, const char *argv[]) {
  (void)argc;
  (void)argv;

  const char input[] = "hello world\n";
  char output[sizeof(input) * 2];
  memset(output, 0x0, sizeof(output));
  fprintf(stderr, "input:    '%s'\n", input);
  size_t len = base64_encode(strlen(input), (const uint8_t*)&input[0], (uint8_t*)&output[0]);
  output[len] = '\0';
  fprintf(stderr, "result:   %s\n", output);
  fprintf(stderr, "expected: %s\n", "aGVsbG8gd29ybGQK");

  return EXIT_SUCCESS;
}
#endif
