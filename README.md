# RFC 2328完全準拠 OSPFv2ネットワークシミュレーター

WebAssemblyとRustで構築されたRFC 2328完全準拠のOSPFv2ネットワークシミュレーターです。ブラウザ上でエンタープライズレベルのOSPFネットワークを構築・可視化・分析できます。

## 主要機能

### OSPFv2プロトコル完全実装（RFC 2328準拠）
- **完全な隣接関係管理**: Down/Init/2-Way/ExStart/Exchange/Loading/Full状態遷移
- **LSAタイプ完全対応**: Router/Network/Summary/AS-External/Opaque LSA
- **マルチエリア対応**: Normal/Stub/Totally Stubby/NSSA/Totally NSSAエリア
- **ネットワークタイプ完全対応**: Broadcast/NBMA/Point-to-Point/Point-to-Multipoint
- **仮想リンク**: 複数エリア間の非連続接続をサポート
- **拡張認証**: Null/Simple Password/Cryptographic認証
- **TOS (Type of Service)**: QoS対応ルーティング

### ネットワークシミュレーション機能
- **リアルタイム可視化**: パケット送受信アニメーション、状態遷移の可視化
- **大規模ネットワーク対応**: 数百台のルーターを持つエンタープライズネットワーク
- **障害シミュレーション**: リンク/ルーター障害の動的シミュレーション
- **パフォーマンス分析**: SPF計算時間、パケット処理時間の詳細測定
- **ルートアグリゲーション**: エリア境界での経路集約

### 独立端末デバイス機能
- **ターミナルデバイス**: OSPFネットワークに接続する独立ホスト
- **拡張Ping機能**: カスタマイズ可能なping（パケットサイズ、TTL、間隔）
- **Traceroute**: ホップごとの経路追跡とレイテンシ測定
- **リアルタイム到達性テスト**: ネットワーク変更時の即座の接続性確認

### パフォーマンス最適化
- **アダプティブチューニング**: ネットワークサイズに応じた自動最適化
- **ルートキャッシュ**: 計算済み経路の効率的なキャッシュ機能
- **並列処理**: 大規模ネットワークでのマルチスレッド処理
- **メモリ管理**: 効率的なLSAエイジング、パケットプール管理

### NBMA ネットワーク対応
- **静的隣接設定**: Frame Relay、ATM等の非ブロードキャストネットワーク
- **ポールタイマー**: dead隣接への定期的Helloパケット送信
- **DR/BDR選出**: NBMAネットワークでの指定ルーター機能

