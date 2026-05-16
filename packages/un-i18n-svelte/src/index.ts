/**
 * un-i18n-svelte: Svelte side of the U.N. series i18n bridge.
 *
 * Paired with the `un-i18n` Rust crate. The Rust side ships locale messages
 * via Tauri commands as one bulk bundle per locale; this package wires those
 * commands into `svelte-i18n` so the rest of the Svelte app can use the
 * regular `$_(key)` / `$_("...", { values: ... })` API with no per-key IPC.
 *
 * 設計の権威ソース: `un-common` repo の `crates/un-i18n/src/lib.rs` 冒頭の
 * ドキュメントコメント。
 */

import { invoke } from "@tauri-apps/api/core";
import { init, locale, register } from "svelte-i18n";

/**
 * Tauri command `i18n_get_svelte_bundle` の戻り値。 `un-i18n` の
 * `SvelteI18nBundle` と 1:1 対応する。
 */
export interface SvelteI18nBundle {
  locale: string;
  messages: Record<string, string>;
}

/**
 * Tauri command 名はアプリ間で揃えてあるが、何かの事情で別名にしたい場合の
 * ためにオーバーライドできるようにしておく。既定値は `un-i18n` の
 * sample 実装に合わせる。
 */
export interface SetupOptions {
  /** Default `"ja-JP"`. svelte-i18n の `fallbackLocale`。 */
  fallbackLocale?: string;
  /** Default `"i18n_available_locales"`. */
  availableLocalesCommand?: string;
  /** Default `"i18n_get_svelte_bundle"`. */
  bundleCommand?: string;
  /** Default `"i18n_resolve_default_locale"`. */
  resolveDefaultCommand?: string;
}

const DEFAULT_OPTIONS: Required<SetupOptions> = {
  fallbackLocale: "ja-JP",
  availableLocalesCommand: "i18n_available_locales",
  bundleCommand: "i18n_get_svelte_bundle",
  resolveDefaultCommand: "i18n_resolve_default_locale",
};

/**
 * svelte-i18n を初期化する。アプリの起動時 (Svelte の `main.ts` 等で) 1 度だけ
 * await すること。`mount(App, ...)` より前に呼ばないと、最初のフレームが
 * fallback locale (typically `ja-JP`) のまま描画されてしまう。
 *
 * 戻り値は実際に適用された初期 locale (`AppRuntimeSettings` の永続化用)。
 */
export async function setupI18n(options: SetupOptions = {}): Promise<string> {
  const opts = { ...DEFAULT_OPTIONS, ...options };

  let supported: string[];
  try {
    supported = await invoke<string[]>(opts.availableLocalesCommand);
  } catch (error) {
    console.error(
      `un-i18n-svelte: failed to list locales via "${opts.availableLocalesCommand}", defaulting to [${opts.fallbackLocale}]`,
      error,
    );
    supported = [opts.fallbackLocale];
  }

  for (const tag of supported) {
    register(tag, async () => {
      const bundle = await invoke<SvelteI18nBundle>(opts.bundleCommand, {
        locale: tag,
      });
      return bundle.messages;
    });
  }

  let initial = opts.fallbackLocale;
  try {
    initial = await invoke<string>(opts.resolveDefaultCommand);
  } catch (error) {
    console.error(
      `un-i18n-svelte: "${opts.resolveDefaultCommand}" failed, using "${opts.fallbackLocale}"`,
      error,
    );
  }

  init({
    fallbackLocale: opts.fallbackLocale,
    initialLocale: initial,
  });

  return initial;
}

/**
 * UI から locale を切り替えるエントリポイント。`AppRuntimeSettings.locale`
 * の永続化は呼び出し側 (典型的には `setAppSetting('locale', tag)`) で行う想定。
 * ここでは svelte-i18n の store だけ更新する。
 *
 * 永続化済設定で `locale = ""` (= "System") を意味する場合は、呼び出し側で
 * 一度 `i18n_resolve_default_locale` を invoke して具体的な BCP-47 タグに
 * 解決してから本関数を呼ぶこと。
 */
export function setUiLocale(tag: string): void {
  locale.set(tag);
}
