//! Platform-independent core logic for yapping: the hold-to-talk session
//! state machine and audio resampling. Unit tested on any OS.

use std::time::{Duration, Instant};

/// Events from the hotkey layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEvent {
    Pressed,
    Released,
    Cancelled,
}

/// What the app should do in response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    StartRecording,
    StopAndTranscribe,
    StopAndDiscard,
    Ignore,
}

/// Hold-to-talk decisions, ported from the macOS AppDelegate:
/// holds shorter than min_hold are accidental taps and discard;
/// Esc mid-hold discards; a stuck hold force-stops at max_hold.
pub struct Session {
    recording: bool,
    cancelled: bool,
    pressed_at: Option<Instant>,
    pub min_hold: Duration,
    pub max_hold: Duration,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            recording: false,
            cancelled: false,
            pressed_at: None,
            min_hold: Duration::from_millis(350),
            max_hold: Duration::from_secs(300),
        }
    }
}

impl Session {
    pub fn handle(&mut self, event: KeyEvent) -> Action {
        self.handle_at(event, Instant::now())
    }

    pub fn handle_at(&mut self, event: KeyEvent, now: Instant) -> Action {
        match event {
            KeyEvent::Pressed => {
                if self.recording {
                    return Action::Ignore;
                }
                self.recording = true;
                self.cancelled = false;
                self.pressed_at = Some(now);
                Action::StartRecording
            }
            KeyEvent::Released => {
                if !self.recording {
                    return Action::Ignore;
                }
                self.recording = false;
                let held = self
                    .pressed_at
                    .map(|t| now.duration_since(t))
                    .unwrap_or_default();
                if self.cancelled || held < self.min_hold {
                    Action::StopAndDiscard
                } else {
                    Action::StopAndTranscribe
                }
            }
            KeyEvent::Cancelled => {
                if self.recording {
                    self.cancelled = true;
                }
                Action::Ignore
            }
        }
    }

    pub fn is_recording(&self) -> bool {
        self.recording
    }

    /// True when a running hold has exceeded max_hold (caller polls).
    pub fn overdue(&self, now: Instant) -> bool {
        self.recording
            && self
                .pressed_at
                .map(|t| now.duration_since(t) > self.max_hold)
                .unwrap_or(false)
    }
}

/// Linear-interpolation resample to 16 kHz mono. Adequate for speech
/// recognition; swap for a windowed-sinc resampler if quality ever bites.
pub fn resample_to_16k_mono(samples: &[f32], channels: usize, rate: u32) -> Vec<f32> {
    if samples.is_empty() || channels == 0 {
        return Vec::new();
    }
    // Downmix interleaved channels
    let mono: Vec<f32> = if channels == 1 {
        samples.to_vec()
    } else {
        samples
            .chunks(channels)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
            .collect()
    };
    if rate == 16_000 {
        return mono;
    }
    let ratio = rate as f64 / 16_000.0;
    let out_len = (mono.len() as f64 / ratio) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let pos = i as f64 * ratio;
        let idx = pos as usize;
        let frac = (pos - idx as f64) as f32;
        let a = mono[idx.min(mono.len() - 1)];
        let b = mono[(idx + 1).min(mono.len() - 1)];
        out.push(a + (b - a) * frac);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(base: Instant, ms: u64) -> Instant {
        base + Duration::from_millis(ms)
    }

    #[test]
    fn hold_and_release_transcribes() {
        let base = Instant::now();
        let mut s = Session::default();
        assert_eq!(s.handle_at(KeyEvent::Pressed, base), Action::StartRecording);
        assert_eq!(
            s.handle_at(KeyEvent::Released, t(base, 1000)),
            Action::StopAndTranscribe
        );
    }

    #[test]
    fn quick_tap_discards() {
        let base = Instant::now();
        let mut s = Session::default();
        s.handle_at(KeyEvent::Pressed, base);
        assert_eq!(
            s.handle_at(KeyEvent::Released, t(base, 100)),
            Action::StopAndDiscard
        );
    }

    #[test]
    fn escape_discards() {
        let base = Instant::now();
        let mut s = Session::default();
        s.handle_at(KeyEvent::Pressed, base);
        s.handle_at(KeyEvent::Cancelled, t(base, 500));
        assert_eq!(
            s.handle_at(KeyEvent::Released, t(base, 1000)),
            Action::StopAndDiscard
        );
    }

    #[test]
    fn double_press_ignored() {
        let base = Instant::now();
        let mut s = Session::default();
        s.handle_at(KeyEvent::Pressed, base);
        assert_eq!(s.handle_at(KeyEvent::Pressed, t(base, 50)), Action::Ignore);
    }

    #[test]
    fn release_without_press_ignored() {
        let mut s = Session::default();
        assert_eq!(s.handle(KeyEvent::Released), Action::Ignore);
    }

    #[test]
    fn overdue_after_max_hold() {
        let base = Instant::now();
        let mut s = Session::default();
        s.handle_at(KeyEvent::Pressed, base);
        assert!(!s.overdue(t(base, 1000)));
        assert!(s.overdue(base + Duration::from_secs(301)));
    }

    #[test]
    fn resample_halves_length_at_32k() {
        let input: Vec<f32> = (0..3200).map(|i| (i as f32).sin()).collect();
        let out = resample_to_16k_mono(&input, 1, 32_000);
        assert!((out.len() as i64 - 1600).abs() <= 2);
    }

    #[test]
    fn stereo_downmix() {
        let input = vec![1.0, 0.0, 1.0, 0.0];
        let out = resample_to_16k_mono(&input, 2, 16_000);
        assert_eq!(out, vec![0.5, 0.5]);
    }
}
