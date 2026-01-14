<script lang="ts">
	// Play page - create or join game
	// TODO: Implement in Phase 6
</script>

<main class="min-h-screen flex flex-col items-center justify-center bg-gray-100">
	<h1 class="text-3xl font-bold text-gray-800 mb-8">Play</h1>
	
	<div class="space-y-6 w-full max-w-sm">
		<button 
			class="w-full px-6 py-4 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition text-lg"
		>
			Create Game
		</button>
		
		<div class="text-center text-gray-500">or</div>
		
		<div class="space-y-2">
			<input 
				type="text" 
				placeholder="Enter game code" 
				class="w-full px-4 py-3 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
			/>
			<button 
				class="w-full px-6 py-3 bg-green-600 text-white rounded-lg hover:bg-green-700 transition"
			>
				Join Game
			</button>
		</div>
	</div>
	
	<a href="/" class="mt-8 text-blue-600 hover:underline">Back to home</a>
</main>
