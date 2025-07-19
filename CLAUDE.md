# プロジェクト基本情報

このプロジェクトは **RFC 2328完全準拠のOSPFv2ネットワークシミュレーター** です。Rustで書かれWebAssemblyに変換してWebブラウザで動作します。

# 要件定義

## 基本機能
- webブラウザ上で動作しGUIで直感的に操作ができます
- OSPFv2でルーティング情報を更新するネットワークのシミュレーションを実施できます
- ルーティングエンジンはRustで実装したRFC 2328準拠の独自OSPFv2エンジンを使用します
- 仮想ルーターの設置、ルーター間の接続、ルーターに対するOSPFv2の設定、各ルーターのOSPF情報の参照ができます
- ルーター間でやり取りするパケットの可視化とアニメーション表示ができます
- ルーターやリンクの障害シミュレーション機能があります
- 時間経過によりどのようなパケットが流れ、ルーティングテーブルが変化していくかリアルタイムでシミュレート表示します

## 高度なOSPFv2機能（RFC 2328準拠）
- **AreaType対応**: Normal/Stub/Totally Stubby/NSSAエリア
- **ネットワークタイプ**: Broadcast/Point-to-Point/NBMA/Point-to-Multipoint
- **Virtual Link**: 複数エリア間の仮想接続
- **Route Aggregation**: ABR/ASBRでのルート集約
- **LSAタイプ**: Router/Network/Summary/ASBR-Summary/AS-External/Opaque LSA
- **認証**: Null/Simple Password/Cryptographic認証
- **TOS (Type of Service)**: QoS対応ルーティング

## 拡張機能
- **独立端末デバイス**: ルーターとは独立したエンドホスト
- **拡張Ping機能**: TTL追跡、Traceroute、統計情報
- **パフォーマンス調整**: 大規模ネットワーク向け最適化
- **リアルタイム監視**: パフォーマンスメトリクスとチューニング

## UI/UX機能
- ダークモード/ライトモードの切り替え機能
- レスポンシブなサイドバーUIとモダンなテーマ対応
- シミュレーション速度調整機能（×1/×0.1）
- イベントログの記録とJSON形式でのエクスポート機能
- 端末配置とpingテスト実行GUI
- パフォーマンスモニター

## 設計思想
- 将来的にはほかのプロトコルにも拡張できるような設計
- 教育・研究・検証用途での活用を想定
- RFC準拠による実機との互換性確保

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
│   ├── network_type.rs # ネットワークタイプ定義
│   ├── network_lsa.rs  # Network LSA生成
│   ├── summary_lsa.rs  # Summary LSA生成
│   ├── as_external_lsa.rs # AS-External LSA生成
│   ├── device.rs       # 基本デバイス定義
│   ├── ospf.rs         # OSPFパケット型定義
│   ├── ospf_auth.rs    # OSPF認証機能
│   ├── ospf_options.rs # OSPFOptionsフィールド
│   ├── ospf_interface_state.rs # インターフェース状態管理
│   ├── ospf_tos.rs     # Type of Service機能
│   ├── ospf_lsa_age_manager.rs # LSA年齢管理
│   ├── opaque_lsa.rs   # Opaque LSA（Type 9/10/11）
│   ├── stub_area.rs    # スタブエリア機能
│   ├── virtual_link.rs # 仮想リンク機能
│   ├── route_aggregation.rs # ルート集約機能
│   ├── terminal_device.rs # 独立端末デバイス
│   ├── terminal_manager.rs # 端末デバイス管理
│   ├── enhanced_ping.rs # 拡張ping機能
│   ├── nbma_support.rs # NBMAネットワークサポート
│   ├── performance_tuning.rs # パフォーマンス調整機能
│   ├── ospf_engine.rs  # メインOSPFエンジン
│   ├── ospf_neighbor.rs # OSPF隣接関係管理
│   ├── ospf_lsa_manager.rs # LSAデータベース管理
│   ├── ospf_packet_processor.rs # OSPFパケット処理
│   ├── ospf_timer.rs   # OSPFタイマー管理
│   ├── ospf_dr_election.rs # DR/BDR選出処理
│   ├── ospf_checksum.rs # OSPFチェックサム計算
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
│       ├── error_handling/ # エラー処理
│       ├── events/     # イベントシステム
│       ├── packets/    # パケット処理
│       └── state/      # 状態管理
├── www/                # フロントエンドコード
│   ├── index.html      # メインHTMLページ
│   ├── index.js        # メインJavaScriptエントリポイント
│   ├── packet-visualizer-enhanced.js # 拡張版パケット可視化
│   ├── modules/        # モジュラーJavaScriptコンポーネント
│   │   ├── app-initializer.js      # アプリケーション初期化
│   │   ├── state-manager.js        # グローバル状態管理
│   │   ├── canvas-renderer.js      # Canvas描画処理
│   │   ├── canvas-interaction.js   # Canvas操作処理
│   │   ├── router-icon.js          # ルーターアイコン描画
│   │   ├── ui-controller.js        # UI制御
│   │   ├── simulation-controller.js # シミュレーション制御
│   │   ├── event-logger.js         # イベントログ管理
│   │   ├── sidebar-ui.js           # サイドバーUI
│   │   ├── router-details-ui.js    # ルーター詳細UI
│   │   ├── theme-manager.js        # テーマ管理
│   │   ├── resizable-panel.js      # リサイズ可能パネル
│   │   ├── animation-effects.js    # アニメーション効果
│   │   ├── display-updater.js      # 表示更新処理
│   │   ├── connection-manager.js   # 接続管理
│   │   ├── router-manager.js       # ルーター管理
│   │   ├── host-manager.js         # ホスト管理
│   │   ├── terminal-manager.js     # 端末デバイス管理
│   │   ├── performance-monitor.js  # パフォーマンス監視
│   │   ├── refactored-ospf-adapter.js # OSPFアダプター
│   │   └── README.md               # モジュール説明
│   ├── styles/         # CSSスタイルシート
│   │   ├── modern-theme.css        # モダンテーマ
│   │   ├── sidebar-modern.css      # モダンサイドバー
│   │   ├── dark-mode.css           # ダークモード
│   │   ├── router-details.css      # ルーター詳細スタイル
│   │   ├── animations.css          # アニメーション定義
│   │   ├── host-config.css         # ホスト設定スタイル
│   │   ├── host-device.css         # ホストデバイススタイル
│   │   ├── config-dialog.css       # 設定ダイアログ
│   │   ├── terminal-manager.css    # 端末管理スタイル
│   │   └── performance-monitor.css # パフォーマンス監視スタイル
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

