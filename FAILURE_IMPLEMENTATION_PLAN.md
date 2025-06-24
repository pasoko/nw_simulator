# 障害発生・回復機能実装計画

## 1. 要件定義

### リンク障害
- リンクをクリックして障害発生/回復を切り替え
- 障害中はパケットが双方向で通信不可
- 視覚的に障害状態を表示（赤色点線）

### ルーター障害
- ルーターをクリックして障害発生/回復を切り替え
- 障害中の動作：
  - パケット送信停止
  - ルーティング情報破棄
  - OSPF隣接関係の解除
- 回復時は初期状態から再開
- 視覚的に障害状態を表示（赤色背景）

## 2. 技術設計

### バックエンド変更

#### データ構造の拡張
```rust
// network.rs
pub struct Link {
    // 既存フィールド
    pub is_failed: bool,  // 新規追加
}

// router.rs
pub struct RouterState {
    // 既存フィールド
    pub is_failed: bool,  // 新規追加
}
```

#### API追加
```rust
// lib.rs
pub fn toggle_link_failure(&mut self, from_id: u32, to_id: u32) -> bool
pub fn toggle_router_failure(&mut self, router_id: u32) -> bool
pub fn get_failures_json(&self) -> String
```

#### 障害処理ロジック
1. パケット送信時にリンク障害チェック
2. パケット受信時にルーター障害チェック
3. 障害ルーターのOSPF処理スキップ
4. 障害回復時の初期化処理

### フロントエンド変更

#### 新規モード追加
```javascript
// state-manager.js
modes: {
    ADD_ROUTER: 'add_router',
    MOVE_ROUTER: 'move_router',
    CONNECT_ROUTERS: 'connect_routers',
    DELETE_ROUTER: 'delete_router',
    DISCONNECT_ROUTERS: 'disconnect_routers',
    TOGGLE_FAILURE: 'toggle_failure'  // 新規追加
}
```

#### クリックハンドラー修正
- キャンバスクリック時に障害モードチェック
- リンククリック判定の実装
- 障害状態切り替えの実装

#### 視覚的表現
```javascript
// canvas-renderer.js
// 障害リンクの描画
if (connection.is_failed) {
    ctx.strokeStyle = '#ff0000';
    ctx.setLineDash([5, 5]);
}

// 障害ルーターの描画
if (router.is_failed) {
    ctx.fillStyle = '#ffcccc';
}
```

## 3. 実装手順

### ステップ1: バックエンドのデータ構造拡張
1. Link構造体にis_failedフィールド追加
2. RouterState構造体にis_failedフィールド追加
3. シリアライズ/デシリアライズの対応

### ステップ2: 障害制御API実装
1. toggle_link_failure関数の実装
2. toggle_router_failure関数の実装
3. get_failures_json関数の実装

### ステップ3: パケット処理の修正
1. send_packet関数で障害チェック
2. process_packet_event関数で障害チェック
3. OSPFエンジンの障害対応

### ステップ4: フロントエンドのモード追加
1. 障害切り替えモードの追加
2. モード切り替えボタンの追加
3. モードインジケーターの更新

### ステップ5: クリックイベント処理
1. リンククリック判定の実装
2. 障害状態切り替えの実装
3. 状態更新とUI反映

### ステップ6: 視覚的表現の実装
1. 障害リンクの描画処理
2. 障害ルーターの描画処理
3. 障害状態インジケーター

### ステップ7: ログとイベント
1. 障害発生/回復イベントの記録
2. イベントログへの表示
3. 統計情報の更新

## 4. テスト計画

### 単体テスト
- リンク障害の状態切り替え
- ルーター障害の状態切り替え
- パケット破棄の動作確認

### 統合テスト
- 障害中のOSPF動作確認
- 障害回復後の再収束確認
- 複数障害の同時発生

### シナリオテスト
1. 単一リンク障害での迂回経路確立
2. ルーター障害での隣接関係再構築
3. 複数障害での孤立ノード発生

## 5. 実装スケジュール

1. バックエンド実装（2時間）
   - データ構造とAPI
   - 障害処理ロジック

2. フロントエンド実装（2時間）
   - UIモードとイベント処理
   - 視覚的表現

3. テストとデバッグ（1時間）
   - 動作確認
   - バグ修正

合計: 約5時間の作業見込み