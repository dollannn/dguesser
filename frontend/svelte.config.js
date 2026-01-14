import adapter from '@sveltejs/adapter-node';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	// Consult https://svelte.dev/docs/kit/integrations
	// for more information about preprocessors
	preprocess: vitePreprocess(),

	kit: {
		// Use adapter-node for Railway/Docker deployment
		adapter: adapter({
			// Output directory for the build
			out: 'build',
			// Precompress files with gzip and brotli
			precompress: true,
			// Environment variable for the host to listen on
			envPrefix: ''
		})
	}
};

export default config;
