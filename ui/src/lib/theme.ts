import { browser } from '$app/environment';
import Blockly from 'blockly';
import { get, writable } from 'svelte/store';

const localTheme = browser && (localStorage.getItem('theme') as 'light' | 'dark');
export const theme = writable<'light' | 'dark'>(localTheme || 'dark');
export const themeIcon = writable<'sun' | 'moon'>(localTheme === 'light' ? 'sun' : 'moon');
export const componentIcon = writable<'radar' | 'check'>('radar');
export function toggleComponent(){
	const newComponentIcon = get(componentIcon) === 'radar' ? 'check' : 'radar';
	componentIcon.set(newComponentIcon);
}
export function toggleDarkMode() {
	if (browser) {
		const newTheme = get(theme) === 'light' ? 'dark' : 'light';
		const newThemeIcon = get(themeIcon) === 'sun' ? 'moon' : 'sun';
		theme.set(newTheme);
		themeIcon.set(newThemeIcon);
		console.log("setting local storage them");
		localStorage.setItem('theme', newTheme);
		const html = document.querySelector('html');
		if (html) html.classList.toggle('dark');
	}
}

export const DarkTheme = Blockly.Theme.defineTheme('dark', {
	base: Blockly.Themes.Classic,
	componentStyles: {
		workspaceBackgroundColour: '#1e1e1e',
		toolboxBackgroundColour: 'blackBackground',
		toolboxForegroundColour: '#fff',
		flyoutBackgroundColour: '#252526',
		flyoutForegroundColour: '#ccc',
		flyoutOpacity: 0.8,
		scrollbarColour: '#797979',
		insertionMarkerColour: '#fff',
		insertionMarkerOpacity: 0.3,
		scrollbarOpacity: 0.4,
		cursorColour: '#d0d0d0'
	},
	name: 'DarkMode'
});
