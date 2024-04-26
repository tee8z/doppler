<script lang="ts">
	import Button from '../components/Button.svelte';
	import Icon from './Icon/Icon.svelte';
	import { browser } from '$app/environment';
	import type { WorkspaceSvg } from 'blockly';
	import Blockly from 'blockly';
	import { javascriptGenerator } from 'blockly/javascript';
	import { onMount } from 'svelte';
	import { initBlocks } from '$lib/blocks';
	import { initGenerators } from '$lib/generators';
	import { toolbox } from '$lib/toolbox';

	export let blocklyTheme: any;
	let workspace: WorkspaceSvg;
	let code = '';
	let copied = false;
	let name = '';

	$: browser && Blockly.getMainWorkspace() && workspace && workspace.setTheme(blocklyTheme);

	function updateCode() {
		code = javascriptGenerator.workspaceToCode(workspace);
	}

	function download() {
		const blob = new Blob([code], { type: 'text/plain' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = `${name}.doppler`;
		a.click();
	}

	async function runFile() {
		console.log(code);
		const blob = new Blob([code], { type: 'text/plain' });
		const formData = new FormData();
		formData.append('dopplerFile', blob, `${name}.doppler`);
		try {
			const response = await fetch('/api/upload', {
				method: 'POST',
				body: formData
			});

			if (!response.ok) {
				throw new Error('Network response was not ok');
			}

			const result = await response.json();
			console.log(result);
		} catch (error) {
			console.error('There was a problem with the fetch operation:', error);
		}
	}

	async function resetCluster() {
		try {
			const response = await fetch('/api/reset', {
				method: 'POST'
			});

			if (!response.ok) {
				throw new Error('Network response was not ok');
			}

			const result = await response.json();
			console.log(result);
		} catch (error) {
			console.error('There was a problem with the fetch operation:', error);
		}
	}

	$: console.log({ blocklyTheme });

	onMount(() => {
		initBlocks();
		initGenerators();
		workspace = Blockly.inject('blockly', { toolbox, trashcan: true, theme: blocklyTheme });
		workspace.addChangeListener(updateCode);
	});
</script>

<!--<div id="blockly" class="w-2/3" />-->
<div class="flex flex-col gap-2 flex-1">
	<div class="flex gap-2">
		<input bind:value={name} placeholder="Enter custom file name" />
		<Button wide on:click={download}>
			<div class="flex justify-center items-center gap-2">Download</div>
		</Button>
		<Button wide on:click={runFile}>
			<div class="flex justify-center items-center gap-2">Run</div>
		</Button>
		<Button wide on:click={resetCluster}>
			<div class="flex justify-center items-center gap-2">Reset</div>
		</Button>
	</div>
	<textarea
		class="w-full h-full bg-green-100 dark:bg-gray-900 outline-1 outline-green-500 rounded-lg"
		id="code"
		bind:value={code}
	/>
</div>
