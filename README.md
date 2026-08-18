# Topcoat Native Explorer

Topcoat の `view!` 記法で設計し、DOM や WebView を使わず、Rust バックエンドとネイティブ WinUI 3 コントロールで動作するファイルエクスプローラー PoC です。

Windows 11の標準ファイルエクスプローラーを基準に、Mica背景、ナビゲーション、コマンドバー、左サイドバー、詳細表示、ステータスバーを再現しています。

## 実装済み

- 実ファイルシステムのフォルダー・ファイル列挙
- 戻る、進む、上へ、更新
- クリック可能なパンくずリスト
- ホーム、既知フォルダー、利用可能ドライブのサイドバー
- 現在フォルダー内のインクリメンタル検索
- 名前、更新日時、種類、サイズの4列表
- フォルダー優先の昇順・降順ソート
- 単一選択とダブルクリックによるフォルダー移動
- 選択項目を「開く」ボタンで開く
- ファイルをWindowsの関連付けアプリで開く
- ローカル時刻による更新日時表示
- ファイル種類、サイズ、汎用アイコン表示
- MicaバックドロップとOSテーマ追従
- UI Automation名とネイティブListView選択

安全のため、この版は読み取り専用です。新規作成、切り取り、コピー、貼り付け、名前変更、削除は外観のみ再現し、明示的に無効化しています。

## Topcoatソース

画面全体は [`app/src/main.rs`](app/src/main.rs) のTopcoat構文で記述されています。状態やイベントの書き味は最初のPoCから変えていません。

```rust
view! { cx =>
    signal explorer = ExplorerState::initial();
    let snapshot = explorer.get();

    <main class="p-3 space-y-2">
        <input
            type="text"
            :value=$(snapshot.query())
            @input=$(|event: Event| {
                explorer.set(explorer.get().with_query(event.target.value))
            })
        >

        <table
            :key=$(snapshot.current_display())
            :rows=$(snapshot.rows())
            :selected=$(snapshot.selected_key())
            :width=$(table_width)
            :height=$(table_height)
            @select=$(|event: Event| explorer.set(explorer.get().select(event.target.value)))
            @activate=$(|event: Event| explorer.set(explorer.get().activate(event.target.value)))
            @sort=$(|event: Event| explorer.set(explorer.get().sort_by(event.target.value)))
        ></table>
    </main>
}
```

`<table>`は今回追加したネイティブ要素です。`:key`で場所ごとにネイティブListViewを識別し、フォルダー移動時に古い選択を持ち越しません。Topcoat公式parserのASTから、次のWinUI 3ツリーへloweringされます。

```text
<table>
  -> StackPanel
       -> Grid（列見出しボタン）
       -> ListView（仮想化、選択）
            -> Grid（各行の4セル）
```

表の行データはrenderer-neutralな`TableRow`です。DOM用の`table/tr/td`を無理に模倣せず、ネイティブ環境で必要な仮想化・選択・アクセシビリティを保つ境界にしています。

## 構成

| パス | 役割 |
|---|---|
| `app/src/main.rs` | Topcoat構文によるExplorer UI |
| `app/src/explorer.rs` | Rustファイルシステム、履歴、検索、ソート、ShellExecute |
| `crates/topcoat-native-macro` | Topcoat ASTからWinUI 3へのlowering |
| `crates/topcoat-native` | signal/event vocabularyとネイティブtable runtime |

依存する公式実装は再現性のためコミット固定しています。

- Topcoat: `a2bd596af2a149f38fcf49570481f356a6cb1069`
- windows-rs / windows-reactor: `845e42a4328ec5b54b97f965798589d997cad177`

## 実行

Windows 10 1809以降またはWindows 11、Rust 1.95以降、Gitが必要です。Micaの本来の外観はWindows 11で表示されます。

```powershell
cargo run -p topcoat-native-demo
```

直接実行する場合:

```powershell
target\debug\topcoat-native-demo.exe
```

起動時は実行ファイル自身が置かれたフォルダーを表示します。

## 検証済み

- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo fmt --all -- --check`
- 133項目を含む実フォルダーの表示とスクロール
- 検索による133件から2件への絞り込み
- 行選択と「開く」ボタンの有効化
- ダブルクリックによる子フォルダーへの移動
- 戻る履歴による元フォルダーへの復帰
- Mica背景、4列配置、日本語の更新日時表示

## 現時点の境界

これはOSのシェルそのものを置き換える段階ではなく、Explorer型ネイティブUIとRustバックエンドがTopcoatから成立することを示すPoCです。次は次の機能が必要です。

1. コピー、移動、削除、名前変更、新規作成と進行状況UI
2. ごみ箱、Undo、競合確認、権限昇格などの安全なファイル操作モデル
3. タブ、コンテキストメニュー、プレビュー、サムネイル
4. `shell:`名前空間、ネットワーク、OneDrive、WSL、ライブラリ統合
5. Windows Search、変更監視、巨大フォルダーの非同期列挙
6. Windows標準アイコン、ファイルプロパティ、シェル拡張

未対応機能を別のUI経路へ黙ってfallbackする実装はありません。

## License

このPoCのコードはMIT Licenseです。依存プロジェクトには各プロジェクトのライセンスが適用されます。
