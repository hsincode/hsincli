# 委譲まわりの実測

`[advisor]` と `fork_turns` の値制約は、どちらもここに残した測定から出ている。仕様の判断が
どの数字に乗っているかを後から辿れるようにしたもので、追試の手順も併せて書いてある。

- [`cost-by-configuration.html`](cost-by-configuration.html) — 親モデル・effort・委譲・advisor
  の組み合わせごとの費用。単価は litellm の `model_cost`、トークンは実セッションと DeepSWE
  ベンチの実測から。推計を含む行には `*` が付いている。

測定日は 2026-09-18、モデルカタログはその時点のもの。

## 1. 子モデルは一度も指定されなかった

`~/.hsin/sessions`（148本）と `~/.codex/sessions`（43本）の全 rollout から `spawn_agent` を
抽出したところ、**118 回すべてで `model` と `reasoning_effort` の引数が省略されていた**。
親モデル別でも Astra 21 回・Sol 6 回・Luna 91 回で例外がない。

原因は二つある。

- `SPAWN_AGENT_INHERITED_MODEL_GUIDANCE`（`core/src/tools/handlers/multi_agents_spec.rs`）が
  「継承するので `model` は省略しろ」と指示している
- `expose_agent_type` は `!config.agent_roles.is_empty()`（`core/src/tools/spec_plan.rs`）なので、
  `[agents.<role>]` を一つも定義していないと `agent_type` 引数自体がスキーマから消える

上位モデルへ上がる経路が、モデルに拒否されたのではなく**ツール定義に存在していなかった**。

## 2. 禁止した値が候補として提示されていた

同じ rollout で `fork_turns` の指定を数えると、`all=32` / `"3"=1`。ポリシーが `all` を
禁じていても候補には出ていたので、モデルは要求してはエラーを受け取っていた（hsin 側の
ポリシー下では 91 回中 14 回）。1 往復を捨てるうえ、次に活かせる情報も返らない。

`max_turns` があるときも予約ツールのスキーマは自由形式の文字列に保ち、説明文で受理範囲を伝える。`all` は説明文から外し、実行時検証で拒否する。

## 3. 子はほとんど文脈を受け取っていない

`fork_turns="all"` を 32/33 回指定していたにもかかわらず、**子の初回入力は中央値
10,441 トークン**（最小 10,160 / 最大 11,098）だった。親が作業の早い段階で spawn するため、
継承すべき履歴がまだ無い。レビュー役を子として立てても、経緯を見ないまま報告することになる。

## 4. 失敗は設計ミスではなく検証ミスだった

DeepSWE v1.1 の 10 タスク（seed 0）を、同じ Luna で effort と委譲の有無だけ変えて実行した。

| 構成 | 正解 | pass@1 | $/task | spawn |
| --- | --- | --- | --- | --- |
| Luna max（委譲なし） | 4 / 9 | 44% | 0.52 | 0 |
| Luna ultra（委譲あり） | 7 / 10 | 70% | 0.90 | 33 |

差がついた 3 タスクはすべて委譲なし側が僅差で落としたもの（f2p 136/137 など）。
spawn 33 回すべてで `agent_type` が省略され、子 32 本は全部 Luna だった。

委譲あり側が落とした 3 件は、いずれも**検証していないのに完了と報告した**型だった。

- `igel` — ツール出力に `No module named pytest` が 15 件残ったまま「テストは実行できなかった」と
  述べて完了宣言。f2p 24 本中 18 本が同一の `ValueError` で失敗
- `vulture` — 「295 passed」を根拠に完了宣言。その 295 本は既存テストで、新機能の 24 本は未実行
- `meriyah` — 410 ステップかけて f2p 47/49。パーサの端 2 件が残った

前者 2 件は、いずれもレビュー用サブエージェントを自分で立てていた（`cache_review`、
`independent_review`）。どちらも Luna で、前者は何も報告せず、後者はレビューではなく実装を
始めていた。**レビューは行われており、質が足りていなかった。**

`[advisor]` がモデルの判断ではなく規則で発火するのはこのためで、会話そのものを渡すのは、
完了したと確信している親の要約が同じ思い込みを引き継ぐため。

## 追試の手順

ベンチの実装は別リポジトリ `hsincode/hsincli-bench`（Pier + DeepSWE v1.1）。

```sh
CODEX_AUTH_JSON_PATH=~/.hsin/auth.json \
  bin/run-pilot.sh --n-tasks 10 --sample-seed 0 -n 12 --arm-parallel 3
bin/summarize.py jobs          # モデル別にトークンから課金する。pier の cost_usd は使わない
```

環境側で踏んだもの。

- Pier はタスクごとに 5〜9GB のイメージをローカルビルドして消さない。10 タスクで 100GB の
  ディスクを使い切り、残りは容量不足とは表示されず `docker compose build` の失敗として現れる
- Docker の既定アドレスプールは合計 31 ネットワークほどで、1 トライアルが 2 個使うため
  15 トライアルを超えると枯れる。colima 側に `default-address-pools` を設定する
- Codex の web 検索はサーバ側実行なので、コンテナのネットワークを遮断しても到達する。既定は
  `cached` で有効なため、オフライン評価では `web_search = "disabled"` を明示する。実際、
  試験対象リポジトリの upstream master を読んでいた
- 30 並列では API が 429 を返し、停滞と timeout 失敗が出た
