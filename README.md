# planorama

Multiple plans can be loaded with explicit state identities: `planorama --state network=network/plan.json --state application=application/plan.json -o architecture.svg`. Identical Terraform addresses remain distinct across states. See [multi-plan usage and identity rules](docs/multi-plan.md) and the [SVG example](examples/multi-plan.svg).

Multiple plans can be loaded with explicit state identities: `planorama --state network=network/plan.json --state application=application/plan.json -o architecture.svg`. Identical Terraform addresses remain distinct across states. See [multi-plan usage and identity rules](docs/multi-plan.md) and the [SVG example](examples/multi-plan.svg).

Import and state-removal operations are represented separately from ordinary action colors. See [Terraform operation metadata](docs/operations.md) for supported JSON shapes, value redaction, and limitations.

Security Group attachments use explicit AWS references to draw `Connection` edges from each Security Group to its consumer. Policy cards and VPC/subnet containment remain intact. See [security-groups.svg](examples/security-groups.svg).

| Consumer | Attachment attribute |
| --- | --- |
| EC2 instance | `vpc_security_group_ids` |
| Load Balancer | `security_groups` |
| RDS instance / cluster | `vpc_security_group_ids` |
| ECS service | `network_configuration.security_groups` |
| VPC endpoint | `security_group_ids` |

Each endpoint must resolve completely and statically to an AWS Security Group. Multiple explicitly identified groups and exact indexed references are supported, including external SG data sources. The parser also checks the entire planned ID collection against the referenced resources' IDs using `change.after`, `after_unknown`, or `planned_values` (and cached prior data-source IDs). Known IDs and individual unknown slots must match in number and value/unknown status. Only the proof result is retained, not the IDs. Mixed literal lists, missing value evidence, unknown whole collections, unresolved parts, dynamic selection, ambiguous unindexed instances, wrong endpoint types/providers, tags, and EC2's name-based `security_groups` field stay ordinary dependencies. This can conservatively leave valid attachments as dependencies when Terraform does not expose enough collection shape information. Missing optional attachment attributes are not diagnosed as errors. ECS task definitions and other unsupported attachment shapes are not inferred. This is an attachment diagram, not an analysis of firewall rules or reachability.

Terraform plan JSON を SVG に変換する Rust 製 CLI です。JSON 解析、依存関係の整理、レイアウト、SVG 生成を Rust 内で行い、Graphviz や Node.js は使用しません。

## Docker で開発

Docker Desktop を Linux コンテナモードで起動してください。ホストへの Rust インストールは不要です。

```sh
docker compose up -d --build dev
docker compose exec dev cargo fmt --check
docker compose exec dev cargo test --locked
docker compose exec dev cargo clippy --locked --all-targets -- -D warnings
docker compose exec dev cargo run --locked -- examples/plan.json -o diagram.svg
```

`diagram.svg` がリポジトリ直下に出力されます。ブラウザで開くと図を閲覧でき、リソースにマウスを重ねると完全なアドレスを確認できます。ソースはバインドマウント、Cargo のキャッシュとビルド成果物は Docker の named volume に保存します。

生成済みの図は [examples/diagram.svg](examples/diagram.svg)、モジュールを含む実際の Terraform plan の図は [examples/terraform.svg](examples/terraform.svg) で確認できます。

## Rust ツールチェーンと品質方針

MSRV（最低対応 Rust バージョン）は **1.85.0** です。[Rust 2024 edition の導入バージョン](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0.html)を下限とし、`Cargo.toml` の `rust-version` に宣言しています。依存関係を更新する場合も `Cargo.lock` をコミットし、MSRV でテストを通してください。

開発用コンテナと `rust-toolchain.toml` は **1.85.1** に固定しています。rustfmt の標準設定を使い、整形結果をツールチェーン間で揃えます。整形する場合は `docker compose run --rm dev cargo fmt --all` を実行します。

Clippy は全ターゲット・全 feature に対して警告をエラーにします。プロジェクト全体の一括 `allow` は追加せず、例外が必要なら最小範囲に限定して PR に理由を記載してください。ツールチェーンを更新する場合は Dockerfile と `rust-toolchain.toml` を一緒に更新します。MSRV を引き上げる場合は Cargo.toml、CI、本文も更新します。

CI は Formatting・Clippy・Tests・MSRV の 4 ジョブを独立して実行します。ビルドとテストには `--locked` を付け、MSRV ジョブは開発用の指定を明示的に上書きして Rust 1.85.0 を検証します。ローカルでも同じ確認ができます。

