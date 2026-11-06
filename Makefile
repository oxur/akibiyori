CARGO ?= cargo
export CARGO_TARGET_DIR := target/observatory-public
COBALT ?= cobalt
PAGEFIND ?= pagefind
EXAMPLE_ROOT := examples/observatory
EXAMPLE_OUTPUT := target/example/observatory-build
EXAMPLE_MARKER := target/example/observatory-build.owned
PREFIX_OUTPUT := target/example/observatory-prefix-build
PREFIX_MARKER := target/example/observatory-prefix-build.owned
REFERENCE_CONFIG := examples/fumadocs-reference.json
REFERENCE_OUTPUT := target/example/fumadocs-reference-build
REFERENCE_MARKER := target/example/fumadocs-reference-build.owned
REFERENCE_PREFIX_OUTPUT := target/example/fumadocs-reference-prefix-build
REFERENCE_PREFIX_MARKER := target/example/fumadocs-reference-prefix-build.owned
PLANETS_CONFIG := examples/planets.json
PLANETS_OUTPUT := target/example/planets-build
PLANETS_MARKER := target/example/planets-build.owned
PLANETS_PREFIX_OUTPUT := target/example/planets-prefix-build
PLANETS_PREFIX_MARKER := target/example/planets-prefix-build.owned
KUBERNETES_CONFIG := examples/kubernetes/config.json
KUBERNETES_OUTPUT := target/example/kubernetes-build
KUBERNETES_MARKER := target/example/kubernetes-build.owned
KUBERNETES_PREFIX_OUTPUT := target/example/kubernetes-prefix-build
KUBERNETES_PREFIX_MARKER := target/example/kubernetes-prefix-build.owned
DOCS_OUTPUT := target/project-docs-build
DOCS_MARKER := target/project-docs-build.owned
DOCS_PREFIX_OUTPUT := target/project-docs-prefix-build
DOCS_PREFIX_MARKER := target/project-docs-prefix-build.owned

.PHONY: icons compile fmt check test docs docs-prefix demo-site demo-site-prefix example example-prefix reference-example reference-example-prefix planets-example planets-example-prefix kubernetes-example kubernetes-example-prefix clean-example

compile:
	$(CARGO) check

icons:
	sh scripts/generate-icons.sh

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

reference-example:
	@if [ -e "$(REFERENCE_OUTPUT)" ] || [ -L "$(REFERENCE_OUTPUT)" ]; then \
	  if [ -L "$(REFERENCE_OUTPUT)" ] || [ ! -d "$(REFERENCE_OUTPUT)" ] || [ ! -f "$(REFERENCE_MARKER)" ]; then echo "refusing to remove unowned $(REFERENCE_OUTPUT)" >&2; exit 1; fi; \
	  rm -rf "$(REFERENCE_OUTPUT)"; \
	fi
	@rm -f "$(REFERENCE_MARKER)"
	COBALT="$(COBALT)" PAGEFIND="$(PAGEFIND)" $(CARGO) run --locked -- build --config "$(REFERENCE_CONFIG)" --output "$(REFERENCE_OUTPUT)" --cobalt "$(COBALT)" --pagefind "$(PAGEFIND)"
	@touch "$(REFERENCE_MARKER)"

reference-example-prefix:
	@if [ -e "$(REFERENCE_PREFIX_OUTPUT)" ] || [ -L "$(REFERENCE_PREFIX_OUTPUT)" ]; then \
	  if [ -L "$(REFERENCE_PREFIX_OUTPUT)" ] || [ ! -d "$(REFERENCE_PREFIX_OUTPUT)" ] || [ ! -f "$(REFERENCE_PREFIX_MARKER)" ]; then echo "refusing to remove unowned $(REFERENCE_PREFIX_OUTPUT)" >&2; exit 1; fi; \
	  rm -rf "$(REFERENCE_PREFIX_OUTPUT)"; \
	fi
	@rm -f "$(REFERENCE_PREFIX_MARKER)"
	COBALT="$(COBALT)" PAGEFIND="$(PAGEFIND)" $(CARGO) run --locked -- build --config "$(REFERENCE_CONFIG)" --output "$(REFERENCE_PREFIX_OUTPUT)" --base-path /fumadocs-reference --cobalt "$(COBALT)" --pagefind "$(PAGEFIND)"
	@touch "$(REFERENCE_PREFIX_MARKER)"

