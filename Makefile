.DEFAULT_GOAL := check

check: fmt-check clippy test test-federation-off check-appcontext check-handler-deps check-handler-repos check-handler-services check-http-adapter-deps check-federation-off-social check-sqlite-is-ap-free check-binaries-are-thin check-federation-off-deps check-nats-isolated check-no-second-web-stack
	@echo "✅ All checks passed"

check-appcontext:
	@if grep -rn "AppContext" crates/application/src --include="*.rs" | grep -q .; then \
	    echo "❌ AppContext found in application crate:"; \
	    grep -rn "AppContext" crates/application/src --include="*.rs"; \
	    exit 1; \
	fi
	@echo "✅ No AppContext in application crate"

check-handler-deps:
	@if grep -rnE "[A-Za-z0-9_]*Deps(\s*\{|::from\()" crates/adapters/http-axum/src --include="*.rs" --exclude-dir=tests --exclude=tests.rs | grep -q .; then \
	    echo "❌ hand-built deps struct found in http-axum:"; \
	    grep -rnE "[A-Za-z0-9_]*Deps(\s*\{|::from\()" crates/adapters/http-axum/src --include="*.rs" --exclude-dir=tests --exclude=tests.rs; \
	    exit 1; \
	fi
	@echo "✅ No hand-built deps structs in http-axum"

check-handler-repos:
	@if grep -rn "app_ctx\.repos" crates/adapters/http-axum/src --include="*.rs" --exclude-dir=tests --exclude=tests.rs | grep -q .; then \
	    echo "❌ app_ctx.repos referenced in http-axum:"; \
	    grep -rn "app_ctx\.repos" crates/adapters/http-axum/src --include="*.rs" --exclude-dir=tests --exclude=tests.rs; \
	    exit 1; \
	fi
	@echo "✅ No app_ctx.repos references in http-axum"


HTTP_ADAPTER_ALLOWED_ADAPTERS :=

check-handler-services:
	@hits=$$(find crates/adapters/http-axum/src -name '*.rs' -not -path '*/tests/*' -not -name tests.rs \
	    | xargs perl -0ne 'while (/app_ctx\s*\.\s*services\s*\.\s*([a-z_]+)/g) { print "$$ARGV: services.$$1\n" unless $$1 eq "review_logger" }'); \
	if [ -n "$$hits" ]; then \
	    echo "❌ http-axum reaches a service port directly (call a use case instead):"; \
	    echo "$$hits"; \
	    exit 1; \
	fi; \
	echo "✅ No direct service-port calls in http-axum"

