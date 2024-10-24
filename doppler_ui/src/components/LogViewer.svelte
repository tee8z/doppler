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
		<input type="checkbox" bind:checked={autoScroll} class="checkbox" />
		<span class="label-text">Auto-scroll</span>
	</label>
	<div bind:this={logContainer} class="log-container">
		{#each $logs as log}
			<p>
				{@html parseLogEntry(log)}
			</p>
		{/each}
	</div>
</div>

<style lang="postcss">
	.log-viewer {
		@apply flex flex-col h-full;
	}

	.auto-scroll-label {
		@apply text-sm mb-2 flex items-center gap-2;
		@apply text-gray-700 dark:text-gray-300;
		@apply flex-shrink-0;
	}

	.checkbox {
		@apply h-4 w-4 rounded border-gray-300 dark:border-gray-600;
		@apply text-green-600 dark:text-green-500;
		@apply focus:ring-green-500 dark:focus:ring-green-400;
		@apply bg-white dark:bg-gray-700;
	}

	.label-text {
		@apply select-none;
	}

	.log-container {
		@apply flex-1; /* Changed from flex-grow */
		@apply overflow-y-auto;
		@apply border border-gray-300 dark:border-gray-600;
		@apply p-2.5 text-sm leading-relaxed;
		@apply bg-black;
		@apply font-mono;
		@apply shadow-inner;
		@apply min-h-0;
	}

	.log-container p {
		@apply m-0 mb-1 whitespace-pre-wrap break-all;
	}

	.log-container :global(.log-level) {
		@apply font-bold;
	}

	.log-container :global(.log-content) {
		@apply font-normal text-gray-200;
	}

	.log-container :global(.trace) {
		@apply text-gray-400;
	}

	.log-container :global(.debug) {
		@apply text-cyan-400;
	}

	.log-container :global(.info) {
		@apply text-green-400;
	}

	.log-container :global(.warn) {
		@apply text-yellow-400;
	}

	.log-container :global(.error) {
		@apply text-rose-400;
	}

	.log-container :global(.default) {
		@apply text-gray-200;
	}

	.log-container :global(.debug),
	.log-container :global(.info),
	.log-container :global(.warn),
	.log-container :global(.error) {
		text-shadow: 0 0 2px rgba(255, 255, 255, 0.2);
	}
</style>
