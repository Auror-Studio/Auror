use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{Receiver, Sender};
use openstudio_core::{
    AudioToUiMsg, ChannelParam, MixerParam, Project, TransportState, UiToAudioMsg,
    TICKS_PER_STEP,
};
use openstudio_dsp::{
    DrumType, DrumVoice, FruityDelay, ParametricEQ2, PolyphonicSynth, Soundgoodizer,
};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub struct AudioEngineHandle {
    pub ui_to_audio_tx: Sender<UiToAudioMsg>,
    pub audio_to_ui_rx: Receiver<AudioToUiMsg>,
    pub is_running: Arc<AtomicBool>,
    pub stream: Option<cpal::Stream>,
}

impl AudioEngineHandle {
    pub fn dummy() -> Self {
        let (tx, _) = crossbeam_channel::unbounded();
        let (_, rx) = crossbeam_channel::unbounded();
        Self {
            ui_to_audio_tx: tx,
            audio_to_ui_rx: rx,
            is_running: Arc::new(AtomicBool::new(false)),
            stream: None,
        }
    }
}

pub struct AudioEngine {
    project: Project,
    sample_rate: f32,
    transport_state: TransportState,
    current_tick: u64,
    tick_accumulator: f64,
    current_step: usize,
    last_triggered_tick: Option<u64>,
    last_triggered_step: Option<usize>,

    // Geradores dedicados de alta fidelidade
    kick: DrumVoice,
    clap: DrumVoice,
    hat: DrumVoice,
    snare: DrumVoice,
    poly_synth: PolyphonicSynth,

    // Efeitos de mixer
    soundgoodizer: Soundgoodizer,
    eq2: ParametricEQ2,
    delay: FruityDelay,
    frame_counter: usize,
    swing: f32,
}

impl AudioEngine {
    pub fn new(sample_rate: f32, project: Project) -> Self {
        Self {
            project,
            sample_rate,
            transport_state: TransportState::Stopped,
            current_tick: 0,
            tick_accumulator: 0.0,
            current_step: 0,
            last_triggered_tick: None,
            last_triggered_step: None,
            kick: DrumVoice::new(DrumType::Kick),
            clap: DrumVoice::new(DrumType::Clap),
            hat: DrumVoice::new(DrumType::ClosedHat),
            snare: DrumVoice::new(DrumType::Snare),
            poly_synth: PolyphonicSynth::default(),
            soundgoodizer: Soundgoodizer::default(),
            eq2: ParametricEQ2::default(),
            delay: FruityDelay::default(),
            frame_counter: 0,
            swing: 0.0,
        }
    }

