use gtk4::prelude::*;
use gtk4::{Box, Button, DrawingArea, Label, Orientation};
use openstudio_core::{Project, UiToAudioMsg};
use crossbeam_channel::Sender;
use std::rc::Rc;
use std::cell::RefCell;

pub struct PlaylistWidget {
    pub container: Box,
    pub canvas: DrawingArea,
}

impl PlaylistWidget {
    pub fn new(
        _project: Rc<RefCell<Project>>,
        _tx: Sender<UiToAudioMsg>,
    ) -> Self {
        let container = Box::new(Orientation::Vertical, 4);
        container.add_css_class("channel-rack-box");

        // Barra de Ferramentas da Playlist
        let header = Box::new(Orientation::Horizontal, 4);
        let title = Label::new(Some("Playlist - Arrangement [F5]"));
        title.add_css_class("lcd-bpm");
        header.append(&title);

        let p_btn = Button::with_label("✏ Pencil [P]");
        p_btn.add_css_class("transport-btn");
        let b_btn = Button::with_label("🖌 Brush [B]");
        b_btn.add_css_class("transport-btn");
        let d_btn = Button::with_label("🗑 Delete [D]");
        d_btn.add_css_class("transport-btn");
        let c_btn = Button::with_label("✂ Slice [C]");
        c_btn.add_css_class("transport-btn");

        header.append(&p_btn);
        header.append(&b_btn);
        header.append(&d_btn);
        header.append(&c_btn);

        container.append(&header);

        // Canvas de visualização de faixas e clipes
        let canvas = DrawingArea::new();
        canvas.set_content_width(800);
        canvas.set_content_height(300);
        canvas.set_hexpand(true);
        canvas.set_vexpand(true);

        canvas.set_draw_func(move |_area, cr, width, height| {
            // Fundo escuro da Playlist
            cr.set_source_rgb(0.13, 0.15, 0.17);
            let _ = cr.paint();

            let track_height = 28.0;
            let bar_width = 80.0;
            let header_w = 70.0;

            // Régua de Tempo Superior
            cr.set_source_rgb(0.18, 0.21, 0.24);
            cr.rectangle(header_w, 0.0, width as f64, 20.0);
            let _ = cr.fill();

            // Desenha divisões de compassos
            let num_bars = ((width as f64 - header_w) / bar_width).ceil() as i32;
            for b in 0..num_bars {
                let x = header_w + b as f64 * bar_width;
                cr.set_source_rgba(0.3, 0.35, 0.42, 0.5);
                cr.set_line_width(1.0);
                cr.move_to(x, 0.0);
                cr.line_to(x, height as f64);
                let _ = cr.stroke();
            }

            // Desenha as Tracks
            let num_tracks = ((height as f64 - 20.0) / track_height).ceil() as i32;
            for t in 0..num_tracks {
                let y = 20.0 + t as f64 * track_height;

                // Cabeçalho da Track
                cr.set_source_rgb(0.16, 0.18, 0.22);
                cr.rectangle(0.0, y, header_w, track_height - 1.0);
                let _ = cr.fill();

                // Linha divisória horizontal
                cr.set_source_rgba(0.22, 0.25, 0.30, 0.6);
                cr.set_line_width(1.0);
                cr.move_to(0.0, y + track_height - 1.0);
                cr.line_to(width as f64, y + track_height - 1.0);
                let _ = cr.stroke();
            }

            // Desenha alguns blocos de Patterns de demonstração
            // Track 1: Pattern 1 nos compassos 1..5
            let clip1_x = header_w + 0.0 * bar_width;
            let clip1_y = 20.0 + 0.0 * track_height + 2.0;
            let clip1_w = 4.0 * bar_width - 2.0;
            let clip1_h = track_height - 4.0;

            // Gradiente Laranja do FL Studio
            cr.set_source_rgb(0.95, 0.55, 0.15);
            cr.rectangle(clip1_x, clip1_y, clip1_w, clip1_h);
            let _ = cr.fill();

            cr.set_source_rgb(1.0, 0.75, 0.35);
            cr.set_line_width(1.0);
            cr.rectangle(clip1_x, clip1_y, clip1_w, clip1_h);
            let _ = cr.stroke();

            // Track 2: Clipes de Áudio nos compassos 1..4 (Azul Celeste)
            let clip2_x = header_w + 0.0 * bar_width;
            let clip2_y = 20.0 + 1.0 * track_height + 2.0;
            let clip2_w = 4.0 * bar_width - 2.0;
            let clip2_h = track_height - 4.0;

            cr.set_source_rgb(0.25, 0.60, 0.95);
            cr.rectangle(clip2_x, clip2_y, clip2_w, clip2_h);
            let _ = cr.fill();

            cr.set_source_rgb(0.45, 0.80, 1.0);
            cr.set_line_width(1.0);
            cr.rectangle(clip2_x, clip2_y, clip2_w, clip2_h);
            let _ = cr.stroke();
        });

        container.append(&canvas);

        Self { container, canvas }
    }
}
