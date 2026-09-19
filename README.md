 # HsinCLI

OpenAI Codex CLI 0.155.1 をベースにした非公式フォークです。設定・認証・セッションを Codex から分離し、サブエージェントの履歴継承を制御できます。

## 主な機能

- コマンド名と表示名は `hsin`
- デフォルトのホームは `~/.hsin`（設定、認証、セッション、app-server 状態を分離）
- `fork_turns` で履歴を `none`、`all`、直近ターン数から選択
- `/effort` で現在のモデルの推論レベルを直接選択
- `hsin update` はソースからのビルド手順を表示

## インストール

```sh
make tools
make install
hsin login
hsin
```

`PREFIX=/path make install` でインストール先を変更できます。`hsin` と `hsincli` は同じCLIの名前です。開発用は `make build`、リリースビルドのみは `make release` です。

## 設定

ユーザー設定は `~/.hsin/config.toml` に記述します。プロジェクト固有の設定は `.codex` を使用します。

```toml
service_tier = "default"

[model_service_tiers]
gpt-5.4 = "priority"

[agents]
default_subagent_service_tier = "priority"

[features]
multi_agent_v2 = true

[model_reasoning_levels]
"gpt-5.6-luna" = ["ultra"]

[hsin.fork]
allow_all = false
default_turns = 1
max_turns = 3
```

- `default_turns`: 省略時の継承ターン数（`none`、`all`、正の整数）
- `max_turns`: 数値指定の上限。省略すると上限なし
- `allow_all`: `fork_turns = "all"` の許可。`max_turns` を設定している場合は、それだけで `all` は拒否されます
- `service_tier`: 新しいターンで使う既定のサービスティア。`default`、`priority`、`flex` を指定できます（旧名 `fast` も使用可能）
- `model_service_tiers`: モデル名ごとのサービスティア。モデル別設定が全体の `service_tier` より優先されます
- `agents.default_subagent_service_tier`: 既定モデルで起動するサブエージェントのサービスティア
- `model_reasoning_levels`: モデルごとに追加で提示する推論レベル。カタログにあるレベルは変更しません
- `features.multi_agent_v2.tool_namespace`: V2 の予約ツールを公開する名前空間。既定値は `agents` です。`collaboration` は一部モデルで予約されているため、スキーマが一致しない設定では使用しないでください

サービスティアはモデルが対応している場合だけリクエストに適用されます。未設定の場合はプロバイダーとモデルの既定値が使われます。

`model_reasoning_levels` は `/model` と `/effort` の選択肢を広げるだけで、モデル側の対応を変えるものではありません。`ultra` を追加する場合は `features.multi_agent_v2 = true` も設定してください。カタログ上 v1 のモデルでは Ultra は委譲を行わず、Max 相当の推論として送信されます。

設定例は [`hsin.example.toml`](hsin.example.toml)、詳細は [`HSIN.md`](HSIN.md) を参照してください。

## 開発

```sh
cd codex-rs
just fmt
just test -p <変更したプロジェクト>
```

## ライセンス

Apache License 2.0（[`LICENSE`](LICENSE)）。

<p align="center"><strong>HsinCLI</strong> is a local coding agent based on OpenAI Codex.
<p align="center">
  <img src="https://github.com/openai/codex/blob/main/.github/codex-cli-splash.png" alt="Codex CLI splash" width="80%" />
</p>
</br>
If you want Codex in your code editor (VS Code, Cursor, Windsurf), <a href="https://developers.openai.com/codex/ide">install in your IDE.</a>
</br>If you want the desktop app experience, run <code>codex app</code> or visit <a href="https://chatgpt.com/codex?app-landing-page=true">the Codex App page</a>.
</br>If you are looking for the <em>cloud-based agent</em> from OpenAI, <strong>Codex Web</strong>, go to <a href="https://chatgpt.com/codex">chatgpt.com/codex</a>.</p>

---

## Quickstart

### Installing and running HsinCLI

Run the following on Mac or Linux to install HsinCLI:

```shell
curl -fsSL https://chatgpt.com/codex/install.sh | sh
```

Run the following on Windows to install HsinCLI:

```shell
powershell -ExecutionPolicy ByPass -c "irm https://chatgpt.com/codex/install.ps1 | iex"
```

The standalone installers download from `https://releases.openai.com/codex` by default and fall back to GitHub Releases if a metadata or asset download is unavailable. To force GitHub Releases, set `CODEX_INSTALLER_USE_RELEASES_OPENAI_COM` to `false` (`0` and `no` are also accepted):

```shell
curl -fsSL https://chatgpt.com/codex/install.sh | CODEX_INSTALLER_USE_RELEASES_OPENAI_COM=false sh
```

```powershell
$env:CODEX_INSTALLER_USE_RELEASES_OPENAI_COM='false'; irm https://chatgpt.com/codex/install.ps1 | iex
```

Codex CLI can also be installed via the following package managers:

```shell
# Install using npm
npm install -g @openai/codex
```

```shell
# Install using Homebrew
brew install --cask codex
```

Then simply run `codex` to get started.

<details>
<summary>You can also go to the <a href="https://github.com/openai/codex/releases/latest">latest GitHub Release</a> and download the appropriate binary for your platform.</summary>

Each GitHub Release contains many executables, but in practice, you likely want one of these:

- macOS
  - Apple Silicon/arm64: `codex-aarch64-apple-darwin.tar.gz`
  - x86_64 (older Mac hardware): `codex-x86_64-apple-darwin.tar.gz`
- Linux
  - x86_64: `codex-x86_64-unknown-linux-musl.tar.gz`
  - arm64: `codex-aarch64-unknown-linux-musl.tar.gz`

Each archive contains a single entry with the platform baked into the name (e.g., `codex-x86_64-unknown-linux-musl`), so you likely want to rename it to `codex` after extracting it.

</details>

### Using Codex with your ChatGPT plan

Run `codex` and select **Sign in with ChatGPT**. We recommend signing into your ChatGPT account to use Codex as part of your Plus, Pro, Business, Edu, or Enterprise plan. [Learn more about what's included in your ChatGPT plan](https://help.openai.com/en/articles/11369540-codex-in-chatgpt).

You can also use Codex with an API key, but this requires [additional setup](https://developers.openai.com/codex/auth#sign-in-with-an-api-key).

## Docs

- [**Codex Documentation**](https://developers.openai.com/codex)
- [**Contributing**](./docs/contributing.md)
- [**Installing & building**](./docs/install.md)
- [**Open source fund**](./docs/open-source-fund.md)

This repository is licensed under the [Apache-2.0 License](LICENSE).
