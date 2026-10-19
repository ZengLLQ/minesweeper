use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Source};
use std::f32::consts::{PI, TAU};
use std::sync::mpsc::{self, SyncSender};
use std::thread;

/// Standard mono sample rate (44.1 kHz) used for all procedurally generated sound effects.
pub const SAMPLE_RATE: u32 = 44_100;

/// Procedural sound effect types triggered by game and UI events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundEffect {
    /// Crisp, warm tactile pop when revealing a single cell (or 1–2 cells).
    RevealSingle,
    /// Ascending pentatonic shimmer cascade when flood-fill reveals multiple cells.
    RevealCascade(usize),
    /// Warm harmonic dyad tap when chording reveals safe neighbor cells.
    Chord,
    /// Upward two-tone tactile tick when placing a flag.
    FlagPlace,
    /// Soft downward two-tone tick when removing a flag.
    FlagRemove,
    /// Uplifting major-9th / pentatonic victory chime arpeggio.
    Win,
    /// Deep sub-bass detonation thud + filtered rumble + descending minor tone on loss.
    Lose,
    /// Subtle minimalist click for UI actions (restart, difficulty switch, pause, settings).
    UiClick,
}

#[derive(Clone, Debug)]
struct SoundBank {
    reveal_single: Vec<f32>,
    cascade_small: Vec<f32>,
    cascade_medium: Vec<f32>,
    cascade_large: Vec<f32>,
    chord: Vec<f32>,
    flag_place: Vec<f32>,
    flag_remove: Vec<f32>,
    win: Vec<f32>,
    lose: Vec<f32>,
    ui_click: Vec<f32>,
}

impl SoundBank {
    fn new() -> Self {
        Self {
            reveal_single: synthesize_effect(SoundEffect::RevealSingle),
            cascade_small: synthesize_effect(SoundEffect::RevealCascade(5)),
            cascade_medium: synthesize_effect(SoundEffect::RevealCascade(14)),
            cascade_large: synthesize_effect(SoundEffect::RevealCascade(32)),
            chord: synthesize_effect(SoundEffect::Chord),
            flag_place: synthesize_effect(SoundEffect::FlagPlace),
            flag_remove: synthesize_effect(SoundEffect::FlagRemove),
            win: synthesize_effect(SoundEffect::Win),
            lose: synthesize_effect(SoundEffect::Lose),
            ui_click: synthesize_effect(SoundEffect::UiClick),
        }
    }

    fn samples_for(&self, effect: SoundEffect) -> &[f32] {
        match effect {
            SoundEffect::RevealSingle => &self.reveal_single,
            SoundEffect::RevealCascade(n) => {
                if n <= 7 {
                    &self.cascade_small
                } else if n <= 20 {
                    &self.cascade_medium
                } else {
                    &self.cascade_large
                }
            }
            SoundEffect::Chord => &self.chord,
            SoundEffect::FlagPlace => &self.flag_place,
            SoundEffect::FlagRemove => &self.flag_remove,
            SoundEffect::Win => &self.win,
            SoundEffect::Lose => &self.lose,
            SoundEffect::UiClick => &self.ui_click,
        }
    }
}

enum AudioCommand {
    Play {
        effect: SoundEffect,
        volume: f32,
    },
}

/// Non-blocking procedural audio manager backed by a dedicated background playback thread.
/// Gracefully degrades to a silent no-op if the host machine has no audio output device.
pub struct SoundManager {
    enabled: bool,
    volume: f32,
    sender: Option<SyncSender<AudioCommand>>,
}

impl SoundManager {
    /// Creates a new `SoundManager` with the given initial mute and volume settings.
    pub fn new(enabled: bool, volume: f32) -> Self {
        let (tx, rx) = mpsc::sync_channel::<AudioCommand>(32);

        let spawn_res = thread::Builder::new()
            .name("minesweeper-audio".to_owned())
            .spawn(move || {
                let bank = SoundBank::new();
                // Keep OutputStream alive for the lifetime of the worker thread.
                let stream_pair = OutputStream::try_default().ok();

                while let Ok(cmd) = rx.recv() {
                    let AudioCommand::Play { effect, volume } = cmd;
                    if volume <= 0.001 {
                        continue;
                    }
                    if let Some((_, ref handle)) = stream_pair {
                        let samples = bank.samples_for(effect).to_vec();
                        let source =
                            SamplesBuffer::new(1, SAMPLE_RATE, samples).amplify(volume.clamp(0.0, 1.0));
                        let _ = handle.play_raw(source);
                    }
                }
            });

        Self {
            enabled,
            volume: sanitize_volume(volume),
            sender: spawn_res.ok().map(|_| tx),
        }
    }

