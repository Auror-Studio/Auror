use gtk4::prelude::*;
use gtk4::{Box, Button, Label, Orientation, ToggleButton};
use openstudio_core::{PlaybackMode, UiToAudioMsg};
use openstudio_gpu::GpuDeviceInfo;
use crossbeam_channel::Sender;

pub struct TransportBar {
    pub container: Box,
    pub play_btn: ToggleButton,
    pub stop_btn: Button,
    pub record_btn: ToggleButton,
    pub song_mode_btn: Button,
    pub pat_mode_btn: Button,
    pub bpm_label: Label,
    pub time_label: Label,
    pub cpu_label: Label,
    pub voices_label: Label,
    pub f5_btn: ToggleButton,
    pub f6_btn: ToggleButton,
    pub f7_btn: ToggleButton,
    pub f9_btn: ToggleButton,
    pub gpu_btn: Button,
}

impl TransportBar {
    pub fn new(
        tx: Sender<UiToAudioMsg>,
        gpu_info: &GpuDeviceInfo,
        initial_bpm: f32,
    ) -> Self {
        let container = Box::new(Orientation::Horizontal, 8);
        container.add_css_class("transport-bar");

        // 1. Grupo Song / Pat Mode Switch
        let mode_box = Box::new(Orientation::Horizontal, 0);
        mode_box.add_css_class("mode-switch");
        let pat_mode_btn = Button::with_label("PAT");
        pat_mode_btn.add_css_class("active");
        let song_mode_btn = Button::with_label("SONG");
        mode_box.append(&pat_mode_btn);
        mode_box.append(&song_mode_btn);

        let tx_pat = tx.clone();
        let pat_clone = pat_mode_btn.clone();
        let song_clone = song_mode_btn.clone();
        pat_mode_btn.connect_clicked(move |_| {
            pat_clone.add_css_class("active");
            song_clone.remove_css_class("active");
            let _ = tx_pat.send(UiToAudioMsg::SetPlaybackMode(PlaybackMode::Pattern));
        });

        let tx_song = tx.clone();
        let pat_clone2 = pat_mode_btn.clone();
        let song_clone2 = song_mode_btn.clone();
        song_mode_btn.connect_clicked(move |_| {
            song_clone2.add_css_class("active");
            pat_clone2.remove_css_class("active");
            let _ = tx_song.send(UiToAudioMsg::SetPlaybackMode(PlaybackMode::Song));
        });

        container.append(&mode_box);

        // 2. Botões Play / Stop / Record
        let play_btn = ToggleButton::with_label("▶ Play");
        play_btn.add_css_class("transport-btn");
        play_btn.add_css_class("play-btn");

        let stop_btn = Button::with_label("■ Stop");
        stop_btn.add_css_class("transport-btn");

        let record_btn = ToggleButton::with_label("● Rec");
        record_btn.add_css_class("transport-btn");
        record_btn.add_css_class("record-btn");

        let tx_play = tx.clone();
        let play_btn_clone = play_btn.clone();
        play_btn.connect_toggled(move |btn| {
            if btn.is_active() {
                let _ = tx_play.send(UiToAudioMsg::Play);
            } else {
                let _ = tx_play.send(UiToAudioMsg::Stop);
            }
        });

        let tx_stop = tx.clone();
        let play_btn_clone2 = play_btn_clone.clone();
        stop_btn.connect_clicked(move |_| {
            play_btn_clone2.set_active(false);
            let _ = tx_stop.send(UiToAudioMsg::Stop);
        });

        let tx_rec = tx.clone();
        record_btn.connect_toggled(move |btn| {
            if btn.is_active() {
                let _ = tx_rec.send(UiToAudioMsg::Record);
            } else {
                let _ = tx_rec.send(UiToAudioMsg::Stop);
            }
        });

        container.append(&play_btn);
        container.append(&stop_btn);
        container.append(&record_btn);

        // 3. Displays Digitais LCD (BPM e Tempo)
        let bpm_display = Box::new(Orientation::Horizontal, 2);
        bpm_display.add_css_class("lcd-display");

        let current_bpm = std::rc::Rc::new(std::cell::RefCell::new(initial_bpm));

        let bpm_dec_btn = Button::with_label("-");
        bpm_dec_btn.add_css_class("transport-btn");
        bpm_display.append(&bpm_dec_btn);

        let bpm_label = Label::new(Some(&format!("{:.1} BPM", initial_bpm)));
        bpm_label.add_css_class("lcd-bpm");
        bpm_display.append(&bpm_label);

        let bpm_inc_btn = Button::with_label("+");
        bpm_inc_btn.add_css_class("transport-btn");
        bpm_display.append(&bpm_inc_btn);

        let tx_bpm_dec = tx.clone();
        let bpm_val_dec = current_bpm.clone();
        let lbl_dec = bpm_label.clone();
        bpm_dec_btn.connect_clicked(move |_| {
            let mut b = bpm_val_dec.borrow_mut();
            *b = (*b - 1.0).max(20.0);
            lbl_dec.set_text(&format!("{:.1} BPM", *b));
            let _ = tx_bpm_dec.send(UiToAudioMsg::SetBpm(*b));
        });

        let tx_bpm_inc = tx.clone();
        let bpm_val_inc = current_bpm.clone();
        let lbl_inc = bpm_label.clone();
        bpm_inc_btn.connect_clicked(move |_| {
            let mut b = bpm_val_inc.borrow_mut();
            *b = (*b + 1.0).min(999.0);
            lbl_inc.set_text(&format!("{:.1} BPM", *b));
            let _ = tx_bpm_inc.send(UiToAudioMsg::SetBpm(*b));
        });

        let time_display = Box::new(Orientation::Horizontal, 4);
        time_display.add_css_class("lcd-display");
        let time_label = Label::new(Some("001:01:000"));
        time_label.add_css_class("lcd-time");
        time_display.append(&time_label);

        container.append(&bpm_display);
        container.append(&time_display);

        // 4. Medidores de Performance (CPU e Polifonia)
        let perf_box = Box::new(Orientation::Horizontal, 6);
        perf_box.add_css_class("lcd-display");
        let cpu_label = Label::new(Some("CPU: 3%"));
        let voices_label = Label::new(Some("Vozes: 0"));
        perf_box.append(&cpu_label);
        perf_box.append(&voices_label);
        container.append(&perf_box);

        // Espaçador expansível
        let spacer = Box::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        container.append(&spacer);

        // 5. Badge de GPU Acelerada
        let gpu_label = format!("⚡ GPU: {} ({})", gpu_info.vendor.brand_tag().to_uppercase(), gpu_info.driver_info);
        let gpu_btn = Button::with_label(&gpu_label);
        gpu_btn.add_css_class("gpu-badge");
        container.append(&gpu_btn);

        // 6. Botões de Alternância de Janelas (Atalhos F5..F9)
        let win_box = Box::new(Orientation::Horizontal, 4);
        let f5_btn = ToggleButton::with_label("Playlist [F5]");
        f5_btn.add_css_class("window-toggle-btn");

        let f6_btn = ToggleButton::with_label("Channel Rack [F6]");
        f6_btn.add_css_class("window-toggle-btn");
        f6_btn.set_active(true); // F6 aberto por padrão

        let f7_btn = ToggleButton::with_label("Piano Roll [F7]");
        f7_btn.add_css_class("window-toggle-btn");

        let f9_btn = ToggleButton::with_label("Mixer [F9]");
        f9_btn.add_css_class("window-toggle-btn");

        win_box.append(&f5_btn);
        win_box.append(&f6_btn);
        win_box.append(&f7_btn);
        win_box.append(&f9_btn);
        container.append(&win_box);

        Self {
            container,
            play_btn,
            stop_btn,
            record_btn,
            song_mode_btn,
            pat_mode_btn,
            bpm_label,
            time_label,
            cpu_label,
            voices_label,
            f5_btn,
            f6_btn,
            f7_btn,
            f9_btn,
            gpu_btn,
        }
    }
}
