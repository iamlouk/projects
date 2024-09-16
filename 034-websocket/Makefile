HARE = hare
HAREFLAGS =

.PHONY: all check clean

# TODO: https://harelang.org/documentation/usage/project-structure.html

all: ws echo

ws: ws.ha
	$(HARE) build $(HAREFLAGS) -lcrypto -o $@ $^

echo: echo.ha
	$(HARE) build $(HAREFLAGS) -o $@ $^

check:
	$(HARE) test $(HAREFLAGS)

clean:
	rm -f ./ws ./echo

