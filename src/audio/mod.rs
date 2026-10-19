use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, OutputStreamHandle, Source};
use std::f32::consts::{PI, TAU};
use std::sync::mpsc::{self, SyncSender};
use std::thread;
use std::time::{Duration, Instant};

/// Standard mono sample rate (44.1 kHz) used for all procedurally generated sound effects.
pub const SAMPLE_RATE: u32 = 44_100;

/// Procedural sound effect types triggered by game and UI events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundEffect {
    /// Crisp, warm tactile pop when revealing a single cell.
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
/// Gracefully degrades to a silent no-op if the host machine has no audio output device,
/// and automatically reconnects if an audio device becomes available later.
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
                let mut stream_pair: Option<(OutputStream, OutputStreamHandle)> = None;
                let mut last_init_attempt: Option<Instant> = None;
                let mut last_played: Option<(SoundEffect, Instant)> = None;

                while let Ok(cmd) = rx.recv() {
                    let AudioCommand::Play { effect, volume } = cmd;
                    if volume <= 0.001 {
                        continue;
                    }

                    let now = Instant::now();
                    // Deduplicate identical sound effects triggered within < 12ms to prevent clipping
                    if let Some((prev_effect, prev_inst)) = last_played {
                        if std::mem::discriminant(&prev_effect) == std::mem::discriminant(&effect)
                            && now.duration_since(prev_inst) < Duration::from_millis(12)
                        {
                            continue;
                        }
                    }
                    last_played = Some((effect, now));

                    // Lazily open or retry opening the default OS audio stream (with a 2s retry cooldown)
                    let can_try_init = last_init_attempt
                        .map(|t| now.duration_since(t) >= Duration::from_secs(2))
                        .unwrap_or(true);
                    if stream_pair.is_none() && can_try_init {
                        last_init_attempt = Some(now);
                        stream_pair = OutputStream::try_default().ok();
                    }

                    let amp = volume.clamp(0.0, 1.0);
                    let mut stream_failed = false;
                    if let Some((_, ref handle)) = stream_pair {
                        let samples = bank.samples_for(effect).to_vec();
                        let source = SamplesBuffer::new(1, SAMPLE_RATE, samples).amplify(amp);
                        if handle.play_raw(source).is_err() {
                            stream_failed = true;
                        }
                    }

                    // If the active stream was invalidated (e.g. device hot-swap), try re-opening once
                    if stream_failed {
                        stream_pair = None;
                        if can_try_init {
                            last_init_attempt = Some(now);
                            stream_pair = OutputStream::try_default().ok();
                            if let Some((_, ref handle)) = stream_pair {
                                let samples = bank.samples_for(effect).to_vec();
                                let source =
                                    SamplesBuffer::new(1, SAMPLE_RATE, samples).amplify(amp);
                                let _ = handle.play_raw(source);
                            }
                        }
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

/// Selects the appropriate sound effect for a non-terminal reveal or chord operation.
///
/// - For a direct cell click (`is_chord == false`), revealing a single cell plays `RevealSingle`,
///   while flood-filling multiple cells plays `RevealCascade(len)`.
/// - For a chord action (`is_chord == true`), revealing only direct neighbors (`dist <= 1`)
///   always plays `Chord` (even when 3–7 neighbors open simultaneously), while a chord that
///   triggers a multi-step flood-fill wave (`dist > 1`) plays `RevealCascade(len)`.
pub fn select_reveal_sound(
    revealed: &[(usize, usize, usize)],
    is_chord: bool,
) -> Option<SoundEffect> {
    let len = revealed.len();
    if len == 0 {
        return None;
    }
    if is_chord {
        let has_flood_cascade = revealed.iter().any(|&(_, _, dist)| dist > 1);
        if has_flood_cascade {
            Some(SoundEffect::RevealCascade(len))
        } else {
            Some(SoundEffect::Chord)
        }
    } else if len > 1 {
        Some(SoundEffect::RevealCascade(len))
    } else {
        Some(SoundEffect::RevealSingle)
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
    if len == 1 {
        samples[0] = 0.0;
        return;
    }
    let fade_in = 88.min(len / 2).max(1); // ~2ms
    let fade_out = 176.min(len / 2).max(1); // ~4ms

    for (i, sample) in samples.iter_mut().take(fade_in).enumerate() {
        let t = i as f32 / fade_in as f32;
        let gain = 0.5 * (1.0 - (PI * t).cos());
        *sample = (*sample * gain).clamp(-0.95, 0.95);
    }
    for i in 0..fade_out {
        let idx = len - 1 - i;
        let t = i as f32 / fade_out as f32;
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

/// Smooth raised-cosine attack and tail release window (`0.0..=1.0`) for a note of length `note_len`.
#[inline]
fn smooth_note_window(t: f32, note_len: f32, attack_sec: f32, release_sec: f32) -> f32 {
    if t <= 0.0 || t >= note_len {
        return 0.0;
    }
    let attack = if t < attack_sec {
        0.5 * (1.0 - (PI * (t / attack_sec.max(1e-5))).cos())
    } else {
        1.0
    };
    let rem = note_len - t;
    let release = if rem < release_sec {
        0.5 * (1.0 - (PI * (rem / release_sec.max(1e-5))).cos())
    } else {
        1.0
    };
    attack * release
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

        let win = smooth_note_window(t, duration, 0.003, 0.012);
        let env = win * (-t * 44.0).exp();
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

            let win = smooth_note_window(t, note_len, 0.005, 0.018);
            let decay = (-t * 26.0).exp();
            let bell = phase.sin() + 0.18 * (phase * 2.0).sin() * (-t * 45.0).exp();
            out[idx] += bell * win * decay * 0.22;
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

        let win1 = smooth_note_window(t, duration, 0.004, 0.015);
        let env1 = win1 * (-t * 34.0).exp();

        // Second note enters 14ms later for a subtle double-tap chord feel
        let env2 = if t >= 0.014 {
            let t2 = t - 0.014;
            p2 += TAU * f2 / SAMPLE_RATE as f32;
            let win2 = smooth_note_window(t2, duration - 0.014, 0.004, 0.015);
            win2 * (-t2 * 38.0).exp()
        } else {
            0.0
        };

        let tone1 = p1.sin() + 0.15 * (p1 * 2.0).sin();
        let tone2 = p2.sin() + 0.15 * (p2 * 2.0).sin();
        *sample = (tone1 * env1 * 0.24) + (tone2 * env2 * 0.22);
    }
    out
}

/// Upward two-tone tactile click when placing a flag (~75ms) with overlapping envelopes.
fn synth_flag_place() -> Vec<f32> {
    let duration = 0.075;
    let total = (SAMPLE_RATE as f32 * duration) as usize;
    let mut out = vec![0.0; total];
    let mut p1 = 0.0_f32;
    let mut p2 = 0.0_f32;

    for (i, sample) in out.iter_mut().enumerate() {
        let t = i as f32 / SAMPLE_RATE as f32;
        // First tone: A4 (440 Hz)
        p1 += TAU * 440.0 / SAMPLE_RATE as f32;
        let win1 = smooth_note_window(t, duration, 0.003, 0.014);
        let env1 = win1 * (-t * 52.0).exp();
        let wave1 = p1.sin() + 0.18 * (p1 * 3.0).sin() * (-t * 80.0).exp();

        // Second tone: E5 (659.25 Hz) enters smoothly at 26ms
        let part2 = if t >= 0.026 {
            let t2 = t - 0.026;
            p2 += TAU * 659.25 / SAMPLE_RATE as f32;
            let win2 = smooth_note_window(t2, duration - 0.026, 0.0035, 0.014);
            let env2 = win2 * (-t2 * 44.0).exp();
            let wave2 = p2.sin() + 0.18 * (p2 * 3.0).sin() * (-t2 * 80.0).exp();
            wave2 * env2 * 0.28
        } else {
            0.0
        };

        *sample = wave1 * env1 * 0.26 + part2;
    }
    out
}

/// Soft downward two-tone tick when removing a flag (~70ms) with overlapping envelopes.
fn synth_flag_remove() -> Vec<f32> {
    let duration = 0.070;
    let total = (SAMPLE_RATE as f32 * duration) as usize;
    let mut out = vec![0.0; total];
    let mut p1 = 0.0_f32;
    let mut p2 = 0.0_f32;

    for (i, sample) in out.iter_mut().enumerate() {
        let t = i as f32 / SAMPLE_RATE as f32;
        // First tone: D5 (587.33 Hz)
        p1 += TAU * 587.33 / SAMPLE_RATE as f32;
        let win1 = smooth_note_window(t, duration, 0.003, 0.014);
        let env1 = win1 * (-t * 54.0).exp();
        let wave1 = p1.sin() + 0.14 * (p1 * 2.0).sin();

        // Second tone: A4 (440.0 Hz) enters smoothly at 24ms
        let part2 = if t >= 0.024 {
            let t2 = t - 0.024;
            p2 += TAU * 440.0 / SAMPLE_RATE as f32;
            let win2 = smooth_note_window(t2, duration - 0.024, 0.0035, 0.014);
            let env2 = win2 * (-t2 * 46.0).exp();
            let wave2 = p2.sin() + 0.14 * (p2 * 2.0).sin();
            wave2 * env2 * 0.24
        } else {
            0.0
        };

        *sample = wave1 * env1 * 0.24 + part2;
    }
    out
}

/// Uplifting major-9th victory chime arpeggio (~620ms).
fn synth_win() -> Vec<f32> {
    // C5, E5, G5, B5, D6
    const CHIME_NOTES: [(f32, f32, f32); 5] = [
        (523.25, 0.00, 0.32),
        (659.25, 0.065, 0.32),
        (783.99, 0.130, 0.34),
        (987.77, 0.195, 0.36),
        (1174.66, 0.265, 0.355),
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

            let win = smooth_note_window(t, hold_sec, 0.006, 0.035);
            let decay = (-t * 7.5).exp();
            let shimmer = phase.sin()
                + 0.22 * (phase * 2.0).sin() * (-t * 14.0).exp()
                + 0.08 * (phase * 3.0).sin() * (-t * 22.0).exp();
            out[idx] += shimmer * win * decay * 0.20;
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
        let win = smooth_note_window(t, duration, 0.003, 0.025);

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

        *sample = (sub + tone + rumble) * win;
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
        let win = smooth_note_window(t, duration, 0.002, 0.006);
        let env = win * (-t * 95.0).exp();
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

            // Sample-to-sample continuity check: verify no internal step discontinuities / mid-buffer clicks
            let max_delta = samples
                .windows(2)
                .fold(0.0_f32, |acc, w| acc.max((w[1] - w[0]).abs()));
            assert!(
                max_delta < 0.12,
                "Effect {effect:?} has an abrupt sample-to-sample jump of {max_delta}"
            );
        }
    }

    #[test]
    fn test_sound_manager_state_volume_sanitization_and_burst_spam() {
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
        assert!(single[0].abs() < 1e-6);

        let zero_cascade = synthesize_effect(SoundEffect::RevealCascade(0));
        let max_cascade = synthesize_effect(SoundEffect::RevealCascade(usize::MAX));
        assert!(!zero_cascade.is_empty());
        assert!(max_cascade.len() > zero_cascade.len());

        // Stress-test worker-thread SoundManager with rapid 100-message spam without blocking
        let mut live_mgr = SoundManager::new(true, 0.0005); // sub-threshold volume stays silent in tests
        for i in 0..100 {
            live_mgr.play(SoundEffect::RevealCascade(i));
            live_mgr.play(SoundEffect::UiClick);
        }
        live_mgr.set_volume(0.0);
        live_mgr.play(SoundEffect::Win);

        // Verify reveal vs chord vs cascade sound selection
        assert_eq!(select_reveal_sound(&[], false), None);
        assert_eq!(select_reveal_sound(&[], true), None);
        assert_eq!(
            select_reveal_sound(&[(2, 2, 0)], false),
            Some(SoundEffect::RevealSingle)
        );
        assert_eq!(
            select_reveal_sound(&[(2, 2, 0), (2, 3, 1), (3, 3, 1)], false),
            Some(SoundEffect::RevealCascade(3))
        );
        // Chording 4 direct neighbors (all at dist == 1) must play Chord, not RevealCascade
        let chord_direct = [(1, 0, 1), (2, 0, 1), (0, 1, 1), (2, 1, 1)];
        assert_eq!(
            select_reveal_sound(&chord_direct, true),
            Some(SoundEffect::Chord)
        );
        // Chording that triggers a multi-step flood-fill wave (dist > 1) plays RevealCascade
        let chord_flood = [(1, 0, 1), (2, 0, 1), (3, 0, 2), (4, 0, 3)];
        assert_eq!(
            select_reveal_sound(&chord_flood, true),
            Some(SoundEffect::RevealCascade(4))
        );
    }
}
