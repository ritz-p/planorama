# planorama

Terraform plan JSON を SVG に変換する Rust 製 CLI です。参照関係と AWS の構成からカード・包含・接続を描画します。Graphviz や Node.js は不要です。

## 使い方

Docker Desktop を Linux コンテナモードで起動し、リポジトリのルートで実行します。

```sh
docker compose up -d --build dev
docker compose exec -T dev cargo run --locked -- examples/plan.json -o diagram.svg
```

`diagram.svg` をブラウザで開いて確認できます。生成済みの [基本例](examples/diagram.svg) と [大規模 AWS 例](examples/terraform-large/diagram.svg) もあります。

実際の plan は Terraform を実行できる環境で JSON に変換し、リポジトリ内に配置してください。

```sh
terraform plan -out=tfplan
terraform show -json tfplan > plan.json
docker compose exec -T dev cargo run --locked -- plan.json -o diagram.svg
```

入力は `terraform show -json` の保存済み plan 形式です。`terraform plan -json` のイベントストリームや state JSON は対象外です。PowerShell 5.1 のリダイレクトは UTF-16 になるため、JSON は UTF-8 で保存してください。

```sh
docker compose exec -T dev cargo run --locked -- --help
docker compose exec -T dev cargo run --locked -- plan.json --changed-only -o changes.svg
docker compose exec -T dev cargo run --locked -- plan.json --focus module.app.aws_instance.web --focus-depth 2
docker compose exec -T dev cargo run --locked -- plan.json --diagnostics
docker compose exec -T dev cargo run --locked -- --state network=network.json --state app=app.json -o architecture.svg
```

`-` は標準入力・標準出力を表します。次の例は POSIX シェル向けです。

```sh
docker compose exec -T dev cargo run --locked -- - -o - < plan.json > diagram.svg
```

## 図の見方と制限

- カードと辺の色・ラベルは作成、更新、削除、置換、読み取り、変更なしを表します。
- 依存辺は参照されるリソースから参照するリソースへ向きます。包含は入れ子の枠、関連付けは破線、接続は太い点線です。
- AWS の VPC・Subnet が空間的なコンテナです。間接参照から導いた配置には `inferred placement` を表示します。
- data リソースは破線と `external` 表示で区別します。一部の AWS メタデータ data は省略します。
- 既定のアドレス表示は Planorama 修飾形式です。`--address-format terraform` で Terraform 本来の表記に切り替えられます。省略されたラベルの全文はカードのタイトルで確認できます。
- 属性値は描画しません。plan JSON 自体には機密値が含まれ得るため、実環境の plan/state はコミットしないでください。

図は構成から解決した関係を表し、Terraform の厳密な実行グラフではありません。式の評価、provider 内部の依存、削除専用リソースの過去の参照は復元しません。未解決・動的・曖昧な参照は保守的に扱い、`--diagnostics` で理由を stderr に出力します。診断は SVG や終了条件を変更しません。経路は決定的に生成しますが、交差や重なりのない最適配置は保証しません。

## ドキュメント

| 内容 | 文書 |
| --- | --- |
| AWS の分類・包含・SG 接続・関連付け | [AWS relationships](docs/aws.md) |
| 線種・推論注記 | [Relationship styles](docs/relationships.md) |
| Route の辺への変換 | [Routes](docs/routes.md) |
| 論理コンポーネント | [Components](docs/components.md) |
| 複数 plan・state 間参照 | [Multi-plan](docs/multi-plan.md)、[Outputs](docs/outputs.md) |
| 表示対象の絞り込み | [Filtering](docs/filtering.md) |
| import・削除・置換・drift | [Operations](docs/operations.md) |
| Terraform check 結果 | [Checks](docs/checks.md) |
| モデル・provider 設定識別子 | [Models](docs/models.md) |
| レイヤー構成・レイアウト | [Architecture](docs/architecture.md)、[Routing](docs/routing.md) |
| AWS アイコンの出典・権利 | [Icons](docs/icons.md) |
| 性能計測 | [Benchmarks](docs/benchmarks.md) |

## 開発・検証

開発ツールチェーンは Rust 1.85.1、MSRV は 1.85.0 です。Dockerfile と `rust-toolchain.toml` を揃え、依存関係の更新時は `Cargo.lock` もコミットします。

```sh
docker compose exec -T dev cargo fmt --all --check
docker compose exec -T dev cargo test --locked --all-targets --all-features
docker compose exec -T dev cargo clippy --locked --all-targets --all-features -- -D warnings
docker build --build-arg RUST_VERSION=1.85.0 -t planorama-msrv .
docker run --rm --mount type=bind,source=.,target=/workspace --env CARGO_TARGET_DIR=/tmp/planorama-target planorama-msrv cargo +1.85.0 test --locked --all-targets --all-features
```

ローカル開発ではソースをコンテナにバインドマウントし、Cargo のキャッシュとビルド成果物を named volume に保存します。`docker compose stop dev` / `start dev` で停止・再開、Dockerfile の変更後は `up -d --build dev` で再作成します。

通常の CI は Ubuntu runner 上で `rust-toolchain.toml` の固定ツールチェーンを使い、Formatting・Tests・Clippy を実行します。開発イメージが追加するのは Rust と rustfmt・Clippy のため、CI では直接実行してイメージ構築を省きます。Tests はコンパイルと実行を別ステップで計測し、毎回 `cargo test --locked --all-targets --all-features` を実行します。

Tests・Clippy は `Swatinem/rust-cache` で Cargo registry/git と `target`（workspace crate を含む）を保存します。キーには runner 環境・ジョブ・Rust compiler・Cargo.toml/Cargo.lock・ツールチェーン・Cargo 設定・ビルド関連環境変数に加え、`build.rs` と `assets/**` のハッシュを含めます。復元後の再ビルド判定は Cargo が行い、キャッシュの有無によらず全対象を検証します。Tests と Clippy はプロファイルと成果物が異なるためジョブ別のキャッシュを使います。
両 workflow は関連入力が変わる `main` への push でも実行し、後続 PR が復元できる既定ブランチのキャッシュを作ります。

テストプロファイルは `opt-level = 1` で経路計算を高速化し、debug assertion と overflow check を明示的に有効にしています。MSRV は Docker で Rust 1.85.0 の全テストを実行し、Cargo.toml/Cargo.lock・ツールチェーン・Docker 関連設定・MSRV workflow の変更で起動します。

`tests/unit` は実装モジュール内に読み込む単体テスト、`tests/integration` は CLI 経由の結合テストです。Cargo.toml に4つの結合テスト対象を登録しています。`--bin planorama` で単体テスト、`--test rendering` などで結合テストを選択できます。

### SVG の更新

```sh
docker compose exec -T dev sh scripts/update-examples.sh
```

PowerShell では `./scripts/update-examples.ps1` が開発コンテナの起動と同じ更新処理を行います。`examples` 配下の全 SVG を、コミット済み JSON と現在のレンダラーから再生成します。公式アイコン原本は変更しません。

### Terraform fixture

```sh
docker compose run --rm terraform-example
docker compose exec -T dev cargo run --locked -- output/terraform-plan.json -o output/terraform.svg
```

組み込みリソースと架空のローカル state を使うため、クラウド認証情報は不要です。`apply` は実行しません。通常の Rust テストはコミット済み JSON を使用します。AWS の追加 fixture は [オフライン生成例](examples/terraform-large/README.md) と [Terraform から取得した例](tests/fixtures/aws-captured/README.md) を参照してください。
