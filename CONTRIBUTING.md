# 開発者ガイド

利用者向けの導入・生成手順は [README](README.md)、開発ルールは [AGENTS.md](AGENTS.md) を参照してください。
CI の起動条件・公開チェック・復旧手順の正本は [docs/ci.md](docs/ci.md) です。
必要な仕様は対象語で検索するか、[文書案内](docs/README.md) から選べます。

## 開発環境

Docker Desktop を Linux コンテナモードで起動し、リポジトリのルートで実行します。

```sh
docker compose up -d --build dev
docker compose exec -T dev cargo build --locked
docker compose exec -T dev mkdir -p output
docker compose exec -T dev cargo run --locked -- examples/plan.json -o output/diagram.svg
docker compose exec -T dev cargo run --locked -- --help
```

ツール・依存の固定版は Dockerfile、rust-toolchain.toml、Cargo.lock を正本とします。
Dockerfile 変更後は上記の build を再実行し、依存更新時は lockfile もコミットします。
実環境の plan/state や認証情報をコミットしないでください。
キャッシュの診断・リセットは [build performance](docs/build-performance.md)、
Wasm のビルドは [browser development](docs/browser-development.md) を参照してください。

## Build provenance

`build.rs` records the engine checkout's full Git HEAD SHA for native and Wasm builds.
Tracked modifications (including staged changes) or non-ignored untracked files add
`-dirty`; such a build does not claim to exactly reproduce the clean commit.
Ignored build outputs do not affect this marker. The marker does not fingerprint
dirty contents. Git must identify this package directory as its repository root;
source archives inside an unrelated checkout cannot inherit its revision.
Git acquisition and distribution checks trust only the current package/checkout
directory through a command-scoped `safe.directory` entry. This supports Docker
bind mounts owned by another UID without changing global Git configuration or
trusting unrelated repositories.
Missing Git, missing metadata, unborn HEAD or failed status checks yield unavailable
revision without failing the build. No environment or hosting-site revision override
is accepted. No paths, remotes, timestamps or input contents enter the metadata.

Cargo rechecks provenance on every build using an intentionally absent build-script
dependency (`build-provenance-always-recheck`); this also catches commits, staging
and new untracked files. This may recompile the package, but dependencies stay cached.
CLI `--version` and the existing HTML generation metadata use the same compiled
value. SVG and graph/layout calculation do not use it. Identical provenance and
generation settings produce identical metadata.

## 検証

```sh
docker compose exec -T dev cargo fmt --all --check
docker compose exec -T dev cargo test --locked --all-targets --all-features
docker compose exec -T dev cargo clippy --locked --all-targets --all-features -- -D warnings
docker compose exec -T dev cargo build --locked
docker compose exec -T dev sh -c 'node --test tests/ci/*.test.cjs'
docker compose exec -T dev node tests/browser/viewer.cjs
docker compose exec -T dev node --test tests/browser/presentation-quality.test.cjs
docker compose exec -T dev node --test tests/browser/webdriver.test.cjs tests/support/tooling.test.cjs
```

`tests/unit` は実装モジュール内に読み込む単体テスト、`tests/integration` は CLI 経由の結合テストです。Cargo.toml に4つの結合テスト対象を登録しています。
`--lib` で単体テスト、`--test rendering` などで結合テストを選択できます。
テストプロファイルは `opt-level = 1` で経路計算を高速化し、debug assertion と overflow check を有効にしています。

MSRV の確認:

```sh
docker build --build-arg RUST_VERSION=1.85.0 -t planorama-msrv .
docker run --rm --mount type=bind,source=.,target=/workspace --env CARGO_TARGET_DIR=/tmp/planorama-target planorama-msrv cargo +1.85.0 test --locked --all-targets --all-features
```