planets-example:
	@if [ -e "$(PLANETS_OUTPUT)" ] || [ -L "$(PLANETS_OUTPUT)" ]; then \
	  if [ -L "$(PLANETS_OUTPUT)" ] || [ ! -d "$(PLANETS_OUTPUT)" ] || [ ! -f "$(PLANETS_MARKER)" ]; then echo "refusing to remove unowned $(PLANETS_OUTPUT)" >&2; exit 1; fi; \
	  rm -rf "$(PLANETS_OUTPUT)"; \
	fi
	@rm -f "$(PLANETS_MARKER)"
	COBALT="$(COBALT)" PAGEFIND="$(PAGEFIND)" $(CARGO) run --locked -- build --config "$(PLANETS_CONFIG)" --output "$(PLANETS_OUTPUT)" --cobalt "$(COBALT)" --pagefind "$(PAGEFIND)"
	@touch "$(PLANETS_MARKER)"

planets-example-prefix:
	@if [ -e "$(PLANETS_PREFIX_OUTPUT)" ] || [ -L "$(PLANETS_PREFIX_OUTPUT)" ]; then \
	  if [ -L "$(PLANETS_PREFIX_OUTPUT)" ] || [ ! -d "$(PLANETS_PREFIX_OUTPUT)" ] || [ ! -f "$(PLANETS_PREFIX_MARKER)" ]; then echo "refusing to remove unowned $(PLANETS_PREFIX_OUTPUT)" >&2; exit 1; fi; \
	  rm -rf "$(PLANETS_PREFIX_OUTPUT)"; \
	fi
	@rm -f "$(PLANETS_PREFIX_MARKER)"
	COBALT="$(COBALT)" PAGEFIND="$(PAGEFIND)" $(CARGO) run --locked -- build --config "$(PLANETS_CONFIG)" --output "$(PLANETS_PREFIX_OUTPUT)" --base-path /planets-reference --cobalt "$(COBALT)" --pagefind "$(PAGEFIND)"
	@touch "$(PLANETS_PREFIX_MARKER)"

kubernetes-example:
	@if [ -e "$(KUBERNETES_OUTPUT)" ] || [ -L "$(KUBERNETES_OUTPUT)" ]; then \
	  if [ -L "$(KUBERNETES_OUTPUT)" ] || [ ! -d "$(KUBERNETES_OUTPUT)" ] || [ ! -f "$(KUBERNETES_MARKER)" ]; then echo "refusing to remove unowned $(KUBERNETES_OUTPUT)" >&2; exit 1; fi; \
	  rm -rf "$(KUBERNETES_OUTPUT)"; \
	fi
	@rm -f "$(KUBERNETES_MARKER)"
	COBALT="$(COBALT)" PAGEFIND="$(PAGEFIND)" $(CARGO) run --locked -- build --config "$(KUBERNETES_CONFIG)" --output "$(KUBERNETES_OUTPUT)" --cobalt "$(COBALT)" --pagefind "$(PAGEFIND)"
	@touch "$(KUBERNETES_MARKER)"

kubernetes-example-prefix:
	@if [ -e "$(KUBERNETES_PREFIX_OUTPUT)" ] || [ -L "$(KUBERNETES_PREFIX_OUTPUT)" ]; then \
	  if [ -L "$(KUBERNETES_PREFIX_OUTPUT)" ] || [ ! -d "$(KUBERNETES_PREFIX_OUTPUT)" ] || [ ! -f "$(KUBERNETES_PREFIX_MARKER)" ]; then echo "refusing to remove unowned $(KUBERNETES_PREFIX_OUTPUT)" >&2; exit 1; fi; \
	  rm -rf "$(KUBERNETES_PREFIX_OUTPUT)"; \
	fi
	@rm -f "$(KUBERNETES_PREFIX_MARKER)"
	COBALT="$(COBALT)" PAGEFIND="$(PAGEFIND)" $(CARGO) run --locked -- build --config "$(KUBERNETES_CONFIG)" --output "$(KUBERNETES_PREFIX_OUTPUT)" --base-path /kubernetes-reference --cobalt "$(COBALT)" --pagefind "$(PAGEFIND)"
	@touch "$(KUBERNETES_PREFIX_MARKER)"

clean-example:
	@for output in "$(EXAMPLE_OUTPUT)" "$(PREFIX_OUTPUT)" "$(REFERENCE_OUTPUT)" "$(REFERENCE_PREFIX_OUTPUT)" "$(PLANETS_OUTPUT)" "$(PLANETS_PREFIX_OUTPUT)" "$(KUBERNETES_OUTPUT)" "$(KUBERNETES_PREFIX_OUTPUT)"; do \
	  marker="$${output}.owned"; \
	  if [ -e "$$output" ] || [ -L "$$output" ]; then \
	    if [ -L "$$output" ] || [ ! -d "$$output" ] || [ ! -f "$$marker" ]; then echo "refusing to remove unowned $$output" >&2; exit 1; fi; \
	    rm -rf "$$output"; \
	  fi; \
	  rm -f "$$marker"; \
	done