    pub fn handle_msg(&mut self, msg: UiToAudioMsg) {
        match msg {
            UiToAudioMsg::Play => {
                self.transport_state = TransportState::Playing;
                self.last_triggered_tick = None;
                self.last_triggered_step = None;
            }
            UiToAudioMsg::Stop => {
                self.transport_state = TransportState::Stopped;
                self.current_tick = 0;
                self.tick_accumulator = 0.0;
                self.current_step = 0;
                self.last_triggered_tick = None;
                self.last_triggered_step = None;
                self.poly_synth.reset(); // Panic reset imediato de todos os osciladores e envelopes
                self.delay.reset(); // Limpa ecos residuais do buffer de delay
                self.kick.silence();
                self.clap.silence();
                self.hat.silence();
                self.snare.silence();
            }
            UiToAudioMsg::Record => {
                self.transport_state = TransportState::Recording;
                self.last_triggered_tick = None;
                self.last_triggered_step = None;
            }
            UiToAudioMsg::SetBpm(bpm) => {
                self.project.tempo = bpm.clamp(20.0, 999.0);
            }
            UiToAudioMsg::SetPlaybackMode(mode) => {
                self.project.playback_mode = mode;
                self.last_triggered_tick = None;
                self.last_triggered_step = None;
                self.poly_synth.reset();
                self.delay.reset();
                self.kick.silence();
                self.clap.silence();
                self.hat.silence();
                self.snare.silence();
            }
            UiToAudioMsg::SeekTick(tick) => {
                self.current_tick = tick;
                self.tick_accumulator = 0.0;
                self.current_step = ((tick / TICKS_PER_STEP) % 16) as usize;
                self.last_triggered_tick = None;
                self.last_triggered_step = None;
                self.poly_synth.reset();
                self.delay.reset();
                self.kick.silence();
                self.clap.silence();
                self.hat.silence();
                self.snare.silence();
            }
            UiToAudioMsg::TriggerNoteOn {
                channel_id,
                pitch,
                velocity,
            } => {
                let vel = if velocity <= 0.0 { 0.8 } else { velocity };
                match channel_id {
                    0 => self.kick.trigger(vel),
                    1 => self.clap.trigger(vel),
                    2 => self.hat.trigger(vel),
                    3 => self.snare.trigger(vel),
                    _ => self.poly_synth.note_on(pitch),
                }

                // Captura e gravação em tempo real no Pattern ativo
                if self.transport_state == TransportState::Recording {
                    let pat_id = self.project.active_pattern;
                    let current_tick = self.current_tick;
                    if let Some(pattern) = self.project.patterns.iter_mut().find(|p| p.id == pat_id) {
                        if channel_id < 4 {
                            let step_idx = ((current_tick / TICKS_PER_STEP) % 16) as usize;
                            let steps = pattern.get_or_create_steps(channel_id, 16);
                            if let Some(st) = steps.get_mut(step_idx) {
                                st.active = true;
                                st.velocity = vel;
                            }
                        } else {
                            let pat_len = (pattern.length_steps as u64) * TICKS_PER_STEP;
                            let rel_tick = if pat_len > 0 { current_tick % pat_len } else { current_tick };
                            let notes = pattern.piano_roll_notes.entry(channel_id).or_insert_with(Vec::new);
                            let new_id = (notes.len() as u64) + 1;
                            notes.push(openstudio_core::Note {
                                id: new_id,
                                pitch,
                                velocity: vel,
                                start_tick: rel_tick,
                                duration_ticks: 48, // 1/2 tempo padrão
                                pan: 0.0,
                                release: 0.5,
                                color_index: 0,
                                muted: false,
                            });
                        }
                    }
                }
            }
            UiToAudioMsg::TriggerNoteOff { channel_id: _, pitch } => {
                self.poly_synth.note_off(pitch);
            }
            UiToAudioMsg::UpdateStep {
                pattern_id,
                channel_id,
                step_index,
                active,
                velocity,
            } => {
                if let Some(pattern) = self.project.patterns.iter_mut().find(|p| p.id == pattern_id) {
                    let steps = pattern.get_or_create_steps(channel_id, 16);
                    if let Some(step) = steps.get_mut(step_index) {
                        step.active = active;
                        step.velocity = velocity;
                    }
                }
            }
            UiToAudioMsg::UpdatePianoRollNotes {
                pattern_id,
                channel_id,
                notes,
            } => {
                if let Some(pattern) = self.project.patterns.iter_mut().find(|p| p.id == pattern_id) {
                    pattern.piano_roll_notes.insert(channel_id, notes);
                }
            }
            UiToAudioMsg::UpdatePlaylistClips(clips) => {
                self.project.playlist_clips = clips;
            }
            UiToAudioMsg::SetChannelParam {
                channel_id,
                param_type,
                value,
            } => {
                if let Some(channel) = self.project.channels.get_mut(channel_id) {
                    match param_type {
                        ChannelParam::Volume => channel.volume = value,
                        ChannelParam::Pan => channel.pan = value,
                        ChannelParam::PitchCoarse => channel.pitch_coarse = value as i32,
                        ChannelParam::PitchFine => channel.pitch_fine = value,
                        ChannelParam::TargetMixerTrack => channel.target_mixer_track = value as usize,
                    }
                }
            }
            UiToAudioMsg::SetMixerParam {
                track_id,
                param_type,
                value,
            } => {
                if let Some(track) = self.project.mixer_tracks.iter_mut().find(|t| t.index == track_id) {
                    match param_type {
                        MixerParam::Volume => track.volume = value,
                        MixerParam::Pan => track.pan = value,
                        MixerParam::StereoSeparation => track.stereo_separation = value,
                        MixerParam::Mute => track.mute = value > 0.5,
                        MixerParam::Solo => track.solo = value > 0.5,
                        MixerParam::SendLevel(tgt) => {
                            if let Some(s) = track.sends.iter_mut().find(|s| s.0 == tgt) {
                                s.1 = value;
                            } else {
                                track.sends.push((tgt, value));
                            }
                        }
                    }
                }
            }
            UiToAudioMsg::SetMixerSlotBypass {
                track_id,
                slot_index,
                enabled,
            } => {
                if track_id == 0 {
                    if slot_index == 0 {
                        self.soundgoodizer.enabled = enabled;
                    } else if slot_index == 2 {
                        self.delay.enabled = enabled;
                    }
                }
                if let Some(track) = self.project.mixer_tracks.iter_mut().find(|t| t.index == track_id) {
                    if let Some(slot) = track.slots.get_mut(slot_index) {
                        slot.enabled = enabled;
                    }
                }
            }
            UiToAudioMsg::SetMixerSlotWet {
                track_id,
                slot_index,
                wet,
            } => {
                if track_id == 0 {
                    if slot_index == 0 {
                        self.soundgoodizer.amount = wet;
                    } else if slot_index == 2 {
                        self.delay.wet = wet;
                    }
                }
                if let Some(track) = self.project.mixer_tracks.iter_mut().find(|t| t.index == track_id) {
                    if let Some(slot) = track.slots.get_mut(slot_index) {
                        slot.wet_mix = wet;
                    }
                }
            }
            UiToAudioMsg::SetSwing(swing) => {
                self.swing = swing.clamp(0.0, 1.0);
            }
            UiToAudioMsg::SetActivePattern(pat_id) => {
                self.project.active_pattern = pat_id;
            }
            UiToAudioMsg::LoadSample { channel_id: _, path: _ } => {
                // Carregamento de sample via Symphonia / Hound
            }
            UiToAudioMsg::SyncProject(p) => {
                self.project = p;
            }
        }
    }

