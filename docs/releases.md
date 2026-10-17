# ネイティブ CLI の配布

現時点で公開済みのリリースはありません。以下は初回公開後の配布仕様です。
[GitHub Releases](https://github.com/ritz-p/planorama/releases) に対象タグの assets が
あることを確認してください。CI の artifact は検証用で、公開リリースではありません。

## 対応環境

| OS / architecture | archive の target 部分 | 検証 runner / 実行環境 |
| --- | --- | --- |
| Linux x86_64 | `x86_64-unknown-linux-gnu` | Ubuntu 22.04、glibc 2.35 以上、system libgcc |
| Windows x86_64 | `x86_64-pc-windows-msvc` | Windows Server 2022、Windows 10 以降向け |
| macOS arm64 | `aarch64-apple-darwin` | macOS 15 以上 |
| macOS x86_64 | `x86_64-apple-darwin` | macOS 15 以上 |

macOS の両 architecture はそれぞれ native runner で検証します。
[GitHub runner 対応表](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
に合わせて `macos-15` / `macos-15-intel` を使用します。
Alpine/musl、Linux arm64、Windows arm64、32-bit 向けは提供しません。
バイナリの署名・macOS notarization は行いません。

## ダウンロードと実行

ファイル名は `planorama-v<VERSION>-<TARGET>.tar.gz` です。
同じ release から archive と `SHA256SUMS` を取得します。
チェックサムは破損検出用で、署名による真正性保証ではありません。
以下の `0.1.0` は初回公開予定バージョンの例であり、公開済みという意味ではありません。

Linux:

```sh
version=0.1.0
target=x86_64-unknown-linux-gnu
archive="planorama-v${version}-${target}.tar.gz"
base="https://github.com/ritz-p/planorama/releases/download/v${version}"
curl -fLO "$base/$archive"
curl -fLO "$base/SHA256SUMS"
grep "  $archive\$" SHA256SUMS | sha256sum -c -
tar -xzf "$archive"
cd "planorama-v${version}-${target}"
./planorama --version
./planorama --help
```

macOS は上記の `target` を Apple Silicon では `aarch64-apple-darwin`、
Intel では `x86_64-apple-darwin` にし、チェックサムの行を次に置き換えます。
検証失敗時は展開・実行せず、再取得してください。

```sh
grep "  $archive\$" SHA256SUMS | shasum -a 256 -c -
```

Windows PowerShell（OS 付属の `tar.exe` を使用）:

```powershell
$version = '0.1.0'
$stem = "planorama-v$version-x86_64-pc-windows-msvc"
$archive = "$stem.tar.gz"
$base = "https://github.com/ritz-p/planorama/releases/download/v$version"
Invoke-WebRequest "$base/$archive" -OutFile $archive
Invoke-WebRequest "$base/SHA256SUMS" -OutFile SHA256SUMS
$lines = @(Get-Content SHA256SUMS | Where-Object { $_.EndsWith("  $archive") })
if ($lines.Count -ne 1) { throw 'Missing or duplicate checksum' }
$expected = ($lines[0] -split '  ')[0]
if ((Get-FileHash $archive -Algorithm SHA256).Hash -ine $expected) { throw 'Checksum mismatch' }
tar -xzf $archive
if ($LASTEXITCODE -ne 0) { throw 'Extraction failed' }
Set-Location $stem
.\planorama.exe --version
.\planorama.exe --help
```

archive には本体、`LICENSE`・`LICENSE-MIT`・`LICENSE-APACHE`、依存クレートと Rust runtime のライセンス・通知、
アイコンの通知、`README.txt` が含まれます。実行には Rust・Docker・Node.js は不要です。
本体は MIT または Apache-2.0 を選択できるデュアルライセンスです。第三者素材には各通知が適用されます。
`pratt 0.4.0` は crate にライセンスファイルがないため、manifest の MIT 宣言・著者と
[同梱補足](../third-party/pratt-0.4.0.txt) を使用します。未確認の欠落は packaging を失敗させます。
既存の plan JSON からの生成には Terraform も不要です。provider evidence の
追加要件は [入力と account の検証](provider-evidence.md) を参照してください。

## 開発者向け公開手順

`Native release` は Rust 1.85.1、コミット済み `Cargo.lock`、`--locked` でビルドします。
固定 toolchain/target を使った再ビルド手順を提供しますが、runner image、system linker、
archive の時刻まで固定した byte-for-byte reproducible build は保証しません。
各 native runner で archive を展開し、元ファイルとの一致、`--version` / `--help`、
小規模 fixture の SVG/HTML の構造を検証します。その後全4対象の archive/checksum を照合します。
通常 PR と手動 dispatch は検証のみで公開しません。

公開を依頼された場合は以下を順に行います。

1. 配布変更を main へマージし、公開予定のコミットを決めます。
2. `Cargo.toml` の version と `vMAJOR.MINOR.PATCH` タグの一致を確認します。
   初期は stable version のみを扱い、pre-release tag は拒否します。
3. 公開予定の同じコミットで Native release の手動実行と通常テストを確認します。
   [Layout quality](ci.md#layout-quality-comparison-390) も同じ SHA の成功を確認し、
   なければ対象 ref で手動実行して checkout SHA を照合します。
4. 公開を依頼された場合に限り、そのコミットに version tag を作成・push します。
5. tag の全 target build と集約検証が成功すると、draft release を作成し、
   archive と `SHA256SUMS` をアップロードします。API の SHA-256 digest・size と
   asset 一覧を確認してから公開します。job ごとの権限は build/verify が read、publish のみ write です。
6. 公開 assets を取得し、チェックサムと案内を確認します。初回公開後は README と
   この文書の「未公開」表記を実際の公開状況に合わせて更新します。

一部 target の失敗は集約・公開を停止します。version mismatch はビルド前に拒否します。
同一 ref の実行は直列化し、既存 release（draft を含む）がある場合は変更せず失敗します。
公開済み assets を上書きせず、修正配布は新バージョンにします。
アップロード途中の失敗は draft に留まります。復旧時は人が draft のタグ・assets・
失敗理由を確認し、未公開 draft を削除してから同じタグの run 全体を再実行します。
公開完了後の再実行も既存 release を検出して停止するため、正常な既存配布に影響しません。

公開制御のテストは `node --test tests/ci/release.test.cjs`、Linux のパッケージ確認は
Docker 内で `cargo build --release --locked --target x86_64-unknown-linux-gnu` の後、
`node .github/scripts/release.cjs x86_64-unknown-linux-gnu` を実行します。
