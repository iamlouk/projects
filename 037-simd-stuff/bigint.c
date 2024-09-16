#include <stdint.h>
#include <stddef.h>
#include <stdio.h>
#include <errno.h>
#include <arm_sve.h>
#include <string.h>

#define PRINT_VEC(typ, s) \
    __attribute__((unused)) \
    static void print_sv##typ(FILE* f, const char *name, sv##typ v) { \
        size_t N = svlen(v); \
        fprintf(f, "%s = [", name); \
        for (size_t i = 0; i < N; i += 1) { \
            fprintf(f, i == 0 ? "%ld" : ", %ld", \
                    (uint64_t)svclastb(svwhilele_b##s(0ul, i), (typ)-1u, v)); \
        } \
        fprintf(f, "]\n"); \
    }

PRINT_VEC(uint8_t, 8);
PRINT_VEC(uint16_t, 16);
PRINT_VEC(uint32_t, 32);
PRINT_VEC(uint64_t, 64);

uint64_t parse_uint(const char *text, const char **endptr) {
    // Initialization:
    // This is optimized for 128 bit SVE, it will work with larger
    // vector lengths, but it won't benefit in any way.
    const svbool_t pt16 = svptrue_pat_b8(SV_VL16);
    const svuint8_t multipliers_u8
        = svdupq_u8(10, 1, 10, 1, 10, 1, 10, 1, 10, 1, 10, 1, 10, 1, 10, 1);
    const svuint16_t multipliers_u16
        = svdupq_u16(100, 1, 100, 1, 100, 1, 100, 1);
    const svuint32_t multipliers_u32
        = svdupq_u32(10000, 1, 10000, 1);
    const svuint64_t multipliers_u64
        = svdupq_u64(100000000, 1);

    // Actuall parsing:
    // FIXME: This load should technically be first-faulting/speculative.
    svuint8_t vu8 = svld1_u8(pt16, (const uint8_t*)&text[0]);
    vu8 = svsub_n_u8_z(pt16, vu8, (uint8_t)'0');
    svbool_t not_digits = svcmpgt_n_u8(pt16, vu8, 9);
    // Error out if the first character is not a digits.
    if (svptest_first(pt16, not_digits))
        return (errno = EINVAL), -1u;

    // Count the number of consecutive digits.
    svbool_t p = svbrkb_z(pt16, not_digits);
    size_t digits = svcntp_b8(pt16, p);
    // TODO: Handle this?
    if (digits > 15)
        return (errno = ERANGE) -1u;
    if (endptr)
        *endptr = text + digits;

#if 1
    // Shift all digits so that the multipliers above match and there is
    // no need to shift or re-calculate those.
    // This works even with non-128 SVE because of the pt16 in the not.
    vu8 = svsplice_u8(svnot_b_z(pt16, p), svdup_u8(0), vu8);
#else
    while (digits++ != 16)
        vu8 = svinsr_n_u8(vu8, 0);
#endif

    // print_svuint8_t(stderr, "vu8", vu8);
    vu8 = svmul_u8_z(pt16, vu8, multipliers_u8);

    svuint16_t vu16 = svadalp_u16_z(pt16, svdup_u16(0), vu8);
    // print_svuint16_t(stderr, "vu16", vu16);
    vu16 = svmul_u16_z(pt16, vu16, multipliers_u16);

    svuint32_t vu32 = svadalp_u32_z(pt16, svdup_u32(0), vu16);
    // print_svuint32_t(stderr, "vu32", vu32);
    vu32 = svmul_u32_z(pt16, vu32, multipliers_u32);

    svuint64_t vu64 = svadalp_u64_z(pt16, svdup_u64(0), vu32);
    // print_svuint64_t(stderr, "vu64", vu64);
    vu64 = svmul_u64_z(pt16, vu64, multipliers_u64);
    return svaddv_u64(pt16, vu64);
}

#ifdef TEST
#include <stdlib.h>
#define COL_RED     "\033[0;31m"
#define COL_GREEN   "\033[0;32m"
#define COL_YELLOW  "\033[0;33m"
#define COL_BLUE    "\033[0;34m"
#define COL_CYAN    "\033[0;36m"
#define COL_GREY    "\033[0;2m"
#define COL_RESET   "\033[0m"
int main() {
    static const struct {
        const char *text;
        uint64_t parsed;
        size_t num_digits;
    } examples[] = {
        {
            .text = "123456789",
            .parsed = 123456789,
            .num_digits = 9
        },
        {
            .text = "4294967295",
            .parsed = UINT32_MAX,
            .num_digits = 10
        },
        {
            .text = "0",
            .parsed = 0,
            .num_digits = 1
        },
        {
            .text = "111111111111111",
            .parsed = 111111111111111,
            .num_digits = 15
        },
        {
            .text = "999999999999999",
            .parsed = 999999999999999,
            .num_digits = 15
        }
    };
    for (size_t i = 0; i < sizeof(examples) / sizeof(examples[0]); i += 1) {
        const char *endptr = NULL;
        errno = 0;
        uint64_t res = parse_uint(examples[i].text, &endptr);
        if (res == -1u && errno != 0) {
            fprintf(stderr, COL_YELLOW "error:" COL_RESET " %s\n", strerror(errno));
            return EXIT_FAILURE;
        }
        if (res != examples[i].parsed) {
            fprintf(stderr, COL_RED "wrong:" COL_RESET " %ld != %ld\n",
                    res, examples[i].parsed);
            return EXIT_FAILURE;
        }
        if (endptr != examples[i].text + examples[i].num_digits) {
            fprintf(stderr, COL_RED "wrong endptr!" COL_RESET "\n");
            return EXIT_FAILURE;
        }
        fprintf(stderr, COL_GREEN "success:" COL_RESET " test=%ld\n", res);
    }

    return EXIT_SUCCESS;
}
#endif