    /// Processa um bloco de áudio estéreo em tempo real
    pub fn process_block(
        &mut self,
        output: &mut [f32],
        audio_to_ui_tx: &Sender<AudioToUiMsg>,
    ) {
        let num_frames = output.len() / 2;

        // Limpa buffer de saída
        for sample in output.iter_mut() {
            *sample = 0.0;
        }

        // Ticks por segundo: (BPM / 60) * 96 ticks
        let ticks_per_second = (self.project.tempo as f64 / 60.0) * (openstudio_core::PPQ as f64);
        let ticks_per_sample = ticks_per_second / (self.sample_rate as f64);

        let mut master_peak_l = 0.0_f32;
        let mut master_peak_r = 0.0_f32;
        let mut ch_peaks = [0.0_f32; 5]; // 0: Kick, 1: Clap, 2: Hat, 3: Snare, 4: Synth

        let block_start_tick = self.current_tick;

        for frame_idx in 0..num_frames {
            // Avança o relógio de transporte se estiver tocando ou gravando
            if self.transport_state == TransportState::Playing || self.transport_state == TransportState::Recording {
                // 1. Disparo imediato na posição atual (garante que notas sob o cursor toquem na hora sem esperar loop!)
                if self.last_triggered_tick != Some(self.current_tick) {
                    self.last_triggered_tick = Some(self.current_tick);

                    match self.project.playback_mode {
                        openstudio_core::PlaybackMode::Pattern => {
                            self.trigger_piano_roll_notes_at_tick(self.current_tick);
                        }
                        openstudio_core::PlaybackMode::Song => {
                            self.trigger_song_mode_clips_at_tick(self.current_tick);
                        }
                    }
                }

                // Cálculo do step atual no Pattern mode (16 passos de 24 ticks cada)
                let current_step = ((self.current_tick / TICKS_PER_STEP) % 16) as usize;
                if self.last_triggered_step != Some(current_step) {
                    self.last_triggered_step = Some(current_step);
                    self.current_step = current_step;
                    if self.project.playback_mode == openstudio_core::PlaybackMode::Pattern {
                        self.trigger_step_sequencer_notes(current_step);
                    }
                    let _ = audio_to_ui_tx.try_send(AudioToUiMsg::CurrentStep(current_step));
                }

                // 2. Acumula tempo e avança para os próximos ticks
                self.tick_accumulator += ticks_per_sample;
                while self.tick_accumulator >= 1.0 {
                    self.tick_accumulator -= 1.0;
                    self.current_tick += 1;

                    if self.project.playback_mode == openstudio_core::PlaybackMode::Song {
                        let song_end_tick = self.project.playlist_clips.iter()
                            .filter(|c| !c.muted)
                            .map(|c| c.start_tick + c.length_ticks)
                            .max()
                            .unwrap_or(TICKS_PER_STEP * 16 * 4);

                        if self.current_tick >= song_end_tick {
                            self.current_tick = 0;
                            self.last_triggered_tick = None;
                            self.last_triggered_step = None;
                            self.poly_synth.all_notes_off();
                            self.kick.silence();
                            self.clap.silence();
                            self.hat.silence();
                            self.snare.silence();
                        }
                    }
                }
            }

            // Somatório de todas as vozes (Bateria + Sintetizador Polifônico)
            let (k_l, k_r) = self.kick.next_sample(self.sample_rate);
            let (c_l, c_r) = self.clap.next_sample(self.sample_rate);
            let (h_l, h_r) = self.hat.next_sample(self.sample_rate);
            let (s_l, s_r) = self.snare.next_sample(self.sample_rate);
            let (p_l, p_r) = self.poly_synth.next_sample(self.sample_rate);

            ch_peaks[0] = ch_peaks[0].max(k_l.abs().max(k_r.abs()));
            ch_peaks[1] = ch_peaks[1].max(c_l.abs().max(c_r.abs()));
            ch_peaks[2] = ch_peaks[2].max(h_l.abs().max(h_r.abs()));
            ch_peaks[3] = ch_peaks[3].max(s_l.abs().max(s_r.abs()));
            ch_peaks[4] = ch_peaks[4].max(p_l.abs().max(p_r.abs()));

            let ch0_vol = self.project.channels.first().map(|c| c.volume).unwrap_or(0.85);
            let ch1_vol = self.project.channels.get(1).map(|c| c.volume).unwrap_or(0.85);
            let ch2_vol = self.project.channels.get(2).map(|c| c.volume).unwrap_or(0.85);
            let ch3_vol = self.project.channels.get(3).map(|c| c.volume).unwrap_or(0.85);
            let ch4_vol = self.project.channels.get(4).map(|c| c.volume).unwrap_or(0.85);

            let mixed_l = k_l * ch0_vol + c_l * ch1_vol + h_l * ch2_vol + s_l * ch3_vol + p_l * ch4_vol;
            let mixed_r = k_r * ch0_vol + c_r * ch1_vol + h_r * ch2_vol + s_r * ch3_vol + p_r * ch4_vol;

            // Aplica plugins do Master (Soundgoodizer + Delay + EQ2) - 100% DRY por padrão!
            let (sg_l, sg_r) = self.soundgoodizer.process_stereo(mixed_l, mixed_r);
            let (dl_l, dl_r) = self.delay.process_stereo(self.sample_rate, sg_l, sg_r);
            let eq_l = self.eq2.process_sample(dl_l);
            let eq_r = self.eq2.process_sample(dl_r);

            // Volume Master e soft clipping
            let master_vol = self.project.master_volume;
            let final_l = (eq_l * master_vol).clamp(-1.0, 1.0);
            let final_r = (eq_r * master_vol).clamp(-1.0, 1.0);

            output[frame_idx * 2] = final_l;
            output[frame_idx * 2 + 1] = final_r;

            master_peak_l = master_peak_l.max(final_l.abs());
            master_peak_r = master_peak_r.max(final_r.abs());
        }

        // Envia telemetria para a UI a cada ~60 quadros
        self.frame_counter += num_frames;
        if self.frame_counter >= (self.sample_rate as usize / 60) {
            self.frame_counter = 0;
            let _ = audio_to_ui_tx.try_send(AudioToUiMsg::PlayheadTick(block_start_tick));
            let _ = audio_to_ui_tx.try_send(AudioToUiMsg::CurrentStep(self.current_step));
            let _ = audio_to_ui_tx.try_send(AudioToUiMsg::IsPlaying(
                self.transport_state == TransportState::Playing || self.transport_state == TransportState::Recording,
            ));

            // Telemetria de pico para Master (0) e Inserts 1 a 5
            let mut meters = vec![(0, master_peak_l, master_peak_r)];
            for (i, peak) in ch_peaks.iter().enumerate() {
                meters.push((i + 1, *peak, *peak));
            }
            let _ = audio_to_ui_tx.try_send(AudioToUiMsg::MixerMeters(meters));
        }
    }

