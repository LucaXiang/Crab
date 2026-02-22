import { writable, derived } from 'svelte/store';
import { es } from './translations/es';
import { en } from './translations/en';
import { zh } from './translations/zh';
import { browser } from '$app/environment';

const translations: Record<string, Record<string, string>> = { es, en, zh };

export const SUPPORTED_LANGS = ['es', 'en', 'zh'] as const;
export type Lang = (typeof SUPPORTED_LANGS)[number];
const DEFAULT_LANG: Lang = 'es';
export const LANG_LABELS: Record<Lang, string> = { es: 'ES', en: 'EN', zh: '中文' };

function detectLang(): Lang {
	if (!browser) return DEFAULT_LANG;

	const params = new URLSearchParams(window.location.search);
	const paramLang = params.get('lang');
	if (paramLang && SUPPORTED_LANGS.includes(paramLang as Lang)) return paramLang as Lang;

	const stored = localStorage.getItem('redcoral-lang');
	if (stored && SUPPORTED_LANGS.includes(stored as Lang)) return stored as Lang;

	const browserLang = navigator.language.slice(0, 2);
	if (SUPPORTED_LANGS.includes(browserLang as Lang)) return browserLang as Lang;

	return DEFAULT_LANG;
}

export const locale = writable<Lang>(DEFAULT_LANG);

export function initI18n() {
	locale.set(detectLang());
}

export function setLang(lang: Lang) {
	if (!SUPPORTED_LANGS.includes(lang)) return;
	locale.set(lang);
	if (browser) {
		localStorage.setItem('redcoral-lang', lang);
		document.documentElement.lang = lang;
	}
}

export const t = derived(locale, ($locale) => {
	return (key: string): string => {
		return translations[$locale]?.[key] ?? translations[DEFAULT_LANG]?.[key] ?? key;
	};
});

/**
 * Get a localized error message for an API error code.
 * Falls back to the raw server message if no translation exists.
 */
export function apiErrorMessage(
	translate: (key: string) => string,
	code: number | null,
	fallback: string
): string {
	if (code !== null) {
		const key = `error.${code}`;
		const msg = translate(key);
		if (msg !== key) return msg;
	}
	return fallback;
}
