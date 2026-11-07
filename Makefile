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
PETSTORE_CONFIG := examples/petstore/config.json
PETSTORE_OUTPUT := target/example/petstore-build
PETSTORE_MARKER := target/example/petstore-build.owned
PETSTORE_PREFIX_OUTPUT := target/example/petstore-prefix-build
PETSTORE_PREFIX_MARKER := target/example/petstore-prefix-build.owned
THEME_CONFIG := examples/observatory/config-theme.json
THEME_OUTPUT := target/example/theme-build
THEME_MARKER := target/example/theme-build.owned
THEME_PREFIX_OUTPUT := target/example/theme-prefix-build
THEME_PREFIX_MARKER := target/example/theme-prefix-build.owned
DOCS_OUTPUT := target/project-docs-build
DOCS_MARKER := target/project-docs-build.owned
DOCS_PREFIX_OUTPUT := target/project-docs-prefix-build
DOCS_PREFIX_MARKER := target/project-docs-prefix-build.owned

.PHONY: icons compile fmt check test docs docs-prefix demo-site demo-site-prefix example example-prefix reference-example reference-example-prefix planets-example planets-example-prefix kubernetes-example kubernetes-example-prefix petstore-example petstore-example-prefix clean-example theme-example theme-example-prefix clean-theme-example

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

petstore-example:
	@if [ -e "$(PETSTORE_OUTPUT)" ] || [ -L "$(PETSTORE_OUTPUT)" ]; then \
	  if [ -L "$(PETSTORE_OUTPUT)" ] || [ ! -d "$(PETSTORE_OUTPUT)" ] || [ -L "$(PETSTORE_MARKER)" ] || [ ! -f "$(PETSTORE_MARKER)" ] || [ "$$(wc -l < "$(PETSTORE_MARKER)")" -ne 1 ] || [ "$$(cat "$(PETSTORE_MARKER)")" != 'akibiyori:petstore:root:v1' ]; then echo "refusing to remove unowned $(PETSTORE_OUTPUT)" >&2; exit 1; fi; \
	  rm -rf "$(PETSTORE_OUTPUT)"; rm -f "$(PETSTORE_MARKER)"; \
	elif [ -e "$(PETSTORE_MARKER)" ] || [ -L "$(PETSTORE_MARKER)" ]; then \
	  if [ -L "$(PETSTORE_MARKER)" ] || [ ! -f "$(PETSTORE_MARKER)" ] || [ "$$(wc -l < "$(PETSTORE_MARKER)")" -ne 1 ] || [ "$$(cat "$(PETSTORE_MARKER)")" != 'akibiyori:petstore:root:v1' ]; then echo "refusing to remove unowned $(PETSTORE_MARKER)" >&2; exit 1; fi; \
	  rm -f "$(PETSTORE_MARKER)"; \
	fi
	COBALT="$(COBALT)" PAGEFIND="$(PAGEFIND)" $(CARGO) run --locked -- build --config "$(PETSTORE_CONFIG)" --output "$(PETSTORE_OUTPUT)" --cobalt "$(COBALT)" --pagefind "$(PAGEFIND)"
	@printf '%s\n' 'akibiyori:petstore:root:v1' > "$(PETSTORE_MARKER)"

