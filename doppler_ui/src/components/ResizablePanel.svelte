<script>
	export let defaultWidth = 400;

	let leftWidth = defaultWidth;
	let isResizing = false;

	function startResizing() {
		isResizing = true;

		const handleResize = (e) => {
			if (isResizing) {
				leftWidth = Math.max(200, Math.min(800, e.clientX));
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

<div class="split-container">
	<div class="left-panel" style:width="{leftWidth}px">
		<slot name="left" />
	</div>

	<div
		class="resizer"
		on:mousedown={startResizing}
		on:dblclick={handleDoubleClick}
		title="Double-click to reset width"
	/>

	<div class="right-panel">
		<slot name="right" />
	</div>
</div>

<style>
	.split-container {
		display: flex;
		position: relative;
	}

	.panel {
		height: 100%;
	}

	.left-panel {
		background: rgba(0, 151, 19, 0.1);
	}

	.right-panel {
		background: white;
	}

	.resizer {
		width: 4px;
		cursor: col-resize;
		background-color: #ccc;
		border-left: 1px solid #999;
		border-right: 1px solid #999;
	}

	.resizer:hover {
		background-color: #999;
	}
</style>
