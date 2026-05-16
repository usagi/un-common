# un-i18n

U.N. シリーズアプリ (UN Motion / UN Avatar / 将来の UN Virtual Eye Tracker 等)
で共有する **Rust 中央集権型 i18n bridge** の中核 crate。

設計の権威ソースは `crates/un-i18n/src/lib.rs` の冒頭ドキュメントコメント。
要点だけ抜粋すると:

| Layer | 動き |
|-------|------|
| 原本 | アプリの `locales/{ja-JP,en-US,…}.toml` 1 セットだけ。 |
| Rust 側 | `rust_i18n::i18n!(..., backend = un_i18n::UnI18nStore::…)` で compile-time に取り込み、`t!()` で消費。 |
| Svelte 側 | `register(locale, async loader)` の loader 内部から **1 度だけ** Tauri command を叩いて bulk 取得。`$_(key)` 評価ごとに IPC は走らない。 |
| プレースホルダー | rust-i18n `%{name}` を bridge が regex で svelte-i18n `{name}` に変換。 |
| locale | BCP-47 完全形 (`ja-JP` / `en-US`)。OS 自動検出は language タグ一致でフォールバック、最終 fallback はアプリ指定 (UN シリーズ既定 `ja-JP`)。 |

## アプリ側の標準パターン

```rust
use std::sync::LazyLock;
use un_i18n::{UnI18nStore, SvelteI18nBundle, resolve_default_locale};

pub static UN_I18N_STORE: LazyLock<UnI18nStore> = LazyLock::new(|| {
    let mut store = UnI18nStore::new();
    store.add_locale_toml("ja-JP", include_str!("../locales/ja-JP.toml"));
    store.add_locale_toml("en-US", include_str!("../locales/en-US.toml"));
    store
});

#[tauri::command]
pub fn i18n_get_svelte_bundle(locale: String) -> Result<SvelteI18nBundle, String> {
    UN_I18N_STORE
        .svelte_bundle(&locale)
        .ok_or_else(|| format!("i18n: locale '{locale}' is not loaded"))
}

#[tauri::command]
pub fn i18n_available_locales() -> Vec<String> {
    UN_I18N_STORE.available_locales()
}

#[tauri::command]
pub fn i18n_resolve_default_locale() -> String {
    resolve_default_locale(&UN_I18N_STORE, "ja-JP")
}
```

そして `src/lib.rs` の先頭で:

```rust
rust_i18n::i18n!("locales", fallback = "ja-JP", backend = (*crate::i18n::UN_I18N_STORE).clone());
```

を宣言する。`UnI18nStore` は `Clone` なので 1 度ロードした store を rust-i18n
に渡しつつ、Tauri command からも `&*UN_I18N_STORE` で参照できる。

## 関連パッケージ

| Package | 言語 | 役割 |
|---------|------|------|
| [`un-i18n`](.) | Rust | 本 crate。TOML → flatten → bundle、`rust_i18n::Backend` 実装、locale 解決。 |
| [`un-i18n-svelte`](../../packages/un-i18n-svelte) | TypeScript / Svelte | `un-i18n` の bundle を `svelte-i18n` に登録する setup helpers。 |

## ライセンス

[MIT](../../LICENSE)
