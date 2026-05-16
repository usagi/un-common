//! Shared i18n bridge for U.N. series applications.
//!
//! # 設計サマリ
//!
//! 各 U.N. シリーズ Tauri アプリ (UN Motion, UN Avatar, 将来の UN Virtual
//! Eye Tracker などなど) で同じパターンを実装している。本クレートはその
//! 共通部分をライブラリ化する。
//!
//! - **原本**: アプリ側に置く `locales/{ja-JP,en-US,...}.toml` 1 セット。
//! - **Rust 側**: `rust_i18n::i18n!(..., backend = un_i18n::UnI18nStore::...)`
//!   で TOML を compile-time に取り込み、`t!()` マクロ / tray menu /
//!   notification 等で消費する。
//! - **Svelte 側 (`svelte-i18n`)**: Tauri command で **同じデータを 1 度だけ
//!   bulk 送信** し、`register(locale, async loader)` の loader 内部から
//!   1 度だけ invoke する。`$_(key)` 評価ごとに IPC は発生しない。
//! - **プレースホルダー**: rust-i18n syntax `%{name}` で記述。bridge が
//!   svelte-i18n syntax `{name}` に regex で変換してから配信する。
//! - **locale 識別子**: BCP-47 完全形 (`ja-JP` / `en-US`)。OS 自動検出は
//!   `sys-locale` を `unic-langid` で正規化し、サポート一覧の language タグ
//!   一致でフォールバック、それでも当たらない場合はアプリ指定の fallback
//!   (UN シリーズ既定では `ja-JP` = 作者の第 1 言語)。
//!
//! # 典型的な利用例
//!
//! ```ignore
//! use std::sync::LazyLock;
//! use un_i18n::{UnI18nStore, SvelteI18nBundle, resolve_default_locale};
//!
//! pub static UN_I18N_STORE: LazyLock<UnI18nStore> = LazyLock::new(|| {
//!     let mut store = UnI18nStore::new();
//!     store.add_locale_toml("ja-JP", include_str!("../locales/ja-JP.toml"));
//!     store.add_locale_toml("en-US", include_str!("../locales/en-US.toml"));
//!     store
//! });
//!
//! // 各アプリの src/lib.rs で:
//! //   rust_i18n::i18n!("locales", fallback = "ja-JP",
//! //                     backend = (*crate::i18n::UN_I18N_STORE).clone());
//! ```

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::LazyLock;

use serde::Serialize;
use unic_langid::LanguageIdentifier;

/// rust-i18n syntax `%{name}` から svelte-i18n syntax `{name}` への 1 対 1
/// 変換用 regex。識別子文字 (`[A-Za-z_][A-Za-z0-9_]*`) のみ対象で、
/// それ以外 (`%{ }` や `%{a-b}` のような不正な書き方) はそのまま通す。
static PLACEHOLDER_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
	regex::Regex::new(r"%\{([A-Za-z_][A-Za-z0-9_]*)\}").expect("placeholder regex must compile")
});

/// rust-i18n のメッセージ文字列を svelte-i18n が期待する形に変換する。
///
/// ```
/// assert_eq!(un_i18n::rust_i18n_placeholder_to_svelte("Hello, %{name}"), "Hello, {name}");
/// ```
pub fn rust_i18n_placeholder_to_svelte(input: &str) -> String {
	PLACEHOLDER_RE.replace_all(input, "{$1}").into_owned()
}

/// 各 locale TOML を読み込み、`(locale, flatten 済 key → value)` を保持する
/// in-memory store。出力 JSON の決定性を担保するため二段とも `BTreeMap`。
#[derive(Debug, Default, Clone)]
pub struct UnI18nStore {
	locales: BTreeMap<String, BTreeMap<String, String>>,
}

impl UnI18nStore {
	pub fn new() -> Self {
		Self::default()
	}

