<script lang="ts">
	import Blockly from 'blockly';
	import { DarkTheme, theme, themeIcon, toggleDarkMode } from '$lib/theme';
	import type { WorkspaceSvg } from 'blockly';
	import Icon from '../components/Icon/Icon.svelte';
	import { onMount } from 'svelte';
	import { initBlocks } from '$lib/blocks';
	import { initGenerators } from '$lib/generators';
	import { javascriptGenerator } from 'blockly/javascript';
	import { toolbox } from '$lib/toolbox';
	import { browser } from '$app/environment';

	let workspace: WorkspaceSvg;
	let code = '';

	$: blocklyTheme = $theme === 'dark' ? DarkTheme : Blockly.Themes.Classic;
	$: browser && Blockly.getMainWorkspace() && workspace.setTheme(blocklyTheme);

	function updateCode() {
		code = javascriptGenerator.workspaceToCode(workspace);
	}

	onMount(() => {
		initBlocks();
		initGenerators();
		workspace = Blockly.inject('blockly', { toolbox, trashcan: true, theme: blocklyTheme});
		workspace.addChangeListener(updateCode);
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
			<textarea class="w-full h-full bg-green-100 dark:bg-gray-900 outline-1 outline-green-500 rounded-lg" id="code">{code}</textarea>
		</div>
	</section>
</main>
