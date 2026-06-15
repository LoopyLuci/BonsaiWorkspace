#![allow(
    clippy::needless_range_loop,
    clippy::manual_clamp,
    clippy::useless_format,
    clippy::needless_borrow,
    clippy::manual_unwrap_or
)]

use std::f32::consts::PI;
/// music-worker — persistent music synthesis sidecar.
///
/// Protocol (stdin → stdout):
///   Request line:  `<id>|<duration_f32>|<prompt>\n`
///   Response:      `OK <id>|<wav_byte_count>\n` followed by raw WAV bytes
///
/// Runs as a TCP server: prints `BONSAI_MUSIC_PORT=<port>` on stdout line 1,
/// then accepts a single long-lived connection for the lifetime of the process.
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::TcpListener;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    // Print port on stderr so it doesn't interfere with binary stdout protocol
    eprintln!("BONSAI_MUSIC_PORT={port}");
    println!("BONSAI_MUSIC_PORT={port}");
    std::io::stdout().flush().ok();

    let (stream, _) = listener.accept().expect("accept");
    let mut reader = BufReader::new(stream.try_clone().expect("clone"));
    let mut writer = BufWriter::new(stream);

    let mut line = String::new();
    while reader.read_line(&mut line).unwrap_or(0) > 0 {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            line.clear();
            continue;
        }

        let parts: Vec<&str> = trimmed.splitn(3, '|').collect();
        if parts.len() < 3 {
            line.clear();
            continue;
        }

        let id = parts[0];
        let duration: f32 = parts[1].parse::<f32>().unwrap_or(8.0).clamp(0.5, 60.0);
        let prompt = parts[2];

        let wav = generate_wav(prompt, duration);
        let header = format!("OK {id}|{}\n", wav.len());
        if writer.write_all(header.as_bytes()).is_err() {
            break;
        }
        if writer.write_all(&wav).is_err() {
            break;
        }
        if writer.flush().is_err() {
            break;
        }
        line.clear();
    }
}

// ── Synthesis ──────────────────────────────────────────────────────────────────

struct SynthParams {
    bpm: f32,
    root_hz: f32,
    scale: Vec<f32>, // semitone offsets
    brightness: f32, // 0 = dark, 1 = bright
    has_drums: bool,
    reverb_mix: f32,
}

fn parse_prompt(prompt: &str) -> SynthParams {
    let lower = prompt.to_lowercase();

    // BPM
    let bpm = if lower.contains("slow") || lower.contains("ambient") || lower.contains("chill") {
        70.0
    } else if lower.contains("fast") || lower.contains("energetic") || lower.contains("upbeat") {
        140.0
    } else if let Some(n) = extract_bpm(&lower) {
        n
    } else {
        100.0
    };

    // Root note
    let root_hz = if lower.contains(" a ") || lower.contains("in a") {
        440.0
    } else if lower.contains(" b ") || lower.contains("in b") {
        493.88
    } else if lower.contains(" c ") || lower.contains("in c") {
        261.63
    } else if lower.contains(" d ") || lower.contains("in d") {
        293.66
    } else if lower.contains(" e ") || lower.contains("in e") {
        329.63
    } else if lower.contains(" f ") || lower.contains("in f") {
        349.23
    } else if lower.contains(" g ") || lower.contains("in g") {
        392.00
    } else {
        220.0
    };

    // Scale
    let scale = if lower.contains("minor")
        || lower.contains("sad")
        || lower.contains("dark")
        || lower.contains("melanchol")
    {
        vec![0.0, 2.0, 3.0, 5.0, 7.0, 8.0, 10.0] // natural minor
    } else if lower.contains("pentatonic") {
        vec![0.0, 2.0, 4.0, 7.0, 9.0]
    } else if lower.contains("blues") {
        vec![0.0, 3.0, 5.0, 6.0, 7.0, 10.0]
    } else if lower.contains("dorian") {
        vec![0.0, 2.0, 3.0, 5.0, 7.0, 9.0, 10.0]
    } else {
        vec![0.0, 2.0, 4.0, 5.0, 7.0, 9.0, 11.0] // major
    };

    let brightness = if lower.contains("dark") || lower.contains("deep") {
        0.2
    } else if lower.contains("bright") || lower.contains("airy") {
        0.9
    } else {
        0.5
    };

    let has_drums =
        !(lower.contains("ambient") || lower.contains("classical") || lower.contains("piano solo"));

    let reverb_mix =
        if lower.contains("reverb") || lower.contains("ambient") || lower.contains("space") {
            0.4
        } else {
            0.15
        };

    SynthParams {
        bpm,
        root_hz,
        scale,
        brightness,
        has_drums,
        reverb_mix,
    }
}

