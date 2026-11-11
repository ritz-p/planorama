# WSL 2 / Ubuntu 26.04 LTS での開発

Docker を起動せずに開発・通常検証・生成する手順です。CI と Docker の検証経路は維持します。ツールの版は `rust-toolchain.toml`、`Dockerfile`、`Cargo.lock` を正本とします。

## Windows 側の準備

通常の PowerShell で `wsl --list --verbose` と `wsl --list --online` を確認します。対象は `Ubuntu-26.04`、WSL の VERSION は `2` です。`docker-desktop` は開発用 Ubuntu として使いません。

登録名が `Ubuntu` の場合は以下の `-d Ubuntu-26.04` を `-d Ubuntu` に読み替えます。起動後に `cat /etc/os-release` で `VERSION_ID="26.04"` を確認してください。

未導入の場合は以下を実行します。WSL 自体の初回導入・Windows 機能の有効化には **管理者として起動した PowerShell** が必要です。既に WSL が有効なら Ubuntu の追加は通常の PowerShell から実行し、昇格を要求された場合だけ管理者権限を使います。再起動を要求されたら Windows を再起動してください。

```powershell
wsl --install -d Ubuntu-26.04
```

通常の PowerShell で起動し、初回に Linux のユーザー名とパスワードを作ります。

```powershell
wsl --list --verbose
wsl -d Ubuntu-26.04
```

以下の `sh` コマンドはすべて Ubuntu の端末で実行します。日常の開発に Windows 管理者権限は不要です。`sudo` は Ubuntu 内のパッケージ導入用で、Cargo を root として実行しません。導入の詳細は [Microsoft WSL](https://learn.microsoft.com/en-us/windows/wsl/install) と [Ubuntu の公式手順](https://ubuntu.com/wsl/docs/latest/howto/install-ubuntu-wsl2/) を参照してください。

## checkout とビルド出力

```sh
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev ca-certificates curl git unzip jq time nodejs librsvg2-bin fonts-dejavu-core
mkdir -p "$HOME/src"
cd "$HOME/src"
git clone https://github.com/ritz-p/planorama.git
cd planorama
git config core.autocrlf false
export CARGO_TARGET_DIR="$PWD/target"
mkdir -p "$CARGO_TARGET_DIR"
findmnt -T "$CARGO_TARGET_DIR"
```

checkout は `~/src/planorama`、Rust の出力はその `target/` に置きます。`/mnt/c`・`/mnt/d` など Windows 側の checkout やビルド出力を使わず、Windows / Docker の target をコピー・共有しません。既存変更は元の checkout に残し、必要なブランチを push / fetch して移してください。Git の認証も WSL 側で準備します。

新しい端末でもリポジトリのルートで `export CARGO_TARGET_DIR="$PWD/target"` を実行します。リポジトリごとに設定し、別リポジトリの target を使う値を `.bashrc` に固定しません。`CARGO_HOME`・`RUSTUP_HOME` も Windows / Docker の値を引き継がず、通常は WSL 内の `~/.cargo`・`~/.rustup` を使います。

## Rust とブラウザ

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/planorama-rustup.sh
sh /tmp/planorama-rustup.sh -y --profile minimal --default-toolchain none
. "$HOME/.cargo/env"
rustup toolchain install 1.85.1 --profile minimal --component rustfmt,clippy
rustup show
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.100 --locked
command -v cargo rustc node
cargo metadata --locked --no-deps --format-version 1 | jq -r .target_directory
```

Rust の版は checkout の `rust-toolchain.toml` と揃えます。`rustup show` で使用中のツールチェーンを確認します。`wasm-bindgen-cli` は `Dockerfile` と同じ版です。`command -v` が Windows の実行ファイルを指していないこと、metadata が `~/src/planorama/target` の絶対パスを返すことを確認します。mold / sccache は高速化用で通常検証の必須条件ではありません。Docker 専用の linker / wrapper 環境変数は WSL に持ち込まないでください。

ブラウザテストは `/usr/bin/chromedriver` を起動します。Ubuntu のパッケージは Debian の Docker イメージと異なり [Chromium snap への移行パッケージ](https://packages.ubuntu.com/resolute/chromium-chromedriver) です。

```sh
systemctl status snapd.socket
sudo apt install -y chromium-browser chromium-chromedriver
chromium-browser --version
/usr/bin/chromedriver --version
```

systemd / snapd が動作しない場合は [WSL の systemd 設定](https://learn.microsoft.com/en-us/windows/wsl/systemd) を確認します。`/etc/wsl.conf` の既存設定を保持して `[boot]` の `systemd=true` を設定し、通常の PowerShell で `wsl --terminate Ubuntu-26.04` を実行して再起動します。終了前に Ubuntu 内の作業を保存してください。snap の導入が停止した場合は `snap changes` / `snap tasks <ID>` で状況を確認します。中止する場合は自分が開始した操作の ID を確認して `sudo snap abort <ID>` を使い、APT が終了してから `sudo dpkg --configure -a` と通常依存の導入を再実行します。ブラウザが導入・起動できない場合はテスト成功とは扱わず、対象 SHA とエラーを報告します。

## ビルド・検証・生成

リポジトリのルートで実行します。

```sh
cargo build --locked
mkdir -p output
cargo run --locked -- examples/plan.json -o output/diagram.svg
cargo fmt --all --check
cargo test --locked --all-targets --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
node --test tests/ci/*.test.cjs
node tests/browser/viewer.cjs
node --test tests/browser/presentation-quality.test.cjs
node --test tests/browser/webdriver.test.cjs tests/support/tooling.test.cjs
```

SVG / HTML の変更時には、生成元と出力の差分・表示を確認します。

```sh
sh scripts/update-examples.sh
cargo run --locked -- tests/fixtures/viewer-plan.json --format html -o examples/viewer.html
```

Wasm の開発・ブラウザ検証:

```sh
cargo build --locked --release --bin planorama
sh scripts/build-wasm.sh
node tests/browser/wasm.cjs
node scripts/serve-browser.cjs
```

Windows のブラウザで `http://localhost:8080` を開き、サーバーは Ctrl+C で停止します。Node.js やブラウザ・フォントの版は Docker と異なるため、描画差分は確認し、環境差だけを理由に baseline を更新しません。

[CONTRIBUTING](../CONTRIBUTING.md) の `docker compose exec -T dev …` は、準備済みの WSL では接頭辞を外して実行できます。ただし Docker build/run、Compose の別サービス、CPU / メモリ制限を使う計測は単純に置き換えません。Terraform fixture の再生成には `Dockerfile` と同じ Terraform 1.9.8 と対象 Provider が別途必要です。通常テストはコミット済み JSON を使います。完全な CI 計測（`scripts/wasm-ci.sh` など）や固定環境の比較は既存 Docker 手順を使用します。配置・配線変更時の Layout quality を含む CI 要件は変わりません。
