.DEFAULT_GOAL := check

# Run the full local check suite — same order as CI would.
check: fmt-check clippy test check-appcontext check-handler-deps check-handler-repos check-presentation-adapters check-federation-off-social check-social-crates-are-ap-free
	@echo "✅ All checks passed"

# Enforce that no application use case imports AppContext (god-object guard).
check-appcontext:
	@if grep -rn "AppContext" crates/application/src --include="*.rs" | grep -q .; then \
	    echo "❌ AppContext found in application crate:"; \
	    grep -rn "AppContext" crates/application/src --include="*.rs"; \
	    exit 1; \
	fi
	@echo "✅ No AppContext in application crate"

# Enforce that presentation never hand-builds a deps struct (composition-root guard).
# Catches struct literals (`FooDeps { ... }`) and conversions (`FooDeps::from(...)`) —
# both are the composition-root pattern that belongs in crates/composition.
#
# Scope is the whole crate, not just handlers/, so that factoring a
# `fn build_login_deps(state) -> LoginDeps { .. }` helper into a sibling module
# does not slip past the guard. One exclusion:
#   src/tests/  — test fixtures legitimately assemble their own state
# `src/main.rs` used to be excluded too ("the binary IS the wiring root") before
# crates/server took over the binary (ADR-0006) — presentation is lib-only now, has no
# main.rs, and the exclusion was dropped rather than left pointing at a file that no
# longer exists.
# NOTE: `\s` is a GNU grep extension, not POSIX ERE. Fine on GNU/Ubuntu runners;
# would need `[[:space:]]` if this ever runs under BusyBox/Alpine grep.
check-handler-deps:
	@if grep -rnE "[A-Za-z0-9_]*Deps(\s*\{|::from\()" crates/presentation/src --include="*.rs" --exclude-dir=tests | grep -q .; then \
	    echo "❌ hand-built deps struct found in presentation:"; \
	    grep -rnE "[A-Za-z0-9_]*Deps(\s*\{|::from\()" crates/presentation/src --include="*.rs" --exclude-dir=tests; \
	    exit 1; \
	fi
	@echo "✅ No hand-built deps structs in presentation"

# Enforce that presentation calls use cases only, never repository methods directly
# (ADR-0006's guard; ADR-0007 records the widening below).
#
# Was: two shape-matching grep passes for `repos.<field>.<method>(` and
# `repos.<field>.clone().<method>(`, needed because `AppContext.repos` was still a
# legitimate way to reach a repository — ~40 sites cloned an Arc out of it to pass
# positionally into a use case, so the guard had to distinguish that legal shape
# from an illegal direct call. That shape-matching was demonstrably incomplete: it
# matches `repos.<field>.<method>(` textually, so it cannot see a bare reference
# pass (`&repos.diary`, no call at all at the read site) or a `let`-bound access
# (`if let Some(x) = repos.federated_profile` then `x.method()` on the *binding*,
# not on `repos` itself). Both shapes existed in production code — Plan C2's Task 6
# review caught it, Task 7 fixed both — while the two-pass guard printed a clean
# ✅ the whole time. ADR-0006's "handlers call use cases only" was false at that
# line for as long as the hole existed.
#
# Now: Plan C2 deleted `AppContext.repos` entirely (ADR-0007), so there is no
# longer any legitimate reason for the substring `app_ctx.repos` to appear in
# production code — not a call, not a clone, not a reference, not a let-binding.
# That makes the rule trivial and unevadeable: any mention at all is an offender.
# No shape to match means no shape to miss.
#
# One exclusion: src/tests/ — test helpers still legitimately build a
# `Repositories` and call `composition::build_deps` to produce test state.
check-handler-repos:
	@if grep -rn "app_ctx\.repos" crates/presentation/src --include="*.rs" --exclude-dir=tests | grep -q .; then \
	    echo "❌ app_ctx.repos referenced in presentation:"; \
	    grep -rn "app_ctx\.repos" crates/presentation/src --include="*.rs" --exclude-dir=tests; \
	    exit 1; \
	fi
	@echo "✅ No app_ctx.repos references in presentation"

# Enforce that presentation depends on at most one adapter crate.
#
# Presentation may name things that RENDER OUTPUT; it may not name things that
# REACH external systems or storage. `template-askama` is the HTML template
# engine, so it stays. See ADR-0008.
#
# This checks Cargo.toml rather than grepping src/, deliberately: `<crate>::`
# greps collide with same-named modules inside `application` and `presentation`
# (`auth::logout` is `application::auth`; `rss::get_user_feed` is
# `handlers::rss`), so the source-level grep cannot decide this. Every directory
# under crates/adapters/ is named exactly like its package, so directory names
# are a safe source of truth.
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