    /// Creates a silent/headless `SoundManager` without spawning an audio output thread (useful for tests).
    pub fn silent(enabled: bool, volume: f32) -> Self {
        Self {
            enabled,
            volume: sanitize_volume(volume),
            sender: None,
        }
    }

    /// Returns whether sound effects are currently enabled.
    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enables or disables sound effects.
    #[inline]
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Returns the current master volume in `0.0..=1.0`.
    #[inline]
    pub fn volume(&self) -> f32 {
        self.volume
    }

    /// Sets the master volume (clamped to `0.0..=1.0`).
    #[inline]
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = sanitize_volume(volume);
    }

    /// Queues a sound effect for non-blocking playback if sound is enabled and volume > 0.
    pub fn play(&self, effect: SoundEffect) {
        if !self.enabled || self.volume <= 0.001 {
            return;
        }
        if let Some(ref tx) = self.sender {
            let _ = tx.try_send(AudioCommand::Play {
                effect,
                volume: self.volume,
            });
        }
    }
}

/// Clamps volume to `0.0..=1.0`, falling back to `0.75` if non-finite.
#[inline]
pub fn sanitize_volume(volume: f32) -> f32 {
    if volume.is_finite() {
        volume.clamp(0.0, 1.0)
    } else {
        0.75
    }
}

/// Synthesizes a mono `44.1 kHz` PCM buffer (`-1.0..=1.0`) for the given `SoundEffect`.
pub fn synthesize_effect(effect: SoundEffect) -> Vec<f32> {
    let mut buf = match effect {
        SoundEffect::RevealSingle => synth_reveal_single(),
        SoundEffect::RevealCascade(count) => synth_reveal_cascade(count),
        SoundEffect::Chord => synth_chord(),
        SoundEffect::FlagPlace => synth_flag_place(),
        SoundEffect::FlagRemove => synth_flag_remove(),
        SoundEffect::Win => synth_win(),
        SoundEffect::Lose => synth_lose(),
        SoundEffect::UiClick => synth_ui_click(),
    };
    apply_edge_declick(&mut buf);
    buf
}

/// Smooths the very first and last few samples to guarantee zero boundary clicks.
fn apply_edge_declick(samples: &mut [f32]) {
    let len = samples.len();
    if len == 0 {
        return;
    }
    let fade_in = 88.min(len / 2); // ~2ms
    let fade_out = 176.min(len / 2); // ~4ms

    for (i, sample) in samples.iter_mut().take(fade_in).enumerate() {
        let t = i as f32 / fade_in.max(1) as f32;
        let gain = 0.5 * (1.0 - (PI * t).cos());
        *sample = (*sample * gain).clamp(-0.95, 0.95);
    }
    for i in 0..fade_out {
        let idx = len - 1 - i;
        let t = i as f32 / fade_out.max(1) as f32;
        let gain = 0.5 * (1.0 - (PI * t).cos());
        samples[idx] = (samples[idx] * gain).clamp(-0.95, 0.95);
    }
    for sample in samples.iter_mut() {
        if !sample.is_finite() {
            *sample = 0.0;
        } else {
            *sample = sample.clamp(-0.95, 0.95);
        }
    }
}

/// Crisp, warm wooden/marimba-like pop for revealing a single cell (~65ms).
fn synth_reveal_single() -> Vec<f32> {
    let duration = 0.065;
    let total = (SAMPLE_RATE as f32 * duration) as usize;
    let mut out = vec![0.0; total];
    let mut phase = 0.0_f32;

    for (i, sample) in out.iter_mut().enumerate() {
        let t = i as f32 / SAMPLE_RATE as f32;
        // Quick upward pitch bend 480 Hz -> 620 Hz
        let freq = 480.0 + 140.0 * (1.0 - (-t * 55.0).exp());
        phase += TAU * freq / SAMPLE_RATE as f32;

        let env = (-t * 46.0).exp() * (1.0 - (-t * 450.0).exp());
        let fundamental = phase.sin();
        let overtone = (phase * 2.0).sin() * 0.22 * (-t * 75.0).exp();
        *sample = (fundamental + overtone) * env * 0.36;
    }
    out
}

