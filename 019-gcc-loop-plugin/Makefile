GCC = /home/lou/zeugs/gcc/gcc-install/usr/local/bin
CFLAGS = -Wall -Wextra -Wno-array-bounds -Wno-format -I$(shell $(GCC)/gcc -print-file-name=plugin)/include -O1 -fPIC -g -std=gnu++17 -fno-rtti

.PHONY: all clean run-example

all: liblcplugin.so liblcrt.a

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
	$(GCC)/gcc -Wall -Wextra -O1 -g ./example.c -fplugin=./liblcplugin.so -L. -llcrt -o example
	./example first-arg foo bar 123 hello world

