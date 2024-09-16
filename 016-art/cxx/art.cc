#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <string>

#include "art.h"

int main(int argc, const char *argv[]) {
	(void) argc;
	(void) argv;

	auto k1 = new std::string("abc");

	art::Art<std::string> art;
	art.lookup((const uint8_t*)"abc");
	art.insert((const uint8_t*) k1->c_str(), k1);

	return EXIT_SUCCESS;
}