petstore-example-prefix:
	@if [ -e "$(PETSTORE_PREFIX_OUTPUT)" ] || [ -L "$(PETSTORE_PREFIX_OUTPUT)" ]; then \
	  if [ -L "$(PETSTORE_PREFIX_OUTPUT)" ] || [ ! -d "$(PETSTORE_PREFIX_OUTPUT)" ] || [ -L "$(PETSTORE_PREFIX_MARKER)" ] || [ ! -f "$(PETSTORE_PREFIX_MARKER)" ] || [ "$$(wc -l < "$(PETSTORE_PREFIX_MARKER)")" -ne 1 ] || [ "$$(cat "$(PETSTORE_PREFIX_MARKER)")" != 'akibiyori:petstore:prefix:v1' ]; then echo "refusing to remove unowned $(PETSTORE_PREFIX_OUTPUT)" >&2; exit 1; fi; \
	  rm -rf "$(PETSTORE_PREFIX_OUTPUT)"; rm -f "$(PETSTORE_PREFIX_MARKER)"; \
	elif [ -e "$(PETSTORE_PREFIX_MARKER)" ] || [ -L "$(PETSTORE_PREFIX_MARKER)" ]; then \
	  if [ -L "$(PETSTORE_PREFIX_MARKER)" ] || [ ! -f "$(PETSTORE_PREFIX_MARKER)" ] || [ "$$(wc -l < "$(PETSTORE_PREFIX_MARKER)")" -ne 1 ] || [ "$$(cat "$(PETSTORE_PREFIX_MARKER)")" != 'akibiyori:petstore:prefix:v1' ]; then echo "refusing to remove unowned $(PETSTORE_PREFIX_MARKER)" >&2; exit 1; fi; \
	  rm -f "$(PETSTORE_PREFIX_MARKER)"; \
	fi
	COBALT="$(COBALT)" PAGEFIND="$(PAGEFIND)" $(CARGO) run --locked -- build --config "$(PETSTORE_CONFIG)" --output "$(PETSTORE_PREFIX_OUTPUT)" --base-path /petstore-reference --cobalt "$(COBALT)" --pagefind "$(PAGEFIND)"
	@printf '%s\n' 'akibiyori:petstore:prefix:v1' > "$(PETSTORE_PREFIX_MARKER)"

theme-example:
	@if [ -e "$(THEME_OUTPUT)" ] || [ -L "$(THEME_OUTPUT)" ]; then \
	  if [ -L "$(THEME_OUTPUT)" ] || [ ! -d "$(THEME_OUTPUT)" ] || [ -L "$(THEME_MARKER)" ] || [ ! -f "$(THEME_MARKER)" ] || [ "$$(wc -l < "$(THEME_MARKER)")" -ne 1 ] || [ "$$(cat "$(THEME_MARKER)")" != 'akibiyori:theme-example:root:v1' ]; then echo "refusing to remove unowned $(THEME_OUTPUT)" >&2; exit 1; fi; \
	  rm -rf "$(THEME_OUTPUT)"; rm -f "$(THEME_MARKER)"; \
	elif [ -e "$(THEME_MARKER)" ] || [ -L "$(THEME_MARKER)" ]; then \
	  if [ -L "$(THEME_MARKER)" ] || [ ! -f "$(THEME_MARKER)" ] || [ "$$(wc -l < "$(THEME_MARKER)")" -ne 1 ] || [ "$$(cat "$(THEME_MARKER)")" != 'akibiyori:theme-example:root:v1' ]; then echo "refusing to remove unowned $(THEME_MARKER)" >&2; exit 1; fi; \
	  rm -f "$(THEME_MARKER)"; \
	fi
	COBALT="$(COBALT)" PAGEFIND="$(PAGEFIND)" $(CARGO) run --locked -- build --config "$(THEME_CONFIG)" --output "$(THEME_OUTPUT)" --cobalt "$(COBALT)" --pagefind "$(PAGEFIND)"
	@printf '%s\n' 'akibiyori:theme-example:root:v1' > "$(THEME_MARKER)"

theme-example-prefix:
	@if [ -e "$(THEME_PREFIX_OUTPUT)" ] || [ -L "$(THEME_PREFIX_OUTPUT)" ]; then \
	  if [ -L "$(THEME_PREFIX_OUTPUT)" ] || [ ! -d "$(THEME_PREFIX_OUTPUT)" ] || [ -L "$(THEME_PREFIX_MARKER)" ] || [ ! -f "$(THEME_PREFIX_MARKER)" ] || [ "$$(wc -l < "$(THEME_PREFIX_MARKER)")" -ne 1 ] || [ "$$(cat "$(THEME_PREFIX_MARKER)")" != 'akibiyori:theme-example:prefix:v1' ]; then echo "refusing to remove unowned $(THEME_PREFIX_OUTPUT)" >&2; exit 1; fi; \
	  rm -rf "$(THEME_PREFIX_OUTPUT)"; rm -f "$(THEME_PREFIX_MARKER)"; \
	elif [ -e "$(THEME_PREFIX_MARKER)" ] || [ -L "$(THEME_PREFIX_MARKER)" ]; then \
	  if [ -L "$(THEME_PREFIX_MARKER)" ] || [ ! -f "$(THEME_PREFIX_MARKER)" ] || [ "$$(wc -l < "$(THEME_PREFIX_MARKER)")" -ne 1 ] || [ "$$(cat "$(THEME_PREFIX_MARKER)")" != 'akibiyori:theme-example:prefix:v1' ]; then echo "refusing to remove unowned $(THEME_PREFIX_MARKER)" >&2; exit 1; fi; \
	  rm -f "$(THEME_PREFIX_MARKER)"; \
	fi
	COBALT="$(COBALT)" PAGEFIND="$(PAGEFIND)" $(CARGO) run --locked -- build --config "$(THEME_CONFIG)" --output "$(THEME_PREFIX_OUTPUT)" --base-path /theme-example --cobalt "$(COBALT)" --pagefind "$(PAGEFIND)"
	@printf '%s\n' 'akibiyori:theme-example:prefix:v1' > "$(THEME_PREFIX_MARKER)"

