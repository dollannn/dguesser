import { browser } from '$app/environment';
import { get } from 'svelte/store';
import { Howl, Howler } from 'howler';

import { soundSettings } from './settings';

export type GameCue =
  | 'game_starting'
  | 'game_start'
  | 'new_round'
  | 'pin_place'
  | 'guess_submit'
  | 'round_complete'
  | 'countdown_tick'
  | 'countdown_tock'
  | 'time_up'
  | 'game_complete'
  | 'winner';

type Waveform = 'sine' | 'triangle' | 'square';

interface ToneStep {
  frequency: number;
  durationMs: number;
  gain?: number;
  waveform?: Waveform;
}

const SAMPLE_RATE = 22_050;
const TAU = Math.PI * 2;

function writeString(view: DataView, offset: number, value: string): void {
  for (let i = 0; i < value.length; i += 1) {
    view.setUint8(offset + i, value.charCodeAt(i));
  }
}

function toBase64(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let binary = '';

  for (let i = 0; i < bytes.length; i += 1) {
    binary += String.fromCharCode(bytes[i]);
  }

  return btoa(binary);
}

function sampleWave(phase: number, waveform: Waveform): number {
  switch (waveform) {
    case 'square':
      return phase % 1 < 0.5 ? 1 : -1;
    case 'triangle': {
      const fraction = phase % 1;
      return 2 * Math.abs(2 * fraction - 1) - 1;
    }
    case 'sine':
    default:
      return Math.sin(TAU * phase);
  }
}

function buildCueSamples(steps: ToneStep[]): Float32Array {
  const totalSamples = steps.reduce(
    (sum, step) => sum + Math.max(1, Math.round((step.durationMs / 1000) * SAMPLE_RATE)),
    0,
  );
  const samples = new Float32Array(totalSamples);

  let cursor = 0;
  for (const step of steps) {
    const stepSamples = Math.max(1, Math.round((step.durationMs / 1000) * SAMPLE_RATE));
    const attackSamples = Math.max(1, Math.min(Math.round(SAMPLE_RATE * 0.008), stepSamples));
    const releaseSamples = Math.max(1, Math.min(Math.round(SAMPLE_RATE * 0.02), stepSamples));
    const gain = step.gain ?? 0.5;
    const waveform = step.waveform ?? 'sine';

    for (let i = 0; i < stepSamples; i += 1) {
      let envelope = 1;

      if (i < attackSamples) {
        envelope = i / attackSamples;
      } else if (i > stepSamples - releaseSamples) {
        envelope = Math.max(0, (stepSamples - i) / releaseSamples);
      }

      const phase = (i * step.frequency) / SAMPLE_RATE;
      const sample = step.frequency > 0 ? sampleWave(phase, waveform) * gain * envelope : 0;
      samples[cursor + i] = sample;
    }

    cursor += stepSamples;
  }

  return samples;
}

function createWavDataUri(steps: ToneStep[]): string {
  const samples = buildCueSamples(steps);
  const bytesPerSample = 2;
  const dataSize = samples.length * bytesPerSample;
  const buffer = new ArrayBuffer(44 + dataSize);
  const view = new DataView(buffer);

  writeString(view, 0, 'RIFF');
  view.setUint32(4, 36 + dataSize, true);
  writeString(view, 8, 'WAVE');
  writeString(view, 12, 'fmt ');
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, 1, true);
  view.setUint32(24, SAMPLE_RATE, true);
  view.setUint32(28, SAMPLE_RATE * bytesPerSample, true);
  view.setUint16(32, bytesPerSample, true);
  view.setUint16(34, 16, true);
  writeString(view, 36, 'data');
  view.setUint32(40, dataSize, true);

  let offset = 44;
  for (const sample of samples) {
    const clamped = Math.max(-1, Math.min(1, sample));
    view.setInt16(offset, clamped < 0 ? clamped * 0x8000 : clamped * 0x7fff, true);
    offset += bytesPerSample;
  }

  return `data:audio/wav;base64,${toBase64(buffer)}`;
}

function createHowl(steps: ToneStep[], volume: number): Howl {
  return new Howl({
    src: [createWavDataUri(steps)],
    format: ['wav'],
    volume,
    preload: true,
    pool: 8,
  });
}

