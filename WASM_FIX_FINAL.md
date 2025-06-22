# WebAssembly 404エラーの最終解決策

## 問題の詳細
ブラウザが `b57ea1b4dc8b0d72fa91.wasm` を探しているが、実際のファイルは別のハッシュ名で存在する。これは webpack の WASM ハンドリングとDockerビルドプロセスの不整合が原因。

## 根本原因
1. webpack が WASM ファイルを自動的にバンドルしようとして、ハッシュ名を変更
2. Dockerビルドとローカルビルドでハッシュが異なる
3. bundle.js が間違ったハッシュのWASMファイルを参照

## 実施した修正

### 1. index.js の修正
WASMファイルを直接 pkg ディレクトリから読み込むように変更：

```javascript
// 修正前
import init, { NetworkSimulator } from './pkg/nw_simulator.js';

// 修正後
import initWasm, { NetworkSimulator } from './pkg/nw_simulator.js';
const init = () => initWasm('./pkg/nw_simulator_bg.wasm');
```

### 2. webpack.config.js の簡素化
WASM の自動処理を無効化し、pkg ディレクトリをそのままコピー：

```javascript
plugins: [
    new CopyPlugin({
        patterns: [
            { from: 'pkg', to: 'pkg' }
        ],
    }),
]
```

### 3. nginx.conf の修正
pkg ディレクトリへのアクセスを明示的に許可：

```nginx
location /pkg/ {
    alias /usr/share/nginx/html/pkg/;
    add_header Cache-Control "public, max-age=31536000";
}
```

## 実行手順

```bash
make fix-wasm-complete
```

または手動で：

```bash
./fix-wasm-complete.sh
```

このスクリプトは：
1. コンテナを停止
2. ビルドアーティファクトを完全にクリーン
3. 依存関係を再インストール
4. クリーンなビルドを実行
5. Dockerイメージを再ビルド（キャッシュなし）
6. コンテナを起動
7. WASMファイルのアクセシビリティをテスト

## 確認方法

### 1. WASMファイルの直接アクセス確認
```bash
curl -I http://localhost:8080/pkg/nw_simulator_bg.wasm
```
- Status: 200 OK
- Content-Type: application/wasm

### 2. ブラウザコンソール確認
http://localhost:8080 にアクセスして、コンソール（F12）で：
- エラーがないこと
- 以下のログが表示されること：
  - "Starting initialization..."
  - "WASM initialized"
  - "NetworkSimulator created"
  - "Application ready"

### 3. Networkタブ確認
- `/pkg/nw_simulator_bg.wasm` が 200 OK で読み込まれていること
- ハッシュ付きのWASMファイル（例：`b57ea1b4dc8b0d72fa91.wasm`）へのリクエストがないこと

## トラブルシューティング

### まだ404エラーが出る場合
1. ブラウザの強制リロード（Ctrl+Shift+R）
2. プライベートブラウジングモードで試す
3. 完全なDockerクリーンアップ：
   ```bash
   sudo docker system prune -a
   make fix-wasm-complete
   ```

### "expected magic word" エラーが出る場合
WASMファイルの代わりにHTMLが返されている。nginx設定を確認：
```bash
sudo docker-compose exec nw-simulator cat /etc/nginx/nginx.conf | grep -A5 "wasm"
```

### コンソールに初期化ログが表示されない場合
JavaScriptエラーが発生している可能性。コンソールの最初のエラーを確認。

## 最終チェックリスト
- [ ] WASMファイルが `/pkg/nw_simulator_bg.wasm` から読み込まれている
- [ ] 404エラーが発生していない
- [ ] コンソールにWebAssemblyエラーがない
- [ ] ボタンクリックが正常に動作する
- [ ] ルーターの追加、接続、削除が可能