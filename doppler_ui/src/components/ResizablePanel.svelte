<script lang="ts">
	import { onMount } from 'svelte';

	export let defaultWidth = 400;
	export let leftPanelBackground = 'rgba(0, 151, 19, 0.1)';
	export let rightPanelBackground = 'white';
	export let zIndex = 0;

	let leftWidth = defaultWidth;
	let isResizing = false;
	let container = null;
	let mounted = false;

	onMount(() => {
		mounted = true;
	});

	function startResizing(e) {
		isResizing = true;
		e.preventDefault();

		const startX = e.clientX;
		const startWidth = leftWidth;

		const handleResize = (e) => {
			if (isResizing) {
				const dx = e.clientX - startX;
				const newWidth = startWidth + dx;
				const containerWidth = container?.offsetWidth || 0;
				const maxWidth = containerWidth - 200; // Ensure at least 200px for right panel
				leftWidth = Math.max(200, Math.min(maxWidth, newWidth));
			}
		};

		const stopResizing = () => {
			isResizing = false;
			window.removeEventListener('mousemove', handleResize);
			window.removeEventListener('mouseup', stopResizing);
		};

		window.addEventListener('mousemove', handleResize);
		window.addEventListener('mouseup', stopResizing);
	}

	function handleDoubleClick() {
		leftWidth = defaultWidth;
	}
</script>

<div class="split-container" bind:this={container} style="position: relative; z-index: {zIndex};">
	<div
		class="left-panel"
		style="width: {leftWidth}px; min-width: {leftWidth}px; max-width: {leftWidth}px; background: {leftPanelBackground};"
	>
		<slot name="left" />
	</div>

	<div
		class="resizer"
		role="separator"
		aria-orientation="vertical"
		aria-valuenow={leftWidth}
		aria-valuemin={200}
		aria-valuemax={800}
		aria-label="Resize panel"
		on:mousedown={startResizing}
		on:dblclick={handleDoubleClick}
		title="Double-click to reset width"
	/>

	<div class="right-panel" style="background: {rightPanelBackground};">
		<slot name="right" />
	</div>
</div>

<style>
	.split-container {
		display: flex;
		width: 100%;
		height: 100%;
		min-width: 0;
	}

	.left-panel {
		flex-shrink: 0;
		height: 100%;
		overflow: auto;
		position: relative;
	}

	.right-panel {
		flex: 1;
		height: 100%;
		min-width: 0;
		overflow: auto;
		position: relative;
	}

	.resizer {
		width: 4px;
		height: 100%;
		cursor: col-resize;
		background-color: #ccc;
		border-left: 1px solid #999;
		border-right: 1px solid #999;
		flex-shrink: 0;
		position: relative;
		z-index: 1;
	}

	.resizer:hover {
		background-color: #999;
	}

	:global(.left-panel > *) {
		width: 100%;
		height: 100%;
	}
</style>
