use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Waveform {
    Sine,
    Triangle,
    Saw,
    Square,
    Noise,
}

#[derive(Debug, Clone)]
pub struct Oscillator {
    pub waveform: Waveform,
    pub frequency: f32,
    pub phase: f32,
    pub phase_offset: f32,
    pub detune_cents: f32, // -100.0 .. +100.0
    pub volume: f32,       // 0.0 .. 1.0
    pub pan: f32,          // -1.0 .. 1.0
}

impl Oscillator {
    pub fn new(waveform: Waveform) -> Self {
        Self {
            waveform,
            frequency: 440.0,
            phase: 0.0,
            phase_offset: 0.0,
            detune_cents: 0.0,
            volume: 1.0,
            pan: 0.0,
        }
    }

    pub fn set_note(&mut self, midi_pitch: u8, coarse: i32, fine_cents: f32) {
        let total_semitones = (midi_pitch as f32 - 69.0) + (coarse as f32) + ((fine_cents + self.detune_cents) / 100.0);
        self.frequency = 440.0 * 2.0_f32.powf(total_semitones / 12.0);
    }

    #[inline]
    pub fn next_sample(&mut self, sample_rate: f32) -> f32 {
        let phase_with_offset = (self.phase + self.phase_offset).fract();
        let sample = match self.waveform {
            Waveform::Sine => (phase_with_offset * 2.0 * PI).sin(),
            Waveform::Triangle => {
                if phase_with_offset < 0.5 {
                    4.0 * phase_with_offset - 1.0
                } else {
                    3.0 - 4.0 * phase_with_offset
                }
            }
            Waveform::Saw => 2.0 * phase_with_offset - 1.0,
            Waveform::Square => {
                if phase_with_offset < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            Waveform::Noise => {
                // Gerador pseudo-aleatório rápido sem alocação
                (self.phase * 12345.678).fract() * 2.0 - 1.0
            }
        };

        // Incrementa a fase
        let phase_inc = self.frequency / sample_rate;
        self.phase = (self.phase + phase_inc).fract();

        sample * self.volume
    }
}

/// Envelope ADSR clássico
#[derive(Debug, Clone)]
pub struct AdsrEnvelope {
    pub attack_sec: f32,
    pub decay_sec: f32,
    pub sustain_level: f32, // 0.0 .. 1.0
    pub release_sec: f32,
    state: EnvelopeStage,
    current_level: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EnvelopeStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

impl AdsrEnvelope {
    pub fn new(attack: f32, decay: f32, sustain: f32, release: f32) -> Self {
        Self {
            attack_sec: attack.max(0.001),
            decay_sec: decay.max(0.001),
            sustain_level: sustain.clamp(0.0, 1.0),
            release_sec: release.max(0.001),
            state: EnvelopeStage::Idle,
            current_level: 0.0,
        }
    }

    pub fn trigger_attack(&mut self) {
        self.state = EnvelopeStage::Attack;
    }

    pub fn trigger_release(&mut self) {
        if self.state != EnvelopeStage::Idle {
            self.state = EnvelopeStage::Release;
        }
    }

    #[inline]
    pub fn is_active(&self) -> bool {
        self.state != EnvelopeStage::Idle
    }

