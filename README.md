# OSPF Network Simulator

WebAssemblyとRustで構築されたOSPFv2ネットワークシミュレーターです。ブラウザ上でルーターの設置、接続、OSPFプロトコルの動作をリアルタイムで可視化できます。

## 機能

- 仮想ルーターの設置と削除
- ルーター間の接続と切断
- OSPFv2プロトコルのリアルタイムシミュレーション
- パケット送受信の可視化アニメーション
- SPFアルゴリズムによるルーティングテーブル計算
- 時間経過によるネットワーク状態の変化をシミュレート
- シミュレーションログの記録とエクスポート

## 必要環境

### 実行環境
- Docker および Docker Compose
- WebAssembly対応のモダンブラウザ（Chrome、Firefox、Edge）

### 開発環境（ソースからビルドする場合）

#### 必須ツール
以下のツールが事前にインストールされている必要があります：

1. **Rust** (stable版)
   - Rustコンパイラとcargoパッケージマネージャー
   - バージョン: 1.70以上推奨

2. **wasm-pack**
   - RustコードをWebAssemblyにコンパイルするツール
   - バージョン: 最新版

3. **Node.js および Yarn**
   - フロントエンドビルドとwebpackの実行
   - バージョン: Node.js v16以上、Yarn v1以上

4. **C/C++コンパイラ**
   - build-essentialパッケージ（gcc、g++、make等）
   - Rust/WebAssemblyのネイティブコンパイルに必要

5. **Git**
   - ソースコードの取得とバージョン管理
   
6. **Docker および Docker Compose**（コンテナ実行の場合）
   - Docker Engine v20以上
   - Docker Compose v2以上

#### 追加の依存関係（自動インストール）
以下はセットアップスクリプトで自動的にインストールされます：
- pkg-config
- libssl-dev（OpenSSL開発ライブラリ）
- curl（インストーラのダウンロード用）

## WSL2 Ubuntu 24.04 セットアップ

WSL2のUbuntu 24.04環境で開発環境を構築する場合は、付属のセットアップスクリプトを使用できます：

```bash
# セットアップスクリプトの実行
./setup-wsl2-ubuntu.sh
```

このスクリプトは以下を自動的にインストール・設定します：
- システムパッケージの更新
- build-essential（gcc、g++、make）
- Rust（最新stable版）
- wasm-pack
- Node.js（LTS版）とYarn
- Docker
- プロジェクトのビルド（オプション）

## クイックスタート

### 1. リポジトリのクローン
```bash
git clone <repository-url>
cd nw_simulator
```

### 2. Dockerコンテナの起動

```bash
make run
```

または直接実行：
```bash
docker-compose up --build
```

### 3. ブラウザでアクセス
```
http://localhost:8080
```

## 主要なMakeコマンド

```bash
make build    # Dockerイメージのビルド
make run      # コンテナの起動（本番モード）
make dev      # 開発モードで起動（ホットリロード対応）
make stop     # コンテナの停止
make clean    # ビルドアーティファクトのクリーンアップ
make logs     # コンテナログの表示
```

## 使用方法

1. **ルーターの追加**: 「Add Router」ボタンをクリック後、キャンバス上をクリックして配置
2. **ルーターの削除**: 「Delete Router」ボタンをクリック後、削除したいルーターを選択
3. **ルーターの接続**: 「Connect Routers」をクリック後、接続したい2つのルーターを選択
4. **接続の解除**: 「Disconnect Routers」をクリック後、切断したい2つのルーターを選択
5. **OSPFの有効化**: 各ルーターの「Enable OSPF」ボタンをクリック
6. **シミュレーション開始**: 「Start Simulation」ボタンをクリック
7. **ルーティングテーブル確認**: ドロップダウンからルーターを選択

## トラブルシューティング

### ビルドエラー：「linker `cc` not found」

**症状**: wasm-packビルド時に「error: linker `cc` not found」エラーが発生

**原因**: C/C++コンパイラがインストールされていない

**解決方法**:
```bash
# Ubuntu/Debian
sudo apt-get update && sudo apt-get install -y build-essential

# Fedora/RHEL
sudo dnf install gcc gcc-c++ make

# macOS
xcode-select --install
```

### wasm-pack: command not found

**症状**: wasm-packコマンドが見つからない

