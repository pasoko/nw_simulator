# verify-buttons.sh 実行ガイド

## 前提条件
1. `make run` でコンテナが正常に起動していること
2. http://localhost:8080 でアプリケーションにアクセスできること

## 実行方法

### 1. コンテナの状態確認
```bash
./check-container.sh
```

### 2. ボタンの検証（正しいコマンド）
```bash
./verify-buttons.sh
```

**注意**: ファイル名は `verify-buttons.sh` です（`verify-button.sh` ではありません）

## エラーが発生した場合

### "Container not running" エラー
```bash
# コンテナを起動
make run

# 少し待ってから再実行
sleep 5
./verify-buttons.sh
```

### sudo権限エラー
```bash
# sudoを使って実行
sudo ./verify-buttons.sh
```

## 手動での確認方法

ブラウザで http://localhost:8080 にアクセスして、以下のボタンが表示されることを確認：

1. **Add Router** - 緑色のボタン
2. **Connect Routers** - 青色のボタン  
3. **Delete Router** - 赤色のボタン
4. **Disconnect Routers** - 赤色のボタン
5. **Start Simulation** - 青色のボタン
6. **Export Log** - 青色のボタン
7. **Clear Log** - 青色のボタン

## make dev のエラーについて

表示されたエラーは `make dev` (開発モード) の実行時のものです。本番環境 (`make run`) には影響しません。

開発モードを使用したい場合は、以下を実行してください：
```bash
# Dockerfile.devのキャッシュをクリア
sudo docker build --no-cache -f Dockerfile.dev -t ospf-network-simulator:dev .
```