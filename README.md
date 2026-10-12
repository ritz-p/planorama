# planorama

Terraform plan JSON を SVG または自己完結型 HTML に変換する Rust 製 CLI です。参照関係と AWS の構成からカード・包含・接続を描画します。Graphviz や Node.js は不要です。

## 使い方

Docker Desktop を Linux コンテナモードで起動し、リポジトリのルートで実行します。

```sh
docker compose up -d --build dev
docker compose exec -T dev cargo run --locked -- examples/plan.json -o diagram.svg
```

`diagram.svg` をブラウザで開いて確認できます。生成済みの [基本例](examples/diagram.svg) と [大規模 AWS 例](examples/terraform-large/diagram.svg) もあります。

検索・選択・ズーム付きの HTML は `--format html` で生成します。生成後はネットワークなしでローカルのブラウザから開けます。[HTML の例](examples/viewer.html) と [操作方法](docs/viewer.md) を参照してください。

```sh
docker compose exec -T dev cargo run --locked -- examples/plan.json --format html -o diagram.html
```

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

- リソースの変更色・ラベルは作成、更新、削除、置換、読み取り、変更なしを表します。関係線の色は関係自身の変更を表し、接続先カードの変更色は引き継ぎません。独立した変更を持たない関係の `no independent change` は、明示的な「変更なし」と区別します。
- 中立色の `Relationship lines` カードで線種・色・矢印の意味を確認できます。依存辺は参照されるリソースから参照するリソースへ向き、包含は入れ子の枠です。関連付けは通常は矢印のない破線ですが、Route の next hop は矢印付きです。接続は矢印のない太い点線で、構成から推論した関係を表し、ネットワーク到達性は保証しません。
- AWS の VPC・Subnet が空間的なコンテナです。間接参照から導いた配置には `inferred placement` を表示します。
- data リソースは破線と `external` 表示で区別します。一部の AWS メタデータ data は省略します。
- import を含むリソースには専用の `import` バッジを表示します。作成・更新・置換・変更なしの色や action は保持し、import ID や identity の値は表示しません。
- 上部は Logical Resources（シアン）、Logical Components（紫）、Relationship Resources の順です。ECS Cluster・Task Definition・IAM 定義・AMI・CloudWatch Log Group・Metric Alarm は最初のセクションにまとめ、空間配置と経路計算から分離します。
- Relationship Resources は1枚に最大10件をまとめ、最終的な本文の横幅に収まる限り右へ並べます。20件ごとの改行や列数の上限はありません。複数 state では共通の本文幅を使い、state 間の配線領域は含めません。[21件の狭い例](examples/relationship-index.svg) は2枚＋1枚に折り返し、[40件の広い例](examples/relationship-index-wide.svg) は4枚を横に並べます。
- Logical Resources は state 内で provider source・resource type・managed/data ごとに中立色のカードへまとめ、個々のリソースを行として表示します。各行の情報と変更数を保持し、リソース数とグループカード数は別に表示します。
- Logical Resources のリソース行と Logical Components のメンバー行は、中立色のテキストと項目全体の下に引く変更色の下線で表示します。長い項目は全文を折り返し、各リソースの識別子と操作情報を保持します。メンバー行から元リソースへ移動できます。Logical Components の見出しは紫です。
- 利用側の `Uses` 行は参照を全件表示し、対応する論理リソース行へ移動できます。依存指定・未解決・表示対象外を区別し、module・instance・state を含む完全な参照アドレスを折り返して表示します。カードと配置領域は内容に合わせて拡張します。
- 既定のアドレス表示は Planorama 修飾形式です。`--address-format terraform` で Terraform 本来の表記に切り替えられます。長いアドレスは要素の区切りを優先して折り返し、全文を表示します。引用符付きの instance key 内のドットは区切りとして扱いません。
- 移動したリソースには `Moved from:` と移動元の完全なアドレスを表示します。空間カード、論理リソース行、Route 子カード、関係一覧の各リソースで確認でき、長い移動元も折り返して表示します。
- Route は所有先を確認できた Route Table 内に子カードとして表示します。表示可能な接続先へは子カードのポートから矢印付きの破線を引き、接続先を描けない場合は状態を文字で表示します。SVG の宛先は安全性を検証した設定定数（CIDR・prefix list ID）だけを表示します。HTML では宛先定数も省略し、その他の属性値はどちらにも描画しません。plan JSON 自体には機密値が含まれ得るため、実環境の plan/state はコミットしないでください。