## 2025-07-19: OSPFv2完全準拠シミュレーター完成
RFC 2328に完全準拠したOSPFv2ネットワークシミュレーターの全機能実装が完了。

### フェーズ5実装完了
- **NBMAネットワークサポート**: Frame Relay/ATMなどブロードキャスト非対応ネットワーク
  - 静的隣接ルーター設定、Poll Timer管理、ユニキャストHello送信
- **パフォーマンス調整機能**: 大規模ネットワーク向け最適化
  - 自動チューニング、リアルタイムメトリクス監視、ルートキャッシング
  - Small/Medium/Large/Real-timeプロファイル対応

### フェーズ4実装完了  
- **独立端末デバイス**: ルーターとは独立したエンドホスト実装
- **拡張Ping機能**: TTL追跡、Traceroute、統計情報、セッション管理
- **GUI拡張**: 端末配置UI、ping実行インターフェース

### フェーズ3実装完了
- **スタブエリア**: Normal/Stub/Totally Stubby/NSSAエリア対応
- **仮想リンク**: RFC 2328 Section 15準拠の複数エリア間接続
- **ルート集約**: ABR/ASBRでのinter-area/external route集約

### フェーズ2実装完了
- **TOS (Type of Service)**: QoS対応ルーティング
- **LSA年齢管理**: MaxAge LSA保持とフラッディング制御
- **Opaque LSA**: Type 9/10/11 Traffic Engineering対応

### フェーズ1実装完了
- **InfTransDelayパラメータ**: LSA送信遅延制御
- **SPF計算パラメータ**: spf_delay, spf_holdtime, spf_max_age
- **Optionsフィールド**: MC/N/P/EA/DC/O-bit完全実装
- **インターフェース状態管理**: 拡張状態追跡システム

## 2025-07-12: OSPFエンジン修正とシミュレーション速度調整機能
- OSPFv2準拠のLSAデータベース更新とMaxAge LSA保持を実装
- LSAフラッディングループ問題を修正
- シミュレーション速度調整機能（×1/×0.1）を追加
- サイドバーに速度切り替えボタンを設置

## 2025-06-29: ツール最新版アップデート
- Node.js: 22.12.0 → 22.17.0 (LTS最新版)
- webpack: 5.89.0 → 5.99.9
- webpack-cli: 5.1.4 → 6.0.1  
- html-webpack-plugin: 5.5.3 → 5.6.3
- copy-webpack-plugin: 11.0.0 → 13.0.0
- Docker base images: rust:1.79→1.88, node:20→22
- 全ビルドとテスト正常動作確認済み