	/// TOML 文字列を 1 locale 分追加する。fail-soft: パースエラー時は
	/// 当該 locale を追加せず `tracing::error!` だけ吐いて呼び出し元に返す。
	/// (大量の locale がある場合、1 つ壊れても他は使えるべきとの判断)
	pub fn add_locale_toml(&mut self, locale: &str, raw: &str) -> &mut Self {
		match toml::from_str::<toml::Value>(raw) {
			Ok(value) => {
				let mut flat = BTreeMap::new();
				flatten_toml(&value, String::new(), &mut flat);
				self.locales.insert(locale.to_string(), flat);
			},
			Err(error) => {
				tracing::error!(locale = locale, %error, "un-i18n: failed to parse locale TOML; skipping");
			},
		}
		self
	}

	/// 現在ロード済の locale 一覧を `BTreeMap` の order (≒辞書順) で返す。
	pub fn available_locales(&self) -> Vec<String> {
		self.locales.keys().cloned().collect()
	}

	/// 指定 locale の全メッセージ (flatten 済 key → value)。
	pub fn messages_for_locale(&self, locale: &str) -> Option<&BTreeMap<String, String>> {
		self.locales.get(locale)
	}

	pub fn has_locale(&self, locale: &str) -> bool {
		self.locales.contains_key(locale)
	}

	/// `ja-JP` のような完全形が無いが `ja` 等の language tag だけ一致する
	/// locale があれば、その完全形を返す。OS 自動検出時のフォールバック用。
	pub fn match_by_language(&self, langid: &LanguageIdentifier) -> Option<String> {
		let target_lang = langid.language.as_str();
		for known in self.locales.keys() {
			if let Ok(parsed) = known.parse::<LanguageIdentifier>()
				&& parsed.language.as_str() == target_lang
			{
				return Some(known.clone());
			}
		}
		None
	}

	/// Svelte 側にそのまま送り返せる JSON 互換 payload を組み立てる。
	/// プレースホルダー変換も済んでいるので、呼び出し元は invoke の
	/// 戻り値をそのまま `register(locale, async () => bundle.messages)` に
	/// 渡せばよい。
	pub fn svelte_bundle(&self, locale: &str) -> Option<SvelteI18nBundle> {
		let raw = self.messages_for_locale(locale)?;
		let messages = raw.iter().map(|(k, v)| (k.clone(), rust_i18n_placeholder_to_svelte(v))).collect();
		Some(SvelteI18nBundle { locale: locale.to_string(), messages })
	}
}

/// nested TOML を `dot.path` flatten key に正規化する。`_version` などのメタ
/// キー (アンダースコア始まり) は除外。
fn flatten_toml(value: &toml::Value, prefix: String, out: &mut BTreeMap<String, String>) {
	match value {
		toml::Value::Table(table) => {
			for (key, v) in table {
				if key.starts_with('_') {
					continue;
				}
				let next = if prefix.is_empty() { key.clone() } else { format!("{prefix}.{key}") };
				flatten_toml(v, next, out);
			}
		},
		toml::Value::String(s) => {
			out.insert(prefix, s.clone());
		},
		_ => {
			// Number / Bool / Array / DateTime 等は i18n message として
			// 意味を持たないので debug ログだけ吐いて捨てる。
			tracing::debug!(key = %prefix, "un-i18n: ignoring non-string i18n value");
		},
	}
}

/// rust-i18n の custom backend として `i18n!()` マクロに渡せるようにする実装。
/// これにより `t!()` マクロが見るメッセージと Tauri command が返すメッセージが
/// 完全に同一インスタンスを共有する。
impl rust_i18n::Backend for UnI18nStore {
	fn available_locales(&self) -> Vec<Cow<'_, str>> {
		self.locales.keys().map(|s| Cow::Borrowed(s.as_str())).collect()
	}

	fn translate(&self, locale: &str, key: &str) -> Option<Cow<'_, str>> {
		self.locales.get(locale)?.get(key).map(|v| Cow::Borrowed(v.as_str()))
	}

	fn messages_for_locale(&self, locale: &str) -> Option<Vec<(Cow<'_, str>, Cow<'_, str>)>> {
		self.locales.get(locale).map(|m| m.iter().map(|(k, v)| (Cow::Borrowed(k.as_str()), Cow::Borrowed(v.as_str()))).collect())
	}
}

