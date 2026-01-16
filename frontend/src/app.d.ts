// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		interface PageData {
			// SSR data will be typed per-route
		}
		// interface PageState {}
		// interface Platform {}
	}
}

export {};
