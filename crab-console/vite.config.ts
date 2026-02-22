import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

const buildVersion = new Date().toISOString().slice(0, 16).replace('T', ' ');

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	define: {
		__BUILD_VERSION__: JSON.stringify(buildVersion)
	}
});