/// Airy ascending major-pentatonic shimmer for flood-fill cascades.
fn synth_reveal_cascade(count: usize) -> Vec<f32> {
    // Pentatonic notes (C5, D5, E5, G5, A5, C6, D6)
    const NOTES: [f32; 7] = [523.25, 587.33, 659.25, 783.99, 880.00, 1046.50, 1174.66];
    let num_notes: usize = if count <= 7 {
        3
    } else if count <= 20 {
        5
    } else {
        7
    };
    let step_delay = 0.030_f32;
    let note_len = 0.12_f32;
    let total_dur = (num_notes.saturating_sub(1) as f32) * step_delay + note_len;
    let total = (SAMPLE_RATE as f32 * total_dur) as usize;
    let mut out = vec![0.0; total];

    for (n_idx, &freq) in NOTES.iter().take(num_notes).enumerate() {
        let start_sample = (n_idx as f32 * step_delay * SAMPLE_RATE as f32) as usize;
        let note_samples = (note_len * SAMPLE_RATE as f32) as usize;
        let mut phase = 0.0_f32;

        for i in 0..note_samples {
            let idx = start_sample + i;
            if idx >= total {
                break;
            }
            let t = i as f32 / SAMPLE_RATE as f32;
            phase += TAU * freq / SAMPLE_RATE as f32;

            let attack = (t / 0.005).min(1.0);
            let decay = (-t * 28.0).exp();
            let bell = phase.sin() + 0.18 * (phase * 2.0).sin() * (-t * 45.0).exp();
            out[idx] += bell * attack * decay * 0.22;
        }
    }
    out
}

/// Warm harmonic fifth dyad tap when chording opens adjacent cells (~95ms).
fn synth_chord() -> Vec<f32> {
    let duration = 0.095;
    let total = (SAMPLE_RATE as f32 * duration) as usize;
    let mut out = vec![0.0; total];
    let mut p1 = 0.0_f32;
    let mut p2 = 0.0_f32;

    for (i, sample) in out.iter_mut().enumerate() {
        let t = i as f32 / SAMPLE_RATE as f32;
        let f1 = 523.25; // C5
        let f2 = 783.99; // G5
        p1 += TAU * f1 / SAMPLE_RATE as f32;
        p2 += TAU * f2 / SAMPLE_RATE as f32;

        let attack1 = (t / 0.004).min(1.0);
        let env1 = attack1 * (-t * 34.0).exp();

        // Second note enters 14ms later for a subtle double-tap chord feel
        let t2 = (t - 0.014).max(0.0);
        let attack2 = (t2 / 0.004).min(1.0);
        let env2 = if t >= 0.014 {
            attack2 * (-t2 * 38.0).exp()
        } else {
            0.0
        };

        let tone1 = p1.sin() + 0.15 * (p1 * 2.0).sin();
        let tone2 = p2.sin() + 0.15 * (p2 * 2.0).sin();
        *sample = (tone1 * env1 * 0.24) + (tone2 * env2 * 0.22);
    }
    out
}

/// Upward two-tone tactile click when placing a flag (~75ms).
fn synth_flag_place() -> Vec<f32> {
    let duration = 0.075;
    let total = (SAMPLE_RATE as f32 * duration) as usize;
    let mut out = vec![0.0; total];
    let mut phase = 0.0_f32;

    for (i, sample) in out.iter_mut().enumerate() {
        let t = i as f32 / SAMPLE_RATE as f32;
        // Steps from A4 (440 Hz) to E5 (659.25 Hz) at 28ms
        let freq = if t < 0.028 { 440.0 } else { 659.25 };
        phase += TAU * freq / SAMPLE_RATE as f32;

        let local_t = if t < 0.028 { t } else { t - 0.028 };
        let attack = (local_t / 0.003).min(1.0);
        let decay = (-local_t * 42.0).exp();
        let wave = phase.sin() + 0.20 * (phase * 3.0).sin() * (-local_t * 80.0).exp();
        *sample = wave * attack * decay * 0.30;
    }
    out
}

