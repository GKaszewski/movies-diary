.DEFAULT_GOAL := check

# Run the full local check suite — same order as CI would.
check: fmt-check clippy test
	@echo "✅ All checks passed"

# Apply rustfmt to all files.
fmt:
	cargo fmt

# Check formatting without modifying files (CI-safe).
fmt-check:
	cargo fmt --check

# Run Clippy and treat warnings as errors.
clippy:
	cargo clippy -- -D warnings

# Run the test suite.
test:
	cargo test

# Apply fmt + clippy auto-fixes in one shot.
fix:
	cargo fmt
	cargo clippy --fix --allow-dirty --allow-staged

.PHONY: check fmt fmt-check clippy test fix