    fn trigger_step_sequencer_notes(&mut self, step_index: usize) {
        let pat_id = self.project.active_pattern;
        if let Some(pattern) = self.project.patterns.iter().find(|p| p.id == pat_id) {
            for (ch_idx, steps) in &pattern.step_data {
                if let Some(step) = steps.get(step_index) {
                    if step.active {
                        let vel = step.velocity;
                        match *ch_idx {
                            0 => self.kick.trigger(vel),
                            1 => self.clap.trigger(vel),
                            2 => self.hat.trigger(vel),
                            3 => self.snare.trigger(vel),
                            _ => self.poly_synth.note_on(60),
                        }
                    }
                }
            }
        }
    }

    fn trigger_piano_roll_notes_at_tick(&mut self, tick: u64) {
        let pat_id = self.project.active_pattern;
        if let Some(pattern) = self.project.patterns.iter().find(|p| p.id == pat_id) {
            let pat_len_ticks = (pattern.length_steps as u64) * TICKS_PER_STEP;
            let rel_tick = if pat_len_ticks > 0 { tick % pat_len_ticks } else { tick };

            for (ch_id, notes) in &pattern.piano_roll_notes {
                for note in notes {
                    if note.muted {
                        continue;
                    }
                    let end_tick = note.start_tick + note.duration_ticks;
                    if (end_tick == rel_tick || (end_tick >= pat_len_ticks && rel_tick == 0)) && *ch_id >= 4 {
                        self.poly_synth.note_off(note.pitch);
                    }
                    if note.start_tick == rel_tick {
                        let vel = note.velocity;
                        match *ch_id {
                            0 => self.kick.trigger(vel),
                            1 => self.clap.trigger(vel),
                            2 => self.hat.trigger(vel),
                            3 => self.snare.trigger(vel),
                            _ => self.poly_synth.note_on(note.pitch),
                        }
                    }
                }
            }
        }
    }

