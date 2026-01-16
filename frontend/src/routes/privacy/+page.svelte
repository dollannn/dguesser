<script lang="ts">
  import SEO from '$lib/components/SEO.svelte';

  const lastUpdated = 'January 2026';
</script>

<SEO
  title="Privacy Policy"
  description="Learn how DGuesser collects, uses, and protects your personal information."
/>

<div class="max-w-3xl mx-auto px-4 py-12">
  <h1 class="text-4xl font-bold text-gray-900 mb-2">Privacy Policy</h1>
  <p class="text-gray-500 mb-8">Last updated: {lastUpdated}</p>

  <div class="bg-primary-50 border border-primary-200 rounded-lg p-4 mb-8">
    <p class="text-primary-800 font-medium">
      TL;DR: We only collect what's essential to make the game work. No tracking, no ads, no selling your data.
    </p>
  </div>

  <div class="prose prose-gray max-w-none space-y-8">
    <section>
      <h2 class="text-2xl font-semibold text-gray-900 mb-4">What We Collect</h2>
      <p class="text-gray-600 leading-relaxed mb-4">
        We collect only the minimum information necessary to provide the game:
      </p>
      
      <h3 class="text-lg font-medium text-gray-800 mt-6 mb-3">If you sign in with Google:</h3>
      <ul class="list-disc list-inside text-gray-600 space-y-2">
        <li><strong>Display name</strong> - shown to other players and on leaderboards</li>
        <li><strong>Email address</strong> - used to identify your account</li>
        <li><strong>Profile picture</strong> - displayed next to your name (optional)</li>
      </ul>

      <h3 class="text-lg font-medium text-gray-800 mt-6 mb-3">Game data:</h3>
      <ul class="list-disc list-inside text-gray-600 space-y-2">
        <li><strong>Game scores</strong> - your guesses and points earned</li>
        <li><strong>Game history</strong> - past games for your statistics</li>
      </ul>

      <h3 class="text-lg font-medium text-gray-800 mt-6 mb-3">If you play as a guest:</h3>
      <p class="text-gray-600 leading-relaxed">
        We create a temporary session to let you play. Guest data is not permanently stored 
        and is automatically deleted.
      </p>
    </section>

    <section>
      <h2 class="text-2xl font-semibold text-gray-900 mb-4">What We Don't Do</h2>
      <ul class="list-disc list-inside text-gray-600 space-y-2">
        <li>No analytics or tracking scripts</li>
        <li>No advertising</li>
        <li>No selling or sharing your data with third parties</li>
        <li>No tracking your location (the game guesses locations, not yours!)</li>
        <li>No cookies beyond essential session management</li>
      </ul>
    </section>

    <section>
      <h2 class="text-2xl font-semibold text-gray-900 mb-4">How We Use Your Information</h2>
      <p class="text-gray-600 leading-relaxed mb-4">Your data is used exclusively to:</p>
      <ul class="list-disc list-inside text-gray-600 space-y-2">
        <li>Display your profile to other players in multiplayer games</li>
        <li>Track your scores and maintain leaderboards</li>
        <li>Show your game history and statistics</li>
        <li>Enable multiplayer functionality</li>
      </ul>
    </section>

    <section>
      <h2 class="text-2xl font-semibold text-gray-900 mb-4">Third-Party Services</h2>
      <p class="text-gray-600 leading-relaxed mb-4">
        We use the following third-party services:
      </p>
      <ul class="list-disc list-inside text-gray-600 space-y-2">
        <li><strong>Google OAuth</strong> - for authentication (their <a href="https://policies.google.com/privacy" class="text-primary-600 hover:underline" target="_blank" rel="noopener">privacy policy</a>)</li>
        <li><strong>Google Maps / Street View</strong> - for game imagery (their <a href="https://policies.google.com/privacy" class="text-primary-600 hover:underline" target="_blank" rel="noopener">privacy policy</a>)</li>
      </ul>
    </section>

    <section>
      <h2 class="text-2xl font-semibold text-gray-900 mb-4">Data Retention</h2>
      <p class="text-gray-600 leading-relaxed">
        Your account data is retained as long as your account exists. You can request deletion 
        of your account and associated data at any time by contacting us.
      </p>
    </section>

    <section>
      <h2 class="text-2xl font-semibold text-gray-900 mb-4">Your Rights</h2>
      <p class="text-gray-600 leading-relaxed mb-4">You have the right to:</p>
      <ul class="list-disc list-inside text-gray-600 space-y-2">
        <li>Access your personal data</li>
        <li>Request correction of inaccurate data</li>
        <li>Request deletion of your data</li>
        <li>Export your data</li>
      </ul>
    </section>

    <section>
      <h2 class="text-2xl font-semibold text-gray-900 mb-4">Contact</h2>
      <p class="text-gray-600 leading-relaxed">
        For any privacy-related questions or requests, please contact us through our GitHub repository.
      </p>
    </section>
  </div>

  <div class="mt-12 pt-8 border-t border-gray-200">
    <a href="/" class="text-primary-600 hover:text-primary-700 font-medium">
      &larr; Back to Home
    </a>
  </div>
</div>
