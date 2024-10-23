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
				class={`tree-item ${
					isFolder(item) ? 'folder' : isDopplerFile(item) ? 'doppler-file' : 'file'
				}`}
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

<style lang="postcss">
	.tree-list {
		@apply list-none m-0 p-0;
	}

	.tree-item {
		@apply w-full cursor-pointer p-2 flex items-center text-left;
		@apply bg-transparent border-none rounded transition-colors;
		@apply text-gray-700 dark:text-gray-100;
		@apply hover:bg-green-100 dark:hover:bg-green-900/30;
		@apply focus:outline-none focus:ring-2 focus:ring-green-400 dark:focus:ring-green-500;
	}

	.folder {
		@apply font-medium;
		@apply text-gray-900 dark:text-blue-200;
	}

	.doppler-file {
		@apply text-green-600 dark:text-green-400;
		@apply hover:text-green-700 dark:hover:text-green-300;
	}

	.file {
		@apply text-gray-600 dark:text-gray-300;
	}

	.icon {
		@apply mr-2 text-lg opacity-90 dark:opacity-100;
	}

	.name {
		@apply flex-1 truncate;
	}

	/* Expanded folder state */
	.folder[aria-expanded='true'] {
		@apply bg-green-50 dark:bg-green-900/20;
		@apply text-green-800 dark:text-green-100;
	}

	/* Active state */
	.tree-item:active {
		@apply bg-green-200 dark:bg-green-800/40;
	}

	/* Hover states */
	.tree-item:hover .name {
		@apply text-green-700 dark:text-green-100;
	}

	.folder:hover {
		@apply text-green-800 dark:text-green-100;
	}
</style>
