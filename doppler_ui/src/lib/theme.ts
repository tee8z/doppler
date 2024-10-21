import { browser } from '$app/environment';
import { get, writable } from 'svelte/store';

const localTheme = browser && (localStorage.getItem('theme') as 'light' | 'dark');
export const theme = writable<'light' | 'dark'>(localTheme || 'dark');
export const themeIcon = writable<'sun' | 'moon'>(localTheme === 'light' ? 'sun' : 'moon');
export const componentIcon = writable<'radar' | 'check'>('radar');
export function toggleComponent() {
	const newComponentIcon = get(componentIcon) === 'radar' ? 'check' : 'radar';
	componentIcon.set(newComponentIcon);
}
export function toggleDarkMode() {
	if (browser) {
		const newTheme = get(theme) === 'light' ? 'dark' : 'light';
		const newThemeIcon = get(themeIcon) === 'sun' ? 'moon' : 'sun';
		theme.set(newTheme);
		themeIcon.set(newThemeIcon);
		console.log('setting local storage them');
		localStorage.setItem('theme', newTheme);
		const html = document.querySelector('html');
		if (html) html.classList.toggle('dark');
	}
}
