<script lang="ts">
	import { DarkTheme, theme, themeIcon, componentIcon, toggleDarkMode, toggleComponent } from '$lib/theme';
	import Icon from '../components/Icon/Icon.svelte';
	import Scripter from '../components/Scripter.svelte';
	import Visualizer from '../components/Visualizer.svelte';
	import Blockly from 'blockly';

	$: blocklyTheme = $theme === 'light' ? Blockly.Themes.Classic : DarkTheme;

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
				<Icon name={$componentIcon} class="h-6 w-6" /></span>
		</div>
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		<!-- svelte-ignore a11y-no-static-element-interactions -->
		<div class="flex items-center gap-2">
			<span class="cursor-pointer h-6 w-6" on:click={toggleDarkMode}
				><Icon name={$themeIcon} class="h-6 w-6" /></span
			>
		</div>
	</section>
	{#if $componentIcon === 'radar'}
		<section class="flex flex-1">
			<Visualizer />
		</section>
	{:else}
		<section class="flex h-full gap-2">
			<Scripter {blocklyTheme} />
		</section>
	{/if}
</main>
