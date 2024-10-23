<script lang="ts">
	import { onMount, createEventDispatcher, onDestroy } from 'svelte';
	import { v7 } from 'uuid';
	import { writable } from 'svelte/store';
	import { slide } from 'svelte/transition';
	import path from 'path-browserify';
	import TreeView from './TreeView.svelte';

	const dispatch = createEventDispatcher();

	let scriptContent: string = '';
	let scriptPath: string = '';
	let scriptName: string = '';
	let scriptId: string = '';
	let isSubmitting: boolean = false;
	let isLoading: boolean = false;
	let submitResult: string = '';
	let isTreeExpanded: boolean = true;
	let selectedScript: any = null;

	const treeStore = writable(null);

	let tree: any;
	const unsubscribe = treeStore.subscribe((value) => {
		tree = value;
	});

	function emitCurrentScriptId() {
		dispatch('scriptSubmitted', scriptId);
	}

	async function updateTree() {
		isLoading = true;
		try {
			const response = await fetch('/api/scripts');
			if (!response.ok) {
				throw new Error('Failed to fetch folder tree');
			}
			const newTree = await response.json();
			console.log('Fetched tree data:', newTree);
			if (newTree && newTree.children && newTree.children.length > 0) {
				treeStore.set(newTree);
			}
		} catch (e) {
			console.error('Error fetching folder tree:', e);
			submitResult = 'Error loading script library. Please try again.';
		} finally {
			isLoading = false;
		}
	}

	async function fetchScriptContent(scriptPath: string): Promise<string> {
		const url = `/api/download/?scriptPath=${encodeURIComponent(scriptPath)}`;
		const response = await fetch(url);
		if (!response.ok) {
			throw new Error('Failed to fetch script content');
		}
		return await response.text();
	}

	async function saveScript(): Promise<string> {
		const id = scriptId || v7();
		const fileName = `${scriptName}_${id}.doppler`;
		const fullPath = path.join(scriptPath, fileName);

		try {
			const response: Response = await fetch('/api/save', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify({ id, fullPath, script: scriptContent })
			});
			if (response.ok) {
				await updateTree();
				scriptId = id;
				emitCurrentScriptId();
				return id;
			} else {
				throw new Error('Failed to save script');
			}
		} catch (error) {
			console.error('Error:', error);
			throw error;
		}
	}

	async function runScript(id: string): Promise<void> {
		const fileName = `${scriptName}_${id}.doppler`;
		const fullPath = path.join(scriptPath, fileName);

		try {
			const response: Response = await fetch('/api/run', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify({ id, fullPath })
			});
			if (response.ok) {
				submitResult = 'Script execution started in the background';
				dispatch('scriptRun', id);
			} else if (response.status === 404) {
				console.warn(response.text);
				submitResult = 'Error: Script not found. It may have been deleted or moved.';
			} else {
				throw new Error('Failed to run script');
			}
		} catch (error) {
			console.error('Error:', error);
			submitResult = 'An error occurred while trying to run the script.';
		}
	}

	async function handleSave(): Promise<void> {
		isSubmitting = true;
		submitResult = '';
		try {
			scriptId = await saveScript();
			submitResult = 'Script saved successfully!';
		} catch (error) {
			submitResult = 'Error saving script. Please try again.';
		} finally {
			isSubmitting = false;
		}
	}

	async function handleRun(): Promise<void> {
		isSubmitting = true;
		submitResult = '';
		try {
			const id = await saveScript();
			await runScript(id);
		} catch (error) {
			submitResult = 'Error running script. Please try again.';
		} finally {
			isSubmitting = false;
		}
	}

	function toggleTree() {
		isTreeExpanded = !isTreeExpanded;
	}

	async function handleScriptSelect(event: CustomEvent) {
		selectedScript = event.detail;
		const fullPath = selectedScript.fullPath;
		const parsedPath = path.parse(fullPath);
		scriptPath = parsedPath.dir;
		const lastUnderscoreIndex = parsedPath.name.lastIndexOf('_');
		scriptName = parsedPath.name.substring(0, lastUnderscoreIndex);
		scriptId = parsedPath.name.substring(lastUnderscoreIndex + 1).replace('.doppler', '') || '';
		emitCurrentScriptId(); // Emit the current script ID when a script is selected
		isLoading = true;
		try {
			scriptContent = await fetchScriptContent(fullPath);
			submitResult = 'Script loaded successfully!';
		} catch (error) {
			console.error('Error fetching script content:', error);
			submitResult = 'Error loading script. Please try again.';
		} finally {
			isLoading = false;
		}
	}

	onMount(async () => {
		scriptContent = '// Write your custom script here\n';
		await updateTree();
	});

	onDestroy(unsubscribe);
</script>

