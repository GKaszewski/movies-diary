.DEFAULT_GOAL := check

check: fmt-check clippy test check-appcontext check-handler-deps check-handler-repos check-presentation-adapters check-federation-off-social check-social-crates-are-ap-free
	@echo "✅ All checks passed"

check-appcontext:
	@if grep -rn "AppContext" crates/application/src --include="*.rs" | grep -q .; then \
	    echo "❌ AppContext found in application crate:"; \
	    grep -rn "AppContext" crates/application/src --include="*.rs"; \
	    exit 1; \
	fi
	@echo "✅ No AppContext in application crate"

check-handler-deps:
	@if grep -rnE "[A-Za-z0-9_]*Deps(\s*\{|::from\()" crates/presentation/src --include="*.rs" --exclude-dir=tests | grep -q .; then \
	    echo "❌ hand-built deps struct found in presentation:"; \
	    grep -rnE "[A-Za-z0-9_]*Deps(\s*\{|::from\()" crates/presentation/src --include="*.rs" --exclude-dir=tests; \
	    exit 1; \
	fi
	@echo "✅ No hand-built deps structs in presentation"

check-handler-repos:
	@if grep -rn "app_ctx\.repos" crates/presentation/src --include="*.rs" --exclude-dir=tests | grep -q .; then \
	    echo "❌ app_ctx.repos referenced in presentation:"; \
	    grep -rn "app_ctx\.repos" crates/presentation/src --include="*.rs" --exclude-dir=tests; \
	    exit 1; \
	fi
	@echo "✅ No app_ctx.repos references in presentation"


PRESENTATION_ALLOWED_ADAPTERS := template-askama

check-presentation-adapters:
	@deps=$$(awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f' crates/presentation/Cargo.toml \
	    | sed -n 's/^\([A-Za-z0-9_-]\{1,\}\)[[:space:]]*=.*/\1/p'); \
	bad=""; \
	for a in $$(ls crates/adapters); do \
	    case " $(PRESENTATION_ALLOWED_ADAPTERS) " in *" $$a "*) continue;; esac; \
	    if echo "$$deps" | grep -qx "$$a"; then bad="$$bad $$a"; fi; \
	done; \
	if [ -n "$$bad" ]; then \
	    echo "❌ presentation depends on adapter crate(s):$$bad"; \
	    echo "   presentation may name renderers, not reachers — see ADR-0008."; \
	    exit 1; \
	fi; \
	echo "✅ presentation depends on no adapter crate but $(PRESENTATION_ALLOWED_ADAPTERS)"


check-federation-off-social:
	@if grep -n "NoopSocialCommand\|NoopSocialQuery" crates/server/src/main.rs | grep -q .; then \
	    echo "❌ server wires social noops — federation-off would report an empty follow graph:"; \
	    grep -n "NoopSocialCommand\|NoopSocialQuery" crates/server/src/main.rs; \
	    exit 1; \
	fi
	@if ! awk '/cfg\(not\(feature = "federation"\)\)/ && !started {started=1} started && /cfg\(feature = "federation"\)/ {exit} started {print}' crates/server/src/main.rs | grep -q "LocalSocialService"; then \
	    echo "❌ LocalSocialService not wired in the federation-off region of crates/server/src/main.rs — federation-off has no social behavior wired at all, not even noops"; \
	    exit 1; \
	fi
	@echo "✅ server wires real social behavior in every configuration"


check-social-crates-are-ap-free:
	@if grep -nE '^[[:space:]]*(k-ap|activitypub[A-Za-z0-9_-]*|[A-Za-z0-9_-]+-federation)[[:space:]]*=' crates/adapters/*-social/Cargo.toml | grep -q .; then \
	    echo "❌ social crate depends on ActivityPub:"; \
	    grep -nE '^[[:space:]]*(k-ap|activitypub[A-Za-z0-9_-]*|[A-Za-z0-9_-]+-federation)[[:space:]]*=' crates/adapters/*-social/Cargo.toml; \
	    exit 1; \
	fi
	@echo "✅ social crates depend on no k-ap, activitypub, or *-federation crate"

fmt:
	cargo fmt

fmt-check:
	cargo fmt --check

clippy:
	cargo clippy -- -D warnings

test:
	cargo test

fix:
	cargo fmt
	cargo clippy --fix --allow-dirty --allow-staged

# Run the dev stack:
#   server  (:3000, in the foreground, Ctrl-C stops it)
#   worker  (background, log at /tmp/movies-diary-worker.log)
#   spa     (:5173, background, log at /tmp/movies-diary-spa.log)
# Tear down the background processes with `make dev-stop`.
# .env enables EVENT_BUS_BACKEND=nats, so both binaries are built with the nats feature.
dev: dev-worker dev-spa
	cargo run -p server --features nats

dev-stop:
	-pkill -f '[t]arget.debug.worker' || true
	-pkill -f '[s]pa/node_modules/[.]bin/vite' || true
	@echo "dev stack stopped"

dev-worker:
	@bash -c 'nohup cargo run -p worker --features nats >> /tmp/movies-diary-worker.log 2>&1 & echo "worker → pid $$! (log: /tmp/movies-diary-worker.log)"'

dev-spa:
	@bash -c 'nohup npm --prefix spa run dev >> /tmp/movies-diary-spa.log 2>&1 & echo "spa    → pid $$! (log: /tmp/movies-diary-spa.log, http://localhost:5173)"'

spa-dev:
	@npm --prefix spa run dev

.PHONY: check fmt fmt-check clippy test fix dev dev-stop dev-worker dev-spa spa-dev check-appcontext check-handler-deps check-handler-repos check-presentation-adapters check-federation-off-social check-social-crates-are-ap-free