```sh
docker compose exec dev cargo fmt --all --check
docker compose exec dev cargo clippy --locked --all-targets --all-features -- -D warnings
docker compose exec dev cargo test --locked --all-targets --all-features
docker build --build-arg RUST_VERSION=1.85.0 -t planorama-msrv .
docker exec --mount type=bind,source=.,target=/workspace --env CARGO_TARGET_DIR=/tmp/planorama-target planorama-msrv cargo +1.85.0 test --locked --all-targets --all-features
```

開発コンテナは Cargo コマンド終了後も起動し続けます。作業を始めるときは `docker compose up -d --build dev` を実行してください。シェルには `docker compose exec dev bash` で入り、`cargo test --locked` などを直接実行できます。シェルを `exit` してもコンテナは停止しません。スクリプトや標準入力を使う場合は `docker compose exec -T dev ...` とします。

```sh
docker compose stop dev
docker compose start dev
docker compose down
```

`stop` / `start` は同じコンテナを停止・再開し、`down` はコンテナを削除します。`down` 後も named volume のキャッシュは残り、`up -d --build dev` で再作成できます。Dockerfile を変更した場合もこのコマンドで再ビルドします。一度だけ実行する場合は、従来の `docker compose run --rm dev cargo test --locked` も利用できます。

## Terraform を含む動作確認

外部のクラウドや認証情報を使わず、組み込みの `terraform_data` リソースと `terraform_remote_state` データソースでサンプル plan を生成できます。Terraform も Docker 内で実行します。

```sh
docker compose run --rm terraform-example
docker compose exec dev cargo run --locked -- output/terraform-plan.json -o output/terraform.svg
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
docker compose exec dev cargo run --locked -- plan.json -o diagram.svg
```

入力は `terraform show -json` の plan 形式です。`terraform plan -json` のイベントストリームや state JSON は対象外です。PowerShell 5.1 ではリダイレクトが UTF-16 になるため、Terraform の JSON を UTF-8 で保存してください。

```sh
docker compose exec dev cargo run --locked -- --help
docker compose exec -T dev cargo run --locked -- - -o - < plan.json > diagram.svg
docker compose exec dev cargo build --locked --release
```

標準入力の例は POSIX シェル向けです。最後のバイナリは Linux 用で、コンテナの `/workspace/target/release/planorama` に生成されます。

## 初期版の表示仕様

内部モデルは Terraform の所有形態を `EntityMode`（`Managed` / `Data`）、アーキテクチャ上の役割を `ResourceRole`（`Container` / `Node` / `Connector` / `Association` / `Policy` / `Controller` / `Unknown`）として別々に保持します。mode は JSON の値を優先し、省略されたサンプルではモジュール部分を除いたリソースアドレスから補います。分類は JSON パーサーから独立した `classification` がリソース型名だけを使って行い、managed / data の違いで役割は変わりません。未知の型は `Unknown` になり、従来どおり描画します。role は意味変換の判定に使用します。

AWS の初期分類は `classification/aws.rs` に閉じ込めています。型名の完全一致で分類し、表示や依存関係は変更しません。

| 型 | 役割 |
|---|---|
| `aws_vpc`, `aws_subnet` | `Container` |
| `aws_instance`, `aws_db_instance` | `Node` |
| `aws_vpc_peering_connection` | `Connector` |
| `aws_route_table_association` | `Association` |
| `aws_security_group`, `aws_network_acl` | `Policy` |
| `aws_autoscaling_group`, `aws_ecs_service` | `Controller` |
| その他 | `Unknown` |

- 作成・更新・削除・置換・読み取り・変更なしを色とラベルで区別します。
- 表示対象は変更なし・削除も残します。後述のメタデータ data だけは省略します。
- 読み取り済みの `data` が `prior_state` にのみ存在する場合も、構成に残っていれば図へ補います。
- コンテナのある図はモジュールの帯を使わず、包含関係を入れ子の枠で表します。モジュール名はカードの注記とアドレス情報に残します。コンテナのない参照図は従来のモジュール帯を使います。
- 参照の矢印は依存元から依存先へ向け、関連付けの線には関係の種類を付けます。
- 構成の式、count、for_each の参照を解析し、モジュールの入力変数・出力経由の参照も追跡します。
- モジュール自体の count / for_each / depends_on の参照を、入れ子を含む子リソースへ反映します。
- 強連結成分をまとめて階層を計算するため、循環参照があっても配置できます。
- 依存階層ごとの並べ替えで線の交差を減らし、ノード間の余白に接続線を通します。同じ入力から同じ SVG を生成します。
- 属性値は一切描画しません。plan JSON 自体には機密値が含まれ得るため、Git 管理から除外しています。

