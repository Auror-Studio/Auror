use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Constante padrão de pulsos por semínima (PPQ - Pulses Per Quarter Note)
/// 96 ticks por semínima = 24 ticks por passo (1/16) no Step Sequencer
pub const PPQ: u64 = 96;
pub const TICKS_PER_STEP: u64 = PPQ / 4; // 24 ticks

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportState {
    Stopped,
    Playing,
    Recording,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackMode {
    Pattern,
    Song,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: u64,
    pub pitch: u8,            // 0..127 (60 = C5)
    pub velocity: f32,        // 0.0 .. 1.0
    pub start_tick: u64,
    pub duration_ticks: u64,
    pub pan: f32,             // -1.0 .. 1.0
    pub release: f32,         // 0.0 .. 1.0
    pub color_index: u8,      // 0..15 (MIDI colors do FL Studio)
    pub muted: bool,
}

impl Default for Note {
    fn default() -> Self {
        Self {
            id: 0,
            pitch: 60,
            velocity: 0.78, // padrão do FL Studio (~100 MIDI velocity)
            start_tick: 0,
            duration_ticks: TICKS_PER_STEP * 4, // 1 batida (1 semínima)
            pan: 0.0,
            release: 0.5,
            color_index: 0,
            muted: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub active: bool,
    pub velocity: f32,
    pub pan: f32,
    pub pitch_offset: f32, // semitons (-12.0 .. +12.0)
}

impl Default for Step {
    fn default() -> Self {
        Self {
            active: false,
            velocity: 0.78,
            pan: 0.0,
            pitch_offset: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelType {
    Sampler,
    Synth3xOsc,
    Soundgoodizer,
    PluginVST3,
    PluginCLAP,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: usize,
    pub name: String,
    pub color: u32,
    pub channel_type: ChannelType,
    pub volume: f32,             // 0.0 .. 1.0 (default 0.78)
    pub pan: f32,                // -1.0 .. 1.0
    pub pitch_coarse: i32,       // em semitons (-24 .. +24)
    pub pitch_fine: f32,         // em cents (-100 .. +100)
    pub target_mixer_track: usize, // 0 = Master, 1..125 = Inserts
    pub muted: bool,
    pub solo: bool,
    pub sample_path: Option<String>,
}

impl Default for Channel {
    fn default() -> Self {
        Self {
            id: 0,
            name: "Sampler".to_string(),
            color: 0x54a0ff,
            channel_type: ChannelType::Sampler,
            volume: 0.78,
            pan: 0.0,
            pitch_coarse: 0,
            pitch_fine: 0.0,
            target_mixer_track: 1, // padrão FL: roteado para Insert 1
            muted: false,
            solo: false,
            sample_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: usize,
    pub name: String,
    pub color: u32,
    pub length_steps: usize,
    /// Steps da grade de 16 passos para cada canal: channel_id -> Vec<Step>
    pub step_data: HashMap<usize, Vec<Step>>,
    /// Notas livres do Piano Roll para cada canal: channel_id -> Vec<Note>
    pub piano_roll_notes: HashMap<usize, Vec<Note>>,
}

impl Pattern {
    pub fn new(id: usize, name: String, steps: usize) -> Self {
        Self {
            id,
            name,
            color: 0xff851b,
            length_steps: steps,
            step_data: HashMap::new(),
            piano_roll_notes: HashMap::new(),
        }
    }

    pub fn get_or_create_steps(&mut self, channel_id: usize, count: usize) -> &mut Vec<Step> {
        self.step_data
            .entry(channel_id)
            .or_insert_with(|| vec![Step::default(); count])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationPoint {
    pub tick: u64,
    pub value: f32,    // 0.0 .. 1.0
    pub tension: f32,  // -1.0 .. 1.0 (curva Bezier estilo FL Studio)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationClip {
    pub id: usize,
    pub name: String,
    pub color: u32,
    pub target_parameter: String,
    pub points: Vec<AutomationPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClipPayload {
    Pattern(usize),        // pattern_id
    Audio(String),         // caminho do arquivo de áudio
    Automation(usize),     // automation_clip_id
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistClip {
    pub id: u64,
    pub track_index: usize, // Linha na Playlist (0..127)
    pub start_tick: u64,
    pub length_ticks: u64,
    pub payload: ClipPayload,
    pub muted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixerSlot {
    pub slot_index: usize, // 0..9 (10 slots por canal)
    pub enabled: bool,
    pub plugin_name: Option<String>,
    pub wet_mix: f32,      // 0.0 .. 1.0 (knob de mixagem do slot)
}

impl Default for MixerSlot {
    fn default() -> Self {
        Self {
            slot_index: 0,
            enabled: true,
            plugin_name: None,
            wet_mix: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixerTrack {
    pub index: usize,        // 0 = Master, 1..125 = Inserts, 126..129 = Sends
    pub name: String,
    pub color: u32,
    pub volume: f32,         // 0.0 .. 1.25 (fader)
    pub pan: f32,            // -1.0 .. 1.0
    pub stereo_separation: f32, // -1.0 (100% separado) .. 1.0 (100% mono)
    pub mute: bool,
    pub solo: bool,
    pub inverted_phase: bool,
    pub slots: Vec<MixerSlot>,
    /// Roteamento: (target_track_index, send_volume)
    pub sends: Vec<(usize, f32)>,
}

impl MixerTrack {
    pub fn new(index: usize, name: String) -> Self {
        let mut slots = Vec::with_capacity(10);
        for i in 0..10 {
            slots.push(MixerSlot {
                slot_index: i,
                enabled: true,
                plugin_name: None,
                wet_mix: 1.0,
            });
        }
        let mut sends = Vec::new();
        if index != 0 {
            // Todos os canais por padrão enviam 100% para a Master (canal 0)
            sends.push((0, 1.0));
        }

        Self {
            index,
            name,
            color: if index == 0 { 0x54a0ff } else { 0x2d3238 },
            volume: 1.0,
            pan: 0.0,
            stereo_separation: 0.0,
            mute: false,
            solo: false,
            inverted_phase: false,
            slots,
            sends,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub tempo: f32,            // BPM padrão 130.0 (estilo FL clássico)
    pub master_volume: f32,
    pub master_pitch: f32,     // em semitons (-12.0 .. +12.0)
    pub playback_mode: PlaybackMode,
    pub active_pattern: usize,
    pub channels: Vec<Channel>,
    pub patterns: Vec<Pattern>,
    pub playlist_clips: Vec<PlaylistClip>,
    pub mixer_tracks: Vec<MixerTrack>,
}

impl Default for Project {
    fn default() -> Self {
        let mut patterns = Vec::new();
        patterns.push(Pattern::new(0, "Pattern 1".to_string(), 16));

        // Cria canais clássicos iniciais do FL Studio: Kick, Clap, Hat, Snare
        let default_channel_names = ["Kick", "Clap", "Hat", "Snare"];
        let default_colors = [0xff6b6b, 0x48dbfb, 0xfeca57, 0x1dd1a1];
        let mut channels = Vec::new();
        for (i, name) in default_channel_names.iter().enumerate() {
            let mut ch = Channel::default();
            ch.id = i;
            ch.name = name.to_string();
            ch.color = default_colors[i];
            ch.target_mixer_track = i + 1; // Inserts 1, 2, 3, 4
            channels.push(ch);
        }

        // Inicializa 126 canais de mixer (0 = Master + 125 Inserts)
        let mut mixer_tracks = Vec::with_capacity(126);
        mixer_tracks.push(MixerTrack::new(0, "Master".to_string()));
        for i in 1..=125 {
            mixer_tracks.push(MixerTrack::new(i, format!("Insert {}", i)));
        }

        Self {
            name: "Novo Projeto".to_string(),
            tempo: 130.0,
            master_volume: 1.0,
            master_pitch: 0.0,
            playback_mode: PlaybackMode::Pattern,
            active_pattern: 0,
            channels,
            patterns,
            playlist_clips: Vec::new(),
            mixer_tracks,
        }
    }
}

/// Mensagens enviadas da UI (GTK4) para a Audio Thread (em tempo real)
#[derive(Debug, Clone)]
pub enum UiToAudioMsg {
    Play,
    Stop,
    Record,
    SetBpm(f32),
    SetPlaybackMode(PlaybackMode),
    SeekTick(u64),
    TriggerNoteOn {
        channel_id: usize,
        pitch: u8,
        velocity: f32,
    },
    TriggerNoteOff {
        channel_id: usize,
        pitch: u8,
    },
    UpdateStep {
        pattern_id: usize,
        channel_id: usize,
        step_index: usize,
        active: bool,
        velocity: f32,
    },
    SetChannelParam {
        channel_id: usize,
        param_type: ChannelParam,
        value: f32,
    },
    SetMixerParam {
        track_id: usize,
        param_type: MixerParam,
        value: f32,
    },
    SetActivePattern(usize),
    LoadSample {
        channel_id: usize,
        path: String,
    },
    SyncProject(Project),
}

#[derive(Debug, Clone, Copy)]
pub enum ChannelParam {
    Volume,
    Pan,
    PitchCoarse,
    PitchFine,
    TargetMixerTrack,
}

#[derive(Debug, Clone, Copy)]
pub enum MixerParam {
    Volume,
    Pan,
    StereoSeparation,
    Mute,
    Solo,
    SendLevel(usize), // target_track_id
}

/// Mensagens enviadas da Audio Thread para a UI (GTK4) a cada bloco/frame
#[derive(Debug, Clone)]
pub enum AudioToUiMsg {
    PlayheadTick(u64),
    CurrentStep(usize),
    IsPlaying(bool),
    /// Medidores de pico (RMS/Peak) por canal de mixer: (track_id, peak_l, peak_r)
    MixerMeters(Vec<(usize, f32, f32)>),
    ActiveVoices(usize),
    CpuLoad(f32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_default_structure() {
        let p = Project::default();
        assert_eq!(p.tempo, 130.0);
        assert_eq!(p.channels.len(), 4);
        assert_eq!(p.mixer_tracks.len(), 126);
        assert_eq!(p.mixer_tracks[0].name, "Master");
        assert_eq!(p.playback_mode, PlaybackMode::Pattern);
    }

    #[test]
    fn test_note_and_ticks() {
        let n = Note::default();
        assert_eq!(n.duration_ticks, TICKS_PER_STEP * 4);
        assert_eq!(TICKS_PER_STEP * 4, PPQ);
    }
}
