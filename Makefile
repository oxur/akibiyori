CARGO ?= cargo
.PHONY: compile fmt check test
compile:
	$(CARGO) check --locked
fmt:
	$(CARGO) fmt --all
check:
	$(CARGO) fmt --all -- --check
	$(CARGO) check --locked
	$(CARGO) clippy --locked --all-targets -- -D warnings
	$(CARGO) test --locked
	$(CARGO) doc --locked --no-deps
test:
	$(CARGO) test --locked