fn extract_bpm(text: &str) -> Option<f32> {
    let re = text.find("bpm")?;
    let before = text[..re].trim_end();
    let start = before
        .rfind(|c: char| !c.is_ascii_digit() && c != '.')
        .map(|i| i + 1)
        .unwrap_or(0);
    before[start..].parse().ok()
}

fn semitone_to_ratio(semitones: f32) -> f32 {
    2.0_f32.powf(semitones / 12.0)
}

fn adsr(t: f32, duration: f32, attack: f32, decay: f32, sustain: f32, release: f32) -> f32 {
    let release_start = duration - release;
    if t < attack {
        t / attack
    } else if t < attack + decay {
        1.0 - (1.0 - sustain) * ((t - attack) / decay)
    } else if t < release_start {
        sustain
    } else if t < duration {
        sustain * (1.0 - (t - release_start) / release)
    } else {
        0.0
    }
}

fn sine(phase: f32) -> f32 {
    (2.0 * PI * phase).sin()
}
fn saw(phase: f32) -> f32 {
    2.0 * (phase - phase.floor()) - 1.0
}
fn square(phase: f32) -> f32 {
    if (phase % 1.0) < 0.5 {
        1.0
    } else {
        -1.0
    }
}

/// Blend waveforms based on brightness: sine → saw → square.
fn osc(phase: f32, brightness: f32) -> f32 {
    if brightness < 0.4 {
        sine(phase)
    } else if brightness < 0.7 {
        sine(phase) * (1.0 - brightness) + saw(phase) * brightness
    } else {
        saw(phase) * (1.0 - brightness) + square(phase) * brightness
    }
}

