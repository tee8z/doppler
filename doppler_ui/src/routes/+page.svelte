<script lang="ts">
	import { theme, themeIcon, componentIcon, toggleDarkMode, toggleComponent } from '$lib/theme';
	import Icon from '../components/Icon/Icon.svelte';
	import Visualizer from '../components/Visualizer.svelte';
	import LogViewer from '../components/LogViewer.svelte';
	import ScriptBuilder from '../components/ScriptBuilder.svelte';
	import Button from '../components/Button.svelte';

	let currentScriptId: string | null = null;
	let activeTab = 'scriptBuilder';

	function handleScriptSubmit(event: CustomEvent<string>) {
		currentScriptId = event.detail;
	}

	function handleScriptRun(event: CustomEvent<string>) {
		currentScriptId = event.detail;
		setActiveTab('logViewer');
	}

	async function handleReset() {
		let resetId = currentScriptId;
		if (!currentScriptId) {
			resetId = null;
		}
		try {
			const response = await fetch('/api/reset', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify({ id: resetId })
			});

			if (!response.ok) {
				throw new Error('Failed to reset script');
			}

			const result = await response.json();
		} catch (error) {
			console.error('Error requesting reset:', error);
		}
	}

	function setActiveTab(tab: string) {
		activeTab = tab;
	}
</script>

<main class="flex flex-col h-screen">
	<section class="flex flex-col justify-between items-center m-4 md:flex-row">
		<div class="flex items-center gap-2">
			<h1>Doppler</h1>
			<a href="https://github.com/tee8z/doppler" class="h-6 w-6"
				><Icon name="octocat" class="h-6 w-6" /></a
			>
		</div>
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		<!-- svelte-ignore a11y-no-static-element-interactions -->
		<div class="flex items-center gap-2">
			<span class="cursor-pointer h-6 w-6" on:click={toggleComponent}>
				<Icon name={$componentIcon} class="h-6 w-6" /></span
			>
		</div>
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		<!-- svelte-ignore a11y-no-static-element-interactions -->
		<div class="flex items-center gap-2">
			<span class="cursor-pointer h-6 w-6" on:click={toggleDarkMode}
				><Icon name={$themeIcon} class="h-6 w-6" /></span
			>
		</div>
	</section>
	<section class="flex flex-1">
		<div class="w-2/3 flex flex-col min-w-0">
			<Visualizer />
		</div>
		<div class="w-1/3 flex-shrink-0">
			<div class="bg-gray-200 p-4">
				<nav class="flex mb-4">
					<Button
						on:click={() => setActiveTab('scriptBuilder')}
						class={activeTab === 'scriptBuilder' ? 'bg-blue-500 text-white' : ''}
					>
						Script Builder
					</Button>
					<Button
						on:click={() => setActiveTab('logViewer')}
						class={activeTab === 'logViewer' ? 'bg-blue-500 text-white' : ''}
					>
						Script Log
					</Button>
					<Button
						on:click={handleReset}
						class={activeTab === 'reset' ? 'bg-blue-500 text-white' : ''}
					>
						Reset
					</Button>
				</nav>
				{#if activeTab === 'scriptBuilder'}
					<ScriptBuilder on:scriptSubmitted={handleScriptSubmit} on:scriptRun={handleScriptRun} />
				{:else if activeTab === 'logViewer'}
					{#if currentScriptId}
						<LogViewer id={currentScriptId} />
					{:else}
						<p>Submit a script to view logs</p>
					{/if}
				{/if}
			</div>
		</div>
	</section>
</main>

<style>
	main {
		display: flex;
		flex-direction: column;
		height: 100vh;
	}

	:global(.split-container) {
		height: 100% !important;
	}

	:global(.left-panel),
	:global(.right-panel) {
		min-height: 0;
	}
</style>
