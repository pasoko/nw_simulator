# Delete Router / Disconnect Routers ボタン表示問題の解決

## 問題の概要
`make run` でコンテナを起動した際、Delete RouterボタンとDisconnect Routersボタンが表示されない問題が発生していました。

## 原因
HtmlWebpackPluginがindex.htmlを処理する際に、何らかの理由で以下の要素を削除していました：
- Delete Routerボタン
- Disconnect Routersボタン  
- `.button.danger` CSSクラス

## 解決方法
webpack.config.jsを修正し、HtmlWebpackPluginを削除してCopyPluginで直接index.htmlをコピーするように変更しました。

### 変更前：
```javascript
plugins: [
    new HtmlWebpackPlugin({
        template: './index.html',
        inject: true,
        minify: false
    }),
    // ...
]
```

### 変更後：
```javascript
plugins: [
    new CopyPlugin({
        patterns: [
            { from: 'index.html', to: 'index.html' },
            { from: 'pkg', to: 'pkg' },
            { from: 'packet-visualizer.js', to: 'packet-visualizer.js' }
        ],
    }),
]
```

## 再ビルド手順
以下のいずれかのコマンドを実行：

```bash
# 推奨：修正と再ビルドを一度に実行
make fix-rebuild

# または手動で実行
make stop
make clean  
make build
make run
```

## 確認方法
1. ブラウザで `http://localhost:8080` にアクセス
2. サイドバーの「Controls」セクションに以下のボタンが表示されることを確認：
   - Delete Router（赤色のボタン）
   - Disconnect Routers（赤色のボタン）