/// Soft downward two-tone tick when removing a flag (~70ms).
fn synth_flag_remove() -> Vec<f32> {
    let duration = 0.070;
    let total = (SAMPLE_RATE as f32 * duration) as usize;
    let mut out = vec![0.0; total];
    let mut phase = 0.0_f32;

    for (i, sample) in out.iter_mut().enumerate() {
        let t = i as f32 / SAMPLE_RATE as f32;
        // Steps down from D5 (587.33 Hz) to A4 (440 Hz) at 26ms
        let freq = if t < 0.026 { 587.33 } else { 440.0 };
        phase += TAU * freq / SAMPLE_RATE as f32;

        let local_t = if t < 0.026 { t } else { t - 0.026 };
        let attack = (local_t / 0.003).min(1.0);
        let decay = (-local_t * 45.0).exp();
        let wave = phase.sin() + 0.14 * (phase * 2.0).sin();
        *sample = wave * attack * decay * 0.25;
    }
    out
}

/// Uplifting major-9th victory chime arpeggio (~620ms).
fn synth_win() -> Vec<f32> {
    // C5, E5, G5, B5, D6
    const CHIME_NOTES: [(f32, f32, f32); 5] = [
        (523.25, 0.00, 0.28),
        (659.25, 0.065, 0.28),
        (783.99, 0.130, 0.30),
        (987.77, 0.195, 0.32),
        (1174.66, 0.265, 0.35),
    ];
    let duration = 0.62;
    let total = (SAMPLE_RATE as f32 * duration) as usize;
    let mut out = vec![0.0; total];

    for &(freq, start_sec, hold_sec) in &CHIME_NOTES {
        let start_idx = (start_sec * SAMPLE_RATE as f32) as usize;
        let len_idx = (hold_sec * SAMPLE_RATE as f32) as usize;
        let mut phase = 0.0_f32;

        for i in 0..len_idx {
            let idx = start_idx + i;
            if idx >= total {
                break;
            }
            let t = i as f32 / SAMPLE_RATE as f32;
            phase += TAU * freq / SAMPLE_RATE as f32;

            let attack = (t / 0.006).min(1.0);
            let decay = (-t * 7.5).exp();
            let shimmer = phase.sin()
                + 0.22 * (phase * 2.0).sin() * (-t * 14.0).exp()
                + 0.08 * (phase * 3.0).sin() * (-t * 22.0).exp();
            out[idx] += shimmer * attack * decay * 0.20;
        }
    }
    out
}

/// Low, punchy sub-bass impact + filtered rumble + descending minor tone on mine detonation (~420ms).
fn synth_lose() -> Vec<f32> {
    let duration = 0.42;
    let total = (SAMPLE_RATE as f32 * duration) as usize;
    let mut out = vec![0.0; total];

    let mut sub_phase = 0.0_f32;
    let mut low_tone_phase = 0.0_f32;
    let mut lp_noise = 0.0_f32;
    // Deterministic LCG for smooth, repeatable filtered noise
    let mut rng_state = 0xA341_316Cu32;

    for (i, sample) in out.iter_mut().enumerate() {
        let t = i as f32 / SAMPLE_RATE as f32;

        // Pitch-dropping sub-bass impact (155 Hz -> 44 Hz)
        let sub_freq = 44.0 + 111.0 * (-t * 18.0).exp();
        sub_phase += TAU * sub_freq / SAMPLE_RATE as f32;
        let sub_env = (-t * 8.5).exp();
        let sub = sub_phase.sin() * sub_env * 0.48;

        // Muted descending minor tone (220 Hz -> 146.8 Hz)
        let tone_freq = 146.8 + 73.2 * (-t * 9.0).exp();
        low_tone_phase += TAU * tone_freq / SAMPLE_RATE as f32;
        let tone_env = (-t * 10.5).exp();
        let tone = (low_tone_phase.sin() + 0.25 * (low_tone_phase * 1.5).sin()) * tone_env * 0.20;

        // Low-pass filtered noise thud
        rng_state = rng_state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let white = ((rng_state >> 9) as f32 / 4_194_304.0) - 1.0;
        let cutoff_alpha = (0.14 * (-t * 7.0).exp()).clamp(0.015, 0.18);
        lp_noise += cutoff_alpha * (white - lp_noise);
        let noise_env = (-t * 11.5).exp();
        let rumble = lp_noise * noise_env * 0.42;

        *sample = sub + tone + rumble;
    }
    out
}