const sounds: Record<GameCue, Howl> | null = browser
  ? {
      game_starting: createHowl(
        [
          { frequency: 523.25, durationMs: 65, waveform: 'triangle', gain: 0.45 },
          { frequency: 659.25, durationMs: 65, waveform: 'triangle', gain: 0.48 },
          { frequency: 783.99, durationMs: 120, waveform: 'sine', gain: 0.52 },
        ],
        0.26,
      ),
      game_start: createHowl(
        [
          { frequency: 659.25, durationMs: 50, waveform: 'triangle', gain: 0.5 },
          { frequency: 880, durationMs: 90, waveform: 'sine', gain: 0.62 },
        ],
        0.3,
      ),
      new_round: createHowl(
        [
          { frequency: 587.33, durationMs: 45, waveform: 'triangle', gain: 0.42 },
          { frequency: 783.99, durationMs: 70, waveform: 'triangle', gain: 0.5 },
        ],
        0.24,
      ),
      pin_place: createHowl(
        [
          { frequency: 466.16, durationMs: 30, waveform: 'square', gain: 0.22 },
          { frequency: 659.25, durationMs: 35, waveform: 'triangle', gain: 0.14 },
        ],
        0.18,
      ),
      guess_submit: createHowl(
        [
          { frequency: 523.25, durationMs: 40, waveform: 'triangle', gain: 0.44 },
          { frequency: 659.25, durationMs: 65, waveform: 'sine', gain: 0.52 },
        ],
        0.26,
      ),
      round_complete: createHowl(
        [
          { frequency: 587.33, durationMs: 55, waveform: 'triangle', gain: 0.46 },
          { frequency: 698.46, durationMs: 65, waveform: 'sine', gain: 0.42 },
          { frequency: 523.25, durationMs: 90, waveform: 'sine', gain: 0.48 },
        ],
        0.24,
      ),
      countdown_tick: createHowl(
        [{ frequency: 880, durationMs: 38, waveform: 'square', gain: 0.32 }],
        0.14,
      ),
      countdown_tock: createHowl(
        [{ frequency: 622.25, durationMs: 38, waveform: 'square', gain: 0.32 }],
        0.14,
      ),
      time_up: createHowl(
        [
          { frequency: 329.63, durationMs: 55, waveform: 'square', gain: 0.34 },
          { frequency: 220, durationMs: 85, waveform: 'triangle', gain: 0.26 },
        ],
        0.2,
      ),
      game_complete: createHowl(
        [
          { frequency: 392, durationMs: 70, waveform: 'triangle', gain: 0.4 },
          { frequency: 523.25, durationMs: 90, waveform: 'sine', gain: 0.46 },
          { frequency: 659.25, durationMs: 120, waveform: 'sine', gain: 0.48 },
        ],
        0.28,
      ),
      winner: createHowl(
        [
          { frequency: 523.25, durationMs: 60, waveform: 'triangle', gain: 0.44 },
          { frequency: 659.25, durationMs: 75, waveform: 'triangle', gain: 0.5 },
          { frequency: 783.99, durationMs: 85, waveform: 'triangle', gain: 0.56 },
          { frequency: 1046.5, durationMs: 170, waveform: 'sine', gain: 0.62 },
        ],
        0.34,
      ),
    }
  : null;

const SILENCE_CUE = browser
  ? createHowl([{ frequency: 440, durationMs: 12, gain: 0 }], 0)
  : null;

soundSettings.initialize();

if (browser) {
  soundSettings.subscribe((state) => {
    Howler.volume(state.volume);
    Howler.mute(!state.enabled);
  });
}

function canPlayAudio(): boolean {
  if (!browser) return false;

  soundSettings.initialize();
  const settings = get(soundSettings);
  return settings.enabled && settings.unlocked;
}

export const gameAudio = {
  async unlock(): Promise<void> {
    if (!browser) return;

    soundSettings.initialize();
    soundSettings.markUnlocked();

    try {
      if (Howler.ctx && Howler.ctx.state === 'suspended') {
        await Howler.ctx.resume();
      }

      if (SILENCE_CUE) {
        const soundId = SILENCE_CUE.play();
        SILENCE_CUE.stop(soundId);
      }
    } catch {
      // Ignore unlock failures; browser policies can still block until the next gesture.
    }
  },

  play(cue: GameCue): boolean {
    if (!sounds || !canPlayAudio()) return false;

    try {
      sounds[cue].play();
      return true;
    } catch {
      return false;
    }
  },

  playGameStarting(): boolean {
    return this.play('game_starting');
  },

  playRoundStart(roundNumber: number): boolean {
    return this.play(roundNumber <= 1 ? 'game_start' : 'new_round');
  },

  playPinPlace(): boolean {
    return this.play('pin_place');
  },

  playGuessSubmitted(): boolean {
    return this.play('guess_submit');
  },

  playRoundComplete(): boolean {
    return this.play('round_complete');
  },

  playCountdownTick(second: number): boolean {
    return this.play(second % 2 === 0 ? 'countdown_tick' : 'countdown_tock');
  },

  playTimeUp(): boolean {
    return this.play('time_up');
  },

  playGameEnd(isWinner: boolean): boolean {
    return this.play(isWinner ? 'winner' : 'game_complete');
  },
};
