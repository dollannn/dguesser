# Suggested sound sources

The current implementation uses lightweight synthesized cues so the feature works immediately without shipping third-party assets.

When you're ready to swap them out for polished sounds, these are the best sources researched for this project:

## Recommended

- **Kenney** — https://kenney.nl/assets/category:Audio
  - Great fit for subtle UI/game cues
  - Easy-to-browse packs like Interface Sounds / UI Audio
  - Many assets are CC0

- **Sonniss GameAudioGDC** — https://sonniss.com/gameaudiogdc
  - High-quality professional sound libraries
  - Royalty-free for commercial use
  - No attribution required
  - Best for start/win/final-results stingers

- **Mixkit** — https://mixkit.co/free-sound-effects/game/
  - Fast to preview and prototype with
  - Good source for short game/UI sounds
  - Uses the Mixkit license

## Also useful

- **Pixabay** — https://pixabay.com/sound-effects/
  - Huge library
  - Usually easy licensing for commercial use, but always verify the current license page

- **Freesound** — https://freesound.org/
  - Best for very specific one-off searches
  - Always verify each clip's license individually
  - Prefer **CC0** or **CC-BY** only; avoid **CC-BY-NC** for shipped commercial work

## Cue replacement suggestions

- `game_starting`, `game_start`, `new_round`: soft synth rise / short positive transition
- `countdown_tick`, `countdown_tock`: dry clock tick pair, very low level
- `round_complete`: short resolve / ping
- `winner`: compact fanfare under 1s
- `game_complete`: softer end-state chime for non-winners

## Format notes

- Prefer `webm` + `mp3` fallback for shipped assets when using Howler
- Keep clips short and trim silence
- Normalize consistently so no one cue is much louder than the others