**解決方法**:
```bash
# wasm-packを再インストール
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# PATHに追加
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### WebAssemblyが読み込まれない

**症状**: ブラウザコンソールに404エラーまたは「expected magic word」エラーが表示される

**原因**: ブラウザが古いbundle.jsファイルをキャッシュしている

**解決方法**:
1. ブラウザキャッシュをクリア
   - Chrome/Edge: F12 → 更新ボタンを右クリック → 「キャッシュの消去とハード再読み込み」
   - Firefox: Ctrl+Shift+R
2. プライベート/シークレットウィンドウで開く
3. 別のポートで起動: `docker run -p 8081:80 ospf-network-simulator:latest`

### ボタンが反応しない

**症状**: UIのボタンをクリックしても何も起こらない

**確認事項**:
1. ブラウザコンソール（F12）で以下のメッセージが表示されているか確認:
   ```
   Starting initialization...
   WASM initialized
   NetworkSimulator created
   Application ready
   ```

2. CSPエラーが出ていないか確認

**解決方法**:
- ブラウザキャッシュをクリア
- `make run`で再起動

### Dockerコンテナが起動しない

**確認コマンド**:
```bash
sudo docker-compose logs -f
```

**解決方法**:
```bash
make clean
make build
make run
```

### Docker権限エラー（Linux）

権限エラーが発生する場合の解決方法：

#### 方法1: Dockerグループに追加（推奨）
```bash
sudo usermod -aG docker $USER
newgrp docker
```
変更を反映するため、一度ログアウトして再ログインしてください。

#### 方法2: セットアップスクリプトを使用
WSL2 Ubuntu環境の場合：
```bash
./setup-wsl2-ubuntu.sh
```
このスクリプトがDockerグループの設定も自動的に行います。

## 開発者向け情報

### プロジェクト構造
```
nw_simulator/
├── Cargo.toml          # Rust依存関係
├── CLAUDE.md           # プロジェクト要件定義
├── src/                # Rustソースコード
│   └── lib.rs          # WebAssemblyエントリポイント
├── www/                # フロントエンド
│   ├── index.html
│   ├── index.js
│   ├── styles.css
│   └── packet-visualizer.js
├── Dockerfile          # 本番用コンテナ定義
├── Dockerfile.dev      # 開発用コンテナ定義
└── docker-compose.yml  # Docker Compose設定
```

### 開発環境セットアップ

**必要なツールのインストール**:

1. **Rust のインストール**（未インストールの場合）:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup default stable
```

2. **wasm-pack のインストール**:
```bash
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
# PATHに追加（.bashrcや.zshrcに追記推奨）
export PATH="$HOME/.cargo/bin:$PATH"
```

3. **C/C++コンパイラのインストール**（Ubuntu/Debian）:
```bash
sudo apt-get update
sudo apt-get install -y build-essential
```

4. **Node.js のインストール**（未インストールの場合）:
```bash
# Node.js公式リポジトリの追加
curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo -E bash -
sudo apt-get install -y nodejs
```

**ソースからビルド**:
```bash
# プロジェクトのビルド
cd www && yarn install
wasm-pack build --target web --out-dir www/pkg

# 開発サーバーの起動
cd www
yarn start
```

**Docker権限の設定**（Dockerを使用する場合）:
```bash
# 現在のユーザーをdockerグループに追加
sudo usermod -aG docker $USER
# 変更を反映（再ログインが必要）
newgrp docker
```

### テスト実行
```bash
cargo test
```

### デバッグ
ブラウザの開発者ツール（F12）でコンソールログを確認してください。WebAssemblyの初期化プロセスは詳細なログを出力します。

## アーキテクチャ

- **バックエンド**: Rust → WebAssembly
- **フロントエンド**: Vanilla JavaScript + Canvas API
- **Webサーバー**: Nginx（Dockerコンテナ内）
- **プロトコル実装**: OSPFv2（隣接関係確立、LSAデータベース同期、SPF計算）

## 開発について

このプロジェクトは生成AI（Claude）を活用して開発されています。コードの生成、リファクタリング、ドキュメント作成において積極的にAIを使用しています。

貢献方法については[CONTRIBUTING.md](CONTRIBUTING.md)をご覧ください。

## ライセンス

このプロジェクトは[MITライセンス](LICENSE)の下で公開されています。