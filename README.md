# planorama

Terraform plan JSON を SVG に変換する Rust 製 CLI です。JSON 解析、依存関係の整理、レイアウト、SVG 生成を Rust 内で行い、Graphviz や Node.js は使用しません。

## Docker で開発

Docker Desktop を Linux コンテナモードで起動してください。ホストへの Rust インストールは不要です。

```sh
docker compose build
docker compose run --rm dev cargo fmt --check
docker compose run --rm dev cargo test --locked
docker compose run --rm dev cargo clippy --locked --all-targets -- -D warnings
docker compose run --rm dev cargo run --locked -- examples/plan.json -o diagram.svg
```

`diagram.svg` がリポジトリ直下に出力されます。ブラウザで開くと図を閲覧でき、リソースにマウスを重ねると完全なアドレスを確認できます。ソースはバインドマウント、Cargo のキャッシュとビルド成果物は Docker の named volume に保存します。

生成済みの図は [examples/diagram.svg](examples/diagram.svg)、モジュールを含む実際の Terraform plan の図は [examples/terraform.svg](examples/terraform.svg) で確認できます。

## Terraform を含む動作確認

外部のクラウドや認証情報を使わず、組み込みの `terraform_data` リソースと `terraform_remote_state` データソースでサンプル plan を生成できます。Terraform も Docker 内で実行します。

```sh
docker compose run --rm terraform-example
docker compose run --rm dev cargo run --locked -- output/terraform-plan.json -o output/terraform.svg
```

サンプルには 14 リソースと 2 データソース、入れ子のモジュール、`count`、`for_each`、モジュールとリソースの `depends_on` が含まれます。`plan` までを実行し、`apply` は実行しません。テスト用に生成した JSON を `tests/fixtures/terraform-plan.json` に保存しており、通常の `cargo test` では Terraform 自体は不要です。

