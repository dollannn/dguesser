<script lang="ts">
  import SEO from '$lib/components/SEO.svelte';

  const lastUpdated = 'January 2026';
</script>

<SEO
  title="Terms of Service"
  description="Read the terms and conditions for using DGuesser, including user responsibilities and service limitations."
/>

<div class="max-w-3xl mx-auto px-4 py-12">
  <h1 class="text-4xl font-bold text-gray-900 mb-2">Terms of Service</h1>
  <p class="text-gray-500 mb-8">Last updated: {lastUpdated}</p>

  <div class="prose prose-gray max-w-none space-y-8">
    <section>
      <h2 class="text-2xl font-semibold text-gray-900 mb-4">1. Acceptance of Terms</h2>
      <p class="text-gray-600 leading-relaxed">
        By accessing and using DGuesser, you agree to be bound by these Terms of Service. 
        If you do not agree to these terms, please do not use our service.
      </p>
    </section>

    <section>
      <h2 class="text-2xl font-semibold text-gray-900 mb-4">2. Description of Service</h2>
      <p class="text-gray-600 leading-relaxed">
        DGuesser is a geography guessing game where players attempt to identify locations 
        around the world using Google Street View imagery. The game can be played solo or 
        in multiplayer mode with friends.
      </p>
    </section>

    <section>
      <h2 class="text-2xl font-semibold text-gray-900 mb-4">3. User Accounts</h2>
      <p class="text-gray-600 leading-relaxed mb-4">
        You can play DGuesser as a guest without creating an account. To save your progress 
        and appear on leaderboards, you may sign in using Google authentication.
      </p>
      <p class="text-gray-600 leading-relaxed">
        When you sign in with Google, we receive basic profile information (name, email, 
        and profile picture) to create your account. You are responsible for maintaining the 
        security of your account.
      </p>
    </section>

    <section>
      <h2 class="text-2xl font-semibold text-gray-900 mb-4">4. Acceptable Use</h2>
      <p class="text-gray-600 leading-relaxed mb-4">You agree not to:</p>
      <ul class="list-disc list-inside text-gray-600 space-y-2">
        <li>Use automated tools or bots to play the game</li>
        <li>Attempt to manipulate scores or leaderboards</li>
        <li>Interfere with other players' experience</li>
        <li>Attempt to access systems or data you are not authorized to access</li>
        <li>Use the service for any illegal purpose</li>
      </ul>
    </section>

    <section>
      <h2 class="text-2xl font-semibold text-gray-900 mb-4">5. Intellectual Property</h2>
      <p class="text-gray-600 leading-relaxed">
        Street View imagery is provided by Google and is subject to Google's terms of service. 
        All other content, design, and functionality of DGuesser are owned by us and protected 
        by applicable intellectual property laws.
      </p>
    </section>

    <section>
      <h2 class="text-2xl font-semibold text-gray-900 mb-4">6. Disclaimer</h2>
      <p class="text-gray-600 leading-relaxed">
        DGuesser is provided "as is" without warranties of any kind. We do not guarantee 
        uninterrupted access to the service or that scores and game history will be preserved 
        indefinitely.
      </p>
    </section>

    <section>
      <h2 class="text-2xl font-semibold text-gray-900 mb-4">7. Changes to Terms</h2>
      <p class="text-gray-600 leading-relaxed">
        We may update these terms from time to time. Continued use of the service after 
        changes constitutes acceptance of the new terms.
      </p>
    </section>

    <section>
      <h2 class="text-2xl font-semibold text-gray-900 mb-4">8. Contact</h2>
      <p class="text-gray-600 leading-relaxed">
        If you have questions about these terms, please contact us through our GitHub repository.
      </p>
    </section>
  </div>

  <div class="mt-12 pt-8 border-t border-gray-200">
    <a href="/" class="text-primary-600 hover:text-primary-700 font-medium">
      &larr; Back to Home
    </a>
  </div>
</div>