check-http-adapter-deps:
	@deps=$$(awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f' crates/adapters/http-axum/Cargo.toml \
	    | sed -n 's/^\([A-Za-z0-9_-]\{1,\}\)[[:space:]]*=.*/\1/p'); \
	bad=""; \
	for a in $$(ls crates/adapters); do \
	    case " $(HTTP_ADAPTER_ALLOWED_ADAPTERS) " in *" $$a "*) continue;; esac; \
	    if echo "$$deps" | grep -qx "$$a"; then bad="$$bad $$a"; fi; \
	done; \
	if [ -n "$$bad" ]; then \
	    echo "❌ http-axum depends on adapter crate(s):$$bad"; \
	    echo "   http-axum may name renderers, not reachers — see ADR-0008."; \
	    exit 1; \
	fi; \
	echo "✅ http-axum depends on no other adapter crate"


check-federation-off-social:
	@if grep -n "NoopSocialCommand\|NoopSocialQuery" crates/bootstrap/src/federation.rs | grep -q .; then \
	    echo "❌ bootstrap wires social noops — federation-off would report an empty follow graph:"; \
	    grep -n "NoopSocialCommand\|NoopSocialQuery" crates/bootstrap/src/federation.rs; \
	    exit 1; \
	fi
	@if ! awk '/cfg\(not\(feature = "federation"\)\)/ && !started {started=1} started && /cfg\(feature = "federation"\)/ {exit} started {print}' crates/bootstrap/src/federation.rs | grep -q "LocalSocialService"; then \
	    echo "❌ LocalSocialService not wired in the federation-off region of crates/bootstrap/src/federation.rs — federation-off has no social behavior wired at all, not even noops"; \
	    exit 1; \
	fi
	@echo "✅ bootstrap wires real social behavior in every configuration"


BINARIES_ALLOWED_DEPS := bootstrap config tokio axum anyhow dotenvy tracing

check-binaries-are-thin:
	@bad=""; \
	for c in server worker; do \
	    deps=$$(awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f' crates/$$c/Cargo.toml \
	        | sed -n 's/^\([A-Za-z0-9_-]\{1,\}\)[[:space:]]*=.*/\1/p'); \
	    for d in $$deps; do \
	        case " $(BINARIES_ALLOWED_DEPS) " in *" $$d "*) continue;; esac; \
	        bad="$$bad $$c:$$d"; \
	    done; \
	done; \
	if [ -n "$$bad" ]; then \
	    echo "❌ binary [dependencies] outside {$(BINARIES_ALLOWED_DEPS)}:$$bad"; \
	    echo "   wiring belongs in crates/bootstrap — see ADR-0013."; \
	    exit 1; \
	fi; \
	echo "✅ server and worker depend only on {$(BINARIES_ALLOWED_DEPS)}"


check-sqlite-is-ap-free:
	@if grep -nE '^[[:space:]]*(k-ap|activitypub[A-Za-z0-9_-]*|[A-Za-z0-9_-]+-federation)[[:space:]]*=' crates/adapters/sqlite/Cargo.toml | grep -q .; then \
	    echo "❌ sqlite crate depends on ActivityPub:"; \
	    grep -nE '^[[:space:]]*(k-ap|activitypub[A-Za-z0-9_-]*|[A-Za-z0-9_-]+-federation)[[:space:]]*=' crates/adapters/sqlite/Cargo.toml; \
	    exit 1; \
	fi
	@echo "✅ sqlite crate depends on no k-ap, activitypub, or *-federation crate"

CARGO_TREE_NAMES := cargo tree -p server -p worker -e normal --prefix none

check-federation-off-deps:
	@tree=$$($(CARGO_TREE_NAMES) --no-default-features) || exit 1; \
	hits=$$(echo "$$tree" | grep -E '^(k-ap|activitypub_federation|activitypub|sqlite-federation) ' | sort -u); \
	if [ -n "$$hits" ]; then \
	    echo "❌ federation-only crates in the --no-default-features server/worker tree:"; \
	    echo "$$hits"; \
	    exit 1; \
	fi; \
	echo "✅ federation-off tree has no k-ap, activitypub_federation, activitypub, or sqlite-federation"

check-nats-isolated:
	@tree=$$($(CARGO_TREE_NAMES)) || exit 1; \
	nats_tree=$$($(CARGO_TREE_NAMES) --features nats) || exit 1; \
	if echo "$$tree" | grep -qE '^(async-nats|nats) '; then \
	    echo "❌ nats crates in the default server/worker tree:"; \
	    echo "$$tree" | grep -E '^(async-nats|nats) ' | sort -u; \
	    exit 1; \
	fi; \
	if ! echo "$$nats_tree" | grep -qE '^async-nats '; then \
	    echo "❌ async-nats missing from the --features nats server/worker tree — feature is not wired"; \
	    exit 1; \
	fi; \
	echo "✅ async-nats only appears with --features nats"

check-no-second-web-stack:
	@for f in "" "--features nats"; do \
	    tree=$$($(CARGO_TREE_NAMES) $$f) || exit 1; \
	    hits=$$(echo "$$tree" | grep -E '^(actix|aws-lc)' | sort -u); \
	    if [ -n "$$hits" ]; then \
	        echo "❌ actix/aws-lc crates in the server/worker tree ($${f:-default}):"; \
	        echo "$$hits"; \
	        exit 1; \
	    fi; \
	done; \
	echo "✅ no actix-* or aws-lc-* in the default or nats server/worker tree"

fmt:
	cargo fmt

fmt-check:
	cargo fmt --check

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

test-federation-off:
	cargo test --workspace --no-default-features

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

.PHONY: check fmt fmt-check clippy test test-federation-off fix dev dev-stop dev-worker dev-spa spa-dev check-appcontext check-handler-deps check-handler-repos check-handler-services check-http-adapter-deps check-federation-off-social check-sqlite-is-ap-free check-binaries-are-thin check-federation-off-deps check-nats-isolated check-no-second-web-stack