# Enforce that the federation-off build wires real social behavior, not silent noops.
#
# With federation off, `NoopSocialCommand::follow` returns Ok(()) and writes nothing,
# and `NoopSocialQuery` always answers zero/empty. The write side is currently
# unreachable over HTTP (the social routes are federation-gated in presentation), but
# the query side is not: `get_local_profile`, `get_page_viewer` and `get_activity_feed`
# read it through fully ungated routes, so a noop there means a federation-off
# instance lies about its own follow graph on every profile view and activity feed
# request — contradicting ADR-0003, which established that local follows bypass
# ActivityPub entirely. `application::social::LocalSocialService` is what must be
# wired instead. See ADR-0009.
#
# Two checks, not one: absence of the noops is necessary but not sufficient — deleting
# the whole `#[cfg(not(feature = "federation"))]` wiring block would also make the
# first check pass, leaving federation-off with nothing wired at all, which is worse
# than the noops. So this also asserts `LocalSocialService` is present — and it must
# be present in the federation-OFF region specifically, not merely anywhere in the
# file: `LocalSocialService` is also constructed inside the federation-ON branch
# (feeding `CompositeSocialAdapter`), so a bare file-wide grep is satisfied by that
# occurrence alone and would not catch the federation-off block being deleted. The
# awk carves out the region from the first `#[cfg(not(feature = "federation"))]` to
# the next `#[cfg(feature = "federation")]` — the same kind of range
# `check-presentation-adapters` uses for `[dependencies]` — and greps only inside it.
# Not an occurrence count: that would work today but break the moment the two
# `LocalSocialService` constructions in `main.rs` are deduplicated (a recorded
# follow-up), which could legitimately drop the total to one.
#
# A grep rather than a test because the decision lives in `wire_dependencies()`, a
# private fn in a binary crate, unreachable from integration tests.
#
# Scope is the single file `crates/server/src/main.rs`, not the whole crate: it is the
# sole wiring site for these ports. `worker` and `tui` never name SocialCommand /
# FollowGraphQuery / BlockQuery at all, so there is nowhere else for the noops to
# reappear. Relocating the wiring out of main.rs would move it out of the guard's
# scope — widen this to `crates/server/src` if that ever happens.
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

# Enforce that `<backend>-social` crates never depend on ActivityPub.
#
# ADR-0009's whole federation-optional property rests on one invariant: a
# `<backend>-social` crate speaks only `domain::ports`; a `<backend>-federation` crate
# speaks ActivityPub; the former must never depend on the latter. It holds today, and
# nothing but this guard enforces it. One manifest line —
# `k-ap = { version = "0.5.0", registry = "gitea" }` in a `-social` crate's Cargo.toml,
# the obvious move when someone wants to reuse a helper like `status_to_str` from
# `sqlite-federation/src/lib.rs` — would silently restore all 76 crates and 19MB that
# federation-off exists to shed, while `make check`, `cargo test` and CI all stay green
# without this guard to catch it. Matches `activitypub` with any suffix, so
# `activitypub_federation = "0.6"` (the upstream crate carrying all 76 transitives) is
# caught too, not just the local `activitypub` crate.
#
# Known gap: a dependency renamed via a `package = "k-ap"` key (`foo = { package =
# "k-ap", ... }`), or declared with a `[dependencies.k-ap]` table header instead of the
# inline form, would not match this line-level regex. Not attempted — nothing in the
# workspace does either today.
check-social-crates-are-ap-free:
	@if grep -nE '^[[:space:]]*(k-ap|activitypub[A-Za-z0-9_-]*|[A-Za-z0-9_-]+-federation)[[:space:]]*=' crates/adapters/*-social/Cargo.toml | grep -q .; then \
	    echo "❌ social crate depends on ActivityPub:"; \
	    grep -nE '^[[:space:]]*(k-ap|activitypub[A-Za-z0-9_-]*|[A-Za-z0-9_-]+-federation)[[:space:]]*=' crates/adapters/*-social/Cargo.toml; \
	    exit 1; \
	fi
	@echo "✅ social crates depend on no k-ap, activitypub, or *-federation crate"

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

.PHONY: check fmt fmt-check clippy test fix check-appcontext check-handler-deps check-handler-repos check-presentation-adapters check-federation-off-social check-social-crates-are-ap-free
