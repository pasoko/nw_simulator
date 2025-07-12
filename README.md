# OSPF Network Simulator

WebAssemblyとRustで構築されたOSPFv2ネットワークシミュレーターです。ブラウザ上でルーターの設置、接続、OSPFプロトコルの動作をリアルタイムで可視化できます。

## 機能

- 仮想ルーターの設置、移動、削除
- ルーター間の接続と切断（コスト設定可能）
- OSPFv2プロトコルのリアルタイムシミュレーション
- パケット送受信の可視化アニメーション（Hello、DD、LSR、LSU、LSAck）
- SPFアルゴリズムによるルーティングテーブル計算
- 時間経過によるネットワーク状態の変化をシミュレート
- リンク障害・復旧シミュレーション（Toggle Failureモード）
- リアルタイムシミュレーション統計表示
- シミュレーションログの記録とJSON形式でのエクスポート
- シミュレーション速度調整機能（×1/×0.1）
- ダークモード/ライトモードの切り替え
- レスポンシブなサイドバーUI
- ルーター詳細情報の表示（OSPF状態、隣接関係、LSAデータベース、ルーティングテーブル）

## 必要環境

### 実行環境
- Docker および Docker Compose
- WebAssembly対応のモダンブラウザ（Chrome、Firefox、Edge）

### 開発環境（ソースからビルドする場合）

#### 必須ツール
以下のツールが事前にインストールされている必要があります：

1. **Rust** (stable版)
   - Rustコンパイラとcargoパッケージマネージャー
   - バージョン: 1.79以上推奨

2. **wasm-pack**
   - RustコードをWebAssemblyにコンパイルするツール
   - バージョン: 0.13.1以上

3. **Node.js および Yarn**
   - フロントエンドビルドとwebpackの実行
   - バージョン: Node.js v22.17.0（LTS）、Yarn v4.9.2
   - 推奨: Voltaを使用したバージョン管理

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

1. **ルーターの追加**: サイドバーの「Add」ツールを選択後、キャンバス上をクリックして配置
2. **ルーターの移動**: 「Move」ツールを選択後、ルーターをドラッグして移動
3. **ルーターの接続**: 「Connect」ツールを選択後、接続したい2つのルーターをクリック（コスト入力可能）
4. **接続の解除**: 「Disconnect」ツールを選択後、切断したい2つのルーターをクリック
5. **ルーターの削除**: 「Delete」ツールを選択後、削除したいルーターをクリック
6. **OSPFの有効化**: サイドバーの各ルーターカードにある「Enable OSPF」ボタンをクリック
7. **リンク障害の切り替え**: 「Toggle Failure」ツールを選択後、リンクをクリック
8. **シミュレーション速度調整**: 「×1」/「×0.1」ボタンで速度を切り替え
9. **シミュレーション開始**: 「Start Simulation」ボタンをクリック
10. **ルーター詳細確認**: サイドバーのルーターカードをクリックして展開

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
│   ├── lib.rs          # WebAssemblyエントリポイント
│   ├── network.rs      # ネットワークトポロジー管理
│   ├── network_type.rs # ネットワークタイプ定義
│   ├── ospf.rs         # OSPFパケット型定義
│   ├── ospf_engine.rs  # OSPFプロトコルエンジン
│   ├── ospf_neighbor.rs # OSPF隣接関係管理
│   ├── ospf_lsa_manager.rs # LSAデータベース管理
│   ├── ospf_packet_processor.rs # OSPFパケット処理
│   ├── ospf_timer.rs   # OSPFタイマー管理
│   ├── ospf_checksum.rs # OSPFチェックサム計算
│   ├── protocol.rs     # プロトコル定義
│   ├── router.rs       # ルーター状態管理
│   ├── route_calculator.rs # ルート計算制御
│   ├── simulation.rs   # シミュレーション制御
│   ├── spf.rs          # 最短経路優先アルゴリズム
│   ├── ui_state.rs     # UI状態管理
│   ├── event_manager.rs # イベント管理
│   ├── failure_manager.rs # 障害シミュレーション管理
│   ├── wasm_interface.rs # WebAssemblyインターフェース
│   └── serialization.rs # シリアライゼーション
├── www/                # フロントエンド
│   ├── index.html      # メインHTMLページ
│   ├── index.js        # メインJavaScriptエントリポイント
│   ├── packet-visualizer-enhanced.js # 拡張版パケット可視化
│   ├── modules/        # モジュラーJavaScriptコンポーネント
│   │   ├── canvas-renderer.js      # Canvas描画処理
│   │   ├── connection-manager.js   # 接続管理
│   │   ├── event-logger.js         # イベントログ管理
│   │   ├── router-manager.js       # ルーター管理
│   │   ├── simulation-controller.js # シミュレーション制御
│   │   ├── state-manager.js        # 状態管理
│   │   ├── animation-effects.js    # アニメーション効果
│   │   ├── app-initializer.js      # アプリケーション初期化
│   │   ├── canvas-interaction.js   # Canvas操作処理
│   │   ├── display-updater.js      # 表示更新処理
│   │   ├── router-details-ui.js    # ルーター詳細UI
│   │   ├── router-icon.js          # ルーターアイコン描画
│   │   ├── sidebar-ui.js           # サイドバーUI
│   │   ├── theme-manager.js        # テーマ管理
│   │   └── ui-controller.js        # UI制御
│   ├── styles/         # CSSスタイルシート
│   │   ├── animations.css   # アニメーション定義
│   │   ├── dark-mode.css    # ダークモード
│   │   ├── modern-theme.css # モダンテーマ
│   │   ├── router-details.css # ルーター詳細スタイル
│   │   └── sidebar-modern.css # モダンサイドバー
│   ├── webpack.config.js # Webpack設定
│   └── package.json    # フロントエンド依存関係
├── Dockerfile          # 本番用コンテナ定義
├── Dockerfile.dev      # 開発用コンテナ定義
├── docker-compose.yml  # Docker Compose設定
└── setup-wsl2-ubuntu.sh # WSL2セットアップスクリプト
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

4. **Node.js のインストール**（Volta推奨）:
```bash
# Voltaのインストール
curl https://get.volta.sh | bash
source ~/.bashrc

# Node.jsとYarnのインストール
volta install node@22.17.0
volta install yarn@4.9.2
```

**従来の方法**（未インストールの場合）:
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