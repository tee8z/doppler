<script lang="ts">
	import Blockly from 'blockly';
	import { DarkTheme, theme, themeIcon, toggleDarkMode } from '$lib/theme';
	import type { WorkspaceSvg } from 'blockly';
	import Icon from '../components/Icon/Icon.svelte';
	import Button from '../components/Button.svelte';
	import Visualizer from '../components/Visualizer.svelte';
	import { onMount } from 'svelte';
	import { initBlocks } from '$lib/blocks';
	import { initGenerators } from '$lib/generators';
	import { javascriptGenerator } from 'blockly/javascript';
	import { toolbox } from '$lib/toolbox';
	import { browser } from '$app/environment';

	let workspace: WorkspaceSvg;
	let code = '';
	let copied = false;

	$: console.log({ blocklyTheme, $theme });

	$: blocklyTheme = $theme === 'light' ? Blockly.Themes.Classic : DarkTheme;
	$: browser && Blockly.getMainWorkspace() && workspace && workspace.setTheme(blocklyTheme);

	function updateCode() {
		code = javascriptGenerator.workspaceToCode(workspace);
	}

	function copy() {
		navigator.clipboard.writeText(code);
		copied = true;
		setTimeout(() => (copied = false), 2000);
	}

	function download() {
		const blob = new Blob([code], { type: 'text/plain' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = 'custom.doppler';
		a.click();
	}

	onMount(() => {
		initBlocks();
		initGenerators();
		workspace = Blockly.inject('blockly', { toolbox, trashcan: true, theme: blocklyTheme });
		workspace.addChangeListener(updateCode);
	});
</script>

<main class="flex flex-col h-screen">
	<section class="flex flex-col justify-between items-center m-4 md:flex-row">
		<div class="flex items-center gap-2">
			<h1>Doppler</h1>
			<Icon name="radar" class="h-6 w-6" />
		</div>
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		<!-- svelte-ignore a11y-no-static-element-interactions -->
		<div class="flex items-center gap-2">
			<a href="https://github.com/tee8z/doppler" class="h-6 w-6"><Icon name="octocat" /></a>
			<span class="cursor-pointer h-6 w-6" on:click={toggleDarkMode}><Icon name={$themeIcon} /></span>
		</div>
	</section>
	<section class="flex flex-1">
		<Visualizer></Visualizer>
	</section>
	<!--
	<section class="flex h-full gap-2">
		<div id="blockly" class="w-2/3" />
		<div class="flex flex-col gap-2 flex-1">
			<div class="flex gap-2">
				<Button wide on:click={copy}>
					<div class="flex justify-center items-center gap-2">
						{#if copied}
							<Icon name="check" />
						{:else}
							<Icon name="copy" />
						{/if}
					</div>
				</Button>
				<Button wide on:click={download}>
					<div class="flex justify-center items-center gap-2">
						<Icon name="download" />
					</div>
				</Button>
			</div>
			<textarea
				class="w-full h-full bg-green-100 dark:bg-gray-900 outline-1 outline-green-500 rounded-lg"
				id="code">{code}</textarea
			>
		</div>
	</section>
-->
</main>