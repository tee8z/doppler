<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	export let value: string;
	export let options: { value: string; label: string }[];
	export let wide = false;

	const dispatch = createEventDispatcher();

	function handleChange(event: Event) {
		const select = event.target as HTMLSelectElement;
		dispatch('change', select.value);
	}
</script>

<div class={`relative ${wide ? 'w-full' : 'inline-block'}`}>
	<select bind:value on:change={handleChange} class="select-input">
		{#each options as option}
			<option value={option.value}>{option.label}</option>
		{/each}
	</select>
	<div class="absolute right-2 top-1/2 -translate-y-1/2 pointer-events-none text-white">
		<svg
			xmlns="http://www.w3.org/2000/svg"
			width="16"
			height="16"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
			stroke-linecap="round"
			stroke-linejoin="round"
		>
			<path d="m6 9 6 6 6-6" />
		</svg>
	</div>
</div>

<style lang="postcss">
	.select-input {
		@apply appearance-none p-2 pr-8 rounded text-white transition-all;
		@apply bg-green-600 hover:bg-green-400;
		@apply dark:bg-gray-700 dark:hover:bg-gray-500;
		@apply border-none outline-none;
	}

	/* Remove default focus outline and add custom one */
	.select-input:focus {
		@apply ring-2 ring-green-400 ring-opacity-50;
		@apply dark:ring-gray-400;
	}

	/* Style the options */
	.select-input option {
		@apply bg-white text-gray-900;
		@apply dark:bg-gray-800 dark:text-white;
	}
</style>
