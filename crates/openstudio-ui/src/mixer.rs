use gtk4::prelude::*;
use gtk4::{
    Box, DrawingArea, Label, Orientation, Scale, ScrolledWindow, ToggleButton,
};
use openstudio_core::{MixerParam, Project, UiToAudioMsg};
use crossbeam_channel::Sender;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub struct MixerWidget {
    pub container: Box,
    pub track_meter_levels: Rc<RefCell<Vec<f32>>>,
    pub meter_canvases: Rc<RefCell<Vec<DrawingArea>>>,
}

impl MixerWidget {
    pub fn new(
        _project: Rc<RefCell<Project>>,
        tx: Sender<UiToAudioMsg>,
    ) -> Self {
        let container = Box::new(Orientation::Vertical, 4);
        container.add_css_class("channel-rack-box");

        // 1. Cabeçalho do Mixer
        let header = Box::new(Orientation::Horizontal, 8);
        let title = Label::new(Some("🎛 Mixer - [F9]"));
        title.add_css_class("lcd-bpm");
        header.append(&title);

        let routing_label = Label::new(Some("Roteamento: 125 Inserts -> Master Out (Som 100% DRY por padrão)"));
        routing_label.add_css_class("lcd-display");
        header.append(&routing_label);

        container.append(&header);

        // 2. Área Principal com Faders e Painel Lateral de 10 Slots
        let main_h_box = Box::new(Orientation::Horizontal, 6);

        let scrolled = ScrolledWindow::new();
        scrolled.set_hexpand(true);
        scrolled.set_vexpand(true);
        scrolled.set_min_content_height(340);
        scrolled.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Never);

        let tracks_box = Box::new(Orientation::Horizontal, 4);

        let track_meter_levels = Rc::new(RefCell::new(vec![0.0_f32; 13])); // 0 = Master, 1..12 = Inserts
        let meter_canvases = Rc::new(RefCell::new(Vec::new()));

        let track_names = [
            "Master", "1. Kick", "2. Clap", "3. Hat", "4. Snare", "5. Synth",
            "Insert 6", "Insert 7", "Insert 8", "Insert 9", "Insert 10", "Insert 11", "Insert 12",
        ];

        // Construção dos canais do Mixer (0 = Master, 1..12 = Inserts)
        for (track_id, track_name) in track_names.iter().enumerate() {
            let track_box = Box::new(Orientation::Vertical, 3);
            track_box.add_css_class("mixer-track-box");
            if track_id == 0 {
                track_box.add_css_class("mixer-track-master");
            }

            // Nome do Canal
            let name_lbl = Label::new(Some(*track_name));
            name_lbl.add_css_class("mixer-track-name");
            track_box.append(&name_lbl);

            // Mini Controle de Pan (-100% .. +100%)
            let pan_scale = Scale::with_range(Orientation::Horizontal, -1.0, 1.0, 0.05);
            pan_scale.set_value(0.0);
            pan_scale.set_size_request(45, -1);
            pan_scale.set_draw_value(false);
            let tx_pan = tx.clone();
            pan_scale.connect_value_changed(move |s| {
                let _ = tx_pan.send(UiToAudioMsg::SetMixerParam {
                    track_id,
                    param_type: MixerParam::Pan,
                    value: s.value() as f32,
                });
            });
            track_box.append(&pan_scale);

            // Linha com Medidor dB e Fader Vertical lado a lado
            let fader_meter_row = Box::new(Orientation::Horizontal, 4);

            // Medidor dB de Pico com Gradiente Fluorescente
            let meter_canvas = DrawingArea::new();
            meter_canvas.set_content_width(if track_id == 0 { 26 } else { 18 });
            meter_canvas.set_content_height(160);

            let levels_draw = track_meter_levels.clone();
            meter_canvas.set_draw_func(move |_area, cr, width, height| {
                cr.set_source_rgb(0.07, 0.08, 0.10);
                let _ = cr.paint();

                let lvls = levels_draw.borrow();
                let lvl = lvls.get(track_id).copied().unwrap_or(0.0).clamp(0.0, 1.0) as f64;
                let meter_h = lvl * height as f64;
                let y_start = height as f64 - meter_h;

                for y_step in (0..height).rev() {
                    let y = y_step as f64;
                    if y >= y_start {
                        let rel = 1.0 - (y / height as f64);
                        if rel < 0.70 {
                            cr.set_source_rgb(0.18, 0.85, 0.35); // Verde (-48dB a -6dB)
                        } else if rel < 0.90 {
                            cr.set_source_rgb(0.98, 0.75, 0.15); // Amarelo/Laranja (-6dB a -1dB)
                        } else {
                            cr.set_source_rgb(0.95, 0.20, 0.18); // Vermelho (Clip 0dB)
                        }
                        cr.rectangle(2.0, y, width as f64 - 4.0, 1.5);
                        let _ = cr.fill();
                    }
                }

                // Moldura sutil
                cr.set_source_rgba(0.25, 0.30, 0.35, 0.5);
                cr.set_line_width(1.0);
                cr.rectangle(1.0, 0.0, width as f64 - 2.0, height as f64);
                let _ = cr.stroke();
            });

            meter_canvases.borrow_mut().push(meter_canvas.clone());
            fader_meter_row.append(&meter_canvas);

            // Fader Deslizante Vertical de Volume (0..125%)
            let vol_scale = Scale::with_range(Orientation::Vertical, 0.0, 1.25, 0.01);
            vol_scale.set_inverted(true); // Cima = Mais alto, Baixo = Mudo
            vol_scale.set_value(1.0);
            vol_scale.set_size_request(-1, 160);
            vol_scale.set_draw_value(false);
            let tx_vol = tx.clone();
            vol_scale.connect_value_changed(move |s| {
                let _ = tx_vol.send(UiToAudioMsg::SetMixerParam {
                    track_id,
                    param_type: MixerParam::Volume,
                    value: s.value() as f32,
                });
            });
            fader_meter_row.append(&vol_scale);

            track_box.append(&fader_meter_row);

            // Botão Mute com LED
            let mute_btn = ToggleButton::with_label("● Mute");
            mute_btn.add_css_class("transport-btn");
            let tx_mute = tx.clone();
            mute_btn.connect_toggled(move |btn| {
                let _ = tx_mute.send(UiToAudioMsg::SetMixerParam {
                    track_id,
                    param_type: MixerParam::Mute,
                    value: if btn.is_active() { 1.0 } else { 0.0 },
                });
            });
            track_box.append(&mute_btn);

            tracks_box.append(&track_box);
        }

