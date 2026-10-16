.PHONY: run build clean

# Go parameters
GOCMD=go
GOBUILD=$(GOCMD) build
GOCLEAN=$(GOCMD) clean
GOMOD=$(GOCMD) mod
BINARY_NAME=lllm

build: $(BINARY_NAME)

$(BINARY_NAME):
	$(GOBUILD) -o $@ .

run: build
	@if [ -f .env ]; then \
		set -a && source .env && set +a && \
		./$(BINARY_NAME); \
	else \
		./$(BINARY_NAME); \
	fi

clean:
	$(GOCLEAN)
	rm -f $(BINARY_NAME)

.PHONY: deps
deps:
	$(GOMOD) download
	$(GOMOD) tidy