CI は [standard / large の2 suite](docs/ci.md#parallel-rust-test-suites) の成功を `Tests` に集約します。
通常実行で ignore する大型回帰は [Routing regressions](docs/ci.md#deferred-routing-regressions-253) を参照してください。

入力・表示設定変更による配置の変動を測定する任意の開発コマンドは
[Layout stability](docs/layout-stability.md) を参照してください。

## リソース対応一覧の更新

型ごとの実行時の分類を調べる場合、Docker 内で必要な prefix を指定します。

```sh
docker compose exec -T dev sh scripts/update-resource-support.sh aws_lambda_
```

型ごとの確認は [必要な範囲の生成](docs/resource-support.md)、固定 Provider の更新は
[AWS inventory](docs/aws-inventory.md)、監査データは [対象型の問い合わせ](docs/aws-semantic-audit.md) を使います。
生成した表は output 配下の一時ファイルで、コミットや一致チェックは不要です。
基本分類の検証は `cargo test --locked --lib provider::aws::basic` で実行します。

## SVG の更新

レンダリングの挙動・生成入力・fixture・生成スクリプトを変更した場合は、Docker 内で再生成し、差分と表示を確認して実装と同じ PR に含めます。生成入力・fixture・生成スクリプトの変更は、図に差分が出ない想定でも再生成して確認します。

```sh
docker compose exec -T dev sh scripts/update-examples.sh
```

PowerShell では `./scripts/update-examples.ps1` が開発コンテナを起動して同じ処理を実行します。
[scripts/generated-svgs.txt](scripts/generated-svgs.txt) が生成対象の正本です。公式アイコン素材は同梱しません。
`hierarchy-routing.svg` と `port-collisions.svg` は既存の snapshot テストを完全一致で選択して更新し、更新処理で全テストは実行しません。

通常 PR では全 SVG を自動生成しません。復旧・補助用の **Update SVG examples** と、その後の検証・status 報告は [CI の生成・復旧手順](docs/ci.md#generation-and-review) を参照してください。
配置・配線・寸法・描画対象が変わる場合は `layout-quality` ラベルを付け、最終コミットの [Layout quality](docs/ci.md#layout-quality-comparison-390) の結果を確認します。

## HTML の更新

```sh
docker compose exec -T dev cargo run --locked -- tests/fixtures/viewer-plan.json --format html -o examples/viewer.html
```

大型の複数 account/state 例は専用の [生成コマンド](examples/multi-account/README.md#regeneration-and-validation) を使います。
HTML の変更は [ブラウザ検証](docs/viewer.md#viewer-source-modules) と上記の操作テストで確認します。

## Terraform fixture

複数の固定 Core / Provider での実 plan 生成・比較は
[互換性 matrix](docs/terraform-compatibility.md)を参照してください。
通常 JSON、resource region override、保存済み plan の provider evidence は別々に検証します。

```sh
docker compose run --rm terraform-example
docker compose exec -T dev cargo run --locked -- output/terraform-plan.json -o output/terraform.svg
```

組み込みリソースと架空のローカル state を使い、クラウド認証情報は不要です。`apply` は実行しません。
通常の Rust テストはコミット済み JSON を使用します。AWS の追加 fixture は [オフライン生成例](examples/terraform-large/README.md) と [Terraform から取得した例](tests/fixtures/aws-captured/README.md) を参照してください。
provider evidence の追加検証は [専用手順](docs/provider-evidence.md#privacy-and-validation) にあります。
この検証では Docker 内に Terraform 1.9.8 / 1.16.6 と AWS 5.100.0 を組み合わせた作業ディレクトリを個別に準備し、外部ネットワークなしで実 plan とローカル STS を検証します。開発イメージの既定 Terraform は変更しません。

## CI・配布

- [PR ごとのキャンセル制御](docs/ci.md#cancelling-superseded-validation)、[依存キャッシュと MSRV](docs/ci.md#toolchain-and-dependency-cache)、[マージ後の検証](docs/ci.md#validation-after-merging)
- [SVG 自動コミット後の status 報告](docs/ci.md#pr-results-and-actions-jobs)と[失敗時の復旧](docs/ci.md#recovery)
- [ネイティブ CLI の公開・再実行手順](docs/releases.md#開発者向け公開手順)

ブランチ・PR・レビューの進め方は [AGENTS.md](AGENTS.md#ブランチ運用) に従います。
