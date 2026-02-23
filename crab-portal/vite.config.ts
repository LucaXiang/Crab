import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';
import { execSync } from 'child_process';
import { readFileSync } from 'fs';

const pkg = JSON.parse(readFileSync('./package.json', 'utf-8'));
let gitHash = 'unknown';
try { gitHash = execSync('git rev-parse --short HEAD').toString().trim(); } catch { /* not a git repo */ }

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	define: {
		__APP_VERSION__: JSON.stringify(pkg.version),
		__GIT_HASH__: JSON.stringify(gitHash)
	}
});
