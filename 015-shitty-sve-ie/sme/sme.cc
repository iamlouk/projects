#include <cstdlib>
#include <cstdio>

#include "arm_sme.h"

int main(int argc, char const *argv[]) {
    svfloat32_t zeros = svdup(0.0f);
    size_t VL = svcntw();
    svbool_t pg = svptrue_b32();

    for (size_t i = 0; i < VL; i++)
        svwrite_hor_za32(0, 0, i, pg, zeros);

    for (size_t i = 0; i < VL; i++) {
        float row[VL];
        float col[VL];
        for (size_t j = 0; j < VL; j++)
            row[j] = col[j] = float(i * VL + j);

        svfloat32_t zrow = svld1(pg, &row[0]);
        svfloat32_t zcol = svld1(pg, &col[0]);
        svmopa_za32_m(0, pg, pg, zrow, zcol);
    }

    printf("data:\n");
    for (size_t i = 0; i < VL; i++) {
        svfloat32_t zrow = svread_hor_za32(zeros, pg, 0, i, 0);
        float row[VL];
        svst1(pg, &row[0], zrow);

        printf(i == 0 ? "[[" : " [");
        for (size_t j = 0; j < VL; j++)
            printf(j == 0 ? "%f" : ", %f", row[j]);
        printf(i == VL - 1 ? "]]\n" : "]\n");
    }

    return 0;
}