    /// Dispara patterns e clipes de áudio organizados ao longo da timeline da Playlist no modo SONG
    fn trigger_song_mode_clips_at_tick(&mut self, tick: u64) {
        let mut triggered_step_patterns = Vec::new();

        for clip in &self.project.playlist_clips {
            if clip.muted {
                continue;
            }

            // 1. Quando o clipe atinge exatamente o final de sua duração, desliga todas as notas ativas do synth disparadas por ele
            if tick == (clip.start_tick + clip.length_ticks) {
                if let openstudio_core::ClipPayload::Pattern(pat_id) = clip.payload {
                    if let Some(pattern) = self.project.patterns.iter().find(|p| p.id == pat_id) {
                        for (ch_id, notes) in &pattern.piano_roll_notes {
                            if *ch_id >= 4 {
                                for note in notes {
                                    self.poly_synth.note_off(note.pitch);
                                }
                            }
                        }
                    }
                }
                continue;
            }

            // 2. Processa o clipe se o tick atual estiver estritamente dentro de seus limites [start_tick, start_tick + length_ticks)
            if tick >= clip.start_tick && tick < (clip.start_tick + clip.length_ticks) {
                if let openstudio_core::ClipPayload::Pattern(pat_id) = clip.payload {
                    if let Some(pattern) = self.project.patterns.iter().find(|p| p.id == pat_id) {
                        let pat_len_ticks = (pattern.length_steps as u64) * TICKS_PER_STEP;
                        if pat_len_ticks == 0 {
                            continue;
                        }
                        let rel_tick = (tick - clip.start_tick) % pat_len_ticks;

                        // Disparo de passos do sequenciador (deduplicado para evitar flams/duplicação se o mesmo pattern estiver empilhado)
                        if rel_tick.is_multiple_of(TICKS_PER_STEP) && !triggered_step_patterns.contains(&pat_id) {
                            triggered_step_patterns.push(pat_id);
                            let step_idx = (rel_tick / TICKS_PER_STEP) as usize;
                            for (ch_idx, steps) in &pattern.step_data {
                                if let Some(step) = steps.get(step_idx) {
                                    if step.active {
                                        let vel = step.velocity;
                                        match *ch_idx {
                                            0 => self.kick.trigger(vel),
                                            1 => self.clap.trigger(vel),
                                            2 => self.hat.trigger(vel),
                                            3 => self.snare.trigger(vel),
                                            _ => self.poly_synth.note_on(60),
                                        }
                                    }
                                }
                            }
                        }

                        // Disparo e corte de notas do Piano Roll
                        for (ch_id, notes) in &pattern.piano_roll_notes {
                            for note in notes {
                                if note.muted {
                                    continue;
                                }
                                let end_tick = note.start_tick + note.duration_ticks;
                                if (end_tick == rel_tick || (end_tick >= pat_len_ticks && rel_tick == 0)) && *ch_id >= 4 {
                                    self.poly_synth.note_off(note.pitch);
                                }
                                if note.start_tick == rel_tick {
                                    let vel = note.velocity;
                                    match *ch_id {
                                        0 => self.kick.trigger(vel),
                                        1 => self.clap.trigger(vel),
                                        2 => self.hat.trigger(vel),
                                        3 => self.snare.trigger(vel),
                                        _ => self.poly_synth.note_on(note.pitch),
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Inicia o subsistema de áudio CPAL em thread dedicada de alta prioridade
pub fn start_audio_engine(project: Project) -> Result<AudioEngineHandle, String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "Nenhum dispositivo de áudio de saída encontrado".to_string())?;

    let config = device
        .default_output_config()
        .map_err(|e| format!("Erro ao obter configuração padrão do dispositivo: {}", e))?;

    let sample_rate = config.sample_rate() as f32;
    let (ui_to_audio_tx, ui_to_audio_rx) = crossbeam_channel::unbounded::<UiToAudioMsg>();
    let (audio_to_ui_tx, audio_to_ui_rx) = crossbeam_channel::unbounded::<AudioToUiMsg>();
    let is_running = Arc::new(AtomicBool::new(true));

    let mut engine = AudioEngine::new(sample_rate, project);

    let err_fn = |err| eprintln!("Erro no stream de áudio CPAL: {}", err);

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_output_stream(
            config.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // Drena mensagens da UI
                while let Ok(msg) = ui_to_audio_rx.try_recv() {
                    engine.handle_msg(msg);
                }
                engine.process_block(data, &audio_to_ui_tx);
            },
            err_fn,
            None,
        ),
        _ => return Err("Formato de amostra não suportado (apenas f32 é aceito)".to_string()),
    }
    .map_err(|e| format!("Falha ao construir stream de áudio: {}", e))?;

    stream
        .play()
        .map_err(|e| format!("Falha ao iniciar reprodução do stream: {}", e))?;

    Ok(AudioEngineHandle {
        ui_to_audio_tx,
        audio_to_ui_rx,
        is_running,
        stream: Some(stream),
    })
}

/// Exporta o projeto de áudio offline para um arquivo WAV PCM estéreo de alta fidelidade
pub fn export_project_to_wav(
    project: &openstudio_core::Project,
    output_path: &std::path::Path,
) -> Result<(), String> {
    let sample_rate = 44100;
    let mut engine = AudioEngine::new(sample_rate as f32, project.clone());
    engine.transport_state = TransportState::Playing;

    let total_ticks = match project.playback_mode {
        openstudio_core::PlaybackMode::Song => {
            project
                .playlist_clips
                .iter()
                .filter(|c| !c.muted)
                .map(|c| c.start_tick + c.length_ticks)
                .max()
                .unwrap_or(TICKS_PER_STEP * 16 * 4)
        }
        openstudio_core::PlaybackMode::Pattern => {
            let pat = project.patterns.iter().find(|p| p.id == project.active_pattern);
            pat.map(|p| p.length_steps as u64 * TICKS_PER_STEP).unwrap_or(384)
        }
    };

    let duration_secs = (total_ticks as f64 / openstudio_core::PPQ as f64) * (60.0 / project.tempo as f64) + 1.0;
    let total_frames = (duration_secs * sample_rate as f64) as usize;

    let spec = hound::WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(output_path, spec)
        .map_err(|e| format!("Falha ao criar arquivo WAV: {}", e))?;

    let (dummy_tx, _) = crossbeam_channel::unbounded();
    let mut block = vec![0.0f32; 1024];
    let mut frames_left = total_frames;

    while frames_left > 0 {
        let frames_this_block = frames_left.min(512);
        let slice = &mut block[..frames_this_block * 2];
        engine.process_block(slice, &dummy_tx);

        for &sample in slice.iter() {
            let s_i16 = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
            writer.write_sample(s_i16).map_err(|e| format!("Erro ao gravar sample: {}", e))?;
        }

        frames_left -= frames_this_block;
    }

    writer.finalize().map_err(|e| format!("Erro ao finalizar arquivo WAV: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use openstudio_core::Project;

    #[test]
    fn test_export_project_to_wav() {
        let project = Project::default();
        let temp_dir = std::env::temp_dir();
        let test_wav_path = temp_dir.join("openstudio_test_export.wav");

        let res = export_project_to_wav(&project, &test_wav_path);
        assert!(res.is_ok(), "WAV export failed: {:?}", res.err());
        assert!(test_wav_path.exists(), "Exported WAV file does not exist");

        // Verify with hound reader
        let reader = hound::WavReader::open(&test_wav_path);
        assert!(reader.is_ok());
        let reader = reader.unwrap();
        assert_eq!(reader.spec().channels, 2);
        assert_eq!(reader.spec().sample_rate, 44100);
        assert_eq!(reader.spec().bits_per_sample, 16);
        assert!(reader.duration() > 0);

        let _ = std::fs::remove_file(test_wav_path);
    }
}