図は構成から解決した関係を表し、Terraform の厳密な実行グラフではありません。式の評価、provider 内部の依存、削除専用リソースの過去の参照は復元しません。未解決・動的・曖昧な参照は保守的に扱い、`--diagnostics` で理由を stderr に出力します。診断は SVG や終了条件を変更しません。経路は決定的に生成しますが、交差や重なりのない最適配置は保証しません。

## ドキュメント

| 内容 | 文書 |
| --- | --- |
| AWS の分類・包含・SG 接続・関連付け | [AWS relationships](docs/aws.md) |
| 線種・推論注記 | [Relationship styles](docs/relationships.md) |
| Route Table 内の子カード | [Routes](docs/routes.md) |
| 論理コンポーネント | [Components](docs/components.md) |
| 複数 plan・state 間参照 | [Multi-plan](docs/multi-plan.md)、[Outputs](docs/outputs.md) |
| 表示対象の絞り込み | [Filtering](docs/filtering.md) |
| ローカル HTML ビューアー | [Viewer](docs/viewer.md) |
| import・移動・削除・置換・drift | [Operations](docs/operations.md) |
| Terraform check 結果 | [Checks](docs/checks.md) |
| モデル・provider 設定識別子 | [Models](docs/models.md) |
| レイヤー構成・レイアウト | [Architecture](docs/architecture.md)、[Routing](docs/routing.md) |
| AWS・Terraform 組み込みリソース用アイコンの出典・ライセンス | [Icons](docs/icons.md) |
| 性能計測 | [Benchmarks](docs/benchmarks.md) |
| CI による SVG 生成・自動コミット・PR 検証結果 | [SVG automation](docs/ci.md) |

## 開発・検証

開発ツールチェーンは Rust 1.85.1、MSRV は 1.85.0 です。Dockerfile と `rust-toolchain.toml` を揃え、依存関係の更新時は `Cargo.lock` もコミットします。

```sh
docker compose exec -T dev cargo fmt --all --check
docker compose exec -T dev cargo test --locked --all-targets --all-features
docker compose exec -T dev cargo clippy --locked --all-targets --all-features -- -D warnings
docker compose exec -T dev cargo build --locked
docker compose exec -T dev python3 tests/browser/viewer.py
docker build --build-arg RUST_VERSION=1.85.0 -t planorama-msrv .
docker run --rm --mount type=bind,source=.,target=/workspace --env CARGO_TARGET_DIR=/tmp/planorama-target planorama-msrv cargo +1.85.0 test --locked --all-targets --all-features
```

ローカル開発ではソースをコンテナにバインドマウントし、Cargo のキャッシュとビルド成果物を named volume に保存します。`docker compose stop dev` / `start dev` で停止・再開、Dockerfile の変更後は `up -d --build dev` で再作成します。

