# HsinCLI

A minimal, unofficial fork of [OpenAI Codex](https://github.com/openai/codex),
based on the stable release `rust-v0.155.0` (`codex-cli 0.155.0`).

The canonical repository is [hsincode/hsincli](https://github.com/hsincode/hsincli),
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
make install                 # release build with ~/.local/bin/hsin and hsincli
hsin --version               # hsin 0.155.0
hsin login
hsin
```

The CLI is installed as `hsin` and `hsincli` under `~/.local/bin`, with the
release binary stored under `~/.local/lib/hsin`. `make install` builds and
installs the release profile; `make release` only builds it at
`codex-rs/target/release/codex`.
`make build` creates a debug executable for development. Upstream's
`error_or_panic` panics on unexpected stream events in debug builds and logs
them in release builds. This fork does not change that stream handling.
`PREFIX=/some/path` changes the install prefix.
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

[model_reasoning_levels]
"gpt-5.6-luna" = ["ultra"]

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

`model_reasoning_levels` adds reasoning levels to the catalog entry for a model,
keyed by slug:

```toml
[model_reasoning_levels]
"gpt-5.6-luna" = ["ultra"]
```

Under ChatGPT auth the server catalog replaces the bundled `models.json`
entirely, so this table is applied to whichever catalog is in force rather than
to the bundled file. It only adds levels: a level the catalog already advertises
keeps its catalog description, an unknown slug is logged and ignored, and the
model still has to accept the level. Ultra is worth pairing with
`features.multi_agent_v2 = true`; on a model the catalog marks `v1`, Ultra sends
the highest non-Ultra level and no delegation instructions.

The CLI name and appearance are fixed; standard `[tui]` preferences still apply
when explicitly configured.

## Slash Commands

`/effort` opens the reasoning-level picker for the model already in use, so the
level can be changed without stepping through the model list in `/model`. It
follows the same rules as that picker: Max and Ultra stay behind
`More reasoning…`, Plan mode still asks where the level applies, and during a
Luna Reserve fallback the level applies to Reserve without switching the routed
model. Levels a model does not advertise are not offered.

## Upstream Updates

Keep the release tag as the base and the Hsin changes small. Fetch a specific
stable release from `upstream`, merge it on a new branch, and validate with
`make schema`, `make test`, `make fix`, and `make fmt` before installation.
Review configuration schema and session-header snapshot changes. The retained
patches can be reviewed separately as history policy, independent home and
branding, and build tooling. Preserve the Apache-2.0 license and upstream notices.

## Validation

The six affected packages ran 9,656 tests, with 44 failures and one timeout on
the first pass. Thirty-two snapshots needed the 0.155.0 version string plus
HsinCLI branding on the voice screens upstream added, and two tests named models
that 0.155.0 renamed or dropped. The rerun after those updates passed 9,643
tests, leaving the thirteen failures described below.

Eleven failures predate the import and reproduce on the merge base, so they
belong to the in-flight fork work rather than to the release: the six
`hsin_fork` policy tests, `debug_config`, the status-line reasoning test, and
the background-task, new-session, and startup default tests.

Two failures depend on the environment rather than on the fork.
`shell_snapshot::tests::snapshot_discovers_and_redacts_shell_initialized_credentials`,
new in 0.155.0, times out after 60 seconds on unmodified upstream as well.
`suite::mcp_optional_startup_grace::...::zero_grace_respects_server_startup_timeout`
fails in the primary working copy and passes for the same commit built in a
separate worktree; optional MCP startup begins roughly 200 ms later in the
former, past the 250 ms server startup timeout the test configures.

The whole workspace suite was not run. The config schema was regenerated without
a diff, `make fmt` reported no changes, and `just bazel-lock-update` completed
without changes to `MODULE.bazel.lock`. Scoped `just fix` proposed only an
unrelated upstream unused-import cleanup, which was excluded from the fork patch.
The built CLI reports `hsin 0.155.0`; installing the release build is a separate
step using `make install`.
