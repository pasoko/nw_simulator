# WebAssembly読み込みエラーの解決

## エラーの内容
```
Failed to load resource: the server responded with a status of 404 (Not Found)
WebAssembly.instantiate(): expected magic word 00 61 73 6d, found 3c 68 74 6d @+0
```

## 原因
1. **WASMファイルのハッシュ不一致**
   - webpackがビルドごとに新しいハッシュでWASMファイルを生成
   - bundle.jsが古いハッシュのWASMファイルを参照

2. **bundle.jsの重複読み込み**
   - HtmlWebpackPluginが自動的に`<script>`タグを挿入
   - 元のHTMLテンプレートにも手動で`<script>`タグが存在

3. **404エラーページの返却**
   - WASMファイルが見つからない場合、nginxが404 HTMLページを返す
   - WebAssemblyがHTMLをWASMとして解釈しようとしてエラー

## 実施した修正

### 1. index.htmlから重複scriptタグを削除
```html
<!-- 削除前 -->
<script src="bundle.js"></script>
<script defer="defer" src="bundle.js"></script>

<!-- 削除後 -->
<!-- HtmlWebpackPluginが自動的に挿入するため、手動での記述は不要 -->
```

### 2. webpack.configの修正
```javascript
output: {
    // ... 
    webassemblyModuleFilename: '[hash].wasm',
}
```

### 3. nginx.confの修正
```nginx
location ~ \.wasm$ {
    types { }
    default_type application/wasm;
}
```

## 再ビルドと実行

```bash
# WASMエラー修正スクリプトを実行
./fix-wasm-error.sh
```

または手動で：
```bash
make stop
make clean
make build
make run
```

## デバッグ方法

### 1. WASMテストページ
http://localhost:8080/wasm-test.html
- WebAssemblyのサポート状況を確認
- WASMファイルの存在とアクセス可能性を確認
- 正しいMIMEタイプで配信されているか確認

### 2. ブラウザのNetworkタブ
1. F12でデベロッパーツールを開く
2. Networkタブを選択
3. ページをリロード
4. .wasmファイルのリクエストを確認
   - Status: 200 OK であることを確認
   - Content-Type: application/wasm であることを確認

### 3. Consoleタブでエラー確認
- WebAssembly関連のエラーがないことを確認
- 初期化ログが正常に表示されることを確認

## トラブルシューティング

### "expected magic word" エラーが続く場合
1. ブラウザキャッシュを完全にクリア
2. プライベートブラウジングモードで試す
3. Dockerコンテナを完全に再ビルド：
   ```bash
   sudo docker system prune -a
   make build
   make run
   ```

### WASMファイルが404の場合
1. コンテナ内のファイルを確認：
   ```bash
   sudo docker exec -it nw_simulator-nw-simulator-1 ls -la /usr/share/nginx/html/
   ```
2. WASMファイル名を確認して、bundle.jsが参照しているファイル名と一致しているか確認

### MIMEタイプエラーの場合
nginxログを確認：
```bash
sudo docker-compose logs nginx
```

## 最終確認項目
- [ ] bundle.jsが1回だけ読み込まれている
- [ ] WASMファイルが200 OKで読み込まれている
- [ ] Content-Typeがapplication/wasmになっている
- [ ] コンソールにWebAssemblyエラーが表示されていない
- [ ] ボタンが正常に動作する