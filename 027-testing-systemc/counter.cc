#include <systemc.h>
#include <cstddef>
#include <cstdlib>
#include <cstdint>
#include <cassert>

/* For every clock signal, increment a counter by one. */
SC_MODULE(Counter) {
	static constexpr size_t NUM_BITS = 3;
	static constexpr uint64_t MAX = (1 << (NUM_BITS - 1)) | ((1 << (NUM_BITS - 1)) - 1);
	static_assert(NUM_BITS <= 64, "Would need larger internal type");

	/* Ports: */
	sc_in<bool> Clock;
	sc_in<bool> Reset;
	sc_in<bool> Enable;
	sc_out<sc_uint<NUM_BITS>> Result;

	/* Internals: */
	uint64_t Count;

	/* Could be called whatever: */
	void tick() {
		/* Reset if resetting... */
		if (Reset.read()) {
			Count = 0;
			Result.write(0);
			return;
		}

		/* Skip most stuff if not enabled... */
		if (!Enable.read())
			return;

		if (Count == MAX)
			Count = 0;
		else
			Count += 1;
		Result.write(Count);
	}

	/* SystemC magic: Module setup */
	SC_CTOR(Counter) {
		/* Register the function as method. */
		SC_METHOD(tick);

		/* Update on every clock (rising or falling?) edge: */
		sensitive << Clock.pos();
	}
};

int sc_main(int argc, char **argv) {
	(void) argc;
	(void) argv;
	sc_clock TestClock("test-clock", 1, SC_NS);
	sc_signal<bool> Reset("reset", 1);
	sc_signal<bool> Enable("enable", 0);
	sc_signal<sc_uint<3>> Result("counter", 0xf);

	Counter C("4BitCounter");
	C.Clock(TestClock);
	C.Reset(Reset);
	C.Enable(Enable);
	C.Result(Result);
	sc_start(1, SC_NS);
	Enable.write(1);
	Reset.write(0);
	uint64_t expexted_values[] = { 0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3 };
	for (size_t i = 0; i < (sizeof(expexted_values) / sizeof(expexted_values[9])); i++) {
		assert(uint64_t(Result.read()) == expexted_values[i]);
		sc_start(1, SC_NS);
	}

	printf("Simulation successful!\n");
	return EXIT_SUCCESS;
}

