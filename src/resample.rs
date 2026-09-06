//! Stateful sample-rate conversion for output devices that do not accept
//! Spotify's native 44.1 kHz stream.
//!
//! Most Spotify tracks arrive as stereo 44.1 kHz audio. Some shared-mode
//! devices, especially on Windows, insist on another rate such as 48 kHz.
//! Restarting an interpolator for every decoder packet can make a small
//! discontinuity at every packet boundary. This converter keeps the filter
//! history between packets so the output is one continuous signal.
//!
//! The implementation is a polyphase, windowed-sinc converter. Only the
//! phases needed by the rational input/output rate ratio are built, and the
//! input is interleaved by channel.

use std::f64::consts::PI;

/// Input samples each output sample is made from. Sixty-four taps keep the
/// audible passband clean while keeping per-packet work bounded.
const TAPS: usize = 64;

pub struct Resampler {
    up: usize,
    down: usize,
    channels: usize,
    /// `up` phases of `TAPS` coefficients each, normalised to unit gain.
    taps: Vec<f32>,
    /// Interleaved input frames still needed by the next output frame.
    input: Vec<f32>,
    /// The input frame at or immediately before the next output position.
    next: usize,
    /// Fractional output position in `1 / up` input steps.
    phase: usize,
}

impl Resampler {
    /// Returns `None` when the rates agree or the configuration is unusable.
    pub fn new(from_hz: u32, to_hz: u32, channels: usize) -> Option<Self> {
        if from_hz == to_hz || from_hz == 0 || to_hz == 0 || channels == 0 {
            return None;
        }
        let divisor = gcd(from_hz, to_hz);
        let up = (to_hz / divisor) as usize;
        let down = (from_hz / divisor) as usize;
        let half = TAPS / 2;
        Some(Self {
            up,
            down,
            channels,
            taps: kernel(up, down),
            input: vec![0.0; (half - 1) * channels],
            next: half - 1,
            phase: 0,
        })
    }

    /// Converts the interleaved samples in one chunk.
    ///
    /// The filter deliberately waits until it has enough future input for a
    /// complete window. The tail remains in `self.input` and is joined to the
    /// next chunk, so chunk boundaries do not change the result.
    pub fn process(&mut self, samples: &[f32]) -> Vec<f32> {
        self.input.extend_from_slice(samples);
        let half = TAPS / 2;
        let frames = self.input.len() / self.channels;
        let expected = samples.len() * self.up / self.down + self.channels;
        let mut out = Vec::with_capacity(expected);
        while self.next + half < frames {
            let taps = &self.taps[self.phase * TAPS..(self.phase + 1) * TAPS];
            let start = (self.next + 1 - half) * self.channels;
            for channel in 0..self.channels {
                let sum: f32 = taps
                    .iter()
                    .enumerate()
                    .map(|(tap, coefficient)| {
                        self.input[start + tap * self.channels + channel] * coefficient
                    })
                    .sum();
                out.push(sum);
            }
            let position = self.phase + self.down;
            self.next += position / self.up;
            self.phase = position % self.up;
        }
        // Keep the history that the next output window can still reach.
        let keep_from = (self.next + 1 - half).min(frames);
        self.input.drain(..keep_from * self.channels);
        self.next -= keep_from;
        out
    }
}

/// Builds every fractional phase of a low-pass Blackman-windowed sinc.
fn kernel(up: usize, down: usize) -> Vec<f32> {
    let half = (TAPS / 2) as f64;
    let cutoff = 0.475 * (up as f64 / down as f64).min(1.0);
    let mut taps = Vec::with_capacity(up * TAPS);
    for phase in 0..up {
        let offset = phase as f64 / up as f64;
        let start = taps.len();
        for tap in 0..TAPS {
            let u = offset + half - 1.0 - tap as f64;
            let x = u / half;
            let window = 0.42 + 0.5 * (PI * x).cos() + 0.08 * (2.0 * PI * x).cos();
            let angle = 2.0 * PI * cutoff * u;
            let sinc = if u == 0.0 { 1.0 } else { angle.sin() / angle };
            taps.push((2.0 * cutoff * sinc * window) as f32);
        }
        let sum: f32 = taps[start..].iter().sum();
        for coefficient in &mut taps[start..] {
            *coefficient /= sum;
        }
    }
    taps
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(hz: f64, rate: u32, frames: usize) -> Vec<f32> {
        (0..frames)
            .flat_map(|frame| {
                let sample = 0.5 * (2.0 * PI * hz * frame as f64 / f64::from(rate)).sin() as f32;
                [sample, sample]
            })
            .collect()
    }

    fn check_tone(from: u32, to: u32, tolerance: f32) {
        let mut resampler = Resampler::new(from, to, 2).expect("rates differ");
        let input = tone(1_000.0, from, from as usize);
        let output = resampler.process(&input);
        let frames = output.len() / 2;
        assert!(
            (frames as i64 - i64::from(to)).abs() < 100,
            "{frames} frames"
        );
        let mut worst = 0.0f32;
        for frame in 200..frames - 200 {
            let time = frame as f64 * f64::from(from) / f64::from(to) / f64::from(from);
            let ideal = (0.5 * (2.0 * PI * 1_000.0 * time).sin()) as f32;
            worst = worst.max((output[2 * frame] - ideal).abs());
            assert_eq!(output[2 * frame], output[2 * frame + 1]);
        }
        assert!(worst < tolerance, "{from} to {to}: off by {worst}");
    }

    #[test]
    fn equal_rates_need_no_converter() {
        assert!(Resampler::new(44_100, 44_100, 2).is_none());
    }

    #[test]
    fn a_tone_keeps_pitch_and_level_when_upsampling() {
        check_tone(44_100, 48_000, 0.005);
    }

    #[test]
    fn a_tone_keeps_pitch_and_level_when_downsampling() {
        check_tone(44_100, 22_050, 0.02);
    }

    #[test]
    fn splitting_input_into_chunks_does_not_change_output() {
        let input = tone(440.0, 44_100, 20_000);
        let whole = Resampler::new(44_100, 48_000, 2)
            .expect("rates differ")
            .process(&input);
        let mut chunked = Resampler::new(44_100, 48_000, 2).expect("rates differ");
        let mut output = Vec::new();
        let mut at = 0;
        for size in [2, 14, 200, 3_256, 6, 1_000, 2, 8_000].iter().cycle() {
            if at >= input.len() {
                break;
            }
            let end = (at + size).min(input.len());
            output.extend(chunked.process(&input[at..end]));
            at = end;
        }
        assert_eq!(output, whole);
    }
}
