use gtk4::prelude::*;
use gtk4::{
    Box, Button, DrawingArea, EventControllerMotion, GestureClick, Label, Orientation, ToggleButton,
};
use openstudio_core::{Note, Project, UiToAudioMsg};
use crossbeam_channel::Sender;
use std::cell::RefCell;
use std::rc::Rc;

pub struct PianoRollWidget {
    pub container: Box,
    pub canvas: DrawingArea,
    pub notes: Rc<RefCell<Vec<Note>>>,
    pub ghost_notes_active: Rc<RefCell<bool>>,
    pub active_key_pressed: Rc<RefCell<Option<i32>>>,
}

impl PianoRollWidget {
    pub fn new(
        _project: Rc<RefCell<Project>>,
        tx: Sender<UiToAudioMsg>,
    ) -> Self {
        let container = Box::new(Orientation::Vertical, 4);
        container.add_css_class("piano-roll-container");

        // 1. Barra de Ferramentas do Piano Roll
        let toolbar = Box::new(Orientation::Horizontal, 4);
        let title = Label::new(Some("Piano Roll - [Sintetizador 3x Osc]"));
        title.add_css_class("lcd-bpm");
        toolbar.append(&title);

        let p_btn = Button::with_label("✏ Pencil [P]");
        p_btn.add_css_class("transport-btn");
        let b_btn = Button::with_label("🖌 Brush [B]");
        b_btn.add_css_class("transport-btn");
        let d_btn = Button::with_label("🗑 Delete [D]");
        d_btn.add_css_class("transport-btn");
        let c_btn = Button::with_label("✂ Slice [C]");
        c_btn.add_css_class("transport-btn");

        let ghost_toggle = ToggleButton::with_label("👻 Ghost Notes [Alt+V]");
        ghost_toggle.add_css_class("window-toggle-btn");
        ghost_toggle.set_active(true);

        let quant_btn = Button::with_label("⚡ Quantize [Alt+Q]");
        quant_btn.add_css_class("transport-btn");
        let strum_btn = Button::with_label("🎸 Strum [Alt+S]");
        strum_btn.add_css_class("transport-btn");

        toolbar.append(&p_btn);
        toolbar.append(&b_btn);
        toolbar.append(&d_btn);
        toolbar.append(&c_btn);
        toolbar.append(&ghost_toggle);
        toolbar.append(&quant_btn);
        toolbar.append(&strum_btn);

        container.append(&toolbar);

        // 2. Canvas Interativo do Piano Roll
        let canvas = DrawingArea::new();
        canvas.set_content_width(900);
        canvas.set_content_height(480);
        canvas.set_hexpand(true);
        canvas.set_vexpand(true);

        // Notas iniciais (Acorde C Minor 7th clássico: C5, D#5, G5, A#5)
        let mut initial_notes = Vec::new();
        initial_notes.push(Note {
            id: 1,
            pitch: 60, // C5
            velocity: 0.85,
            start_tick: 0,
            duration_ticks: 96,
            pan: 0.0,
            release: 0.5,
            color_index: 0,
            muted: false,
        });
        initial_notes.push(Note {
            id: 2,
            pitch: 63, // D#5
            velocity: 0.80,
            start_tick: 0,
            duration_ticks: 96,
            pan: 0.0,
            release: 0.5,
            color_index: 0,
            muted: false,
        });
        initial_notes.push(Note {
            id: 3,
            pitch: 67, // G5
            velocity: 0.85,
            start_tick: 0,
            duration_ticks: 96,
            pan: 0.0,
            release: 0.5,
            color_index: 0,
            muted: false,
        });
        initial_notes.push(Note {
            id: 4,
            pitch: 70, // A#5
            velocity: 0.75,
            start_tick: 0,
            duration_ticks: 96,
            pan: 0.0,
            release: 0.5,
            color_index: 0,
            muted: false,
        });

        let notes_cell = Rc::new(RefCell::new(initial_notes));
        let ghost_notes_active = Rc::new(RefCell::new(true));
        let active_key_pressed = Rc::new(RefCell::new(None));

        let keyboard_width = 54.0;
        let key_height = 20.0;
        let step_width = 32.0;

        // --- GESTOS DE MOUSE: CLIQUE EM TEMPO REAL ---
        let gesture = GestureClick::new();
        gesture.set_button(0); // Escuta botão esquerdo (1), do meio (2) e direito (3)

        let notes_click = notes_cell.clone();
        let active_key_click = active_key_pressed.clone();
        let tx_click = tx.clone();
        let canvas_click = canvas.clone();

        gesture.connect_pressed(move |g, _n_press, x, y| {
            let button = g.current_button();

            if x < keyboard_width {
                // CLIQUE NAS TECLAS DO PIANO (Coluna da Esquerda)
                let row = (y / key_height) as i32;
                let pitch = (76 - row).clamp(24, 96);

                *active_key_click.borrow_mut() = Some(pitch);
                // Dispara NoteOn em tempo real com baixa latência!
                let _ = tx_click.send(UiToAudioMsg::TriggerNoteOn {
                    channel_id: 4,
                    pitch: pitch as u8,
                    velocity: 0.85,
                });
                canvas_click.queue_draw();
            } else {
                // CLIQUE NA GRADE DE NOTAS (Lado Direito)
                let row = (y / key_height) as i32;
                let pitch = (76 - row).clamp(24, 96);
                let step_idx = ((x - keyboard_width) / step_width).max(0.0) as u64;
                let tick = step_idx * 24;

                if button == 1 {
                    // BOTÃO ESQUERDO: Adiciona ou edita nota
                    let mut notes = notes_click.borrow_mut();
                    // Verifica se já existe nota exatamente nesta posição
                    let exists = notes.iter().any(|n| n.pitch == pitch as u8 && n.start_tick == tick);
                    if !exists {
                        let new_id = notes.len() as u64 + 1;
                        notes.push(Note {
                            id: new_id,
                            pitch: pitch as u8,
                            velocity: 0.82,
                            start_tick: tick,
                            duration_ticks: 96, // 1 batida (4 passos)
                            pan: 0.0,
                            release: 0.5,
                            color_index: 0,
                            muted: false,
                        });
                        // Audição instantânea da nota inserida!
                        let _ = tx_click.send(UiToAudioMsg::TriggerNoteOn {
                            channel_id: 4,
                            pitch: pitch as u8,
                            velocity: 0.82,
                        });
                    }
                    canvas_click.queue_draw();
                } else if button == 3 {
                    // BOTÃO DIREITO: Remove nota sob o cursor
                    let mut notes = notes_click.borrow_mut();
                    let mut to_remove = None;
                    for (i, note) in notes.iter().enumerate() {
                        let n_row = 76 - note.pitch as i32;
                        let n_y = n_row as f64 * key_height;
                        let n_x = keyboard_width + (note.start_tick as f64 / 24.0) * step_width;
                        let n_w = (note.duration_ticks as f64 / 24.0) * step_width;

                        if x >= n_x && x <= n_x + n_w && y >= n_y && y <= n_y + key_height {
                            to_remove = Some(i);
                            break;
                        }
                    }
                    if let Some(idx) = to_remove {
                        notes.remove(idx);
                        canvas_click.queue_draw();
                    }
                }
            }
        });

        let active_key_release = active_key_pressed.clone();
        let tx_release = tx.clone();
        let canvas_release = canvas.clone();

        gesture.connect_released(move |_g, _n_press, _x, _y| {
            if let Some(pitch) = active_key_release.borrow_mut().take() {
                // Silencia a tecla do piano quando o usuário solta o botão do mouse
                let _ = tx_release.send(UiToAudioMsg::TriggerNoteOff {
                    channel_id: 4,
                    pitch: pitch as u8,
                });
                canvas_release.queue_draw();
            }
        });

        canvas.add_controller(gesture);

        // Movimento do mouse para glissando (arrastar pelas teclas do piano)
        let motion = EventControllerMotion::new();
        let active_key_motion = active_key_pressed.clone();
        let tx_motion = tx.clone();
        let canvas_motion = canvas.clone();

        motion.connect_motion(move |_m, x, y| {
            let mut active = active_key_motion.borrow_mut();
            if let Some(prev_pitch) = *active {
                if x < keyboard_width {
                    let row = (y / key_height) as i32;
                    let new_pitch = (76 - row).clamp(24, 96);
                    if new_pitch != prev_pitch {
                        let _ = tx_motion.send(UiToAudioMsg::TriggerNoteOff {
                            channel_id: 4,
                            pitch: prev_pitch as u8,
                        });
                        let _ = tx_motion.send(UiToAudioMsg::TriggerNoteOn {
                            channel_id: 4,
                            pitch: new_pitch as u8,
                            velocity: 0.85,
                        });
                        *active = Some(new_pitch);
                        canvas_motion.queue_draw();
                    }
                }
            }
        });

        canvas.add_controller(motion);

        // --- RENDERIZAÇÃO GRÁFICA DO PIANO ROLL ---
        let notes_draw = notes_cell.clone();
        let ghost_draw = ghost_notes_active.clone();
        let active_key_draw = active_key_pressed.clone();

        canvas.set_draw_func(move |_area, cr, width, height| {
            // Fundo da área de trabalho
            cr.set_source_rgb(0.11, 0.13, 0.15);
            let _ = cr.paint();

            let num_keys = (height as f64 / key_height).ceil() as i32;

            // 1. Desenha linhas de fundo da grade
            for i in 0..num_keys {
                let y = i as f64 * key_height;
                let pitch = 76 - i;

                let is_black_key = match pitch.rem_euclid(12) {
                    1 | 3 | 6 | 8 | 10 => true,
                    _ => false,
                };

                if is_black_key {
                    cr.set_source_rgb(0.08, 0.09, 0.11);
                    cr.rectangle(keyboard_width, y, width as f64 - keyboard_width, key_height);
                    let _ = cr.fill();
                }

                // Linha divisória horizontal sutil
                cr.set_source_rgba(0.18, 0.22, 0.26, 0.6);
                cr.set_line_width(0.7);
                cr.move_to(keyboard_width, y);
                cr.line_to(width as f64, y);
                let _ = cr.stroke();
            }

            // 2. Linhas verticais dos compassos e batidas
            let num_steps = ((width as f64 - keyboard_width) / step_width).ceil() as i32;
            for s in 0..num_steps {
                let x = keyboard_width + s as f64 * step_width;
                if s % 4 == 0 {
                    // Batida principal (1/4 de compasso) - mais forte
                    cr.set_source_rgba(0.32, 0.38, 0.46, 0.8);
                    cr.set_line_width(1.2);
                } else {
                    // Subdivisão 1/16
                    cr.set_source_rgba(0.16, 0.19, 0.23, 0.5);
                    cr.set_line_width(0.6);
                }
                cr.move_to(x, 0.0);
                cr.line_to(x, height as f64);
                let _ = cr.stroke();
            }

            // 3. Desenho das Notas da Grade
            let notes = notes_draw.borrow();
            for note in notes.iter() {
                let row = (76 - note.pitch as i32) as f64;
                let y = row * key_height + 1.5;
                let x = keyboard_width + (note.start_tick as f64 / 24.0) * step_width + 0.5;
                let note_w = ((note.duration_ticks as f64 / 24.0) * step_width - 1.0).max(4.0);

                // Preenchimento Verde Neon característico do FL Studio
                cr.set_source_rgb(0.18, 0.85, 0.45);
                cr.rectangle(x, y, note_w, key_height - 3.0);
                let _ = cr.fill();

                // Borda de relevo 3D na nota
                cr.set_source_rgb(0.35, 0.98, 0.60);
                cr.set_line_width(1.0);
                cr.rectangle(x, y, note_w, key_height - 3.0);
                let _ = cr.stroke();
            }

            // 4. Ghost Notes (se ativado via Alt+V)
            if *ghost_draw.borrow() {
                let ghost_pitch_row = (76 - 48) as f64;
                let ghost_y = ghost_pitch_row * key_height + 1.5;
                cr.set_source_rgba(0.6, 0.7, 0.8, 0.25);
                cr.rectangle(keyboard_width, ghost_y, step_width * 4.0, key_height - 3.0);
                let _ = cr.fill();
            }

            // 5. TECLADO VERTICAL DO PIANO (Coluna Esquerda com efeito 3D e sombra)
            cr.set_source_rgb(0.12, 0.14, 0.17);
            cr.rectangle(0.0, 0.0, keyboard_width, height as f64);
            let _ = cr.fill();

            let active_key = *active_key_draw.borrow();

            for i in 0..num_keys {
                let y = i as f64 * key_height;
                let pitch = 76 - i;

                let is_black = match pitch.rem_euclid(12) {
                    1 | 3 | 6 | 8 | 10 => true,
                    _ => false,
                };

                let is_pressed = active_key == Some(pitch);

                if is_pressed {
                    // Tecla ativa iluminada em Azul Celeste brilhante
                    cr.set_source_rgb(0.20, 0.65, 1.0);
                    cr.rectangle(0.0, y, keyboard_width, key_height);
                    let _ = cr.fill();
                } else if is_black {
                    // Tecla preta em relevo
                    cr.set_source_rgb(0.06, 0.07, 0.09);
                    cr.rectangle(0.0, y + 2.0, keyboard_width * 0.65, key_height - 4.0);
                    let _ = cr.fill();

                    cr.set_source_rgb(0.18, 0.20, 0.24);
                    cr.set_line_width(1.0);
                    cr.rectangle(0.0, y + 2.0, keyboard_width * 0.65, key_height - 4.0);
                    let _ = cr.stroke();
                } else {
                    // Tecla branca com chanfro
                    cr.set_source_rgb(0.82, 0.86, 0.90);
                    cr.rectangle(0.0, y + 0.5, keyboard_width - 1.0, key_height - 1.0);
                    let _ = cr.fill();

                    cr.set_source_rgb(0.35, 0.38, 0.42);
                    cr.set_line_width(0.8);
                    cr.rectangle(0.0, y + 0.5, keyboard_width - 1.0, key_height - 1.0);
                    let _ = cr.stroke();

                    // Nome da nota na tecla C (C4, C5, C6...)
                    if pitch.rem_euclid(12) == 0 {
                        let octave = (pitch / 12) - 1;
                        cr.set_source_rgb(0.2, 0.25, 0.3);
                        cr.set_font_size(9.0);
                        cr.move_to(keyboard_width - 18.0, y + key_height - 5.0);
                        let _ = cr.show_text(&format!("C{}", octave));
                    }
                }
            }

            // Sombra da coluna do teclado sobre a grade
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.35);
            cr.rectangle(keyboard_width, 0.0, 3.0, height as f64);
            let _ = cr.fill();
        });

        // Alternar Ghost Notes
        let ghost_ref = ghost_notes_active.clone();
        let canvas_toggle = canvas.clone();
        ghost_toggle.connect_toggled(move |btn| {
            *ghost_ref.borrow_mut() = btn.is_active();
            canvas_toggle.queue_draw();
        });

        container.append(&canvas);

        Self {
            container,
            canvas,
            notes: notes_cell,
            ghost_notes_active,
            active_key_pressed,
        }
    }
}
