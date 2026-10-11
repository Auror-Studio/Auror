use gtk4::prelude::*;
use gtk4::{Box, DrawingArea, Label, Orientation, ScrolledWindow, ToggleButton};
use openstudio_core::{Project, UiToAudioMsg};
use crossbeam_channel::Sender;
use std::rc::Rc;
use std::cell::RefCell;

pub struct MixerWidget {
    pub container: Box,
    pub master_meter_level: Rc<RefCell<f32>>,
}

impl MixerWidget {
    pub fn new(
        project: Rc<RefCell<Project>>,
        _tx: Sender<UiToAudioMsg>,
    ) -> Self {
        let container = Box::new(Orientation::Vertical, 4);
        container.add_css_class("channel-rack-box");

        // Cabeçalho do Mixer
        let header = Box::new(Orientation::Horizontal, 8);
        let title = Label::new(Some("Mixer - Master [F9]"));
        title.add_css_class("lcd-bpm");
        header.append(&title);

        let routing_label = Label::new(Some("Roteamento: 125 Inserts -> Master Out"));
        routing_label.add_css_class("lcd-display");
        header.append(&routing_label);

        container.append(&header);

        // Área horizontal contendo os faders do Mixer e o painel de 10 slots
        let main_h_box = Box::new(Orientation::Horizontal, 8);

        let scrolled = ScrolledWindow::new();
        scrolled.set_hexpand(true);
        scrolled.set_vexpand(true);
        scrolled.set_min_content_height(320);

        let tracks_box = Box::new(Orientation::Horizontal, 4);

        let master_meter_level = Rc::new(RefCell::new(0.0_f32));

        // 1. Canal Master (Canal 0)
        let master_box = Box::new(Orientation::Vertical, 4);
        master_box.add_css_class("mixer-track-box");
        master_box.add_css_class("mixer-track-master");

        let m_name = Label::new(Some("Master"));
        m_name.add_css_class("mixer-track-name");
        master_box.append(&m_name);

        // Medidor de Pico dB do Master com Gradiente FL Studio
        let meter_canvas = DrawingArea::new();
        meter_canvas.set_content_width(32);
        meter_canvas.set_content_height(180);
        let m_level_draw = master_meter_level.clone();

        meter_canvas.set_draw_func(move |_area, cr, width, height| {
            // Fundo escuro do medidor
            cr.set_source_rgb(0.08, 0.09, 0.11);
            let _ = cr.paint();

            let lvl = (*m_level_draw.borrow()).clamp(0.0, 1.0) as f64;
            let meter_h = lvl * height as f64;
            let y_start = height as f64 - meter_h;

            // Desenha barras do gradiente (Verde -> Amarelo -> Vermelho)
            for y_step in (0..height as i32).rev() {
                let y = y_step as f64;
                if y >= y_start {
                    let rel = 1.0 - (y / height as f64);
                    if rel < 0.70 {
                        // Verde (-48dB a -6dB)
                        cr.set_source_rgb(0.18, 0.85, 0.35);
                    } else if rel < 0.90 {
                        // Amarelo / Laranja (-6dB a -1dB)
                        cr.set_source_rgb(0.98, 0.75, 0.15);
                    } else {
                        // Vermelho (Pico / Clip 0dB)
                        cr.set_source_rgb(0.95, 0.20, 0.18);
                    }
                    cr.rectangle(4.0, y, width as f64 - 8.0, 1.5);
                    let _ = cr.fill();
                }
            }

            // Borda do medidor
            cr.set_source_rgba(0.3, 0.35, 0.4, 0.6);
            cr.set_line_width(1.0);
            cr.rectangle(2.0, 0.0, width as f64 - 4.0, height as f64);
            let _ = cr.stroke();
        });

        master_box.append(&meter_canvas);

        let m_mute = ToggleButton::with_label("Mute");
        m_mute.add_css_class("transport-btn");
        master_box.append(&m_mute);

        tracks_box.append(&master_box);

        // 2. Inserts 1 a 12 (com visualização expandida)
        let proj = project.borrow();
        for i in 1..=12 {
            let track_box = Box::new(Orientation::Vertical, 4);
            track_box.add_css_class("mixer-track-box");

            let trk_name = if let Some(t) = proj.mixer_tracks.get(i) {
                t.name.clone()
            } else {
                format!("Insert {}", i)
            };

            let name_lbl = Label::new(Some(&trk_name));
            name_lbl.add_css_class("mixer-track-name");
            track_box.append(&name_lbl);

            let dummy_meter = DrawingArea::new();
            dummy_meter.set_content_width(24);
            dummy_meter.set_content_height(180);
            dummy_meter.set_draw_func(move |_area, cr, width, height| {
                cr.set_source_rgb(0.08, 0.09, 0.11);
                let _ = cr.paint();
                cr.set_source_rgba(0.25, 0.30, 0.35, 0.5);
                cr.set_line_width(1.0);
                cr.rectangle(2.0, 0.0, width as f64 - 4.0, height as f64);
                let _ = cr.stroke();
            });
            track_box.append(&dummy_meter);

            let mute_btn = ToggleButton::with_label("Mute");
            mute_btn.add_css_class("transport-btn");
            track_box.append(&mute_btn);

            tracks_box.append(&track_box);
        }

        scrolled.set_child(Some(&tracks_box));
        main_h_box.append(&scrolled);

        // 3. Painel de 10 Slots de Efeitos (Lado Direito)
        let slots_box = Box::new(Orientation::Vertical, 4);
        slots_box.add_css_class("channel-rack-box");
        slots_box.set_size_request(200, -1);

        let slots_header = Label::new(Some("Effect Slots (Master)"));
        slots_header.add_css_class("lcd-bpm");
        slots_box.append(&slots_header);

        let default_slots = [
            "Slot 1: Soundgoodizer",
            "Slot 2: Fruity Parametric EQ 2",
            "Slot 3: Fruity Delay 3",
            "Slot 4: (None)",
            "Slot 5: (None)",
            "Slot 6: (None)",
            "Slot 7: (None)",
            "Slot 8: (None)",
            "Slot 9: (None)",
            "Slot 10: (None)",
        ];

        for slot_str in &default_slots {
            let row = Box::new(Orientation::Horizontal, 4);
            row.add_css_class("mixer-slot-row");
            let lbl = Label::new(Some(slot_str));
            row.append(&lbl);
            slots_box.append(&row);
        }

        main_h_box.append(&slots_box);
        container.append(&main_h_box);

        Self {
            container,
            master_meter_level,
        }
    }

    pub fn update_master_meter(&self, level: f32) {
        *self.master_meter_level.borrow_mut() = level;
    }
}
