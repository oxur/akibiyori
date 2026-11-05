CARGO ?= cargo
export CARGO_TARGET_DIR := target/observatory-public
COBALT ?= cobalt
PAGEFIND ?= pagefind
EXAMPLE_ROOT := examples/observatory
EXAMPLE_OUTPUT := target/example/observatory-build
EXAMPLE_MARKER := target/example/observatory-build.owned
PREFIX_OUTPUT := target/example/observatory-prefix-build
PREFIX_MARKER := target/example/observatory-prefix-build.owned

.PHONY: compile fmt check test example example-prefix clean-example

compile:
	$(CARGO) check

fmt:
	$(CARGO) fmt --all

check:
	$(CARGO) fmt --all -- --check
	$(CARGO) check --locked
	$(CARGO) clippy --locked --all-targets -- -D warnings
	COBALT="$(COBALT)" PAGEFIND="$(PAGEFIND)" $(CARGO) test --locked
	$(CARGO) doc --locked --no-deps

test:
	$(CARGO) test --locked

example:
	@if [ -e "$(EXAMPLE_OUTPUT)" ] || [ -L "$(EXAMPLE_OUTPUT)" ]; then \
	  if [ -L "$(EXAMPLE_OUTPUT)" ] || [ ! -d "$(EXAMPLE_OUTPUT)" ] || [ ! -f "$(EXAMPLE_MARKER)" ]; then echo "refusing to remove unowned $(EXAMPLE_OUTPUT)" >&2; exit 1; fi; \
	  rm -rf "$(EXAMPLE_OUTPUT)"; \
	fi
	@rm -f "$(EXAMPLE_MARKER)"
	COBALT="$(COBALT)" PAGEFIND="$(PAGEFIND)" $(CARGO) run --locked -- build --config "$(EXAMPLE_ROOT)/config.json" --output "$(EXAMPLE_OUTPUT)" --cobalt "$(COBALT)" --pagefind "$(PAGEFIND)"
	@touch "$(EXAMPLE_MARKER)"

example-prefix:
	@if [ -e "$(PREFIX_OUTPUT)" ] || [ -L "$(PREFIX_OUTPUT)" ]; then \
	  if [ -L "$(PREFIX_OUTPUT)" ] || [ ! -d "$(PREFIX_OUTPUT)" ] || [ ! -f "$(PREFIX_MARKER)" ]; then echo "refusing to remove unowned $(PREFIX_OUTPUT)" >&2; exit 1; fi; \
	  rm -rf "$(PREFIX_OUTPUT)"; \
	fi
	@rm -f "$(PREFIX_MARKER)"
	COBALT="$(COBALT)" PAGEFIND="$(PAGEFIND)" $(CARGO) run --locked -- build --config "$(EXAMPLE_ROOT)/config.json" --output "$(PREFIX_OUTPUT)" --base-path /akibiyori-test --cobalt "$(COBALT)" --pagefind "$(PAGEFIND)"
	@touch "$(PREFIX_MARKER)"

clean-example:
	@for output in "$(EXAMPLE_OUTPUT)" "$(PREFIX_OUTPUT)"; do \
	  marker="$${output}.owned"; \
	  if [ -e "$$output" ] || [ -L "$$output" ]; then \
	    if [ -L "$$output" ] || [ ! -d "$$output" ] || [ ! -f "$$marker" ]; then echo "refusing to remove unowned $$output" >&2; exit 1; fi; \
	    rm -rf "$$output"; \
	  fi; \
	  rm -f "$$marker"; \
	done
