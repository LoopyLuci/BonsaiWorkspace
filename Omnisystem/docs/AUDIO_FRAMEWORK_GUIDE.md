# Audio Framework Guide - Real-Time Audio Processing

**Enterprise-grade audio engine for games, music production, and sound design**

---

## Overview

The Audio Framework provides:
- **Real-Time Audio I/O** - JACK, ALSA, CoreAudio, WASAPI
- **DSP Processing** - Filters, effects, synthesis
- **MIDI Support** - Keyboard input, controller mapping
- **Audio Editing** - Waveform manipulation, mixing
- **Plugin System** - Audio plugin standards (VST, AU)
- **Latency-Optimized** - Sub-1ms latency targeting

---

## Core Architecture

```
Application Layer
    ↓
Audio Engine (real-time)
    ↓
DSP Graph (effects chain)
    ↓
Mixing & Routing
    ↓
Audio Device I/O
```

---

## Quick Start

```titan
use omnisystem::audio::*

fun main() -> Result<(), str> {
    // Initialize audio engine
    let audio = AudioEngine::new()?
    audio.set_sample_rate(48000)?
    audio.set_block_size(256)?
    
    // Create oscillator
    let osc = Oscillator::new()
        .with_waveform(Waveform::Sine)
        .with_frequency(440.0)
    
    // Create mixer
    let mut mixer = Mixer::new(channels: 2)
    mixer.add_channel("osc", osc)
    
    // Main audio loop
    audio.start_callback(|buffer: &mut AudioBuffer| {
        mixer.process(buffer)
    })?
    
    std::thread::sleep(Duration::from_secs(5))
    audio.stop()?
    
    Ok(())
}
```

---

## Audio I/O

### Device Management

```titan
fun list_audio_devices() -> Result<(), str> {
    let engine = AudioEngine::new()?
    
    println!("Input Devices:");
    for device in engine.input_devices()? {
        println!("  {} (channels: {})", device.name, device.channels)
    }
    
    println!("\nOutput Devices:");
    for device in engine.output_devices()? {
        println!("  {} (channels: {})", device.name, device.channels)
    }
    
    Ok(())
}
```

### Audio Streaming

```titan
fun stream_audio_file(engine: &AudioEngine, path: &str) -> Result<(), str> {
    let mut stream = AudioFile::open(path)?
    
    println!("Sample rate: {}", stream.sample_rate())
    println!("Channels: {}", stream.channels())
    println!("Duration: {:.2}s", stream.duration_seconds())
    
    // Stream to device
    engine.stream_audio(&mut stream)?
    
    Ok(())
}
```

---

## Synthesis

### Oscillators

```titan
fun create_oscillators() -> Result<Vec<Oscillator>, str> {
    let sine = Oscillator::new()
        .with_waveform(Waveform::Sine)
        .with_frequency(440.0)
        .with_amplitude(0.5)
    
    let square = Oscillator::new()
        .with_waveform(Waveform::Square)
        .with_frequency(330.0)
        .with_amplitude(0.3)
        .with_pulse_width(0.5)
    
    let triangle = Oscillator::new()
        .with_waveform(Waveform::Triangle)
        .with_frequency(220.0)
        .with_amplitude(0.4)
    
    let sawtooth = Oscillator::new()
        .with_waveform(Waveform::Sawtooth)
        .with_frequency(110.0)
        .with_amplitude(0.3)
    
    Ok(vec![sine, square, triangle, sawtooth])
}
```

### Envelopes (ADSR)

```titan
fun create_envelope() -> Result<Envelope, str> {
    let envelope = Envelope::adsr()
        .with_attack(0.01)      // 10ms
        .with_decay(0.2)        // 200ms
        .with_sustain(0.7)      // 70% level
        .with_release(0.5)      // 500ms
    
    Ok(envelope)
}

fun play_note(engine: &AudioEngine, pitch: f32) -> Result<(), str> {
    let osc = Oscillator::new()
        .with_waveform(Waveform::Sawtooth)
        .with_frequency(pitch)
    
    let envelope = create_envelope()?
    
    let mut synth = envelope.apply(osc)
    
    engine.play(&mut synth)?
    
    Ok(())
}
```

### Filters