<main class="dark:bg-gray-900 dark:text-white">
	<h1 class="text-green-600 dark:text-green-400">Doppler Script Editor</h1>
	<div class="editor-layout">
		<div class="script-editor">
			<div class="editor-nav">
				<div class="script-info">
					<input
						bind:value={scriptPath}
						placeholder="Enter script folder path (e.g., folder/subfolder)"
						type="text"
						class="input-field"
					/>
					<input
						bind:value={scriptName}
						placeholder="Enter script name (without _id.doppler)"
						type="text"
						class="input-field"
					/>
				</div>
				<div class="button-group">
					<button on:click={handleRun} disabled={isSubmitting || isLoading} class="run-btn">
						{isSubmitting ? 'Running...' : 'Run'}
					</button>
					<button on:click={handleSave} disabled={isSubmitting || isLoading} class="save-btn">
						{isSubmitting ? 'Saving...' : 'Save'}
					</button>
				</div>
			</div>
			<textarea
				bind:value={scriptContent}
				placeholder="Write your custom doppler script here..."
				disabled={isLoading}
				class="script-textarea"
			></textarea>
			{#if submitResult}
				<p class="submit-result">{submitResult}</p>
			{/if}
			{#if isLoading}
				<p class="loading-message">Loading script...</p>
			{/if}
		</div>
		<div class="tree-view" class:expanded={isTreeExpanded}>
			<div class="tree-header">
				<button on:click={toggleTree} class="toggle-btn">
					{#if isTreeExpanded}
						&raquo;
					{:else}
						&laquo;
					{/if}
				</button>
				<h2 class="dark:text-green-400">Script Library</h2>
			</div>
			{#if isTreeExpanded}
				<div transition:slide>
					{#if $treeStore}
						<TreeView tree={$treeStore} on:select={handleScriptSelect} />
					{:else if isLoading}
						<p class="loading">Loading script library...</p>
					{:else}
						<p class="no-scripts">No scripts found. Create a new script to get started.</p>
					{/if}
				</div>
			{/if}
		</div>
	</div>
</main>

<style lang="postcss">
	main {
		@apply max-w-none m-0 p-5 h-screen box-border flex flex-col;
	}

	.editor-layout {
		@apply flex gap-0 flex-1 overflow-hidden;
	}

	.tree-view {
		@apply flex-none w-[250px] transition-[flex-basis] duration-300 overflow-hidden;
		@apply border-l border-gray-300 dark:border-gray-600;
	}

	.tree-view:not(.expanded) {
		@apply w-[30px];
	}

	.script-editor {
		@apply flex-1 flex flex-col overflow-hidden;
	}

	.editor-nav {
		@apply flex justify-between p-2.5;
		@apply bg-gray-100 dark:bg-gray-800;
		@apply border-b border-gray-300 dark:border-gray-600;
	}

	.script-info {
		@apply flex-1 mr-2.5;
	}

	.input-field {
		@apply w-full mb-1.5 p-2 rounded;
		@apply bg-white dark:bg-gray-700;
		@apply border border-gray-300 dark:border-gray-600;
		@apply text-gray-900 dark:text-white;
		@apply focus:outline-none focus:ring-2 focus:ring-green-400 dark:focus:ring-green-500;
		@apply transition-colors;
	}

	.button-group {
		@apply flex flex-col gap-1.5;
	}

	.save-btn,
	.run-btn {
		@apply p-2 text-sm border-none rounded cursor-pointer transition-colors w-[100px];
		@apply text-white;
	}

	.save-btn {
		@apply bg-green-600 hover:bg-green-400;
		@apply dark:bg-gray-700 dark:hover:bg-gray-500;
	}

	.run-btn {
		@apply bg-green-600 hover:bg-green-400;
		@apply dark:bg-gray-700 dark:hover:bg-gray-500;
	}

	.save-btn:disabled,
	.run-btn:disabled {
		@apply bg-gray-400 dark:bg-gray-600 cursor-not-allowed;
	}

	.script-textarea {
		@apply flex-1 w-full p-2.5 font-mono resize-none border-none;
		@apply bg-white dark:bg-gray-800;
		@apply text-gray-900 dark:text-white;
		@apply border-b border-gray-300 dark:border-gray-600;
	}

	.script-textarea:disabled {
		@apply bg-gray-100 dark:bg-gray-700;
	}

	.submit-result,
	.loading-message {
		@apply p-2.5 m-0;
		@apply bg-gray-50 dark:bg-gray-800;
		@apply text-gray-900 dark:text-white;
	}

	.toggle-btn {
		@apply text-2xl leading-none p-0 px-1.5;
		@apply bg-transparent border-none cursor-pointer;
		@apply text-gray-600 dark:text-gray-300;
		@apply hover:text-green-600 dark:hover:text-green-400;
		@apply transition-colors;
	}

	.tree-header {
		@apply flex items-center p-1.5;
		@apply bg-gray-100 dark:bg-gray-800;
		@apply border-b border-gray-300 dark:border-gray-600;
	}

	.loading,
	.no-scripts {
		@apply p-2.5 text-gray-600 dark:text-gray-400;
	}
</style>
