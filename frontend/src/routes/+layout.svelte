<script lang="ts">
	import '../app.css';

	let { children } = $props();
</script>

<svelte:head>
	<title>DGuesser</title>
</svelte:head>

{@render children()}