同じモジュール内の隣接列では、共通の依存元または依存先を持つ辺を明示的に束ねます。fan-out は各依存先の入次数が 1、fan-in は各依存元の出次数が 1 のグループを対象にし、多対多の関係を誤って表現しないようにします。幹線の縦区間はグループ全体で予約し、無関係な辺との共有を防ぎます。分岐点には丸印を付け、各辺の title と矢印を残します。長い辺・循環・モジュール間の辺は個別に描画します。

fan-out と fan-in を続けたサンプルは [bundling.svg](examples/bundling.svg) です。再生成できます。

```sh
docker compose exec dev cargo run --locked -- examples/bundling-plan.json -o examples/bundling.svg
```

配置の並べ替えでは、2 階層以上離れた辺を途中の階層ごとの仮想ノードで分割します。仮想ノードは依存元のモジュール帯に置き、通常のノードと一緒に重心による並べ替えへ参加させます。仮想ノードの行は余白として残るため、長い辺が多いと図の高さが増えます。リソースのアドレスや依存関係は変更せず、仮想ノードをカードとして描画することもありません。接続線は元の辺ごとに 1 本の折れ線として生成します。

## 制限

以下の階層・レーン・束ね処理は Container のない図に適用します。Container のある図は親子を縦に配置し、カードと無関係な枠を避ける直角経路を探索します。こちらは幹線の共有や交差数の最小化を行わないため、依存が多い場合は図が縦長になり、線が重なる場合があります。

経路候補は直角の折れ線として生成します。隣接列・同じ列では空いている縦レーンを最大 3 本、離れた列では各水平通路を候補にします。候補を「カード内部の通過数 → 線の重複長 → 交差数 → 曲がり数 → 経路長」の辞書順で比較し、同点なら生成順で選びます。短さより重なりの回避を優先します。
レイアウトの品質計測ヘルパーを `tests/support/layout_metrics.rs` に用意しています。`layout::metrics::measure` に経路を渡すと、異なるエッジ間の重複長・交差数・90 度の曲がり数・総経路長を取得できます。テスト時だけ読み込まれ、CLI の出力には影響しません。

重複長はエッジの組ごとに加算します（3 本が同じ区間で重なる場合は 3 組分）。交差は直交する線分の内部同士で交わる点をエッジの組ごとに数え、端点で接するだけの場合や自己交差は含めません。長さゼロの線分と同一直線上の分割点を整理してから計測するため、点の追加や経路の向きを変えても値は変わりません。距離の単位は SVG 座標です。
縦の接続線は列間の通路ごとに使用中の y 区間を記録します。重ならないレーンを先頭から再利用し、すべて埋まっている場合はレーンを追加します。必要なレーン数に応じて列間と SVG の横幅を広げ、ノードへのはみ出しを防ぎます。
列間を広げた場合は経路を再計算し、最終的な座標で水平区間の重なりを評価します。

縦レーンは選択された候補だけを予約し、必要に応じて列間と SVG の横幅を広げます。列幅が変わったら候補を再評価し、最終的な経路がカードを横切らない配置を保ちます。

水平・垂直の重複長は座標ごとの疎なセグメント木で照会します。交差は既に選択した辺との厳密な内部交差を数え、端点の接触は数えません。候補は辺の順に選ぶため大域的な最適解は保証しません。交差の評価は既存の線分を走査するため、密なグラフでは計算時間が増えます。

図は構成から推定した参照関係で、Terraform の厳密な実行グラフではありません。構成にない参照、provider 内部の依存、削除専用リソースの過去の依存関係は復元しません。`count` / `for_each` の参照に固定の数値・文字列キーがある場合は、そのインスタンスに限定します。動的な添字や添字のない参照は、既知のモジュールスコープ内の候補へ展開します。Terraform が同一式に出力する参照の接頭辞は、より具体的な参照へまとめます。同一式がコレクション全体と特定要素の両方を使う場合、参照一覧だけでは両者を区別できないことがあります。

