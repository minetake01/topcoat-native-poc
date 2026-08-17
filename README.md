# Topcoat Native WinUI 3 PoC

Topcoat の `view!` 記法を維持しつつ、DOM の代わりに本物の WinUI 3 コントロールを生成する最小実証です。アプリ、状態管理、イベント処理は Rust で動作し、WebView は使いません。

## 実証したこと

- Topcoat 公式の `topcoat-view-grammar` で既存の `view! { cx => ... }` 構文を解析する
- `signal`、Rust 式、`if`、`for`、`let` をそのまま Rust のリアクティブ描画へ lowering する
- HTML 風の要素を WinUI 3 の `StackPanel`、`TextBlock`、`TextBox`、`Button` に変換する
- `@click` と `@input` をネイティブイベントへ接続し、signal 更新時に UI を再調停する
- WinUI 3 の `MicaBackdrop` をウィンドウ背景へ適用する
- 未対応の DOM/CSS 機能は黙って近似せず、コンパイル時にエラーにする

動作例は [`app/src/main.rs`](app/src/main.rs) です。UI のソースは次のように Topcoat の形を保っています。

```rust
view! { cx =>
    signal count = 0_i32;
    signal name = String::from("Topcoat");

    <main class="p-6 space-y-4">
        <input
            type="text"
            :value=$(name.get())
            @input=$(|event: Event| name.set(event.target.value))
        >
        <p>(format!("Hello, {}. Count: {}", name.get(), count.get()))</p>
        <button @click=$(|_event: Event| count.set(count.get() + 1))>
            "Increment"
        </button>
    </main>
}
```

## 設計

```text
Topcoat view! source
        |
        v
topcoat-view-grammar (upstream parser, pinned commit)
        |
        v
topcoat-native-macro (WinUI lowering, this PoC)
        |
        v
windows-reactor + Windows App SDK / WinUI 3
        |
        v
native Windows controls
```

Topcoat の合意形成で得られた表面言語を fork して再発明せず、公式 parser の公開 AST をレンダラー境界として利用しています。独自部分は AST から WinUI 3 への変換だけです。

依存リビジョンは再現性のため固定しています。

- Topcoat: `a2bd596af2a149f38fcf49570481f356a6cb1069`
- windows-rs / windows-reactor: `845e42a4328ec5b54b97f965798589d997cad177`

## 対応表

| Topcoat 側 | WinUI 3 側 |
|---|---|
| `signal` | `windows_reactor::SetState<T>` |
| `main`, `div` | `StackPanel` |
| `h1`, `h2`, `h3`, `p`, `span`, `label` | `TextBlock` |
| `input type="text"` | `TextBox` |
| `button` | `Button` |
| `@click` | `Button::click` callback |
| `@input`, `@change` | `TextBox::text_changed` callback |
| `if`, `for`, `let`, Rust 式 | Rust の制御フローと式 |
| `id`, `aria-label`, `title` | Automation ID、Automation Name、ToolTip |
| `flex`, `flex-row`, `flex-col`, `gap-N`, `space-x-N`, `space-y-N`, `p-N` | `StackPanel` の向き、間隔、padding |

## 実行

必要なものは Windows 10 1809 以降または Windows 11、Rust 1.95 以降、Git、ネットワーク接続です。初回ビルドでは固定リビジョンの Git 依存を取得します。

```powershell
cargo run -p topcoat-native-demo
```

ウィンドウ背景には `Backdrop::Mica` を指定しています。Mica を隠さないよう、Topcoat から生成するルートパネルには不透明な背景を設定していません。Windows 11 では壁紙色を取り込んだ Windows 設定に近い外観になり、OS の視覚効果・アクセシビリティ方針に応じた表示制御は WinUI 3 が担当します。

確認済みの操作:

1. `Increment` で `Count: 0` から更新される
2. 3 回目で Rust の `if` による条件テキストが現れる
3. 名前欄を変更すると `Hello, ...` が即時更新される
4. `Reset` でカウントと条件表示が戻る

## PoC の境界

対応していないものは、Topcoat component 呼び出し、動的タグ/属性名、raw JavaScript handler、任意 CSS、DOM API、`match`、DOCTYPE です。これらを WebView や隠れた fallback に流す経路はありません。使用すると proc macro がコンパイルエラーを返します。

本格実装に進む場合は、次の順序が自然です。

1. Topcoat AST と各ネイティブ renderer の間に、renderer-neutral な UI HIR を定義する
2. component、keyed list、focus、validation、accessibility の意味論を固定する
3. WinUI 3 renderer の control/property/event 対応を拡張する
4. 同じ HIR に SwiftUI と Jetpack Compose renderer を追加する
5. compile-fail test とネイティブ UI 自動テストを整備する

これは Topcoat 本体の fork ではなく、レンダラー交換が成立するかを確かめる独立 PoC です。

## License

この PoC のコードは MIT License です。依存プロジェクトには各プロジェクトのライセンスが適用されます。
