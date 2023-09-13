import { get, writable } from 'svelte/store';

export const theme = writable<'light' | 'dark'>('dark');
export const themeIcon = writable<'sun' | 'moon'>('sun');

export function toggleDarkMode() {
	const newTheme = get(theme) === 'light' ? 'dark' : 'light';
	const newThemeIcon = get(themeIcon) === 'sun' ? 'moon' : 'sun';
	theme.set(newTheme);
	themeIcon.set(newThemeIcon);
	localStorage.setItem('theme', newTheme);
	const html = document.querySelector('html');
	if (html) html.classList.toggle('dark');
}