構成 JSON の `locals` に参照情報が含まれている場合は、モジュールごとのスコープを保って連鎖を追跡します。循環参照は停止し、包含判定では不完全な解決として扱います。JSON にローカル値の定義が含まれない場合は復元できません。Terraform 式の評価やソースファイルの読み込みは行いません。

Container のない図はモジュールごとの帯と依存階層を使い、Container のある図はモジュール境界に依存しない包含配置を使います。ノードを横切らない直角の経路を計算しますが、交差数の最小化は保証せず、密なグラフでは線同士が重なることがあります。長いラベルは省略し、完全なアドレスを SVG の title に保持します。構成情報がない plan は、リソースと変更種別のみを表示します。

入力形式の詳細は [Terraform JSON Output Format](https://developer.hashicorp.com/terraform/internals/json-format) を参照してください。

## コード構成

大きな AWS 構成の回帰サンプルは [terraform-large](examples/terraform-large/README.md) と [生成 SVG](examples/terraform-large/diagram.svg) にあります。4モジュールに分けた Terraform JSON 形式のソースから、AWS 認証なしで代表的な plan fixture を Rust で再生成できます。fixture は実際の `terraform show` 出力ではなく、参照・配置の検証用データです。

`Node::resource_address()` は Terraform の正規アドレスとモジュール情報を借用する構造化ビューです。`terraform()` は入力のアドレスをそのまま返し、`qualified()` は root リソースだけに表示用の `module.root.` を付け、`local()` はモジュール部分を除いたカード表示名を返します。インデックスや引用符は保持します。SVG は `data-qualified-address` に表示用アドレスを記録します。参照解決・グラフの識別・配置には合成プレフィックスを使いません。実在する `module.root` と表示名が一致する場合もあるため、識別には必ず Terraform アドレスを使います。

`svg/paths.rs` は直角の経路点を SVG に変換するときだけ角を丸めます。標準半径は 8 px で、前後の区間長の半分を超えないよう縮めます。短い区間には 0.5 px 単位も使い、始点・終点と矢印の向きは維持します。経路探索・配置・衝突判定の座標は変更しません。

data は役割とは別に破線と `external` バッジで表示します。VPC・Subnet などの Container は外部リソースでも枠になり、managed の子を包含できます。data の検索条件から包含を推測しません。Policy・Controller の表示ルールも併用します。未知の data 型は情報を失わないよう外部カードとして残します。

メタデータ型の `data.aws_region`、`data.aws_partition`、`data.aws_caller_identity`、`data.aws_availability_zones`、`data.aws_iam_policy_document` は描画用グラフから省略します。同じ型でも managed リソースは省略しません。省略ノードに接続する辺も除き、表示ノード間の直接の辺を保持します。経由した関係の推測・付け替えは行いません。元の Terraform グラフは全情報を保持し、SVG の件数・凡例は表示対象のみを集計します。外部リソースのサンプルは [data.svg](examples/data.svg)（入力: [data-plan.json](examples/data-plan.json)）です。

コンテナの枠線は外周だけに描き、ヘッダーの内枠は描きません。入れ子の祖先・子孫間の参照線は省略し、包含を枠だけで表します。兄弟間・外部との接続や関連付けの辺は維持します。省略した参照もグラフと SVG のタイトル情報には残します。

コンテナはヘッダーだけでなく枠全体を変更種別の色で塗ります。最上位を従来の背景色とし、包含の深さに応じて同系色を5段階で濃くします。6階層目以降は5段階目の色を使います。通常のリソースカードの色と変更種別の凡例は維持します。

`layout/containers.rs` は Container を子の大きさに合わせた入れ子の枠として配置します。包含辺は枠で表し、その他の辺はカードのヘッダーと無関係なコンテナを避ける直角経路で接続します。循環・複数の親候補がある包含辺は入れ子にせず線を残します。最上位の配置は包含ツリーを単位に接続先をたどり、関連するツリーを近くに置きます。モジュールごとの配置境界は設けず、各カードに元のモジュール名を表示します。Container がない図は従来の階層配置を使います。

コンテナ内と最上位の並び順には、モジュールを除いた名前、リソースの種別・役割・変更内容、方向と種類を含む接続関係から計算したキーを使います。接続先のキーを繰り返し反映するため、同名リソースも周囲の構成で区別できます。これらの情報で区別できない対称なリソース同士は、モジュールパスを含む完全な Terraform アドレスで同順位を解消します。ポート割り当てと経路の処理順にもこの識別情報を使い、入力グラフのノードや辺を並べ替えても配置と経路を維持します。構造的に区別できない場合は、モジュール名の変更によって並び順が変わることがあります。

`semantic/containment.rs` は managed の Subnet の `vpc_id`、EC2 の `subnet_id` が対応する親へ一意に解決する場合、その依存辺を `Containment` にします。タグ・`depends_on`・定数 ID・未解決参照・複数候補は変換しません。data の親を参照する managed の子には対応しますが、data の検索条件は包含に変換しません。包含推定はノード数・辺数・順序を保持し、その後に関連付けの変換を実行します。SVG では `data-edge-kind="containment"` を保持し、確定した包含関係を入れ子の枠として表現します。サンプルは [containment.svg](examples/containment.svg) です。
`Policy`（Security Group、NACL）は属性値を含めない軽量なカードと `policy` ラベルで表示します。`Controller`（ASG、ECS Service）は通常のカードに `controller` ラベルを付けます。どちらもリソース名・変更種別・依存辺・分類を保持し、省略しません。ルール一覧や管理対象のグループ化は行わず、今後の専用表示に備えて分類を残します。

`semantic/associations.rs` は managed の関連付けリソースを `Association` 辺へ置き換えます。対応するルールは次のとおりです。

| リソース | 端点の属性 | 描画する関係 |
| --- | --- | --- |
| `aws_route_table_association` | `subnet_id`, `route_table_id` | Subnet → Route Table |
| `aws_lb_target_group_attachment` | `target_group_arn`, `target_id` | Target Group → EC2 Instance |
| `aws_vpc_endpoint_route_table_association` | `vpc_endpoint_id`, `route_table_id` | VPC Endpoint → Route Table |

両端が静的かつ一意に解決でき、元の依存辺がその2本だけの場合に限定します。追加の依存先・利用元、複数候補、未解決参照、動的な選択、定数だけの ID、data の関連付けは元のカードを残します。Target Group の IP・Lambda・ALB ターゲットも対象外です。元のアドレスと変更種別は辺に保持します。全ルールを同じグラフ上で判定してからまとめてノード番号を振り直すため、変換途中の削除によって後続の判定が変わることはありません。生グラフは変更しません。SVG の意味付き辺には `data-edge-kind` を付けます。サンプルは [association.svg](examples/association.svg) と [aws-relationships.svg](examples/aws-relationships.svg) です。

`TerraformPlan` は描画モデルから独立した `TerraformEntity` と `TerraformReference`、属性名・参照先・解決状態を保持します。属性値そのものや architecture の役割は保持しません。`depends_on`、count、for_each は依存参照には反映しますが、属性の参照情報には混ぜません。属性参照が未解決・複数候補の場合、後続の意味変換は元の関係を維持できます。

CLI の処理は `plan::parse → TerraformPlan → semantic::transform → ArchitectureGraph → Layout → SVG` の順です。意味変換の入口で provider と resource type から役割を分類し、元の Terraform facts を変更せず独立した描画用グラフを返します。包含推定の後、対応する関連付けリソースを辺へ変換し、ノード番号を再割り当てします。変換規則は `semantic` 内に順序を明示し、JSON 読み取りや SVG 生成から分離します。詳細は [モデル境界](docs/models.md) を参照してください。

plan の依存参照は `TerraformReference { from, to }` で保持し、意味変換の入口で architecture の `Edge { from, to, kind, change }` に変換します。`EdgeKind` は `Dependency` / `Association` / `Connection` / `Containment` を区別します。関連付けを辺へ変換する際は、元のアドレスと変更種別を `change` に残します。同じ端点を持つ別の関連付けも個別に保持します。

関連付けの変更種別は辺と矢印の色、辺のタイトル、凡例の件数に反映します。意味付き辺を含む SVG はリソース数とカード数を区別し、辺を `relationships` と表示します。各辺のタイトルで関係の種類を確認できます。通常の依存辺だけの場合は従来の説明・表示を維持します。

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

`tests/unit/` はパーサー内部・参照解決・配置や経路・描画ヘルパーを検証し、対象モジュール内にコンパイルします。`tests/integration/` は実際の CLI バイナリを起動して入力 JSON から SVG までの動作を検証し、内部モジュールを読み込みません。共通の入力データは `tests/fixtures/` や既存の `examples/` を参照し、複製しません。`tests/support/layout_metrics.rs` は単体テスト専用の計測ヘルパーです。

Cargo.toml の `autotests = false` と `[[test]]` で 3 つの結合テストを明示的に登録しています。結合テストのクレートを増やすときは登録も追加してください。`cargo test --locked` は単体・結合の両方を実行します。CLI の結合テストだけなら `docker compose run --rm dev cargo test --locked --test cli`、単体テストだけなら `docker compose run --rm dev cargo test --locked --bin planorama` を使えます。

```text
tests/
├── integration/          # 公開 CLI 経由の結合テスト
│   ├── cli.rs            # 引数・標準入出力・エラー
│   ├── terraform_plan.rs # 実際の plan の依存関係
│   ├── rendering.rs      # JSON → SVG の回帰確認
│   └── support.rs        # CLI 起動ヘルパー
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

単体テストは実装側の `#[cfg(test)]` と `#[path = "..."]` で読み込みます。テストのために内部関数を公開する必要はなく、実装ファイルには読み込み宣言だけを置きます。実行方法はこれまでどおり `docker compose exec dev cargo test --locked` です。

Multi-subnet ALBs (`subnets`) and ECS Services (`network_configuration.subnets`) use one physical containment parent: a single resolved subnet or the nearest unambiguous common container ancestor. Multiple subnet references remain visible Connection edges. Unresolved references, unrelated resource types, missing ancestry, ambiguous parents, and cycles do not invent a parent. Nested ECS subnet expressions are analyzed separately from security-group fields; flattened block references cannot establish membership. Data resources remain external, while a known data VPC can contain managed resources. See [the multi-container regression diagram](examples/multi-container.svg).
AWS classification covers the large Terraform example and selected networking, load-balancing, compute, database, storage, and API resources. VPC and Subnet are the only spatial containers. Gateways, routes, load balancers, and API entry points are Connectors; orchestration and target-group resources are Controllers; IAM and listener rules are Policies. Attachment resources are Associations; only the explicitly supported relationship rules above lower them to edges. Other associations remain visible cards. Data entities use the same type classifier. Unknown and unlisted types retain the Unknown fallback. These roles describe visualization behavior, not a claim that logical ownership implies physical containment.
All rendered nodes expose their actual geometry through `Layout::bounds`. Container membership stores indices only; SVG rendering and both routing paths consume the same bounds. Container ports span the expanded outer frame, while header bounds remain available to protect labels when routing through an ancestor. Ordinary-card bundling and scoring also use bounds; `positions` remains the placement-coordinate view.

Container routing compares obstacle-free shortest paths with occupancy-aware alternatives. Shared segment coverage and crossing costs discourage congested corridors; arrival direction adds a bend penalty. Edges are routed in structural order for deterministic results, including when input nodes/edges are reordered. Routes never cross unrelated container frames or card/header interiors; ancestor traversal and nesting-only edge suppression retain their existing semantics.

Inside each architecture container, non-containment relationships place direct children in columns from left to right. Relationships between descendants influence the ranks of their containing siblings. Cycles share a column, and children in the same column remain vertically separated. Nested containers are sized before their parents, using the final child bounds. Module grouping does not affect membership or placement. Dense ranks wrap into adjacent columns using an internal aspect-ratio heuristic; a virtual root applies the same packing to top-level resources without adding a rendered node. Rank groups remain ordered and 40px corridors separate siblings. See examples/dense-architecture.svg for six sibling subnets and six independent root resources. Multi-container Connection relationships keep related sibling containers consecutive during wrapping and bias their shared resource toward the median of their centers. This adjustment stays inside the packed envelope and preserves rank, containment, and sibling clearances. Compatible connections can share a clear gutter between source headers and the target boundary; blocked or more costly bundles fall back to independent routing. Each relationship retains its own SVG path, title, and metadata. See [the three-subnet ALB example](examples/spanning-dense.svg).

Resource labels default to full Planorama-qualified addresses (`module.root.aws_vpc.main` for root resources). Use `--address-format terraform` for the original Terraform-native spelling, or `--address-format qualified` explicitly. Child module paths and instance indices are preserved in both modes. Labels may wrap or truncate to fit, but each card's title contains the complete selected address; `data-terraform-address` and `data-qualified-address` retain both forms. This display option does not change reference resolution or layout.

Resource addresses display 24px official AWS service icons for mapped AWS resource types. Icons are shared SVG symbols embedded in the document, with no external assets or runtime downloads. Unmapped types render without icons; address format, graph identity, and ResourceRole remain independent. Artwork source, version, and export licensing are documented in [Resource icons](docs/icons.md).

実際の Terraform 1.9.8 / AWS provider 5.100.0 の `terraform show -json` 出力を使った追加の互換性 fixture は [aws-captured](tests/fixtures/aws-captured/README.md) にあります。ネットワークを無効にして取得した plan で、通常のテストに Terraform や AWS 認証情報は不要です。

ノードの `ProviderIdentity` は `provider_name` を優先し、ない場合は configuration の `provider_config_key` と `provider_config.full_name` から解決します。`hashicorp/aws` のような2要素の source は `registry.terraform.io/` を補います。明示情報と型名による推定は区別して保持し、情報がない最小入力では `aws_` 型だけを従来どおり AWS と推定します。明示された別 provider や解決できない provider binding は AWS として扱いません。分類・アイコン・AWS 固有の意味変換はこの識別情報を参照します。

`--diagnostics` を指定すると、参照解決と意味変換の保守的なフォールバック理由を stderr に出力します。SVG と通常の終了条件は変わりません。

```sh
planorama plan.json -o diagram.svg --diagnostics
```

診断には Terraform アドレス、属性名、未解決参照・複数インスタンス候補・動的選択・alias/local/module の循環・期待属性の欠落・型不一致・包含親の曖昧さなどの理由を含みます。属性値や式の内容は出力しません。正常に解決した複数 Subnet への接続は曖昧さとして報告しません。定数 ID や Terraform JSON に定義が含まれない local など、対応するリソースを安全に特定できない場合にも理由を表示します。診断は入力の不正や Terraform 自体のエラーを意味するとは限りません。オプションを省略すると診断は出力しません。

Network containment also recognizes `aws_nat_gateway.subnet_id` and the `vpc_id` of VPC endpoints, route tables, security groups, and network ACLs. Only complete, static, single AWS endpoints produce containment; constants, tags, data queries, ambiguous and dynamic references remain ordinary relationships. See [network scope example](examples/network-scope.svg).

RDS instances and clusters inherit network placement through a statically resolved `db_subnet_group_name` and the group's `subnet_ids`. One subnet places the workload within that subnet; multiple subnets use their nearest common parent. Unresolved, dynamic, literal or incompatible references leave placement unchanged. The subnet-group card and dependencies are preserved. See [RDS subnet-group example](examples/rds-subnet-group.svg).

DB and ElastiCache subnet-group cards use their referenced subnets' common VPC, including when only one subnet is referenced. Mixed or missing VPC ancestry leaves the group outside containers. Original subnet dependencies and the reference chain supporting the derived placement are retained.

Lambda and EKS (`vpc_config.subnet_ids`) and OpenSearch (`vpc_options.subnet_ids`) use the same common-VPC placement. Only complete, static AWS subnet references qualify; flattened block references and sibling fields do not establish placement. These cards retain their subnet dependencies and show the inferred-placement note. See [VPC workloads](examples/vpc-workloads.svg).

Resource-change identity is the pair of the canonical Terraform address and optional Terraform deposed key. Current and deposed objects remain separate cards with their own actions; SVG records the key in data-deposed-key without rewriting the primary address. Configuration references resolve only to current objects, and current configuration is not applied to deposed predecessors. Planned values fill the current object only. See tests/fixtures/deposed-plan.json.

Nested configuration blocks expose dotted attribute paths generically, including network_configuration.security_groups and default_action.forward.target_group.arn. Repeated blocks merge references at the same path in deterministic order. A path is complete only when every repeated block supplies complete reference provenance; missing, literal or malformed entries prevent semantic inference. Existing top-level aggregate references remain available. Literal values and expression metadata are not traversed as attribute paths.

Moved-resource provenance is stored in TerraformEntity.previous_address and projected to Node.previous_address only when Terraform reports previous_address. TerraformEntity.address remains the current canonical input identity, and old addresses do not become dependency aliases. Relationship lowering copies the provenance into EdgeChange.previous_address. SVG cards and lowered relationships expose escaped data-previous-address metadata; ordinary resources and action classification are unchanged. This metadata can be inspected without retaining or reparsing raw plan JSON.

Layer responsibilities and dependency rules: [Architecture contracts](docs/architecture.md).

Layout stages, shared routing primitives and attachment policy: [Routing](docs/routing.md).
