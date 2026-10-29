# 文書案内

作業に関連する正本を選び、その参照先・参照元と実装・生成元を確認してください。関連する仕様変更は同じ PR で反映します。全文書の監査は横断的な変更や明示的な依頼で必要な場合に行います。開発・言語・ブランチのルールは [AGENTS.md](../AGENTS.md) が正本です。

| 確認したいこと | 正本と参照範囲 |
| --- | --- |
| CLI の導入・生成 | [README](../README.md)、[設定](configuration.md)、[選択](filtering.md) |
| 開発環境・ローカル検証・生成物更新 | [CONTRIBUTING](../CONTRIBUTING.md) |
| CI の起動条件・判定・復旧 | [CI](ci.md)。性能比較・採否の過去記録は [CI 履歴](ci-history.md)、linker/compiler cache は [build performance](build-performance.md) |
| 公開 | [ネイティブ release](releases.md)、[Wasm engine](browser-engine.md#standalone-engine-without-artwork) |
| AWS 共通契約とサービス固有条件 | [AWS](aws.md) から該当サービスへ。分類・包含・scope とサービス固有の証拠条件は別に確認する |
| AWS の対応範囲・監査 | [監査方法と現行判断](aws-semantic-audit.md)、[履歴](aws-semantic-history.md)、[inventory](aws-inventory.md)。履歴の対象 commit と現行の判断データを区別する |
| ブラウザ利用・共通 API | [Browser](browser.md)。分割実行・memory・配布は [engine integration](browser-engine.md)、build・検証は [development](browser-development.md) |
| 生成後の HTML 操作・privacy | [Viewer](viewer.md)。生成 UI の lifecycle は [Browser](browser.md#privacy-and-lifecycle) |
| Terraform 入力・scope 証拠 | [互換性](terraform-compatibility.md)、[provider evidence](provider-evidence.md)、[複数 state](multi-plan.md)、[outputs](outputs.md) |
| 内部モデル・表示・配線 | [architecture](architecture.md)、[models](models.md)、[resource presentation](resource-presentation.md)、[relationships](relationships.md)、[routing](routing.md) |
| 品質判定・計測 | [Layout quality](ci.md#layout-quality-comparison-390)、[diagnostics](layout-diagnostics.md)、[presentation](presentation-quality.md)。[stability](layout-stability.md) と [benchmarks](benchmarks.md) は別の計測であり gate の代用にしない |

## 生成文書

生成結果を手動修正せず、生成元を更新します。図の入力・生成処理も変更する場合は [SVG 更新ルール](../AGENTS.md#生成-svg-の更新) に従います。

| 生成結果 | 生成元・更新方法 |
| --- | --- |
| [resource-support.md](resource-support.md) | provider 実装、[説明文](../tests/unit/resource_support_intro.txt)、[収集・検証](../tests/unit/resource_support.rs)。`sh scripts/update-resource-support.sh` / `--check` |
| [aws-semantic-gaps.md](aws-semantic-gaps.md) | [判断データ](aws-semantic-decisions.json)、[固定 schema evidence](aws-semantic-evidence.json)、resource support。[監査更新手順](aws-semantic-audit.md#maintenance-and-verification) |
| [coverage 例](../examples/coverage.md)、[selection coverage 例](../examples/selection-coverage.md) | CLI と fixture。[大型例の生成手順](../examples/terraform-large/README.md) |

サービス契約を変更したら、対応一覧・判断データへの影響も確認します。生成表の基本分類や schema 候補を、そのまま専用関係や完全対応の証拠にしないでください。

## 文書整理 #963 の判断

| 整理候補 | 採否・理由と保存先 |
| --- | --- |
| 毎回の全文書参照 | 関連する正本・参照関係・生成元の確認へ変更。全体監査は目的を明示して必要時に実施し、仕様に影響する関連文書の更新は維持する |
| Issue・PR・報告の言語 | 日本語を基本とする。コード・識別子・固有名詞・引用は原文、利用者の明示指定を優先する |
| CI の現行運用と過去計測の混在 | [履歴](ci-history.md)へ分離。CI 本文に判断の結論・再検討条件を残し、復旧・権限・SHA 検証・baseline の判断理由は現行文書に保存する |
| AWS を単一文書で管理 | 共通契約と6サービス群へ分割。Deployment scopes に混在していた EFS/EBS/SSM/Secrets/KMS/ACM/Route 53 等も専用節へ移す。サービス固有の Provider 版・known-value・scope・lifecycle・除外条件は省略しない |
| 完了済み監査の削除 | 不採用。対象 commit、公式契約、判断理由、未対応範囲、follow-up、再検討条件を [履歴](aws-semantic-history.md)に保存。現行判断・方法・更新手順を入口に残す |
| ブラウザ文書の読者別整理 | 利用・共通 API、engine 組み込み、開発検証に分割。UI と API が同じ options・named-state 契約を使う部分は重複を避けて共有し、読者別入口と UI 設定見出しで区別する。privacy・失敗時の Worker 破棄・memory 制限は維持する |
| 既存見出しの削除 | 不採用。旧 URL は案内見出しとして維持し、内部リンクは正本へ更新する。文面の「上記・下記」が別文書を指す箇所は明示リンクに置換する |
| 過去記録を外部リンクだけにする | 不採用。Actions artifact の期限や外部削除に依存せず読めるよう、判断と比較条件をリポジトリ内に保存する。既存の PR・固定 commit・実行リンクも維持する |
| 自動生成表の再編集 | 不採用。正本のファイル名と参照先見出しを維持するため生成元・生成結果は変更不要。既存検証で一致を確認する |

今回の棚卸しでは既存 Markdown 全体の役割と参照関係、生成元を確認しました。仕様の全文翻訳、監査の再実施、CI・描画挙動の変更は含みません。

### 規模と保存した情報

比較元は `6289535`。末尾の空行を除く行数で比較しています。「分割先込み」は元の入口と今回追加した対応文書の合計で、旧見出し案内・目次も含みます。

| 元文書 | 変更前 | 変更後の入口 | 分割先込み |
| --- | ---: | ---: | ---: |
| `aws.md` | 2,360 | 511 | 2,715 |
| `ci.md` | 930 | 814 | 970 |
| `browser.md` | 689 | 362 | 726 |
| `aws-semantic-audit.md` | 1,350 | 154 | 1,433 |

全体の行数は目次・互換リンクにより増えています。本文を短縮して証拠を失うより、必要な正本だけを選べる構成を採用しました。元の本文を段落単位で比較し、リンク表現の変更と UI 設定見出しの挿入以外に欠落がないことを確認しています。CI の baseline 判断表、監査の証拠・保留理由、AWS の値証明・型・scope・lifecycle 条件、ブラウザの privacy・memory・failure 契約を保存しました。

2026-10-29 に、CI 履歴の PR #408・#410・#411・#675・#886、比較 run `38224372411`・`38225451104`、固定 commit `8fb4aef052f7d941b0a5da6b08a4fc610661a504` の旧 CI 文書を GitHub 経由で取得できることを確認しました。現行 CI に残す baseline 判断の PR #661・#671・#672・#663 も取得確認済みです。将来リンク先が利用できなくても判断条件を読めるよう、本文をこのリポジトリに残しています。

### 検証範囲

Docker 内の一時 JavaScript 検証で Markdown 56文書の内部リンク1,519件のファイル・見出しの存在を確認し、欠落はありませんでした。変更前4文書の見出しと157件の外部参照を保持しています。本文比較はリンク先と空白を正規化して段落ごとに照合し、参照表現を変更した5段落と UI 見出しを挿入した1段落を目視確認しました。

既存検証は `sh scripts/update-resource-support.sh --check`（1テスト成功）、`node scripts/aws-semantic-audit.cjs --check` と `node --test tests/support/aws-semantic-audit.test.cjs tests/support/aws-inventory.test.cjs tests/support/aws-inventory-diff.test.cjs`（31テスト成功）を実行しました。`git diff --check` も成功しています。Markdown だけの変更なので SVG 再生成・Layout quality は対象外です。

resource support 検証の初回は Docker volume の容量不足で停止しました。共有 cache を削除せず、十分な容量のある bind mount 内の `output/` へ `CARGO_TARGET_DIR`・`TMPDIR` を変更し、`CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0` で再実行して成功しました。これは検証時の設定で、Docker・CI の構成は変更していません。
