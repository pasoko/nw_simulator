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

- Docker および Docker Compose
- WebAssembly対応のモダンブラウザ（Chrome、Firefox、Edge）

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
または
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

権限エラーが発生する場合は、ユーザーをdockerグループに追加:
```bash
sudo usermod -aG docker $USER
newgrp docker
```
変更を反映するため、一度ログアウトして再ログインしてください。

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

**必要なツール**:
- Rust (latest stable)
- wasm-pack
- Node.js (v16+)

**ソースからビルド**:
```bash
# wasm-packのインストール
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# プロジェクトのビルド
wasm-pack build --target web --out-dir www/pkg

# 開発サーバーの起動
cd www
npm install
npm start
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

## ライセンス

このプロジェクトは教育目的で作成されています。