clean-theme-example:
	@for mode in root prefix; do \
	  case "$$mode" in \
	    root) output="$(THEME_OUTPUT)"; identity='akibiyori:theme-example:root:v1' ;; \
	    prefix) output="$(THEME_PREFIX_OUTPUT)"; identity='akibiyori:theme-example:prefix:v1' ;; \
	  esac; \
	  marker="$$output.owned"; \
	  if [ -e "$$output" ] || [ -L "$$output" ]; then \
	    if [ -L "$$output" ] || [ ! -d "$$output" ] || [ -L "$$marker" ] || [ ! -f "$$marker" ] || [ "$$(wc -l < "$$marker")" -ne 1 ] || [ "$$(cat "$$marker")" != "$$identity" ]; then echo "refusing to remove unowned $$output" >&2; exit 1; fi; \
	    rm -rf "$$output"; rm -f "$$marker"; \
	  elif [ -e "$$marker" ] || [ -L "$$marker" ]; then \
	    if [ -L "$$marker" ] || [ ! -f "$$marker" ] || [ "$$(wc -l < "$$marker")" -ne 1 ] || [ "$$(cat "$$marker")" != "$$identity" ]; then echo "refusing to remove unowned $$marker" >&2; exit 1; fi; \
	    rm -f "$$marker"; \
	  fi; \
	done

clean-example:
	@for output in "$(EXAMPLE_OUTPUT)" "$(PREFIX_OUTPUT)" "$(REFERENCE_OUTPUT)" "$(REFERENCE_PREFIX_OUTPUT)" "$(PLANETS_OUTPUT)" "$(PLANETS_PREFIX_OUTPUT)" "$(KUBERNETES_OUTPUT)" "$(KUBERNETES_PREFIX_OUTPUT)"; do \
	  marker="$${output}.owned"; \
	  if [ -e "$$output" ] || [ -L "$$output" ]; then \
	    if [ -L "$$output" ] || [ ! -d "$$output" ] || [ ! -f "$$marker" ]; then echo "refusing to remove unowned $$output" >&2; exit 1; fi; \
	    rm -rf "$$output"; \
	  fi; \
	  rm -f "$$marker"; \
	done
	@for mode in root prefix; do \
	  case "$$mode" in \
	    root) output="$(PETSTORE_OUTPUT)"; identity='akibiyori:petstore:root:v1' ;; \
	    prefix) output="$(PETSTORE_PREFIX_OUTPUT)"; identity='akibiyori:petstore:prefix:v1' ;; \
	  esac; \
	  marker="$$output.owned"; \
	  if [ -e "$$output" ] || [ -L "$$output" ]; then \
	    if [ -L "$$output" ] || [ ! -d "$$output" ] || [ -L "$$marker" ] || [ ! -f "$$marker" ] || [ "$$(wc -l < "$$marker")" -ne 1 ] || [ "$$(cat "$$marker")" != "$$identity" ]; then echo "refusing to remove unowned $$output" >&2; exit 1; fi; \
	    rm -rf "$$output"; rm -f "$$marker"; \
	  elif [ -e "$$marker" ] || [ -L "$$marker" ]; then \
	    if [ -L "$$marker" ] || [ ! -f "$$marker" ] || [ "$$(wc -l < "$$marker")" -ne 1 ] || [ "$$(cat "$$marker")" != "$$identity" ]; then echo "refusing to remove unowned $$marker" >&2; exit 1; fi; \
	    rm -f "$$marker"; \
	  fi; \
	done

# Project documentation is built independently from the assembled four-demo hub.
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
