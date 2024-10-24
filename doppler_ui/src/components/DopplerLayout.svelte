<script lang="ts">
	import { theme, themeIcon, componentIcon, toggleDarkMode, toggleComponent } from '$lib/theme';
	import Icon from './Icon/Icon.svelte';
	import Visualizer from './Visualizer.svelte';
	import ResizablePanel from './ResizablePanel.svelte';
	import LogViewer from './LogViewer.svelte';
	import ScriptBuilder from './ScriptBuilder.svelte';
	import Button from './Button.svelte';

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
			<a href="https://github.com/tee8z/doppler" class="h-6 w-6">
				<Icon name="octocat" class="h-6 w-6" />
			</a>
		</div>
		<div class="flex items-center gap-2">
			<span class="cursor-pointer h-6 w-6" on:click={toggleComponent}>
				<Icon name={$componentIcon} class="h-6 w-6" />
			</span>
		</div>
		<div class="flex items-center gap-2">
			<span class="cursor-pointer h-6 w-6" on:click={toggleDarkMode}>
				<Icon name={$themeIcon} class="h-6 w-6" />
			</span>
		</div>
	</section>

	<section class="flex flex-1 relative overflow-hidden">
		<ResizablePanel
			defaultWidth={1000}
			leftPanelBackground="white"
			rightPanelBackground="rgb(229 231 235)"
			zIndex={2}
		>
			<div slot="left" class="h-full w-full">
				<Visualizer />
			</div>

			<div slot="right" class="h-full w-full">
				<div class="bg-gray-200 p-4 h-full">
					<nav class="flex gap-2 mb-4">
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

					<div class="overflow-auto">
						{#if activeTab === 'scriptBuilder'}
							<ScriptBuilder
								on:scriptSubmitted={handleScriptSubmit}
								on:scriptRun={handleScriptRun}
							/>
						{:else if activeTab === 'logViewer'}
							{#if currentScriptId}
								<LogViewer id={currentScriptId} />
							{:else}
								<p>Submit a script to view logs</p>
							{/if}
						{/if}
					</div>
				</div>
			</div>
		</ResizablePanel>
	</section>
</main>

<style>
	:global(.split-container) {
		height: 100%;
	}

	:global(.left-panel),
	:global(.right-panel) {
		position: relative;
	}

	:global(.left-panel > *, .right-panel > *) {
		height: 100%;
		width: 100%;
	}

	:global(.left-panel .split-container) {
		height: 100%;
		width: 100%;
	}
</style>
