# Hsin entry points. Keep upstream's justfile and crate names intact.
CLI_NAME ?= hsin
PREFIX ?= $(HOME)/.local
export CARGO_BUILD_JOBS ?= 2
export CARGO_PROFILE_DEV_DEBUG ?= 0
export CARGO_PROFILE_TEST_DEBUG ?= 0
export HSIN_CLI_NAME = $(CLI_NAME)
export NEXTEST_TEST_THREADS ?= 2

.PHONY: check test build release install fmt schema fix lint tools test-tools
check:
	just fmt-check
	cd codex-rs && cargo check -p codex-cli

test: test-tools
	# Keep terminal-dependent upstream snapshots reproducible under tmux and NO_COLOR.
	env -u NO_COLOR -u TMUX -u TMUX_PANE -u TERM_PROGRAM -u GHOSTTY_RESOURCES_DIR TERM=xterm-256color python3 scripts/hsin-build.py just test -p codex-config -p codex-core -p codex-tui -p codex-utils-home-dir -p codex-utils-cli -p codex-cli $(TEST_ARGS)

build:
	python3 scripts/hsin-build.py cargo build -p codex-cli -p codex-code-mode-host --bin codex --bin codex-code-mode-host

test-tools:
	python3 scripts/hsin-build.py cargo build -p codex-code-mode-host -p codex-exec -p codex-rmcp-client --bin codex-code-mode-host --bin codex-exec --bin test_stdio_server

release:
	python3 scripts/hsin-build.py cargo build --release -p codex-cli -p codex-code-mode-host --bin codex --bin codex-code-mode-host

install: release
	install -Dm755 codex-rs/target/release/codex "$(PREFIX)/lib/hsin/codex"
	install -Dm755 codex-rs/target/release/codex-code-mode-host "$(PREFIX)/lib/hsin/codex-code-mode-host"
	install -d "$(PREFIX)/bin"
	ln -sfn ../lib/hsin/codex "$(PREFIX)/bin/$(CLI_NAME)"

fmt:
	just fmt

schema:
	just write-config-schema

fix:
	just fix -p codex-config -p codex-core -p codex-tui -p codex-utils-home-dir -p codex-utils-cli -p codex-cli

lint:
	just clippy -p codex-config -p codex-core -p codex-tui -p codex-utils-home-dir -p codex-utils-cli -p codex-cli

tools:
	cargo install --locked just cargo-nextest cargo-insta dotslash
