<script lang="ts">
	import Blockly from 'blockly';
	import { theme, themeIcon, toggleDarkMode } from '$lib/theme';
	import type { WorkspaceSvg } from 'blockly';
	import Icon from '../components/Icon/Icon.svelte';
	import { onMount } from 'svelte';
	import { initBlocks } from '$lib/blocks';
	import { initGenerators } from '$lib/generators';
	import { javascriptGenerator } from 'blockly/javascript';
	import { toolbox } from '$lib/toolbox';

	let workspace: WorkspaceSvg;
	let code = '';

	function updateCode() {
		code = javascriptGenerator.workspaceToCode(workspace);
	}

	onMount(() => {
		initBlocks();
		initGenerators();

		workspace = Blockly.inject('blockly', { toolbox, trashcan: true });
		workspace.addChangeListener(updateCode);

		Blockly.Theme.defineTheme('dark', {
			base: Blockly.Themes.Classic,
			componentStyles: {
				workspaceBackgroundColour: '#1e1e1e',
				toolboxBackgroundColour: 'blackBackground',
				toolboxForegroundColour: '#fff',
				flyoutBackgroundColour: '#252526',
				flyoutForegroundColour: '#ccc',
				flyoutOpacity: 1,
				scrollbarColour: '#797979',
				insertionMarkerColour: '#fff',
				insertionMarkerOpacity: 0.3,
				scrollbarOpacity: 0.4,
				cursorColour: '#d0d0d0'
			},
			name: ''
		});
	});
</script>

<main class="flex flex-col h-screen">
	<section class="flex justify-between items-center m-4">
		<h1>Doppler</h1>
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		<!-- svelte-ignore a11y-no-static-element-interactions -->
		<span class="cursor-pointer" on:click={toggleDarkMode}><Icon name={$themeIcon} /></span>
	</section>
	<section class="flex h-full">
		<div id="blockly" class="w-2/3" />
		<div class="flex flex-col gap-2 flex-1">
			<button class="bg-green-200 p-2 rounded-lg" on:click={updateCode}>Update</button>
			<textarea class="w-full h-full bg-green-100 rounded-lg" id="code">{code}</textarea>
		</div>
	</section>
</main>
