# ボタン機能が動作しない問題の解決

## 問題の原因

ボタンが反応しない原因は、nginx の Content-Security-Policy (CSP) ヘッダーが WebAssembly の実行をブロックしていたためです。

### 具体的な原因：
1. CSP ヘッダーに `'wasm-unsafe-eval'` ディレクティブが含まれていない
2. WebAssembly モジュールがロードできず、JavaScript エラーが発生
3. その結果、イベントリスナーの設定が失敗

## 解決方法

### 方法1: CSP ヘッダーを修正（推奨）
nginx.conf の CSP ヘッダーに `'wasm-unsafe-eval'` を追加：

```nginx
add_header Content-Security-Policy "default-src 'self' 'unsafe-inline' 'unsafe-eval' 'wasm-unsafe-eval' data: blob:; script-src 'self' 'unsafe-inline' 'unsafe-eval' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline';" always;
```

### 方法2: シンプルな nginx 設定を使用
CSP ヘッダーを削除した簡単な設定を使用：

```bash
# nginx-simple.conf を使用する場合
sudo docker build --build-arg NGINX_CONF=nginx-simple.conf -t ospf-network-simulator:latest .
```

## 再ビルドと実行

```bash
# 完全な修正と再ビルド
./final-fix.sh
```

または手動で：

```bash
make stop
make clean
make build
make run
```

## 動作確認

1. **メインアプリケーション**: http://localhost:8080
   - すべてのボタンがクリック可能か確認
   - ルーターの追加、接続、削除が動作するか確認

2. **テストページ**: http://localhost:8080/test.html
   - JavaScript、WebAssembly の動作を個別にテスト
   - エラーメッセージを確認

## トラブルシューティング

### ブラウザのコンソールでエラーを確認
1. F12 でデベロッパーツールを開く
2. Console タブでエラーメッセージを確認
3. 特に以下のエラーに注意：
   - `Refused to compile or instantiate WebAssembly`
   - `Content Security Policy` 関連のエラー

### キャッシュのクリア
```
Ctrl + Shift + R (Windows/Linux)
Cmd + Shift + R (Mac)
```

### Docker ログの確認
```bash
sudo docker-compose logs -f
```

## 確認済みの修正内容

1. ✓ webpack.config.js - packet-visualizer.js をバンドルに含める
2. ✓ nginx.conf - CSP ヘッダーに 'wasm-unsafe-eval' を追加
3. ✓ index.html - Delete Router と Disconnect Routers ボタンが存在
4. ✓ index.js - イベントリスナーが正しく設定されている