.DEFAULT_GOAL := check

# Run the full local check suite — same order as CI would.
check: fmt-check clippy test check-appcontext
	@echo "✅ All checks passed"

# Enforce that no application use case imports AppContext (god-object guard).
check-appcontext:
	@if grep -rn "AppContext" crates/application/src --include="*.rs" | grep -q .; then \
	    echo "❌ AppContext found in application crate:"; \
	    grep -rn "AppContext" crates/application/src --include="*.rs"; \
	    exit 1; \
	fi
	@echo "✅ No AppContext in application crate"

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
