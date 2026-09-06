//! A dependency-free stereo limiter for the final local-playback stage.
//!
//! Equalizer boosts are intentionally kept in floating-point samples until
//! the end of the processing chain. This limiter provides the one final
//! ceiling before samples reach the output, using the louder channel to drive
//! a shared gain so the stereo image never moves. It has a short look-ahead,
//! a soft knee, and attack/release smoothing instead of hard clipping.

use std::collections::VecDeque;

/// Limiting completes one decibel under full scale, leaving room for peaks
/// created between samples by a later output-rate conversion.
const THRESHOLD_DB: f64 = -1.0;
/// Gain reduction starts two decibels below the threshold and reaches the
/// full requested reduction two decibels above it.
const KNEE_DB: f64 = 2.0;
const ATTACK_MS: f64 = 1.5;
const RELEASE_MS: f64 = 100.0;
/// Looking ahead lets the gain arrive before a transient reaches the output.
const LOOKAHEAD_MS: f64 = 8.0;

fn coefficient(milliseconds: f64, sample_rate: f64) -> f64 {
    if milliseconds <= 0.0 {
        1.0
    } else {
        1.0 - (-1.0 / (milliseconds / 1_000.0 * sample_rate)).exp()
    }
}

fn to_db(linear: f64) -> f64 {
    20.0 * linear.max(1e-12).log10()
}

fn from_db(db: f64) -> f64 {
    10f64.powf(db / 20.0)
}

/// Soft-knee gain reduction for a peak's distance above the threshold.
fn reduction_db(over_db: f64) -> f64 {
    if over_db <= -KNEE_DB / 2.0 {
        0.0
    } else if over_db >= KNEE_DB / 2.0 {
        over_db
    } else {
        let above = over_db + KNEE_DB / 2.0;
        above * above / (2.0 * KNEE_DB)
    }
}

/// Stateful stereo limiter. Its look-ahead queue makes each processed block
/// leave exactly as many samples as it received; the stream is delayed only
/// by the queue's fixed length.
pub struct Limiter {
    gain: f64,
    attack: f64,
    release: f64,
    held: VecDeque<[f64; 2]>,
}

impl Limiter {
    pub fn new(sample_rate: f64) -> Self {
        let lookahead = ((LOOKAHEAD_MS / 1_000.0 * sample_rate).round() as usize).max(1);
        Self {
            gain: 1.0,
            attack: coefficient(ATTACK_MS, sample_rate),
            release: coefficient(RELEASE_MS, sample_rate),
            held: VecDeque::from(vec![[0.0; 2]; lookahead]),
        }
    }

    /// Limits interleaved stereo samples to `full_scale`.
    ///
    /// When the output sink applies volume after this stage, `full_scale` is
    /// the pre-volume level that becomes one at the speaker. This preserves
    /// quiet-listener headroom for EQ boosts. A non-positive scale means the
    /// output is silent and needs no finite ceiling.
    pub fn process(&mut self, samples: &mut [f64], full_scale: f64) {
        if !(full_scale.is_finite() && full_scale > 0.0) {
            return;
        }
        let threshold_db = to_db(full_scale) + THRESHOLD_DB;
        for frame in samples.chunks_mut(2) {
            let peak = frame
                .iter()
                .fold(0.0f64, |loudest, sample| loudest.max(sample.abs()));
            let target = from_db(-reduction_db(to_db(peak) - threshold_db));
            let smoothing = if target < self.gain {
                self.attack
            } else {
                self.release
            };
            self.gain += (target - self.gain) * smoothing;

            self.held
                .push_back([frame[0], frame.get(1).copied().unwrap_or(0.0)]);
            let due = self.held.pop_front().unwrap_or([0.0; 2]);
            for (sample, held) in frame.iter_mut().zip(due) {
                *sample = (held * self.gain).clamp(-full_scale, full_scale);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f64 = 44_100.0;
    const DELAY: usize = 353;

    fn tone(level: f64, frames: usize) -> Vec<f64> {
        (0..frames * 2).map(|_| level).collect()
    }

    fn peak(samples: &[f64]) -> f64 {
        samples
            .iter()
            .fold(0.0f64, |loudest, sample| loudest.max(sample.abs()))
    }

    #[test]
    fn quiet_sound_passes_untouched() {
        let mut limiter = Limiter::new(RATE);
        let mut samples = tone(0.5, DELAY * 2);
        limiter.process(&mut samples, 1.0);
        assert!(
            samples[DELAY * 2..]
                .iter()
                .all(|sample| (sample - 0.5).abs() < 1e-12)
        );
    }

    #[test]
    fn loud_boost_is_bounded() {
        let mut limiter = Limiter::new(RATE);
        let mut samples = tone(4.0, 44_100);
        limiter.process(&mut samples, 1.0);
        assert!(peak(&samples) <= 1.0);
    }

    #[test]
    fn a_transient_is_caught_before_the_output_ceiling() {
        let mut limiter = Limiter::new(RATE);
        let mut samples = tone(0.0, DELAY);
        samples.extend(tone(4.0, DELAY * 2));
        limiter.process(&mut samples, 1.0);
        let loudest = peak(&samples);
        assert!(loudest < 0.95, "gain was not down in time: {loudest}");
        assert!(loudest > from_db(THRESHOLD_DB), "signal was not limited");
    }

    #[test]
    fn stereo_channels_share_gain() {
        let mut limiter = Limiter::new(RATE);
        let mut samples: Vec<f64> = (0..DELAY * 4).flat_map(|_| [4.0, 0.25]).collect();
        limiter.process(&mut samples, 1.0);
        for frame in samples[DELAY * 2..].chunks(2) {
            assert!((frame[0] / frame[1] - 16.0).abs() < 1e-9);
        }
    }

    #[test]
    fn a_post_volume_ceiling_preserves_headroom() {
        let mut limiter = Limiter::new(RATE);
        let mut samples = tone(1.0, DELAY * 2);
        limiter.process(&mut samples, 4.0);
        assert!(
            samples[DELAY * 2..]
                .iter()
                .all(|sample| (sample - 1.0).abs() < 1e-12)
        );
    }

    #[test]
    fn release_is_slow_and_blocks_pumping() {
        let mut limiter = Limiter::new(RATE);
        limiter.process(&mut tone(4.0, 4_410), 1.0);
        let held = limiter.gain;
        assert!(held < 0.3);
        limiter.process(&mut tone(0.1, 441), 1.0);
        assert!(limiter.gain > held && limiter.gain < 1.0);
    }

    #[test]
    fn every_block_keeps_its_input_length() {
        let mut limiter = Limiter::new(RATE);
        for frames in [1, 7, 512, 4_410] {
            let mut samples = tone(0.3, frames);
            let length = samples.len();
            limiter.process(&mut samples, 1.0);
            assert_eq!(samples.len(), length);
        }
    }
}