/// `svelte-i18n` の `register(locale, loader)` 経由で受け取るペイロード。
/// `messages` の中身は既に rust-i18n の `%{name}` → svelte-i18n の `{name}`
/// 変換が済んでいる。
#[derive(Debug, Clone, Serialize)]
pub struct SvelteI18nBundle {
	pub locale: String,
	pub messages: BTreeMap<String, String>,
}

/// アプリ側の永続設定で `locale = ""` (system 自動) になっていた場合に
/// 解決すべき具体的な locale を返す:
///
/// 1. OS locale を `sys-locale::get_locale()` で取得。
/// 2. store に同じ完全形が登録されていればそれを返す (例: OS = `en-US`、
///    store に `en-US` 登録あり → `en-US`)。
/// 3. 完全形が無くても language タグだけ一致するエントリがあればそれを使う
///    (例: OS = `ja-Hira`、store に `ja-JP` 登録あり → `ja-JP`)。
/// 4. それでも当たらなければ `fallback` (典型的には `"ja-JP"`)。
pub fn resolve_default_locale(store: &UnI18nStore, fallback: &str) -> String {
	let Some(raw) = sys_locale::get_locale() else {
		return fallback.to_string();
	};
	if store.has_locale(&raw) {
		return raw;
	}
	match raw.parse::<LanguageIdentifier>() {
		Ok(langid) => store.match_by_language(&langid).unwrap_or_else(|| fallback.to_string()),
		Err(_) => fallback.to_string(),
	}
}

/// rust-i18n のグローバル locale を更新するだけの薄いヘルパー。アプリの
/// `apply_locale` がいくつもの場所から呼ばれる動線になっている都合上、
/// `un_i18n::apply_locale` という 1 つの API に集約しておくと grep しやすい。
pub fn apply_locale(locale: &str) {
	rust_i18n::set_locale(locale);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn rust_i18n_placeholders_convert_to_svelte_braces() {
		assert_eq!(rust_i18n_placeholder_to_svelte("Hello, %{name}"), "Hello, {name}");
		assert_eq!(rust_i18n_placeholder_to_svelte("Score: %{value}/%{max}"), "Score: {value}/{max}");
		// 識別子に使えない文字 (`-` など) は変換しない。
		assert_eq!(rust_i18n_placeholder_to_svelte("plain %{ } %{a-b}"), "plain %{ } %{a-b}");
	}

	#[test]
	fn flatten_nested_toml_keys() {
		let raw = "[a]\nx = \"X\"\n[a.b]\ny = \"Y\"\n";
		let v: toml::Value = toml::from_str(raw).unwrap();
		let mut out = BTreeMap::new();
		flatten_toml(&v, String::new(), &mut out);
		assert_eq!(out.get("a.x").map(String::as_str), Some("X"));
		assert_eq!(out.get("a.b.y").map(String::as_str), Some("Y"));
	}

	#[test]
	fn version_key_is_excluded() {
		let raw = "_version = 1\n[a]\nx = \"X\"\n";
		let v: toml::Value = toml::from_str(raw).unwrap();
		let mut out = BTreeMap::new();
		flatten_toml(&v, String::new(), &mut out);
		assert_eq!(out.len(), 1);
		assert!(out.contains_key("a.x"));
	}

	#[test]
	fn svelte_bundle_round_trip() {
		let mut store = UnI18nStore::new();
		store.add_locale_toml("ja-JP", "greet = \"Hello, %{name}\"\n");
		let bundle = store.svelte_bundle("ja-JP").unwrap();
		assert_eq!(bundle.locale, "ja-JP");
		assert_eq!(bundle.messages.get("greet").map(String::as_str), Some("Hello, {name}"));
	}

	#[test]
	fn match_by_language_falls_back_to_full_form() {
		let mut store = UnI18nStore::new();
		store.add_locale_toml("ja-JP", "greet = \"こんにちは\"\n");
		let langid: LanguageIdentifier = "ja-Hira".parse().unwrap();
		assert_eq!(store.match_by_language(&langid).as_deref(), Some("ja-JP"));
	}

	#[test]
	fn resolve_default_locale_uses_fallback_when_store_empty() {
		let store = UnI18nStore::new();
		// OS locale に関係なく fallback が返る (store にも何も無いため)。
		assert_eq!(resolve_default_locale(&store, "ja-JP"), "ja-JP");
	}
}
