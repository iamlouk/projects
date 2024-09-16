# The system installation of GCC will work as well on most systems, but I wanted a GCC with `--enable-checking`.
GCCDIR = /home/lou/zeugs/gcc/gcc-install/usr/local/bin

# `-fno-rtti` and the plugin include path are obligatory for this to work!
CFLAGS = -Wall -Wextra -Wno-array-bounds -Wno-format -I$(shell $(GCCDIR)/gcc -print-file-name=plugin)/include -O1 -fPIC -g -std=gnu++17 -fno-rtti

.PHONY: all clean run-example

all: liblcplugin.so liblcrt.a

# There is no need to compile the plugin itself with the compiler it will be used in:
liblcplugin.so: loop-counter-plugin.cc
	gcc $(CFLAGS) -o $@ -shared $^

liblcrt.a: loop-counter-rt.cc
	gcc $(CFLAGS) -c -o liblcrt.o $^
	ar -rcs $@ liblcrt.o

clean:
	rm -f *.so *.a
	rm -f *.o
	rm -f ./example

run-example: liblcplugin.so liblcrt.a example.c
	# This is where the magic happens:
	$(GCCDIR)/gcc -Wall -Wextra -O1 -g ./example.c -fplugin=./liblcplugin.so -L. -llcrt -o example
	# Using a shared library for the definition of the injected calls would work as well.
	./example first-arg foo bar 123 hello world

