# 文書案内

変更に関連する正本と参照元・参照先・生成元を確認します。全文書の監査、記述言語、開発・ブランチ運用の方針は [AGENTS.md](../AGENTS.md) を参照してください。

| 対象 | 正本 |
| --- | --- |
| CLI・開発 | [利用手順](../README.md)、[設定](configuration.md)、[選択](filtering.md)、[開発・検証・生成物更新](../CONTRIBUTING.md) |
| CI・公開 | [判定・復旧](ci.md)、[計測履歴](ci-history.md)、[build performance](build-performance.md)、[CLI release](releases.md)、[Wasm release](browser-engine.md#standalone-engine-without-artwork) |
| AWS | [共通・サービス別契約](aws.md)、[監査方法・現行判断](aws-semantic-audit.md)、[監査履歴](aws-semantic-history.md)、[inventory](aws-inventory.md) |
| ブラウザ | [利用・共通 API・privacy](browser.md)、[engine・分割実行・memory](browser-engine.md)、[開発検証](browser-development.md)、[生成後の HTML 操作](viewer.md) |
| Terraform 入力 | [互換性](terraform-compatibility.md)、[provider evidence](provider-evidence.md)、[複数 state](multi-plan.md)、[outputs](outputs.md) |
| 内部モデル・表示 | [architecture](architecture.md)、[models](models.md)、[resource presentation](resource-presentation.md)、[relationships](relationships.md)、[routing](routing.md) |
| 品質・計測 | [Layout quality](ci.md#layout-quality-comparison-390)、[diagnostics](layout-diagnostics.md)、[presentation](presentation-quality.md)。[stability](layout-stability.md)・[benchmarks](benchmarks.md) は別の計測であり gate の代用にしない |

## 生成文書

生成結果は手動修正せず、生成元を更新します。基本分類・schema 候補は専用関係や完全対応の証拠ではありません。サービス契約を変更したら一覧・判断データへの影響も確認してください。

| 生成結果 | 生成元・更新方法 |
| --- | --- |
| [resource-support.md](resource-support.md) | provider 実装、[説明文](../tests/unit/resource_support_intro.txt)、[収集処理](../tests/unit/resource_support.rs)。`sh scripts/update-resource-support.sh` / `--check` |
| [aws-semantic-gaps.md](aws-semantic-gaps.md) | [判断データ](aws-semantic-decisions.json)、[schema evidence](aws-semantic-evidence.json)、resource support。[更新手順](aws-semantic-audit.md#maintenance-and-verification) |
| [coverage](../examples/coverage.md)・[selection coverage](../examples/selection-coverage.md) | CLI と fixture。[生成手順](../examples/terraform-large/README.md) |

## 文書整理 #963 の判断

| 候補 | 採否・保存内容 |
| --- | --- |
| 全文書参照・言語 | 関連文書確認を基本とし、全体監査は必要時に目的を明示。Issue・PR・報告は日本語を基本とし、原文の識別子等と利用者指定を尊重 |
| CI・監査の履歴分離 | 採用。現行判断・復旧・baseline 理由を入口に残し、比較条件・対象 commit・証拠・未対応範囲・再検討条件は履歴へ |
| AWS・Browser の分割 | AWS は共通契約と6サービス群、Browser は利用/API・組み込み・開発検証へ。UI と API の共通 options は重複定義しない |
| 重複本文の集約 | 監査の固定版・検証・参照先・証拠条件、CI の cache/dispatch、Browser の lifecycle を共通化。AWS の表示条件・30例の HTML 生成手順を短文化し、サービス固有の型・値・scope・lifecycle・例外を保持 |
| 証拠・旧リンク・生成表の削除 | 不採用。既存見出しはリンク付き見出しで維持。証拠本文を外部リンクだけにせず、生成表は元の正本・検証を維持。重複する目次と作業説明は削減 |

### 規模と検証

比較元は作業開始時 `6289535` と初回 PR `b7f9bad`。文字数は Unicode 文字数から CR/LF だけを除き、目次・コード・リンクも含めます。改行位置による削減は数えません。各行は分割先込みで、文書案内も合計へ加算します。

| 文書群 | 作業開始前 | 初回 PR | 最終 |
| --- | ---: | ---: | ---: |
| AWS | 161,362 | 174,050 | 160,460 |
| CI | 61,204 | 63,184 | 54,882 |
| Browser | 42,092 | 43,755 | 38,460 |
| Audit | 96,707 | 101,318 | 84,944 |
| Guide | 0 | 4,961 | 3,306 |
| 合計 | 361,365 | 387,268 | 342,052 |

56文書の内部リンク1,379件・旧見出し・外部参照の保持と変更段落を確認しました。Docker 内で `sh scripts/update-resource-support.sh --check`、`node scripts/aws-semantic-audit.cjs --check`、audit/inventory の既存テストを実行し、計32件成功。`git diff --check` も成功しています。Markdown のみの変更なので SVG 再生成・Layout quality は対象外です。

CI の比較 PR #408・#410・#411・#675・#886、run `38224372411`・`38225451104`、旧文書 commit `8fb4aef052f7d941b0a5da6b08a4fc610661a504`、baseline 判断の PR #661・#671・#672・#663 は2026-10-29に取得確認済みです。判断条件はリンク先の保存期限に依存せず本文にも残しています。
