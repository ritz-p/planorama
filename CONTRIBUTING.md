# 開発者ガイド

利用者向けの導入・生成手順は [README](README.md)、開発ルールは [AGENTS.md](AGENTS.md) を参照してください。
CI の起動条件・公開チェック・復旧手順の正本は [docs/ci.md](docs/ci.md) です。

## 開発環境

Docker Desktop を Linux コンテナモードで起動し、リポジトリのルートで実行します。

```sh
docker compose up -d --build dev
docker compose exec -T dev cargo build --locked
docker compose exec -T dev mkdir -p output
docker compose exec -T dev cargo run --locked -- examples/plan.json -o output/diagram.svg
docker compose exec -T dev cargo run --locked -- --help
```

開発ツールチェーンは Rust 1.85.1、MSRV は 1.85.0 です。Dockerfile と `rust-toolchain.toml` を揃え、依存関係の更新時は `Cargo.lock` もコミットします。
ソースをコンテナへバインドマウントし、Cargo registry/git とビルド成果物は named volume に保存します。
`docker compose stop dev` / `docker compose start dev` で停止・再開し、Dockerfile の変更後は `docker compose up -d --build dev` で再作成します。

開発イメージには Terraform 1.9.8、SVG 描画確認用の `rsvg-convert` と DejaVu フォント、HTML 操作テスト用の Chromium・ChromeDriver・Node.js を含みます。
Wasm 用には `wasm32-unknown-unknown` と wasm-bindgen-cli 0.2.100 を含みます。[ビルド・ブラウザ検証](docs/browser.md)では同じ Docker 環境を使用します。
実環境の plan/state や認証情報をコミットしないでください。

Linux x86_64 の Compose 開発環境では native target の linker に mold 2.40.4 を使います。
Wasm の `wasm-ld`、他 OS/architecture、公開 release の設定は変更しません。
Dockerfile は upstream 配布物の SHA-256 を検証して導入します。
Docker 外では `sudo sh scripts/install-mold.sh` の後、
`CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="$PWD/scripts/native-linker.sh" cargo test --locked`
で有効化できます（GCC 12.1 以降が必要）。設定しない通常の Cargo は既存 linker を使います。
Compose 内で無効化するには `docker compose exec -T -e CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=cc dev cargo test --locked` を使います。
linker の切替は Cargo の再ビルドを伴います。比較手順と結果は [build performance](docs/build-performance.md) を参照してください。

Compose は sccache 0.10.0 を compiler wrapper として使い、2 GiB 上限の named volume
`sccache` にコンパイル結果を保存します。コンテナ再作成後も再利用できます。
ローカルの incremental compilation は Cargo の既定を維持します。
統計は `docker compose exec -T dev sccache --show-stats`、無効化は
`docker compose exec -T -e RUSTC_WRAPPER= dev cargo build --locked` です。
削除・CI との役割分担・測定条件は [compiler cache](docs/build-performance.md#compiler-cache) を参照してください。

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
`cargo test` を継続する理由と nextest の比較は [計測結果](docs/ci.md#test-runner-evaluation-254) にあります。

## リソース対応一覧の更新

provider の型定義・分類・表示・scope・サービス名を変更したら、Docker 内で次を実行します。

```sh
docker compose exec -T dev sh scripts/update-resource-support.sh
docker compose exec -T dev sh scripts/update-resource-support.sh --check
```

生成先は [docs/resource-support.md](docs/resource-support.md) です。型名は provider と remote-output 解決の Rust ソースの文字列リテラルから収集し、分類・表示・scope・サービス名は実装関数で評価します。確認済み resource alias の関係ルールは canonical 型の参照先を使います。新しい定義形式や provider を追加する場合は `tests/unit/resource_support.rs` の収集処理も更新してください。関係ルールのリンクは条件付き処理の参照先であり、無条件の対応保証ではありません。
AWS managed resource の正本は固定 Provider inventory です。全型との差分、基本分類表の追加方法、schema・登録コード・公式 docs の再採取手順は [AWS inventory](docs/aws-inventory.md) を参照してください。基本表示の baseline と保存済み plan の provider evidence の検証版は別の契約です。
CI の `Resource support` は一覧の一致検証と inventory 抽出・差分比較のオフラインテストを実行します。固定版どうしの取得・比較は [baseline 比較手順](docs/aws-inventory.md#compare-candidate-baselines) を参照してください。図を生成しないため、この一覧や比較ツールだけの更新に SVG 再生成や Layout quality は不要です。

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