# Project documentation is built independently from the assembled three-demo hub.
docs:
	@if [ -e "$(DOCS_OUTPUT)" ] || [ -L "$(DOCS_OUTPUT)" ]; then \
	  if [ -L "$(DOCS_OUTPUT)" ] || [ ! -d "$(DOCS_OUTPUT)" ] || [ -L "$(DOCS_MARKER)" ] || [ ! -f "$(DOCS_MARKER)" ] || [ "$$(wc -l < "$(DOCS_MARKER)")" -ne 1 ] || [ "$$(cat "$(DOCS_MARKER)")" != 'akibiyori:project-docs:root:v1' ]; then echo "refusing to remove unowned $(DOCS_OUTPUT)" >&2; exit 1; fi; \
	  rm -rf "$(DOCS_OUTPUT)"; rm -f "$(DOCS_MARKER)"; \
	elif [ -e "$(DOCS_MARKER)" ] || [ -L "$(DOCS_MARKER)" ]; then \
	  if [ -L "$(DOCS_MARKER)" ] || [ ! -f "$(DOCS_MARKER)" ] || [ "$$(wc -l < "$(DOCS_MARKER)")" -ne 1 ] || [ "$$(cat "$(DOCS_MARKER)")" != 'akibiyori:project-docs:root:v1' ]; then echo "refusing to remove unowned $(DOCS_MARKER)" >&2; exit 1; fi; \
	  rm -f "$(DOCS_MARKER)"; \
	fi
	COBALT="$(COBALT)" PAGEFIND="$(PAGEFIND)" $(CARGO) run --locked -- build --config akibiyori-docs.json --output "$(DOCS_OUTPUT)" --cobalt "$(COBALT)" --pagefind "$(PAGEFIND)"
	@printf '%s\n' 'akibiyori:project-docs:root:v1' > "$(DOCS_MARKER)"

docs-prefix:
	@if [ -e "$(DOCS_PREFIX_OUTPUT)" ] || [ -L "$(DOCS_PREFIX_OUTPUT)" ]; then \
	  if [ -L "$(DOCS_PREFIX_OUTPUT)" ] || [ ! -d "$(DOCS_PREFIX_OUTPUT)" ] || [ -L "$(DOCS_PREFIX_MARKER)" ] || [ ! -f "$(DOCS_PREFIX_MARKER)" ] || [ "$$(wc -l < "$(DOCS_PREFIX_MARKER)")" -ne 1 ] || [ "$$(cat "$(DOCS_PREFIX_MARKER)")" != 'akibiyori:project-docs:prefix:v1' ]; then echo "refusing to remove unowned $(DOCS_PREFIX_OUTPUT)" >&2; exit 1; fi; \
	  rm -rf "$(DOCS_PREFIX_OUTPUT)"; rm -f "$(DOCS_PREFIX_MARKER)"; \
	elif [ -e "$(DOCS_PREFIX_MARKER)" ] || [ -L "$(DOCS_PREFIX_MARKER)" ]; then \
	  if [ -L "$(DOCS_PREFIX_MARKER)" ] || [ ! -f "$(DOCS_PREFIX_MARKER)" ] || [ "$$(wc -l < "$(DOCS_PREFIX_MARKER)")" -ne 1 ] || [ "$$(cat "$(DOCS_PREFIX_MARKER)")" != 'akibiyori:project-docs:prefix:v1' ]; then echo "refusing to remove unowned $(DOCS_PREFIX_MARKER)" >&2; exit 1; fi; \
	  rm -f "$(DOCS_PREFIX_MARKER)"; \
	fi
	COBALT="$(COBALT)" PAGEFIND="$(PAGEFIND)" $(CARGO) run --locked -- build --config akibiyori-docs.json --output "$(DOCS_PREFIX_OUTPUT)" --base-path /akibiyori-docs --cobalt "$(COBALT)" --pagefind "$(PAGEFIND)"
	@printf '%s\n' 'akibiyori:project-docs:prefix:v1' > "$(DOCS_PREFIX_MARKER)"

DEMO_SITE_ROOT := target/demo-site/root
DEMO_SITE_PREFIX := target/demo-site/prefix

demo-site:
	$(CARGO) build --locked
	AKIBIYORI="$(CARGO_TARGET_DIR)/debug/akibiyori" COBALT="$(COBALT)" PAGEFIND="$(PAGEFIND)" scripts/build-demo-site.sh root "$(DEMO_SITE_ROOT)"

demo-site-prefix:
	$(CARGO) build --locked
	AKIBIYORI="$(CARGO_TARGET_DIR)/debug/akibiyori" COBALT="$(COBALT)" PAGEFIND="$(PAGEFIND)" scripts/build-demo-site.sh prefix "$(DEMO_SITE_PREFIX)"
