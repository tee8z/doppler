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

	$: console.log({ blocklyTheme });

	onMount(() => {
		initBlocks();
		initGenerators();
		workspace = Blockly.inject('blockly', { toolbox, trashcan: true, theme: blocklyTheme });
		workspace.addChangeListener(updateCode);
	});
</script>

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
