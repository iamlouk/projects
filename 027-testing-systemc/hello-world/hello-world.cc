#include <systemc.h>

SC_MODULE (hello) {
	SC_CTOR (hello) {}

	void say_hello() {
		std::cout << "Hello World!" << std::endl;
	}
};

int sc_main(int argc, char* argv[]) {
	(void) argc;
	(void) argv;
	hello h("hello");
	h.say_hello();
	return 0;
}
