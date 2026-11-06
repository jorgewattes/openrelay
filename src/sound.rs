//! Soft siren sound for the buzzer: a gentle rising/falling tone ("wail")
//! made of sine waves, faded in and out, at low volume.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct Siren {
    on: Arc<AtomicBool>,
    device: Option<tinyaudio::OutputDevice>,
    failed: bool,
}

impl Default for Siren {
    fn default() -> Self {
        Siren { on: Arc::new(AtomicBool::new(false)), device: None, failed: false }
    }
}

impl Siren {
    /// Turn the sound on or off. The audio output is opened the first time it
    /// is needed (in browsers this happens after a click, so it is allowed).
    pub fn set(&mut self, on: bool) {
        self.on.store(on, Ordering::Relaxed);
        if on && self.device.is_none() && !self.failed {
            self.start();
        }
    }

    fn start(&mut self) {
        let params = tinyaudio::OutputDeviceParameters {
            channels_count: 2,
            sample_rate: 44_100,
            channel_sample_count: 2_205,
        };
        let on = self.on.clone();
        let rate = params.sample_rate as f32;
        let ch = params.channels_count;
        let mut t = 0.0f32; // seconds
        let mut phase = 0.0f32; // oscillator phase (0..1)
        let mut gain = 0.0f32; // smoothed volume
        let result = tinyaudio::run_output_device(params, move |data| {
            let target = if on.load(Ordering::Relaxed) { 0.07 } else { 0.0 };
            for frame in data.chunks_mut(ch) {
                // Smooth fade (about 60 ms) so it never clicks.
                gain += (target - gain) * (1.0 / (0.06 * rate));
                // Wail: 620 Hz ↔ 880 Hz, 1.6 s per cycle, sinusoidal sweep.
                let sweep = 0.5 - 0.5 * (std::f32::consts::TAU * t / 1.6).cos();
                let freq = 620.0 + 260.0 * sweep;
                phase = (phase + freq / rate).fract();
                let x = (std::f32::consts::TAU * phase).sin();
                // A touch of the 2nd harmonic gives it a horn-like colour, still soft.
                let h = (2.0 * std::f32::consts::TAU * phase).sin() * 0.18;
                let s = (x + h) * gain;
                for v in frame.iter_mut() {
                    *v = s;
                }
                if gain > 1e-4 {
                    t += 1.0 / rate;
                } else {
                    t = 0.0;
                }
            }
        });
        match result {
            Ok(d) => self.device = Some(d),
            Err(_) => self.failed = true,
        }
    }
}