### 高度なUI機能
- **ダークモード/ライトモード**: 現代的なテーマ切り替え
- **リアルタイム詳細表示**: OSPF状態、LSAデータベース、ルーティングテーブル
- **シミュレーション速度調整**: ×1/×0.1の可変速度制御
- **イベントログ**: 包括的なネットワークイベント記録とJSON形式エクスポート
- **レスポンシブUI**: サイドバー、詳細パネルの動的リサイズ
- **ちらつき防止**: 差分更新による画面のちらつき完全解消
- **自動リアルタイム更新**: ユーザー操作不要で情報が自動更新

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
11. **端末デバイスの追加**: 「Terminal」ツールを選択後、キャンバス上をクリックして配置
12. **端末の接続**: 端末をドラッグしてルーターに近づけると自動接続
13. **Pingテスト**: 端末を右クリックしてPingダイアログを開き、送信先を選択

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
├── Cargo.toml          # Rust依存関係設定
├── CLAUDE.md           # プロジェクト要件定義と実装指針
├── src/                # Rustソースコード（RFC 2328完全準拠）
│   ├── lib.rs          # WebAssemblyエントリポイント
│   ├── network.rs      # ネットワークトポロジー管理
│   ├── network_type.rs # ネットワークタイプ定義
│   ├── network_lsa.rs  # Network LSA生成・管理
│   ├── summary_lsa.rs  # Summary LSA処理
│   ├── as_external_lsa.rs # AS-External LSA処理
│   ├── opaque_lsa.rs   # Opaque LSA対応
│   ├── device.rs       # 汎用デバイス定義
│   ├── ospf.rs         # OSPFv2パケット型定義
│   ├── ospf_auth.rs    # OSPF認証（Null/Simple/Cryptographic）
│   ├── ospf_options.rs # OSPFオプションフィールド
│   ├── ospf_interface_state.rs # インターフェース状態管理
│   ├── ospf_tos.rs     # Type of Service対応
│   ├── ospf_lsa_age_manager.rs # LSAエイジング管理
│   ├── ospf_engine.rs  # OSPFプロトコルエンジン
│   ├── ospf_neighbor.rs # OSPF隣接関係管理
│   ├── ospf_lsa_manager.rs # LSAデータベース管理
│   ├── ospf_packet_processor.rs # OSPFパケット処理
│   ├── ospf_timer.rs   # OSPFタイマー管理
│   ├── ospf_dr_election.rs # DR/BDR選出処理
│   ├── ospf_checksum.rs # OSPFチェックサム計算
│   ├── stub_area.rs    # Stubエリア実装
│   ├── virtual_link.rs # 仮想リンク機能
│   ├── route_aggregation.rs # ルート集約
│   ├── terminal_device.rs # 独立端末デバイス
│   ├── terminal_manager.rs # 端末デバイス管理
│   ├── enhanced_ping.rs # 拡張Ping機能
│   ├── nbma_support.rs # NBMAネットワーク対応
│   ├── performance_tuning.rs # パフォーマンスチューニング
│   ├── protocol.rs     # プロトコル定義
│   ├── router.rs       # ルーター状態管理
│   ├── route_calculator.rs # ルート計算制御
│   ├── simulation.rs   # シミュレーション制御
│   ├── spf.rs          # 最短経路優先アルゴリズム
│   ├── ui_state.rs     # UI状態管理
│   ├── event_manager.rs # イベント管理
│   ├── failure_manager.rs # 障害シミュレーション管理
│   ├── ping_manager.rs # Ping管理
│   ├── wasm_interface.rs # WebAssemblyインターフェース
│   ├── serialization.rs # シリアライゼーション
│   └── ospf_refactored/ # リファクタリング版OSPFモジュール
│       ├── error_handling/ # エラー処理システム
│       ├── events/     # イベントシステム
│       ├── packets/    # パケット処理システム
│       └── state/      # 状態管理システム
├── www/                # フロントエンドコード
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
│   │   ├── refactored-ospf-adapter.js # OSPFアダプター
│   │   ├── resizable-panel.js      # リサイズ可能パネル
│   │   ├── router-details-ui.js    # ルーター詳細UI
│   │   ├── router-icon.js          # ルーターアイコン描画
│   │   ├── sidebar-ui.js           # サイドバーUI
│   │   ├── theme-manager.js        # テーマ管理
│   │   ├── ui-controller.js        # UI制御
│   │   ├── nbma-gui.js             # NBMA設定GUI
│   │   ├── performance-monitor.js  # パフォーマンス監視
│   │   ├── host-manager.js         # ホストデバイス管理
│   │   └── terminal-manager.js     # 端末デバイス管理
│   ├── styles/         # CSSスタイルシート
│   │   ├── animations.css   # アニメーション定義
│   │   ├── dark-mode.css    # ダークモード
│   │   ├── modern-theme.css # モダンテーマ
│   │   ├── router-details.css # ルーター詳細スタイル
│   │   ├── sidebar-modern.css # モダンサイドバー
│   │   ├── host-config.css    # ホスト設定スタイル
│   │   ├── host-device.css    # ホストデバイススタイル
│   │   ├── config-dialog.css  # 設定ダイアログ
│   │   ├── terminal-manager.css # 端末管理スタイル
│   │   └── performance-monitor.css # パフォーマンス監視スタイル
│   ├── webpack.config.js # Webpack設定
│   └── package.json    # フロントエンド依存関係
├── pkg/                # 生成されるWebAssemblyファイル
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

### コア技術スタック
- **バックエンド**: Rust → WebAssembly（高性能ネットワークシミュレーション）
- **フロントエンド**: Vanilla JavaScript + Canvas API（リアルタイム可視化）
- **Webサーバー**: Nginx（Dockerコンテナ内での軽量配信）
- **プロトコル実装**: RFC 2328完全準拠 OSPFv2

### OSPFv2エンジン設計
- **状態管理**: 完全な隣接関係FSM（7状態遷移）
- **LSAデータベース**: 分散同期とエイジング機能
- **SPF計算**: Dijkstraアルゴリズムの最適化実装
- **タイマー管理**: Hello/Dead/Wait/Retransmissionタイマー
- **エリア管理**: マルチエリア対応（Normal/Stub/NSSA）
- **認証**: Cryptographic認証を含む3レベル対応

### パフォーマンス設計
- **メモリ効率**: LSAプール、パケットプール管理
- **計算最適化**: 増分SPF、ルートキャッシュ
- **スケーラビリティ**: 数百ルーターまでの大規模対応
- **リアルタイム**: 1ms精度のイベント処理

## 最近の更新 (2025-07)

### UI/UXの大幅改善
- **画面ちらつきの完全解消**: 差分DOM更新とイベント委譲パターンの実装
- **リアルタイム自動更新**: ユーザー操作不要でルーティングテーブル等が自動更新
- **パフォーマンス最適化**: requestAnimationFrameを使用したスムーズな描画

### ターミナルデバイス機能の完成
- **独立端末デバイス**: ルーターとは独立したエンドホストの実装
- **拡張Ping機能**: カスタマイズ可能なping、Traceroute、統計情報
- **GUI拡張**: 端末配置・移動・接続UI、ping実行インターフェース

### パフォーマンス調整機能
- **アダプティブチューニング**: ネットワークサイズに応じた自動最適化
- **リアルタイムメトリクス**: SPF計算時間、パケット処理時間の監視
- **プロファイル対応**: Small/Medium/Large/Real-timeプロファイル

## 開発について

このプロジェクトは生成AI（Claude）を活用して開発されています。コードの生成、リファクタリング、ドキュメント作成において積極的にAIを使用しています。

貢献方法については[CONTRIBUTING.md](CONTRIBUTING.md)をご覧ください。

## ライセンス

このプロジェクトは[MITライセンス](LICENSE)の下で公開されています。