fn generate_wav(prompt: &str, duration: f32) -> Vec<u8> {
    let sr = 44100u32;
    let n = (duration * sr as f32) as usize;
    let mut mix = vec![0.0f32; n];
    let p = parse_prompt(prompt);

    let beat_dur = 60.0 / p.bpm;
    let scale = &p.scale;

    // ── Bass line ─────────────────────────────────────────────────────────────
    {
        let bass_pattern = [0usize, 0, 4, 4, 2, 2, 3, 3];
        let mut phase = 0.0f32;
        for i in 0..n {
            let t = i as f32 / sr as f32;
            let beat_idx = ((t / beat_dur) as usize) % bass_pattern.len();
            let scale_idx = bass_pattern[beat_idx] % scale.len();
            let semitone = scale[scale_idx];
            let freq = p.root_hz * 0.5 * semitone_to_ratio(semitone);
            let beat_t = t % beat_dur;
            let env = adsr(beat_t, beat_dur * 0.9, 0.01, 0.1, 0.6, 0.15);
            // Bass: sine + mild sub
            let s = (sine(phase) * 0.7 + sine(phase * 2.0) * 0.2) * env * 0.35;
            mix[i] += s;
            phase += freq / sr as f32;
            if phase >= 1.0 {
                phase -= 1.0;
            }
        }
    }

    // ── Chord pads ────────────────────────────────────────────────────────────
    {
        let chord_roots = [0usize, 3, 4, 2]; // degrees: I IV V III
        let chord_dur = beat_dur * 4.0;
        let mut phases = [0.0f32; 4];
        for i in 0..n {
            let t = i as f32 / sr as f32;
            let chord_idx = ((t / chord_dur) as usize) % chord_roots.len();
            let root_deg = chord_roots[chord_idx];
            let chord_t = t % chord_dur;
            let env = adsr(chord_t, chord_dur * 0.95, 0.08, 0.2, 0.7, 0.3);
            // Triad: root + 3rd + 5th
            let degrees = [
                root_deg,
                (root_deg + 2) % scale.len(),
                (root_deg + 4) % scale.len(),
                (root_deg + 6) % scale.len(),
            ];
            let mut s = 0.0f32;
            for (k, &deg) in degrees.iter().enumerate() {
                let freq =
                    p.root_hz * semitone_to_ratio(scale[deg]) * if k < 2 { 1.0 } else { 2.0 };
                s += osc(phases[k], p.brightness);
                phases[k] += freq / sr as f32;
                if phases[k] >= 1.0 {
                    phases[k] -= 1.0;
                }
            }
            mix[i] += s * env * 0.12;
        }
    }

    // ── Melody ────────────────────────────────────────────────────────────────
    {
        let mel_pattern = [0usize, 2, 4, 7, 4, 2, 5, 3, 1, 4, 6, 4, 2, 1, 3, 0];
        let mel_note_dur = beat_dur * 0.5;
        let mut phase = 0.0f32;
        for i in 0..n {
            let t = i as f32 / sr as f32;
            let note_idx = ((t / mel_note_dur) as usize) % mel_pattern.len();
            let deg = mel_pattern[note_idx] % scale.len();
            let freq = p.root_hz * 2.0 * semitone_to_ratio(scale[deg]);
            let note_t = t % mel_note_dur;
            let env = adsr(note_t, mel_note_dur * 0.85, 0.005, 0.05, 0.6, 0.1);
            mix[i] += osc(phase, p.brightness) * env * 0.18;
            phase += freq / sr as f32;
            if phase >= 1.0 {
                phase -= 1.0;
            }
        }
    }

    // ── Drums ─────────────────────────────────────────────────────────────────
    if p.has_drums {
        let mut kick_noise = 0.0f32;
        let mut snare_noise = fastrand::f32();
        for i in 0..n {
            let t = i as f32 / sr as f32;
            let beat_pos = (t / beat_dur) % 4.0;
            let beat_t = t % beat_dur;

            // Kick on beats 1 and 3
            if beat_t < 0.003 && (beat_pos < 0.05 || (beat_pos > 1.95 && beat_pos < 2.05)) {
                kick_noise = 1.0;
            }
            if kick_noise > 0.0 {
                let kick_freq = 60.0 * (-beat_t * 20.0).exp() + 40.0;
                mix[i] += sine(kick_noise) * (-beat_t * 8.0).exp() * 0.5;
                kick_noise += kick_freq / sr as f32;
            }

            // Snare on beats 2 and 4
            if beat_t < 0.003 && (beat_pos > 0.95 && beat_pos < 1.05 || beat_pos > 2.95) {
                snare_noise = 1.0;
            }
            if snare_noise > 0.001 {
                let noise = fastrand::f32() * 2.0 - 1.0;
                mix[i] +=
                    (noise * 0.3 + sine(snare_noise * 200.0) * 0.1) * (-beat_t * 18.0).exp() * 0.3;
                snare_noise *= 0.9998;
            }

            // Hi-hat (16th notes, every beat_dur/4)
            let hihat_t = t % (beat_dur / 4.0);
            if hihat_t < beat_dur / 32.0 {
                let noise = fastrand::f32() * 2.0 - 1.0;
                mix[i] += noise * (-hihat_t * 80.0 / beat_dur).exp() * 0.08;
            }
        }
    }

    // ── Simple reverb (Schroeder) ─────────────────────────────────────────────
    if p.reverb_mix > 0.01 {
        let delays = [1471usize, 1699, 2039, 2293];
        let mut buffers: Vec<Vec<f32>> = delays.iter().map(|&d| vec![0.0; d]).collect();
        let mut heads = [0usize; 4];
        let mut wet = vec![0.0f32; n];
        for i in 0..n {
            let mut acc = 0.0f32;
            for k in 0..4 {
                let d = delays[k];
                acc += buffers[k][heads[k]];
                buffers[k][heads[k]] = mix[i] + buffers[k][heads[k]] * 0.6;
                heads[k] = (heads[k] + 1) % d;
            }
            wet[i] = acc * 0.25;
        }
        for i in 0..n {
            mix[i] = mix[i] * (1.0 - p.reverb_mix) + wet[i] * p.reverb_mix;
        }
    }

    // ── Limiter ───────────────────────────────────────────────────────────────
    for s in &mut mix {
        *s = s.clamp(-0.95, 0.95);
    }

    // ── Fade in/out ───────────────────────────────────────────────────────────
    let fade_len = (sr as f32 * 0.05) as usize;
    for i in 0..fade_len.min(n) {
        let t = i as f32 / fade_len as f32;
        mix[i] *= t;
        if n > fade_len {
            mix[n - 1 - i] *= t;
        }
    }

    encode_wav_f32(&mix, sr)
}

fn encode_wav_f32(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let data_len = (samples.len() * 4) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);

    // RIFF header
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    // fmt chunk (IEEE float, 1 channel)
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    out.extend_from_slice(&3u16.to_le_bytes()); // PCM float
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * 4).to_le_bytes()); // byte rate
    out.extend_from_slice(&4u16.to_le_bytes()); // block align
    out.extend_from_slice(&32u16.to_le_bytes()); // bits per sample
                                                 // data chunk
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for &s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}
