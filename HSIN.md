# HsinCLI

A minimal, unofficial fork of [OpenAI Codex](https://github.com/openai/codex),
based on the stable release `rust-v0.154.0` (`codex-cli 0.154.0`).

The canonical repository is [hsincode/hsin](https://github.com/hsincode/hsin),
with `main` as the default branch. It replaces the former Codeberg repository.
`origin` points to this GitHub repository; `upstream` points to `openai/codex`.

The fork retains three changes: an independent `~/.hsin` home, configurable
subagent history inheritance, and HsinCLI command/session branding. Rendering,
themes, input controls, and stream processing follow the upstream release.

## Build and Install

Linux prerequisites: a C/C++ toolchain, pkg-config, OpenSSL development headers,
Rust/rustup, Python 3.11+, uv, ripgrep, and bubblewrap.

```sh
make tools
make install                 # release build at ~/.local/bin/hsin
hsin --version               # hsin 0.154.0
hsin login
hsin
```

The executable and Code Mode host are installed together under
`~/.local/lib/hsin`. `make install` builds and installs the release profile;
`make release` only builds it at `codex-rs/target/release/codex`.
`make build` creates a debug executable for development. Upstream's
`error_or_panic` panics on unexpected stream events in debug builds and logs
them in release builds. This fork does not change that stream handling.
`CLI_NAME=mycli` changes the installed command
and its help/version name; `PREFIX=/some/path` changes the install prefix.
Internal crate names, binaries, and protocols keep their upstream names.
`hsin update` reports the source build procedure, protecting the fork from
replacement by the official installer.

## Configuration

Settings, authentication, sessions, and the app-server daemon default to
`~/.hsin`, separate from upstream Codex. `CODEX_HOME` remains an explicit
compatibility override. Project configuration continues to use `.codex`.
Do not share a home with a running upstream daemon: its tools would use the
upstream history policy.

See [hsin.example.toml](hsin.example.toml) for an example:

```toml
[features]
multi_agent_v2 = true

[hsin.fork]
allow_all = false
default_turns = 1
max_turns = 3
```

V2 `fork_turns` accepts `"none"`, `"all"`, or a positive integer string.
Omitted or blank values use `default_turns`, which defaults to one recent
user turn. Full history is disabled by default. `max_turns` is optional;
omitting it removes the numeric cap. Explicit full-history or over-limit
requests fail before a child is created. Invalid policy defaults fail at
configuration loading. These limits count turns, not tokens.

V1 retains its upstream boolean interface: omitted/false starts fresh;
`fork_context=true` requires full-history permission. To permit upstream
full-history defaults, set `allow_all=true`, `default_turns="all"`, and omit
`max_turns`. Normal configuration layering and `-c` overrides apply.

Old `[hsin]` appearance keys (`display_name`, `mascot`, `workspace`) are ignored
and can be removed. Standard `[tui]` preferences still apply when explicitly
configured; omit them to use upstream defaults.

## Upstream Updates

Keep the release tag as the base and the Hsin changes small. Fetch a specific
stable release from `upstream`, merge it on a new branch, and validate with
`make schema`, `make test`, `make fix`, and `make fmt` before installation.
Review configuration schema and session-header snapshot changes. The retained
patches can be reviewed separately as history policy, independent home and
branding, and build tooling. Preserve the Apache-2.0 license and upstream notices.

## Validation

The six affected packages ran 9,207 tests. After reviewing and updating the
branding/version snapshots and the TUI startup-name check, the 5,250-test
follow-up covering all non-core packages and the agent/history tests passed,
including the final focused rerun of the status-copy snapshot test.

One core integration test remains unresolved:
`suite::hooks::async_hook_finishing_while_idle_waits_for_the_next_turn::user_turn`.
It times out after retry, matching the failure recorded by the previous fork.
The whole workspace suite was not run. The config schema was regenerated, and
`just bazel-lock-update` completed without changes to `MODULE.bazel.lock`.
Scoped `just fix` completed successfully. Its unrelated upstream unused-import
cleanup was excluded from the fork patch.
The built CLI reports `hsin 0.154.0`; installing the release build is a separate
step using `make install`.
