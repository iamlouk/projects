#include <cstdlib>

#include "vectorclass_sve.h"

Vecf32 foo(Vecf32 x, Vecf32 y) {
	Vecf32 a = x + y;
	Vecf32 b = x * y;
	b += x;
	a *= x;
	Vecf32 c = x - y;
	return select(c < x, a + b + c, a - b - c);
}