        scrolled.set_child(Some(&tracks_box));
        main_h_box.append(&scrolled);

        // 3. Painel de 10 Slots de Efeitos (Lado Direito com Chave de Bypass e Knobs Reais)
        let slots_box = Box::new(Orientation::Vertical, 4);
        slots_box.add_css_class("channel-rack-box");
        slots_box.set_size_request(240, -1);

        let slots_header = Label::new(Some("Effect Slots (Master)"));
        slots_header.add_css_class("lcd-bpm");
        slots_box.append(&slots_header);

        let slot_info = [
            (0, "Soundgoodizer", false, 0.50),
            (1, "Parametric EQ 2", false, 1.00),
            (2, "Fruity Delay 3", false, 0.00), // Desligado por padrão (0% wet)
            (3, "(None)", false, 0.0),
            (4, "(None)", false, 0.0),
            (5, "(None)", false, 0.0),
            (6, "(None)", false, 0.0),
            (7, "(None)", false, 0.0),
            (8, "(None)", false, 0.0),
            (9, "(None)", false, 0.0),
        ];

        for (slot_idx, name, default_enabled, default_wet) in slot_info {
            let row = Box::new(Orientation::Horizontal, 4);
            row.add_css_class("mixer-slot-row");

            // Botão LED de Bypass (Ligar / Desligar Efeito)
            let led_btn = ToggleButton::with_label(if default_enabled { "🟢" } else { "⚪" });
            led_btn.set_active(default_enabled);
            led_btn.add_css_class("transport-btn");
            led_btn.set_size_request(28, -1);

            let tx_bypass = tx.clone();
            let led_clone = led_btn.clone();
            led_btn.connect_toggled(move |btn| {
                let active = btn.is_active();
                led_clone.set_label(if active { "🟢" } else { "⚪" });
                let _ = tx_bypass.send(UiToAudioMsg::SetMixerSlotBypass {
                    track_id: 0,
                    slot_index: slot_idx,
                    enabled: active,
                });
            });
            row.append(&led_btn);

            // Nome do Efeito
            let lbl = Label::new(Some(&format!("{}. {}", slot_idx + 1, name)));
            lbl.set_hexpand(true);
            lbl.set_xalign(0.0);
            row.append(&lbl);

            // Slider de Mix Wet/Dry (0% a 100%)
            let mix_scale = Scale::with_range(Orientation::Horizontal, 0.0, 1.0, 0.05);
            mix_scale.set_value(default_wet);
            mix_scale.set_size_request(45, -1);
            mix_scale.set_draw_value(false);

            let tx_wet = tx.clone();
            mix_scale.connect_value_changed(move |s| {
                let _ = tx_wet.send(UiToAudioMsg::SetMixerSlotWet {
                    track_id: 0,
                    slot_index: slot_idx,
                    wet: s.value() as f32,
                });
            });
            row.append(&mix_scale);

            slots_box.append(&row);
        }

        main_h_box.append(&slots_box);
        container.append(&main_h_box);

        Self {
            container,
            track_meter_levels,
            meter_canvases,
        }
    }

    /// Atualiza os medidores de pico (RMS/Peak) em tempo real a 60 FPS
    pub fn update_meters(&self, meters: &[(usize, f32, f32)]) {
        let mut lvls = self.track_meter_levels.borrow_mut();
        for &(track_id, peak_l, peak_r) in meters {
            let max_peak = peak_l.max(peak_r);
            if track_id < lvls.len() {
                lvls[track_id] = max_peak;
            }
        }
        for canvas in self.meter_canvases.borrow().iter() {
            canvas.queue_draw();
        }
    }

    pub fn update_master_meter(&self, level: f32) {
        let mut lvls = self.track_meter_levels.borrow_mut();
        if !lvls.is_empty() {
            lvls[0] = level;
        }
        if let Some(canvas) = self.meter_canvases.borrow().first() {
            canvas.queue_draw();
        }
    }
}