```titan
fun apply_filters(signal: &mut Vec<f32>) -> Result<(), str> {
    // Low-pass filter
    let lpf = Filter::lowpass(cutoff: 5000.0, resonance: 0.7)?
    let filtered = lpf.process(signal)?
    
    // High-pass filter
    let hpf = Filter::highpass(cutoff: 100.0, resonance: 0.5)?
    let filtered = hpf.process(&filtered)?
    
    // Band-pass filter
    let bpf = Filter::bandpass(center: 1000.0, width: 500.0)?
    let filtered = bpf.process(&filtered)?
    
    Ok(())
}
```

---

## MIDI Support

### MIDI Input

```titan
fun handle_midi() -> Result<(), str> {
    let midi = MidiInput::new()?
    
    midi.on_note_on(|note: u8, velocity: u8| {
        println!("Note ON: {} (velocity: {})", note, velocity)
        // Play note
    })
    
    midi.on_note_off(|note: u8| {
        println!("Note OFF: {}", note)
        // Stop note
    })
    
    midi.on_control_change(|controller: u8, value: u8| {
        println!("CC {}: {}", controller, value)
        // Update parameter
    })
    
    midi.on_pitch_bend(|bend: i16| {
        println!("Pitch bend: {}", bend)
        // Apply pitch bend
    })
    
    midi.start_listening()?
    
    Ok(())
}
```

### MIDI Output

```titan
fun send_midi() -> Result<(), str> {
    let midi = MidiOutput::new()?
    
    // Send note on
    midi.send_note_on(channel: 0, note: 60, velocity: 100)?
    
    std::thread::sleep(Duration::from_millis(500))
    
    // Send note off
    midi.send_note_off(channel: 0, note: 60)?
    
    // Send control change
    midi.send_control_change(channel: 0, controller: 7, value: 100)?
    
    Ok(())
}
```

---

## Effects & DSP

### Built-in Effects

```titan
fun apply_effects(audio: &mut AudioBuffer) -> Result<(), str> {
    // Reverb
    let reverb = Reverb::new()
        .with_room_size(0.5)
        .with_damping(0.5)
        .with_wet_dry(0.3)
    
    // Delay
    let delay = Delay::new()
        .with_time(0.5)  // 500ms
        .with_feedback(0.6)
        .with_wet_dry(0.5)
    
    // Chorus
    let chorus = Chorus::new()
        .with_rate(1.5)
        .with_depth(0.002)
        .with_wet_dry(0.5)
    
    // Distortion
    let distortion = Distortion::new()
        .with_drive(5.0)
        .with_tone(0.5)
    
    // Compression
    let compressor = Compressor::new()
        .with_threshold(-20.0)
        .with_ratio(4.0)
        .with_attack(0.005)
        .with_release(0.1)
    
    // Equalization
    let eq = ParametricEQ::new()
        .add_band(frequency: 100.0, gain: -3.0, q: 1.0)?
        .add_band(frequency: 1000.0, gain: 6.0, q: 1.0)?
        .add_band(frequency: 10000.0, gain: 3.0, q: 1.0)?
    
    Ok(())
}
```

### Custom DSP Processing

```titan
fun custom_dsp(input: &[f32]) -> Result<Vec<f32>, str> {
    let mut output = vec![]
    
    for sample in input {
        // Simple gain processing
        let processed = sample * 2.0
        
        // Soft clipping
        let clipped = if processed > 1.0 {
            1.0
        } else if processed < -1.0 {
            -1.0
        } else {
            processed
        }
        
        output.push(clipped)
    }
    
    Ok(output)
}
```

---

## Mixing & Routing

### Mixing Desk

```titan
fun setup_mixer() -> Result<Mixer, str> {
    let mut mixer = Mixer::new(channels: 2)
    
    // Audio tracks
    mixer.add_channel("drums", drums_channel)?
    mixer.add_channel("bass", bass_channel)?
    mixer.add_channel("synth", synth_channel)?
    mixer.add_channel("vocals", vocals_channel)?
    
    // Set levels
    mixer.set_level("drums", 0.8)?
    mixer.set_level("bass", 0.7)?
    mixer.set_level("synth", 0.6)?
    mixer.set_level("vocals", 0.9)?
    
    // Panning
    mixer.set_pan("drums", 0.0)?      // Center
    mixer.set_pan("bass", 0.0)?       // Center
    mixer.set_pan("synth", -0.3)?     // Left
    mixer.set_pan("vocals", 0.3)?     // Right
    
    // Mute/Solo
    mixer.mute("synth", true)?
    mixer.solo("vocals", true)?
    
    Ok(mixer)
}
```

### Bus Routing