通常の CI は Ubuntu runner 上で `rust-toolchain.toml` の固定ツールチェーンを使い、Formatting・Tests・Clippy を直接実行します。開発イメージには SVG の描画確認用に `rsvg-convert` と DejaVu フォント、HTML のオフライン操作テスト用に Chromium・ChromeDriver・Python Selenium を含みます。Tests は単体テスト・通常の結合テスト・example などを実行する `standard` と、重い結合テスト `large_example` を実行する `large` の2ジョブを並列実行します。各ジョブでコンパイルと実行を別ステップにし、両方の成功を従来の `Tests` チェックに集約します。対象は Cargo metadata から選択し、`--locked --all-features` を維持します。[分割と計測の詳細](docs/ci.md#parallel-rust-test-suites) を参照してください。

Formatting・Tests・Clippy は同じ PR の新しい `pull_request` 実行が始まると、同じ workflow の古い実行をキャンセルします。workflow ごとの固定キーと PR 番号で分離するため、別の PR・別の workflow は干渉しません。手動実行（SVG 自動更新後の PR 指定 dispatch を含む）とマージ後の検証は実行 ID ごとに独立し、完了済みのチェック結果も残ります。詳細は [CI のキャンセル制御](docs/ci.md#cancelling-superseded-validation) を参照してください。

Tests・Clippy は `Swatinem/rust-cache` で Cargo registry/git と `target` 内の依存クレートの成果物を保存します。workspace crate は毎回ビルドし、ソース変更後に更新できない同一キーのキャッシュへ成果物を保存しません。キーには runner 環境・Rust compiler・Cargo.toml/Cargo.lock・ツールチェーン・Cargo 設定・ビルド関連環境変数に加え、`build.rs` と `assets/**` のハッシュを含めます。Tests の2ジョブは既存の `tests` キーを共有し、Clippy はプロファイルと成果物が異なるため別のジョブキーを使います。`v1-rust-dependencies` prefix で旧 workspace 成果物を含むキャッシュと区別します。復元後の再ビルド判定は Cargo が行い、キャッシュの有無によらず全対象を検証します。
両 workflow は関連入力が変わる PR の `main` へのマージ後にも実行し、後続 PR が復元できる既定ブランチのキャッシュを作ります。`pull_request_target` の `closed` イベントでマージ済みの場合だけ起動し、読み取り権限でイベント時点の `main` を検証します。SVG の自動コミットに付く `[skip ci]` が squash merge のメッセージに残っても、マージ後の検証を実行できます。`main` へ直接 push した場合は Actions から手動実行してください。

テストプロファイルは `opt-level = 1` で経路計算を高速化し、debug assertion と overflow check を明示的に有効にしています。MSRV は Docker で Rust 1.85.0 の全テストを実行し、Cargo.toml/Cargo.lock・ツールチェーン・Docker 関連設定・MSRV workflow の変更で起動します。

`tests/unit` は実装モジュール内に読み込む単体テスト、`tests/integration` は CLI 経由の結合テストです。Cargo.toml に4つの結合テスト対象を登録しています。`--bin planorama` で単体テスト、`--test rendering` などで結合テストを選択できます。

### SVG の更新

同一リポジトリ内の PR は、レンダラー・生成入力の変更を push すると CI がサンプル SVG を生成し、差分がある場合は同じ PR ブランチへ自動コミットします。通常の更新では最新の push 差分と前のコミットの生成完了を確認し、画像が更新済みならドキュメントや無関係なテストだけの更新では生成を繰り返しません。生成中の追加 push や前回の生成失敗では、必要な再生成を続けます。

自動コミットには `[skip ci]` を付けて承認待ちになる重複の PR イベント実行を抑えます。新しいコミットを PR ブランチへ反映する前に、Formatting・Clippy・Tests と変更内容に応じた MSRV・Workflow tests を Pending で登録し、反映後にこれらの検証を明示的に起動します。**Report PR validation** が実際の検証結果を確認し、同じコミットの PR チェックへ同名の commit status として反映します。**SVG validation dispatch** の成功は起動要求の確認を表し、検証の成功は各チェックで確認します。

この通常の同一リポジトリ内の処理に追加の承認や個人トークンは不要です。結果の報告は reporter が既定ブランチへマージされてから有効になります。生成対象、チェック名と必須設定、手動再実行、fork PR の差分取得は [SVG automation](docs/ci.md) を参照してください。

```sh
docker compose exec -T dev sh scripts/update-examples.sh
```

PowerShell では `./scripts/update-examples.ps1` が開発コンテナの起動と同じ更新処理を行います。[生成対象一覧](scripts/generated-svgs.txt) にある全サンプル SVG を、現在のレンダラーから再生成します。JSON 由来のサンプルに加え、`hierarchy-routing.svg` と `port-collisions.svg` は既存の fixture を構築する単体テストをそれぞれ完全一致で選択して更新します。この処理では全テストを実行しません。アイコン原本は生成対象に含めません。

### Terraform fixture

```sh
docker compose run --rm terraform-example
docker compose exec -T dev cargo run --locked -- output/terraform-plan.json -o output/terraform.svg
```

組み込みリソースと架空のローカル state を使うため、クラウド認証情報は不要です。`apply` は実行しません。通常の Rust テストはコミット済み JSON を使用します。AWS の追加 fixture は [オフライン生成例](examples/terraform-large/README.md) と [Terraform から取得した例](tests/fixtures/aws-captured/README.md) を参照してください。
