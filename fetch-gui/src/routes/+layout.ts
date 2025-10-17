// Tauri doesn't have a Node.js server to do proper SSR
// so we use adapter-static with a fallback to index.html to put the site in SPA mode
// See: https://svelte.dev/docs/kit/single-page-apps
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
// export const ssr = false;

// Instead of SPA mode, I am using SSG mode according to: https://svelte.dev/docs/kit/adapter-static
// which the Tauri docs recommended I follow to swap to SSG mode if that is what I prefer:
// https://v2.tauri.app/start/frontend/sveltekit/#disable-ssr
// Not sure if this will affect the functionality I can access with TS svelte pages. May change in the future
export const prerender = true;
