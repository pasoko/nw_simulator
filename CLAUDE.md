# プロジェクト基本情報

このプロジェクトは Rust で書かれ WebAssembly に変換して動作する web アプリケーションです。

# 要件定義

‐ webブラウザ上で動作しGUIで操作ができます
‐ OSPFv2でルーティング情報を更新するネットワークのシミュレーションを実施できます
‐ ルーティングエンジンはRustで実装した独自のOSPFv2エンジンを使用します
‐ 仮想ルーターの設置、ルーター間の接続、ルーターに対するOSPFv2の設定、各ルーターのOSPF情報の参照ができます
‐ ルーター間でやり取りするパケットの可視化とアニメーション表示ができます
‐ ルーターやリンクの障害シミュレーション機能があります
- 時間経過によりどのようなパケットが流れ、ルーティングテーブルが変化していくかリアルタイムでシミュレート表示します
- 将来的にはほかのプロトコルにも拡張できるような設計になっています
- イベントログの記録とJSON形式でのエクスポート機能があります

# 共通コマンド
- JavaScript Tool Managerとしてvoltaを使用します。
- Node.jsは最新の安定版LTS（v22.17.0）を使用します
- nodeのパッケージマネージャーとしてはyarn（v4.9.2）を使います
- WebAssemblyビルドにはwasm-pack（v0.13.1）を使用します

# コードスタイル

- ES6 モジュール構文（import/export）を使用
- 可能な限り分割代入を活用
- 関数名は snake_case、クラス名は PascalCase で統一
- 推奨するフォルダ構成は以下を参照してください
.
├── Cargo.toml          # Rustの依存関係設定
├── src/                # Rustソースコード
│   ├── lib.rs          # WebAssemblyエントリポイント
│   ├── network.rs      # ネットワークトポロジー管理
│   ├── ospf.rs         # OSPFプロトコル実装
│   ├── ospf_engine.rs  # OSPFエンジンコア
│   ├── protocol.rs     # プロトコル定義
│   ├── router.rs       # ルーター状態管理
│   ├── simulation.rs   # シミュレーション制御
│   ├── spf.rs          # 最短経路優先アルゴリズム
│   └── ui_state.rs     # UI状態管理
├── www/                # フロントエンドコード
│   ├── index.html      # メインHTMLページ
│   ├── index.js        # メインJavaScriptエントリポイント
│   ├── packet-visualizer.js # パケット可視化
│   ├── modules/        # モジュラーJavaScriptコンポーネント
│   │   ├── canvas-renderer.js
│   │   ├── connection-manager.js
│   │   ├── event-logger.js
│   │   ├── router-manager.js
│   │   ├── simulation-controller.js
│   │   └── state-manager.js
│   ├── webpack.config.js # Webpack設定
│   └── package.json    # フロントエンド依存関係
├── pkg/                # 生成されるWebAssemblyファイル
├── docker-compose.yml  # Docker設定
├── Dockerfile          # Docker設定
└── setup-wsl2-ubuntu.sh # WSL2セットアップスクリプト

# ワークフロー

- 変更完了後は必ず型チェックを実行
- 全テストではなく単体テストを優先して実行

# 最新アップデート履歴

## 2025-06-29: ツール最新版アップデート
- Node.js: 22.12.0 → 22.17.0 (LTS最新版)
- webpack: 5.89.0 → 5.99.9
- webpack-cli: 5.1.4 → 6.0.1  
- html-webpack-plugin: 5.5.3 → 5.6.3
- copy-webpack-plugin: 11.0.0 → 13.0.0
- Docker base images: rust:1.79→1.88, node:20→22
- 全ビルドとテスト正常動作確認済み
