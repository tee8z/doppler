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

<main>
	<h1>Doppler Script Editor</h1>
	<div class="editor-layout">
		<div class="script-editor">
			<div class="editor-nav">
				<div class="script-info">
					<input
						bind:value={scriptPath}
						placeholder="Enter script folder path (e.g., folder/subfolder)"
						type="text"
					/>
					<input
						bind:value={scriptName}
						placeholder="Enter script name (without _id.doppler)"
						type="text"
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
				<h2>Script Library</h2>
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

<style>
	textarea:disabled {
		background-color: #f0f0f0;
	}
	.toggle-btn {
		font-size: 1.5em;
		line-height: 1;
		padding: 0 5px;
	}
	input {
		width: 100%;
		margin-bottom: 10px;
		padding: 5px;
	}
	main {
		max-width: none;
		margin: 0;
		padding: 20px;
		height: 100vh;
		box-sizing: border-box;
		display: flex;
		flex-direction: column;
	}
	.editor-layout {
		display: flex;
		gap: 0;
		flex: 1;
		overflow: hidden;
	}
	.tree-view {
		flex: 0 0 250px;
		transition: flex-basis 0.3s ease;
		overflow: hidden;
		border-left: 1px solid #ccc;
	}
	.tree-view:not(.expanded) {
		flex-basis: 30px;
	}
	.script-editor {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	.editor-nav {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 10px;
		background-color: #f0f0f0;
		border-bottom: 1px solid #ccc;
		align-items: start;
	}
	.script-info {
		flex: 1;
		margin-right: 10px;
	}
	.script-info input {
		width: 100%;
		margin-bottom: 5px;
		padding: 5px;
	}
	.button-group {
		display: flex;
		flex-direction: column;
		gap: 5px;
	}
	.save-btn,
	.run-btn {
		padding: 8px 16px;
		font-size: 14px;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		transition: background-color 0.3s;
		width: 100px;
	}
	.save-btn {
		background-color: #4caf50;
		color: white;
	}
	.save-btn:hover {
		background-color: #45a049;
	}
	.run-btn {
		background-color: #008cba;
		color: white;
	}
	.run-btn:hover {
		background-color: #007b9e;
	}
	.save-btn:disabled,
	.run-btn:disabled {
		background-color: #cccccc;
		cursor: not-allowed;
	}
	textarea {
		flex: 1;
		width: 100%;
		padding: 10px;
		font-family: monospace;
		resize: none;
		border: none;
		border-bottom: 1px solid #ccc;
	}
	.submit-result,
	.loading-message {
		padding: 10px;
		margin: 0;
		background-color: #f9f9f9;
	}
	button {
		padding: 10px 20px;
		font-size: 16px;
	}
	.tree-header {
		display: flex;
		align-items: center;
		padding: 5px;
	}
	.toggle-btn {
		background: none;
		border: none;
		cursor: pointer;
		padding: 5px;
	}
</style>
