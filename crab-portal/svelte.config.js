import adapter from '@sveltejs/adapter-static';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	kit: {
		adapter: adapter({
			pages: 'build',
			assets: 'build',
			fallback: '200.html',
			precompress: false,
			strict: false
		}),
		prerender: {
			entries: ['*', '/support', '/renew', '/terms', '/privacy', '/cookies', '/legal'],
			handleHttpError: ({ path }) => {
				// Ignore missing screenshot placeholders during prerender
				if (path.startsWith('/screenshots/')) return;
				throw new Error(`404 ${path}`);
			}
		}
	}
};

export default config;