データソースは `examples/terraform/shared-state.json` の架空のローカル state を読みます。[terraform_remote_state](https://developer.hashicorp.com/terraform/language/state/remote-state-data) を使った次の 2 パターンを確認できます。

- `data.terraform_remote_state.existing`：plan 時に読み取り済みで、図では `unchanged`。
- `data.terraform_remote_state.after_ready`：`terraform_data.ready` の変更を待って apply 時に読むため、図では紫色の `read`。

両方のデータソースを `terraform_data.from_state` が参照します。図では `terraform_data.ready → data.terraform_remote_state.after_ready → terraform_data.from_state` の依存関係を確認できます。

簡単な `examples/plan.json` は手書きの AWS 風サンプルで、`data.aws_ami.latest` をモジュールの入力変数経由で 2 つの EC2 インスタンスが参照する例も含みます。

## 実際の plan を図にする

Terraform を実行できる環境で保存済み plan を JSON に変換し、このディレクトリに `plan.json` として配置します。

```sh
terraform plan -out=tfplan
terraform show -json tfplan > plan.json
docker compose run --rm dev cargo run --locked -- plan.json -o diagram.svg
```

入力は `terraform show -json` の plan 形式です。`terraform plan -json` のイベントストリームや state JSON は対象外です。PowerShell 5.1 ではリダイレクトが UTF-16 になるため、Terraform の JSON を UTF-8 で保存してください。

```sh
docker compose run --rm dev cargo run --locked -- --help
docker compose run --rm -T dev cargo run --locked -- - -o - < plan.json > diagram.svg
docker compose run --rm dev cargo build --locked --release
```

標準入力の例は POSIX シェル向けです。最後のバイナリは Linux 用で、コンテナの `/workspace/target/release/planorama` に生成されます。

## 初期版の表示仕様

- 作成・更新・削除・置換・読み取り・変更なしを色とラベルで区別します。
- 変更なしのリソースも含め、削除されるリソースも残します。
- 読み取り済みの `data` が `prior_state` にのみ存在する場合も、構成に残っていれば図へ補います。
- モジュールのインスタンスごとに枠でまとめます。
- 矢印は依存元から依存先へ向けます。
- 構成の式、count、for_each の参照を解析し、モジュールの入力変数・出力経由の参照も追跡します。
- モジュール自体の count / for_each / depends_on の参照を、入れ子を含む子リソースへ反映します。
- 強連結成分をまとめて階層を計算するため、循環参照があっても配置できます。
- 依存階層ごとの並べ替えで線の交差を減らし、ノード間の余白に接続線を通します。同じ入力から同じ SVG を生成します。
- 属性値は一切描画しません。plan JSON 自体には機密値が含まれ得るため、Git 管理から除外しています。

## 制限

水平の接続線は、各通路で使用中の x 区間を記録して配置します。重なりのない通路を優先し、同じ通路でも区間が離れていれば再利用します。すべての候補で重なる場合は重なりの合計長が最小の通路を選び、同点では経路の短さと固定順で決定します。

図は構成から推定した参照関係で、Terraform の厳密な実行グラフではありません。構成にない参照、provider 内部の依存、削除専用リソースの過去の依存関係は復元しません。`count` / `for_each` のリソース・モジュール参照は対応する全インスタンスへ展開するため、実際より多くの線になる場合があります。locals を介する参照は追跡しません。

配置はモジュールごとの帯と依存階層を使います。ノードを横切らない直角の経路を計算しますが、交差数の最小化は保証せず、密なグラフでは線同士が重なることがあります。長いラベルは省略し、完全なアドレスを SVG の title に保持します。構成情報がない plan は、リソースと変更種別のみを表示します。

入力形式の詳細は [Terraform JSON Output Format](https://developer.hashicorp.com/terraform/internals/json-format) を参照してください。

## コード構成

モジュールは `mod.rs` を使わず、同名の `.rs` とディレクトリで構成します。`plan` は Terraform plan JSON の読み取りを担当し、サンプル設定の `examples/terraform/` と区別しています。

```text
src/
├── main.rs               # 起動・終了コード・エラー表示
├── cli.rs                # 引数解析・ファイル入出力・処理の呼び出し
├── model.rs              # Action / Node / Graph の共通データ型
├── plan.rs               # plan JSON → Graph
├── plan/
│   ├── address.rs        # モジュール・インスタンスのアドレス解析
│   └── references.rs     # 参照解決・依存関係の構築
├── layout.rs             # Graph → Layout、座標・経路のデータ型
├── layout/
│   ├── rank.rs           # 循環参照の処理・依存階層の計算
│   ├── placement.rs      # ノード配置・交差を減らす並べ替え
│   └── routing.rs        # 接続線の経路計算
├── svg.rs                # Graph + Layout → SVG
└── svg/
    └── style.rs          # 配色・変更種別ラベル・文字の省略
```

`cli` が `plan::parse` → `Layout::new` → `svg::render` の順に呼び出します。`model` は JSON や描画に依存せず、`layout` は Terraform 固有の入力形式を扱いません。`svg` は計算済みの座標と経路を受け取って描画します。

テストコードとテストデータは、ルートの `tests/` に集約しています。

```text
tests/
├── cli.rs                # CLI の結合テスト
├── fixtures/
│   └── terraform-plan.json
└── unit/
    ├── plan.rs           # plan の読み取り・参照解決
    ├── plan/
    │   └── address.rs    # アドレス解析
    ├── layout.rs         # 配置・経路
    ├── layout/
    │   └── rank.rs       # 循環参照・依存階層
    └── svg.rs            # 描画・エスケープ
```

単体テストは実装側の `#[cfg(test)]` と `#[path = "..."]` で読み込みます。テストのために内部関数を公開する必要はなく、実装ファイルには読み込み宣言だけを置きます。実行方法はこれまでどおり `docker compose run --rm dev cargo test --locked` です。