/// Subtle, ultra-short tactile UI click (~32ms).
fn synth_ui_click() -> Vec<f32> {
    let duration = 0.032;
    let total = (SAMPLE_RATE as f32 * duration) as usize;
    let mut out = vec![0.0; total];
    let mut phase = 0.0_f32;

    for (i, sample) in out.iter_mut().enumerate() {
        let t = i as f32 / SAMPLE_RATE as f32;
        let freq = 720.0 * (-t * 25.0).exp().max(0.5);
        phase += TAU * freq / SAMPLE_RATE as f32;
        let env = (t / 0.002).min(1.0) * (-t * 95.0).exp();
        *sample = phase.sin() * env * 0.20;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_synthesized_effects_are_valid_finite_and_declicked() {
        let effects = [
            SoundEffect::RevealSingle,
            SoundEffect::RevealCascade(3),
            SoundEffect::RevealCascade(12),
            SoundEffect::RevealCascade(35),
            SoundEffect::Chord,
            SoundEffect::FlagPlace,
            SoundEffect::FlagRemove,
            SoundEffect::Win,
            SoundEffect::Lose,
            SoundEffect::UiClick,
        ];

        for effect in effects {
            let samples = synthesize_effect(effect);
            assert!(
                samples.len() >= 1000,
                "Effect {effect:?} should have non-trivial sample length"
            );

            // All samples must be finite and within [-0.95, 0.95]
            for &s in &samples {
                assert!(s.is_finite(), "Non-finite sample in {effect:?}");
                assert!(
                    (-0.951..=0.951).contains(&s),
                    "Sample {s} out of safe headroom in {effect:?}"
                );
            }

            // Signal must have audible peak energy
            let peak = samples.iter().fold(0.0_f32, |acc, &x| acc.max(x.abs()));
            assert!(
                peak > 0.05,
                "Effect {effect:?} should have audible peak amplitude (got {peak})"
            );

            // First and last samples must be 0.0 due to edge de-clicking
            assert!(
                samples.first().copied().unwrap_or(1.0).abs() < 1e-5,
                "First sample of {effect:?} must start at zero"
            );
            assert!(
                samples.last().copied().unwrap_or(1.0).abs() < 1e-5,
                "Last sample of {effect:?} must end at zero"
            );
        }
    }

    #[test]
    fn test_sound_manager_state_and_volume_sanitization() {
        assert!((sanitize_volume(0.5) - 0.5).abs() < 1e-6);
        assert!((sanitize_volume(-2.0) - 0.0).abs() < 1e-6);
        assert!((sanitize_volume(5.0) - 1.0).abs() < 1e-6);
        assert!((sanitize_volume(f32::NAN) - 0.75).abs() < 1e-6);

        let mut mgr = SoundManager::silent(true, 0.8);
        assert!(mgr.is_enabled());
        assert!((mgr.volume() - 0.8).abs() < 1e-6);

        mgr.play(SoundEffect::RevealSingle);
        mgr.set_enabled(false);
        assert!(!mgr.is_enabled());
        mgr.play(SoundEffect::Win);

        mgr.set_volume(1.5);
        assert!((mgr.volume() - 1.0).abs() < 1e-6);

        // Edge cases: empty or tiny buffers in declick helper and extreme cascade counts
        let mut empty: [f32; 0] = [];
        apply_edge_declick(&mut empty);
        let mut single = [0.8_f32];
        apply_edge_declick(&mut single);
        assert!(single[0].is_finite());

        let zero_cascade = synthesize_effect(SoundEffect::RevealCascade(0));
        let max_cascade = synthesize_effect(SoundEffect::RevealCascade(usize::MAX));
        assert!(!zero_cascade.is_empty());
        assert!(max_cascade.len() > zero_cascade.len());

        // Worker-thread SoundManager lifecycle (with volume 0.0 so tests never emit audible sound)
        let live_mgr = SoundManager::new(true, 0.0);
        live_mgr.play(SoundEffect::UiClick);
    }
}