```titan
fun setup_routing(mixer: &mut Mixer) -> Result<(), str> {
    // Create buses
    mixer.create_bus("drums_bus")?
    mixer.create_bus("music_bus")?
    mixer.create_bus("master")?
    
    // Route tracks to buses
    mixer.route_to_bus("drums", "drums_bus")?
    mixer.route_to_bus("bass", "music_bus")?
    mixer.route_to_bus("synth", "music_bus")?
    mixer.route_to_bus("vocals", "music_bus")?
    
    // Route buses to master
    mixer.route_to_bus("drums_bus", "master")?
    mixer.route_to_bus("music_bus", "master")?
    
    // Add effects to bus
    mixer.add_effect_to_bus("master", Compressor::new())?
    mixer.add_effect_to_bus("music_bus", Reverb::new())?
    
    Ok(())
}
```

---

## Audio File Operations

### Recording

```titan
fun record_audio(engine: &AudioEngine, duration: f32) -> Result<(), str> {
    let mut recorder = AudioRecorder::new()?
    recorder.start()?
    
    std::thread::sleep(Duration::from_secs_f32(duration))
    
    let audio_data = recorder.stop()?
    recorder.save_as("recording.wav")?
    
    Ok(())
}
```

### Editing

```titan
fun edit_audio(path: &str) -> Result<(), str> {
    let mut audio = AudioFile::open(path)?
    
    // Trim
    audio.trim(start_seconds: 1.0, end_seconds: 10.0)?
    
    // Fade in/out
    audio.fade_in(duration: 0.5)?
    audio.fade_out(duration: 0.5)?
    
    // Normalize
    audio.normalize(target_level: -3.0)?
    
    // Resample
    audio.resample(target_sample_rate: 44100)?
    
    audio.save_as("output.wav")?
    
    Ok(())
}
```

---

## Performance Optimization

### Buffer Management

```titan
fun optimize_buffers(engine: &AudioEngine) -> Result<(), str> {
    // Ring buffer for low-latency
    let ring_buffer = RingBuffer::new(capacity: 4096)
    
    // Lock-free queues for thread-safe data passing
    let (producer, consumer) = lockfree_queue::create(capacity: 8192)
    
    // Preallocate audio buffers
    let mut buffer_pool = vec![]
    for _ in 0..10 {
        buffer_pool.push(AudioBuffer::new(size: 256))
    }
    
    Ok(())
}
```

### Real-Time Safety

```titan
fun real_time_processing(buffer: &mut AudioBuffer) {
    // Rule 1: No allocations in audio thread
    // Rule 2: No locks with contention
    // Rule 3: Bounded computation time
    
    for sample in buffer.samples_mut() {
        // Only do DSP work here
        *sample = (*sample * 0.5).tanh()
    }
}
```

---

## Advanced Features

### Spatial Audio (3D)

```titan
fun setup_spatial_audio() -> Result<(), str> {
    let spatial = SpatialAudio::new()?
    
    // Position source in 3D space
    let source = AudioSource::new()
        .with_position(vec3(1.0, 0.0, 0.0))
        .with_velocity(vec3(0.1, 0.0, 0.0))
    
    // Position listener
    spatial.set_listener_position(vec3(0.0, 0.0, 0.0))
    spatial.set_listener_forward(vec3(0.0, 0.0, -1.0))
    spatial.set_listener_up(vec3(0.0, 1.0, 0.0))
    
    // HRTF processing
    spatial.use_hrtf(true)
    
    Ok(())
}
```

### Convolution & Impulse Responses

```titan
fun convolution_reverb(input: &[f32], ir: &[f32]) -> Result<Vec<f32>, str> {
    let conv = Convolver::new(ir)?
    let output = conv.process(input)?
    Ok(output)
}
```

---

## Best Practices

✅ **DO**
- Use ring buffers for real-time
- Preallocate memory
- Use lock-free structures
- Monitor DSP load
- Test with various hardware

❌ **DON'T**
- Allocate in audio callback
- Use sleeping/blocking calls
- Print to console in real-time thread
- Use high-overhead abstractions
- Ignore latency

---

## Next Steps

- [PHYSICS_FRAMEWORK_GUIDE.md](PHYSICS_FRAMEWORK_GUIDE.md) - Physics engine
- [GAME_FRAMEWORK_GUIDE.md](GAME_FRAMEWORK_GUIDE.md) - Game development
- [MUSIC_PRODUCTION_PLATFORM.md](MUSIC_PRODUCTION_PLATFORM.md) - DAW application

---

**Audio Framework** - Professional-grade real-time audio processing!