    #[inline]
    pub fn next_sample(&mut self, sample_rate: f32) -> f32 {
        let dt = 1.0 / sample_rate;
        match self.state {
            EnvelopeStage::Idle => {
                self.current_level = 0.0;
            }
            EnvelopeStage::Attack => {
                let inc = dt / self.attack_sec;
                self.current_level += inc;
                if self.current_level >= 1.0 {
                    self.current_level = 1.0;
                    self.state = EnvelopeStage::Decay;
                }
            }
            EnvelopeStage::Decay => {
                let dec = dt / self.decay_sec;
                self.current_level -= dec;
                if self.current_level <= self.sustain_level {
                    self.current_level = self.sustain_level;
                    self.state = EnvelopeStage::Sustain;
                }
            }
            EnvelopeStage::Sustain => {
                self.current_level = self.sustain_level;
            }
            EnvelopeStage::Release => {
                let rel = dt / self.release_sec;
                self.current_level -= rel;
                if self.current_level <= 0.0001 {
                    self.current_level = 0.0;
                    self.state = EnvelopeStage::Idle;
                }
            }
        }
        self.current_level
    }
}

/// Filtro Biquad clássico (Audio EQ Cookbook de Robert Bristow-Johnson)
#[derive(Debug, Clone)]
pub struct BiquadFilter {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Default for BiquadFilter {
    fn default() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }
}

impl BiquadFilter {
    pub fn set_lowpass(&mut self, sample_rate: f32, cutoff_hz: f32, q: f32) {
        let omega = 2.0 * PI * (cutoff_hz / sample_rate).clamp(0.0001, 0.499);
        let alpha = omega.sin() / (2.0 * q.max(0.1));
        let cos_w = omega.cos();

        let a0 = 1.0 + alpha;
        self.b0 = ((1.0 - cos_w) / 2.0) / a0;
        self.b1 = (1.0 - cos_w) / a0;
        self.b2 = ((1.0 - cos_w) / 2.0) / a0;
        self.a1 = (-2.0 * cos_w) / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    pub fn set_highpass(&mut self, sample_rate: f32, cutoff_hz: f32, q: f32) {
        let omega = 2.0 * PI * (cutoff_hz / sample_rate).clamp(0.0001, 0.499);
        let alpha = omega.sin() / (2.0 * q.max(0.1));
        let cos_w = omega.cos();

        let a0 = 1.0 + alpha;
        self.b0 = ((1.0 + cos_w) / 2.0) / a0;
        self.b1 = (-(1.0 + cos_w)) / a0;
        self.b2 = ((1.0 + cos_w) / 2.0) / a0;
        self.a1 = (-2.0 * cos_w) / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    pub fn set_bandpass(&mut self, sample_rate: f32, center_hz: f32, q: f32) {
        let omega = 2.0 * PI * (center_hz / sample_rate).clamp(0.0001, 0.499);
        let alpha = omega.sin() / (2.0 * q.max(0.1));
        let cos_w = omega.cos();

        let a0 = 1.0 + alpha;
        self.b0 = alpha / a0;
        self.b1 = 0.0;
        self.b2 = -alpha / a0;
        self.a1 = (-2.0 * cos_w) / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    pub fn set_peaking(&mut self, sample_rate: f32, center_hz: f32, q: f32, gain_db: f32) {
        let a = 10.0_f32.powf(gain_db / 40.0);
        let omega = 2.0 * PI * (center_hz / sample_rate).clamp(0.0001, 0.499);
        let alpha = omega.sin() / (2.0 * q.max(0.1));
        let cos_w = omega.cos();

        let a0 = 1.0 + alpha / a;
        self.b0 = (1.0 + alpha * a) / a0;
        self.b1 = (-2.0 * cos_w) / a0;
        self.b2 = (1.0 - alpha * a) / a0;
        self.a1 = (-2.0 * cos_w) / a0;
        self.a2 = (1.0 - alpha / a) / a0;
    }

    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        let out = self.b0 * input + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = out;
        out
    }

    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

/// 3x Osc: Sintetizador de 3 Osciladores clássico do FL Studio
#[derive(Debug, Clone)]
pub struct Fruity3xOsc {
    pub osc1: Oscillator,
    pub osc2: Oscillator,
    pub osc3: Oscillator,
    pub filter: BiquadFilter,
    pub envelope: AdsrEnvelope,
}

impl Default for Fruity3xOsc {
    fn default() -> Self {
        let osc1 = Oscillator::new(Waveform::Saw);
        let mut osc2 = Oscillator::new(Waveform::Saw);
        let mut osc3 = Oscillator::new(Waveform::Sine);

        osc2.detune_cents = 7.0; // detune clássico para sonoridade encorpada
        osc3.volume = 0.5;

        let mut filter = BiquadFilter::default();
        filter.set_lowpass(44100.0, 8000.0, 1.0);

        Self {
            osc1,
            osc2,
            osc3,
            filter,
            envelope: AdsrEnvelope::new(0.01, 0.2, 0.7, 0.3),
        }
    }
}

impl Fruity3xOsc {
    pub fn trigger_note(&mut self, pitch: u8) {
        self.osc1.set_note(pitch, 0, 0.0);
        self.osc2.set_note(pitch, 0, 0.0);
        self.osc3.set_note(pitch, -12, 0.0); // 1 oitava abaixo (sub-oscillator)
        self.envelope.trigger_attack();
    }

    pub fn release_note(&mut self) {
        self.envelope.trigger_release();
    }

    #[inline]
    pub fn next_sample(&mut self, sample_rate: f32) -> (f32, f32) {
        let env = self.envelope.next_sample(sample_rate);
        if env <= 0.0001 {
            return (0.0, 0.0);
        }

        let s1 = self.osc1.next_sample(sample_rate);
        let s2 = self.osc2.next_sample(sample_rate);
        let s3 = self.osc3.next_sample(sample_rate);

        let sum = (s1 + s2 + s3) * 0.33 * env;
        let filtered = self.filter.process(sum);

        // Estéreo leve
        (filtered, filtered)
    }

    pub fn is_active(&self) -> bool {
        self.envelope.is_active()
    }
}

/// Soundgoodizer: O lendário maximizer/saturador de 1 botão do FL Studio!
#[derive(Debug, Clone)]
pub struct Soundgoodizer {
    pub enabled: bool,
    pub mode: u8, // 0 = A, 1 = B, 2 = C, 3 = D
    pub amount: f32, // 0.0 .. 1.0
}

impl Default for Soundgoodizer {
    fn default() -> Self {
        Self {
            enabled: false, // Bypassado por padrão para som limpo de estúdio
            mode: 0,
            amount: 0.5,
        }
    }
}

impl Soundgoodizer {
    #[inline]
    pub fn process_stereo(&self, left: f32, right: f32) -> (f32, f32) {
        if !self.enabled || self.amount <= 0.001 {
            return (left, right);
        }

        let (drive, wet_boost) = match self.mode {
            0 => (1.5 + self.amount * 2.5, 1.2), // Modo A: Clássico pumping de graves e agudos
            1 => (2.0 + self.amount * 3.0, 1.3), // Modo B: Warm punchy mids
            2 => (1.2 + self.amount * 2.0, 1.1), // Modo C: Limpo e transparente
            _ => (2.5 + self.amount * 4.0, 1.4), // Modo D: Saturação agressiva
        };

        // Função de soft-clipping / saturação analógica com curva tangente hiperbólica
        let sat_l = (left * drive).tanh() * wet_boost;
        let sat_r = (right * drive).tanh() * wet_boost;

        let dry = 1.0 - (self.amount * 0.7);
        let wet = self.amount * 0.7;

        (left * dry + sat_l * wet, right * dry + sat_r * wet)
    }
}

/// Parametric EQ 2: Clone das 7 bandas do clássico do FL Studio
#[derive(Debug, Clone)]
pub struct ParametricEQ2 {
    pub bands: [BiquadFilter; 7],
    pub gains_db: [f32; 7],
    pub frequencies: [f32; 7],
    pub q_factors: [f32; 7],
}

impl Default for ParametricEQ2 {
    fn default() -> Self {
        // Frequências clássicas padrão do Fruity Parametric EQ 2:
        // 1: 30 Hz (Sub), 2: 100 Hz (Bass), 3: 450 Hz (Low Mid), 4: 1500 Hz (Mid),
        // 5: 3500 Hz (High Mid), 6: 8000 Hz (Treble), 7: 16000 Hz (Air)
        let freqs = [30.0, 100.0, 450.0, 1500.0, 3500.0, 8000.0, 16000.0];
        let qs = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let gains = [0.0; 7];

        let mut eq = Self {
            bands: Default::default(),
            gains_db: gains,
            frequencies: freqs,
            q_factors: qs,
        };
        eq.recalculate_filters(44100.0);
        eq
    }
}

impl ParametricEQ2 {
    pub fn recalculate_filters(&mut self, sample_rate: f32) {
        for i in 0..7 {
            self.bands[i].set_peaking(sample_rate, self.frequencies[i], self.q_factors[i], self.gains_db[i]);
        }
    }

    #[inline]
    pub fn process_sample(&mut self, mut sample: f32) -> f32 {
        for band in &mut self.bands {
            sample = band.process(sample);
        }
        sample
    }
}

/// Fruity Delay 3 equivalente: Delay estéreo sincronizado ao BPM
#[derive(Debug, Clone)]
pub struct FruityDelay {
    pub enabled: bool,
    buffer_l: Vec<f32>,
    buffer_r: Vec<f32>,
    write_pos: usize,
    pub delay_time_sec: f32,
    pub feedback: f32, // 0.0 .. 0.95
    pub wet: f32,      // 0.0 .. 1.0 (0.0 = 100% seco/dry por padrão!)
}

impl Default for FruityDelay {
    fn default() -> Self {
        let max_samples = 44100 * 2; // até 2 segundos de delay
        Self {
            enabled: false, // Bypassado por padrão para som 100% seco e punchy
            buffer_l: vec![0.0; max_samples],
            buffer_r: vec![0.0; max_samples],
            write_pos: 0,
            delay_time_sec: 0.35, // ~1/4 batida em 130 BPM
            feedback: 0.45,
            wet: 0.0, // Zero reverb/delay por padrão
        }
    }
}

impl FruityDelay {
    #[inline]
    pub fn process_stereo(&mut self, sample_rate: f32, in_l: f32, in_r: f32) -> (f32, f32) {
        if !self.enabled || self.wet <= 0.001 {
            return (in_l, in_r);
        }

        let delay_samples = ((self.delay_time_sec * sample_rate) as usize).clamp(1, self.buffer_l.len() - 1);
        let read_pos = (self.write_pos + self.buffer_l.len() - delay_samples) % self.buffer_l.len();

        let delayed_l = self.buffer_l[read_pos];
        let delayed_r = self.buffer_r[read_pos];

        self.buffer_l[self.write_pos] = in_l + delayed_l * self.feedback;
        self.buffer_r[self.write_pos] = in_r + delayed_r * self.feedback;

        self.write_pos = (self.write_pos + 1) % self.buffer_l.len();

        (in_l + delayed_l * self.wet, in_r + delayed_r * self.wet)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrumType {
    Kick,
    Clap,
    ClosedHat,
    Snare,
}

#[derive(Debug, Clone)]
pub struct DrumVoice {
    pub drum_type: DrumType,
    pub active: bool,
    pub velocity: f32,
    time_sec: f32,
    phase: f32,
    noise_seed: f32,
    filter: BiquadFilter,
}

impl DrumVoice {
    pub fn new(drum_type: DrumType) -> Self {
        Self {
            drum_type,
            active: false,
            velocity: 0.8,
            time_sec: 0.0,
            phase: 0.0,
            noise_seed: 0.12345,
            filter: BiquadFilter::default(),
        }
    }

    pub fn trigger(&mut self, velocity: f32) {
        self.active = true;
        self.velocity = velocity.clamp(0.01, 1.0);
        self.time_sec = 0.0;
        self.phase = 0.0;
        self.filter.reset();
    }

    #[inline]
    fn pseudo_noise(&mut self) -> f32 {
        self.noise_seed = (self.noise_seed * 1103515245.0 + 12345.0).fract();
        self.noise_seed * 2.0 - 1.0
    }

    #[inline]
    pub fn next_sample(&mut self, sample_rate: f32) -> (f32, f32) {
        if !self.active {
            return (0.0, 0.0);
        }

        let dt = 1.0 / sample_rate;
        let t = self.time_sec;
        self.time_sec += dt;

        let sample = match self.drum_type {
            DrumType::Kick => {
                if t > 0.45 {
                    self.active = false;
                    return (0.0, 0.0);
                }
                let pitch_env = 48.0 + 130.0 * (-t * 32.0).exp();
                let click = if t < 0.006 { 1.2 * (1.0 - t / 0.006) } else { 0.0 };
                self.phase += pitch_env * dt;
                let sine = (self.phase * 2.0 * PI).sin();
                let amp = (-t * 7.5).exp();
                let raw = (sine + click) * amp * self.velocity * 1.5;
                raw.tanh()
            }
            DrumType::Clap => {
                if t > 0.30 {
                    self.active = false;
                    return (0.0, 0.0);
                }
                let noise = self.pseudo_noise();
                let pulse = if t < 0.012 {
                    1.0
                } else if t < 0.024 {
                    0.8
                } else {
                    (-(t - 0.024) * 16.0).exp()
                };
                self.filter.set_bandpass(sample_rate, 1250.0, 1.5);
                let filtered = self.filter.process(noise);
                (filtered * pulse * self.velocity * 1.6).tanh()
            }
            DrumType::ClosedHat => {
                if t > 0.09 {
                    self.active = false;
                    return (0.0, 0.0);
                }
                let noise = self.pseudo_noise();
                self.filter.set_highpass(sample_rate, 7800.0, 2.0);
                let filtered = self.filter.process(noise);
                let amp = (-t * 55.0).exp();
                (filtered * amp * self.velocity * 1.4).tanh()
            }
            DrumType::Snare => {
                if t > 0.35 {
                    self.active = false;
                    return (0.0, 0.0);
                }
                let pitch = 160.0 + 80.0 * (-t * 28.0).exp();
                self.phase += pitch * dt;
                let tone = (self.phase * 2.0 * PI).sin() * (-t * 16.0).exp();

                let noise = self.pseudo_noise();
                self.filter.set_bandpass(sample_rate, 2600.0, 1.2);
                let snare_wire = self.filter.process(noise) * (-t * 12.0).exp();

                let mix = tone * 0.45 + snare_wire * 0.75;
                (mix * self.velocity * 1.5).tanh()
            }
        };

        (sample, sample)
    }
}

#[derive(Debug, Clone)]
pub struct PolyVoice {
    pub synth: Fruity3xOsc,
    pub pitch: u8,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct PolyphonicSynth {
    pub voices: Vec<PolyVoice>,
}

impl Default for PolyphonicSynth {
    fn default() -> Self {
        let mut voices = Vec::with_capacity(16);
        for _ in 0..16 {
            voices.push(PolyVoice {
                synth: Fruity3xOsc::default(),
                pitch: 0,
                active: false,
            });
        }
        Self { voices }
    }
}

impl PolyphonicSynth {
    pub fn note_on(&mut self, pitch: u8) {
        let mut slot = None;
        for (i, v) in self.voices.iter().enumerate() {
            if !v.active || !v.synth.is_active() {
                slot = Some(i);
                break;
            }
        }
        let idx = slot.unwrap_or(0);
        self.voices[idx].active = true;
        self.voices[idx].pitch = pitch;
        self.voices[idx].synth.trigger_note(pitch);
    }

    pub fn note_off(&mut self, pitch: u8) {
        for v in &mut self.voices {
            if v.active && v.pitch == pitch {
                v.active = false;
                v.synth.release_note();
            }
        }
    }

    #[inline]
    pub fn next_sample(&mut self, sample_rate: f32) -> (f32, f32) {
        let mut sum_l = 0.0;
        let mut sum_r = 0.0;
        let mut active_count = 0;

        for v in &mut self.voices {
            if v.synth.is_active() {
                let (l, r) = v.synth.next_sample(sample_rate);
                sum_l += l;
                sum_r += r;
                active_count += 1;
            } else {
                v.active = false;
            }
        }

        if active_count > 1 {
            let scale = 1.0 / (active_count as f32).sqrt();
            (sum_l * scale, sum_r * scale)
        } else {
            (sum_l, sum_r)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oscillator_sine() {
        let mut osc = Oscillator::new(Waveform::Sine);
        osc.frequency = 440.0;
        let s1 = osc.next_sample(44100.0);
        let s2 = osc.next_sample(44100.0);
        assert!(s1.abs() <= 1.0);
        assert!(s2.abs() <= 1.0);
    }

    #[test]
    fn test_soundgoodizer() {
        let sg = Soundgoodizer::default();
        let (out_l, out_r) = sg.process_stereo(0.5, 0.5);
        assert!(out_l > 0.0);
        assert!(out_r > 0.0);
    }

    #[test]
    fn test_parametric_eq2() {
        let mut eq = ParametricEQ2::default();
        let sample = eq.process_sample(0.5);
        assert!((sample - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_drum_voices() {
        let mut kick = DrumVoice::new(DrumType::Kick);
        kick.trigger(0.9);
        let (k_l, k_r) = kick.next_sample(44100.0);
        assert!(k_l != 0.0 && k_r != 0.0);

        let mut clap = DrumVoice::new(DrumType::Clap);
        clap.trigger(0.8);
        let (c_l, _) = clap.next_sample(44100.0);
        assert!(c_l != 0.0);
    }

    #[test]
    fn test_polyphonic_synth() {
        let mut synth = PolyphonicSynth::default();
        synth.note_on(60);
        synth.note_on(64);
        let (l, r) = synth.next_sample(44100.0);
        assert!(l != 0.0 && r != 0.0);
        synth.note_off(60);
        synth.note_off(64);
    }

    #[test]
    fn test_fruity_delay_dry_by_default() {
        let mut delay = FruityDelay::default();
        assert_eq!(delay.wet, 0.0);
        assert!(!delay.enabled);
        let (out_l, out_r) = delay.process_stereo(44100.0, 0.75, 0.75);
        assert_eq!(out_l, 0.75);
        assert_eq!(out_r, 0.75);
    }
}
