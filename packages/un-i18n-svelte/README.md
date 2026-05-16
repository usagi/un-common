# un-i18n-svelte

U.N. シリーズアプリの **Svelte 側 i18n bridge**。 Rust 側の
[`un-i18n`](../../crates/un-i18n) crate が Tauri command 経由で配信する
bulk-loaded メッセージを `svelte-i18n` に登録する setup helpers。

## インストール

```bash
npm add "@usagi.network/un-i18n-svelte"
# peer deps（未導入なら）
npm add @tauri-apps/api svelte-i18n
```

## 使い方

```ts
// src/main.ts
import { mount } from "svelte";
import App from "./App.svelte";
import { setupI18n } from "@usagi.network/un-i18n-svelte";

await setupI18n();

mount(App, { target: document.getElementById("app")! });
```

`App.svelte` 内では通常の `svelte-i18n` API を使う:

```svelte
<script lang="ts">
  import { _ } from "svelte-i18n";
  import { setUiLocale } from "@usagi.network/un-i18n-svelte";
</script>

<h1>{$_("app.name")}</h1>
<button onclick={() => setUiLocale("en-US")}>English</button>
```

## API

### `setupI18n(options?: SetupOptions): Promise<string>`

`svelte-i18n` を初期化する。

- 戻り値: 実際に適用された初期 locale (BCP-47 完全形)。
- アプリの `mount(App, ...)` より **必ず前** に await すること。

`SetupOptions`:

| キー | 既定値 | 説明 |
|------|--------|------|
| `fallbackLocale` | `"ja-JP"` | svelte-i18n の `fallbackLocale` (UN シリーズの設計上、作者の第 1 言語)。 |
| `availableLocalesCommand` | `"i18n_available_locales"` | Tauri command 名。 |
| `bundleCommand` | `"i18n_get_svelte_bundle"` | Tauri command 名。 |
| `resolveDefaultCommand` | `"i18n_resolve_default_locale"` | Tauri command 名。 |

### `setUiLocale(tag: string): void`

UI から locale を切り替える。`svelte-i18n` の `locale` store を更新する
だけのシンプルなラッパー。`AppRuntimeSettings.locale` の永続化は呼び出し側
で行う想定。

## UN `apps/*-supervisor` での取り込み

- **Rust** (`un-i18n`): `Cargo.toml` で `git = "https://github.com/usagi/un-common.git"` 等（リリースブランチでは `rev` でピン留め推奨）。
- **npm**: レジストリの [`@usagi.network/un-i18n-svelte`](https://www.npmjs.com/package/@usagi.network/un-i18n-svelte) を `dependencies` に追加し、`main.ts` / `App.svelte` から上記スコープ名で import する。

## ライセンス

[MIT](../../LICENSE)
