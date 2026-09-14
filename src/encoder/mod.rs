//! TMS5220 LPC encoder, adapted from speakie_enc (which only ships as a binary).

use std::f64::consts::PI;

use output::Output;
use pitch::PitchEstimator;
use reflector::Reflector;

mod filter;
mod output;
mod pitch;
mod reflector;

pub const SAMPLE_RATE: u32 = 8000;
const FRAME_SIZE: usize = 200;
const WINDOW_SIZE: usize = 300;

pub struct Settings {
    pub gain: f64,
    pub voiced_threshold: f64,
    /// Pitch search range, in samples at 8 kHz.
    pub min_period: usize,
    pub max_period: usize,
    pub whisper: bool,
}

/// Encodes 8 kHz mono samples (i16 scale) to a TMS5220 bitstream ending in a stop frame.
pub fn encode(samples: &[f64], settings: &Settings) -> Vec<u8> {
    let mut out = Output::default();
    let hw = hamming_window();
    let n_frames = samples.len().div_ceil(FRAME_SIZE);
    let filtered = filter::lowpass(samples);
    let preemph = convolve(samples);
    for i in 0..n_frames {
        let base = i * FRAME_SIZE;
        let filtered_slice = (0..WINDOW_SIZE)
            .map(|i| filtered.get(base + i).cloned().unwrap_or_default())
            .collect::<Vec<_>>();
        let mut period = if settings.whisper {
            0.0
        } else {
            PitchEstimator::new(&filtered_slice, settings.min_period, settings.max_period)
                .estimate()
        };
        // Written as a negated `>=` so NaN confidence (digital silence) counts as unvoiced.
        if !(period > 0.0
            && reflector::confidence(&filtered_slice, period.round() as usize)
                >= settings.voiced_threshold)
        {
            period = 0.0;
        }
        let lpc_input = if period == 0.0 { samples } else { &preemph };
        let windowed = (0..WINDOW_SIZE)
            .map(|i| lpc_input.get(base + i).cloned().unwrap_or_default() * hw[i])
            .collect::<Vec<_>>();
        let reflector = Reflector::new(&windowed);
        let mut rms = reflector.rms();
        if period == 0.0 {
            rms *= 0.5;
        } else {
            // Compensate for energy of chirp. Note: it would likely be more
            // accurate to measure RMS energy of (truncated) chirp.
            rms *= period * (1. / 60.);
        }
        out.frame(settings.gain * rms, period, &reflector.ks()[1..]);
    }
    out.pack(15, 4);
    out.pack(0, 7);
    out.reap()
}

fn hamming_window() -> Vec<f64> {
    (0..WINDOW_SIZE)
        .map(|i| 0.54 - 0.46 * (2. * PI * i as f64 / (WINDOW_SIZE - 1) as f64).cos())
        .collect()
}

// Filter was computed as inverse FFT of 1/FFT(chirp)
const INV_CHIRP: [f64; 52] = [
    3.50463373e-02,
    -5.26616519e-02,
    1.31415516e-01,
    -2.58773275e-01,
    4.81184554e-01,
    -7.82785270e-01,
    1.03541716e+00,
    -1.10341696e+00,
    6.87983148e-01,
    2.16731498e-01,
    -9.48286645e-01,
    6.10189416e-01,
    3.30566190e-01,
    -3.17949706e-01,
    -4.23637262e-01,
    -8.63620459e-03,
    6.71765897e-01,
    3.14808586e-01,
    -3.11478357e-02,
    1.92005988e-02,
    -1.24954871e-01,
    -1.99774949e-01,
    -1.01084941e-01,
    4.43973504e-02,
    1.41367975e-02,
    -1.21299209e-01,
    -4.40414891e-02,
    9.12421065e-02,
    5.53856583e-02,
    -4.67263991e-02,
    -6.88859302e-02,
    -1.87744830e-02,
    2.11123339e-02,
    5.13570807e-02,
    8.27863631e-02,
    6.20288495e-02,
    7.15608471e-04,
    -2.85681320e-02,
    -1.25371199e-02,
    -5.04754889e-03,
    -2.16811461e-02,
    -2.87548991e-02,
    -1.55170026e-02,
    4.53067505e-03,
    5.72569839e-04,
    6.57422333e-03,
    -4.75623079e-03,
    -1.44444911e-02,
    -5.08071962e-03,
    -6.75921576e-03,
    2.16096095e-02,
    8.06522593e-03,
];

const DELAY: usize = 15;

fn convolve(inp: &[f64]) -> Vec<f64> {
    (0..inp.len())
        .map(|i| {
            INV_CHIRP
                .iter()
                .enumerate()
                .map(|(j, y)| {
                    y * inp
                        .get(i.wrapping_sub(j.wrapping_sub(DELAY)))
                        .cloned()
                        .unwrap_or_default()
                })
                .sum()
        })
        .collect()
}
