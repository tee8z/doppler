<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { createSSELogListener } from '$lib/log_listener';

	export let id: string;
	export let baseUrl: string = '/api/logs';
	const url = `${baseUrl}?id=${encodeURIComponent(id)}`;
	console.log(url);
	const { connect, disconnect, logs } = createSSELogListener({
		url: url,
		onMessage: (message) => {},
		onError: (error) => console.error('SSE error:', error),
		onDisconnect: () => console.log('Disconnected from SSE'),
		onConnect: () => console.log('Connected to SSE'),
		maxRetries: 3,
		retryInterval: 3000
	});

	let autoScroll = true;
	let logContainer: HTMLElement;

	function scrollToBottom() {
		if (autoScroll && logContainer) {
			logContainer.scrollTop = logContainer.scrollHeight;
		}
	}

	$: {
		if ($logs) {
			scrollToBottom();
		}
	}

	onMount(() => {
		connect();
	});

	onDestroy(() => {
		disconnect();
	});

	function getLogColor(logLevel: string) {
		switch (logLevel.toUpperCase()) {
			case 'TRACE':
				return 'trace';
			case 'DEBUG':
				return 'debug';
			case 'INFO':
				return 'info';
			case 'WARN':
				return 'warn';
			case 'ERROR':
				return 'error';
			default:
				return 'default';
		}
	}

	function parseLogEntry(log: string) {
		const match = log.match(/^\[([A-Z]+)\]\s*(.*)/);
		if (match) {
			const [, level, content] = match;
			return `<span class="log-level ${getLogColor(level)}">[${level}] <span class="log-content">${content}</span></span>`;
		}
		return `<span class="log-content">${log}</span>`;
	}
</script>

<div class="log-viewer">
	<label class="auto-scroll-label">
		<input type="checkbox" bind:checked={autoScroll} />
		Auto-scroll
	</label>
	<div bind:this={logContainer} class="log-container">
		{#each $logs as log}
			<p>
				{@html parseLogEntry(log)}
			</p>
		{/each}
	</div>
</div>

<style>
	.log-viewer {
		display: flex;
		flex-direction: column;
		height: 100%;
	}
	.auto-scroll-label {
		font-size: 0.8rem;
		margin-bottom: 0.5rem;
		display: flex;
		align-items: center;
	}
	.log-container {
		flex-grow: 1;
		min-height: 500px;
		max-height: 80vh;
		overflow-y: auto;
		border: 1px solid #ccc;
		padding: 10px;
		font-size: 0.9rem;
		line-height: 1.4;
		background-color: #1e1e1e;
		color: #d4d4d4;
	}
	.log-container p {
		margin: 0 0 0.5em 0;
		white-space: pre-wrap;
		word-break: break-all;
	}
	.log-container :global(.log-level) {
		font-weight: bold;
	}
	.log-container :global(.log-content) {
		font-weight: normal;
		color: white;
	}
	.log-container :global(.trace) {
		color: white;
	}
	.log-container :global(.debug) {
		color: cyan;
	}
	.log-container :global(.info) {
		color: green;
	}
	.log-container :global(.warn) {
		color: yellow;
	}
	.log-container :global(.error) {
		color: magenta;
	}
	.log-container :global(.default) {
		color: white;
	}
</style>
