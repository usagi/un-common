# un-common

U.N. シリーズアプリ (UN Avatar, UN Motion, 今後追加予定の UN Virtual Eye Tracker
や UN Virtual Avatar Connect など) の **共通基盤を集約する polyglot monorepo**。

Rust crate と TypeScript / Svelte パッケージを 1 つの git repo で管理し、
それぞれ `Cargo workspace` / `npm workspaces` で参照する。

```
un-common/
├── Cargo.toml              # Rust workspace 定義
├── package.json            # npm workspaces 定義
├── crates/
│   └── un-i18n/            # rust-i18n + Tauri command bridge の共通実装
└── packages/
    └── un-i18n-svelte/     # svelte-i18n 側ローダー
```

## 採用ライブラリの方向性

| 用途 | パッケージ | 補足 |
|------|------------|------|
| Rust 側 i18n | [`un-i18n`](crates/un-i18n) | `rust-i18n` v4 + `unic-langid` を内包し、`SvelteI18nBundle` を吐く Tauri command 用バックエンド。 |
| Svelte 側 i18n | [`un-i18n-svelte`](packages/un-i18n-svelte) | `svelte-i18n` の `register` / `init` を Tauri `invoke()` で繋ぐ薄い setup helper。 |

## 開発

### Rust

```bash
cargo build --workspace
cargo test  --workspace
```

### TypeScript / Svelte

```bash
npm install
npm run check      # tsc --noEmit
npm run build      # tsc -p ...
```

## ライセンス

[MIT](LICENSE)
