<script>
	import { themeIcon, toggleDarkMode } from "$lib/theme";
	import Button from "../components/Button.svelte";
	import Icon from "../components/Icon/Icon.svelte";


</script>
<h1>Welcome to SvelteKit</h1>
<p>Visit <a href="https://kit.svelte.dev">kit.svelte.dev</a> to read the documentation</p>
<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<span class="cursor-pointer" on:click={toggleDarkMode}><Icon name={$themeIcon}></Icon></span>