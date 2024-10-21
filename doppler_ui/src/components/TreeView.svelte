<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { slide } from 'svelte/transition';

	export let tree: any;
	export let level: number = 0;
	export let parentPath: string = '';

	const dispatch = createEventDispatcher();

	function handleSelect(item: any) {
		const fullPath = parentPath ? `${parentPath}/${item.label}` : item.label;
		console.log(fullPath);
		dispatch('select', { ...item, fullPath });
	}

	function isFolder(item: any): boolean {
		return item.children && Array.isArray(item.children);
	}

	function isDopplerFile(item: any): boolean {
		return item.label.endsWith('.doppler');
	}

	function toggleFolder(item: any) {
		item.expanded = !item.expanded;
		tree = tree; // Force a re-render
	}

	function handleKeyDown(event: KeyboardEvent, item: any) {
		if (event.key === 'Enter' || event.key === ' ') {
			if (isFolder(item)) {
				toggleFolder(item);
			} else {
				handleSelect(item);
			}
		}
	}

	function getIcon(item: any): string {
		if (isFolder(item)) {
			return item.expanded ? '📂' : '📁';
		}
		if (isDopplerFile(item)) {
			return '🔵';
		}
		return '📄';
	}
</script>

<ul class="tree-list" style="padding-left: {level * 20}px">
	{#each tree.children ? tree.children : [tree] as item}
		<li>
			<button
				type="button"
				class={isFolder(item) ? 'folder' : isDopplerFile(item) ? 'doppler-file' : 'file'}
				on:click={() => (isFolder(item) ? toggleFolder(item) : handleSelect(item))}
				on:keydown={(event) => handleKeyDown(event, item)}
				aria-expanded={isFolder(item) ? item.expanded : undefined}
			>
				<span class="icon">{getIcon(item)}</span>
				<span class="name">{item.label}</span>
			</button>
			{#if isFolder(item) && item.expanded}
				<div transition:slide>
					<svelte:self
						tree={item}
						level={level + 1}
						parentPath={parentPath ? `${parentPath}/${item.label}` : item.label}
						on:select
					/>
				</div>
			{/if}
		</li>
	{/each}
</ul>

<style>
	.tree-list {
		list-style-type: none;
		margin: 0;
		padding: 0;
	}
	.folder,
	.file,
	.doppler-file {
		cursor: pointer;
		padding: 5px;
		display: flex;
		align-items: center;
		background: none;
		border: none;
		width: 100%;
		text-align: left;
	}
	.folder:hover,
	.file:hover,
	.doppler-file:hover {
		background-color: #f0f0f0;
	}
	.icon {
		margin-right: 5px;
	}
	.name {
		flex: 1;
	}
	.doppler-file {
		color: #0066cc;
	}
</style>
