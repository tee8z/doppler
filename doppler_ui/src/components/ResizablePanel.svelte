<script>
	export let defaultWidth = 400;

	let leftWidth = defaultWidth;
	let isResizing = false;
	let container;

	function startResizing(e) {
		isResizing = true;
		e.preventDefault();

		const handleResize = (e) => {
			if (isResizing) {
				const containerWidth = container.offsetWidth;
				const maxWidth = containerWidth - 200; // Ensure at least 200px for right panel
				leftWidth = Math.max(
					200,
					Math.min(maxWidth, e.clientX - container.getBoundingClientRect().left)
				);
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

<div class="split-container" bind:this={container}>
	<div
		class="left-panel"
		style="width: {leftWidth}px; min-width: {leftWidth}px; max-width: {leftWidth}px;"
	>
		<slot name="left"></slot>
	</div>

	<!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
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
	></div>

	<div class="right-panel">
		<slot name="right"></slot>
	</div>
</div>

<style>
	.split-container {
		display: flex;
		position: relative;
		width: 100%;
		height: 100%;
		min-width: 0;
	}

	.left-panel {
		flex-shrink: 0;
		height: 100%;
		background: rgba(0, 151, 19, 0.1);
		overflow: auto;
	}

	.right-panel {
		flex: 1;
		height: 100%;
		min-width: 0;
		background: white;
		overflow: hidden;
	}

	.resizer {
		width: 4px;
		height: 100%;
		cursor: col-resize;
		background-color: #ccc;
		border-left: 1px solid #999;
		border-right: 1px solid #999;
		flex-shrink: 0;
	}

	.resizer:hover {
		background-color: #999;
	}

	:global(.left-panel > *) {
		width: 100%;
		height: 100%;
	}
</style>
