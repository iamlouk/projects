CXX = g++
CXXFLAGS = -std=c++17 -Wall -Wextra -Os -I./systemc-install/include/
LDFLAGS = -L./systemc-install/lib-linux64 -lm -lsystemc -Wl,-rpath=./systemc-install/lib-linux64

.PHONY: ./hello-world/hello-world.run ./counter.run ./mini-cpu.run

./hello-world/hello-world: ./hello-world/hello-world.cc
	$(CXX) $(CXXFLAGS) $(LDFLAGS) $< -o $@

./hello-world/hello-world.run: ./hello-world/hello-world
	LD_LIBRARY_PATH=$(realpath ./systemc-install/lib-linux64) ./$<

./counter: ./counter.cc
	$(CXX) $(CXXFLAGS) $(LDFLAGS) $< -o $@

./counter.run: ./counter
	LD_LIBRARY_PATH=$(realpath ./systemc-install/lib-linux64) ./$<

./mini-cpu: ./mini-cpu.cc
	$(CXX) $(CXXFLAGS) $(LDFLAGS) $< -o $@

./mini-cpu.run: ./mini-cpu
	LD_LIBRARY_PATH=$(realpath ./systemc-install/lib-linux64) ./$<


