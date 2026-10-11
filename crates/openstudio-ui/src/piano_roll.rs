use gtk4::prelude::*;
use gtk4::{
    Box, Button, DrawingArea, DropDown, EventControllerMotion, GestureClick, Label,
    Orientation, ScrolledWindow, StringList, ToggleButton,
};
use openstudio_core::{Note, Project, UiToAudioMsg, TICKS_PER_STEP};
use crossbeam_channel::Sender;
use std::cell::RefCell;
use std::rc::Rc;

pub const MIN_PITCH: i32 = 24; // C1
pub const MAX_PITCH: i32 = 96; // C7
pub const TOTAL_KEYS: i32 = MAX_PITCH - MIN_PITCH + 1; // 73 teclas (6 oitavas completas)

pub const RULER_HEIGHT: f64 = 24.0;
pub const KEY_HEIGHT: f64 = 18.0;
pub const VELOCITY_HEIGHT: f64 = 64.0;
pub const KEYBOARD_WIDTH: f64 = 64.0;
pub const STEP_WIDTH: f64 = 26.0;
pub const TOTAL_STEPS: usize = 128; // 8 compassos completos (32 tempos)

#[derive(Debug, Clone, Copy, PartialEq)]
enum DragMode {
    None,
    Keyboard { pitch: u8 },
    Ruler,
    DrawingNote {
        note_id: u64,
        pitch: u8,
        start_tick: u64,
    },
    ResizingNote {
        note_id: u64,
        initial_duration: u64,
        start_x: f64,
    },
    MovingNote {
        note_id: u64,
        initial_pitch: u8,
        initial_tick: u64,
        start_x: f64,
        start_y: f64,
    },
    EditingVelocity,
}

#[derive(Clone)]
pub struct PianoRollWidget {
    pub container: Box,
    pub canvas: DrawingArea,
    pub scrolled: ScrolledWindow,
    pub project: Rc<RefCell<Project>>,
    pub active_channel_id: Rc<RefCell<usize>>,
    pub ghost_notes_active: Rc<RefCell<bool>>,
    pub active_key_pressed: Rc<RefCell<Option<i32>>>,
    pub playhead_tick: Rc<RefCell<u64>>,
    pub ghost_toggle: ToggleButton,
    pub channel_dropdown: DropDown,
}

impl PianoRollWidget {
    pub fn new(
        project: Rc<RefCell<Project>>,
        tx: Sender<UiToAudioMsg>,
    ) -> Self {
        let container = Box::new(Orientation::Vertical, 2);
        container.add_css_class("piano-roll-container");

        let active_channel_id = Rc::new(RefCell::new(4)); // Padrão: 3x Osc Synth (canal 4)
        let ghost_notes_active = Rc::new(RefCell::new(true));
        let active_key_pressed = Rc::new(RefCell::new(None));
        let playhead_tick = Rc::new(RefCell::new(0));
        let drag_mode = Rc::new(RefCell::new(DragMode::None));

        // 1. Barra de Ferramentas Superior do Piano Roll
        let toolbar = Box::new(Orientation::Horizontal, 6);
        toolbar.add_css_class("piano-roll-toolbar");

        let title_label = Label::new(Some("🎹 Piano Roll"));
        title_label.add_css_class("lcd-bpm");
        toolbar.append(&title_label);

        // Seletor dinâmico de Canal / Instrumento
        let channel_names: Vec<String> = {
            let p = project.borrow();
            p.channels
                .iter()
                .map(|c| format!("{}. {}", c.id + 1, c.name))
                .collect()
        };
        let channel_strings: Vec<&str> = channel_names.iter().map(|s| s.as_str()).collect();
        let str_list = StringList::new(&channel_strings);
        let channel_dropdown = DropDown::new(Some(str_list), None::<gtk4::Expression>);
        channel_dropdown.add_css_class("piano-roll-dropdown");
        // Seleciona o 3x Osc Synth por padrão (índice 4 se existir, ou o último)
        let default_idx = if channel_names.len() > 4 { 4 } else { 0 };
        channel_dropdown.set_selected(default_idx as u32);
        toolbar.append(&channel_dropdown);

        // Ferramentas de Edição
        let p_btn = Button::with_label("✏ Draw [P]");
        p_btn.add_css_class("transport-btn");
        let d_btn = Button::with_label("🗑 Erase [D]");
        d_btn.add_css_class("transport-btn");

        let ghost_toggle = ToggleButton::with_label("👻 Ghost Notes [Alt+V]");
        ghost_toggle.add_css_class("window-toggle-btn");
        ghost_toggle.set_active(true);

        let quant_btn = Button::with_label("⚡ Quantize [Alt+Q]");
        quant_btn.add_css_class("transport-btn");

        let strum_btn = Button::with_label("🎸 Strum [Alt+S]");
        strum_btn.add_css_class("transport-btn");

        let clear_btn = Button::with_label("✖ Clear");
        clear_btn.add_css_class("transport-btn");

        toolbar.append(&p_btn);
        toolbar.append(&d_btn);
        toolbar.append(&ghost_toggle);
        toolbar.append(&quant_btn);
        toolbar.append(&strum_btn);
        toolbar.append(&clear_btn);

        container.append(&toolbar);

        // 2. Área de Rolagem com Dimensões Completas de DAW Profissional
        let scrolled = ScrolledWindow::new();
        scrolled.set_hexpand(true);
        scrolled.set_vexpand(true);
        scrolled.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic);

        let canvas = DrawingArea::new();
        let total_canvas_width = KEYBOARD_WIDTH + (TOTAL_STEPS as f64) * STEP_WIDTH;
        let total_canvas_height = RULER_HEIGHT + (TOTAL_KEYS as f64) * KEY_HEIGHT + VELOCITY_HEIGHT;

        canvas.set_content_width(total_canvas_width as i32);
        canvas.set_content_height(total_canvas_height as i32);

        scrolled.set_child(Some(&canvas));
        container.append(&scrolled);

        // Centraliza a visão inicial nas oitavas médias (C4..C6)
        let scrolled_clone = scrolled.clone();
        scrolled.connect_map(move |_| {
            let row_c5 = (MAX_PITCH - 60) as f64;
            let center_y = RULER_HEIGHT + row_c5 * KEY_HEIGHT - 220.0;
            scrolled_clone.vadjustment().set_value(center_y.max(0.0));
        });

        // 3. Conexões da Barra de Ferramentas
        let active_ch_drop = active_channel_id.clone();
        let canvas_drop = canvas.clone();
        channel_dropdown.connect_selected_notify(move |dd| {
            let selected = dd.selected() as usize;
            *active_ch_drop.borrow_mut() = selected;
            canvas_drop.queue_draw();
        });

        let ghost_ref = ghost_notes_active.clone();
        let canvas_ghost = canvas.clone();
        ghost_toggle.connect_toggled(move |btn| {
            *ghost_ref.borrow_mut() = btn.is_active();
            canvas_ghost.queue_draw();
        });

        // Quantize
        let proj_quant = project.clone();
        let ch_quant = active_channel_id.clone();
        let tx_quant = tx.clone();
        let canvas_quant = canvas.clone();
        quant_btn.connect_clicked(move |_| {
            let ch_id = *ch_quant.borrow();
            let mut p = proj_quant.borrow_mut();
            let pat_id = p.active_pattern;
            if let Some(pat) = p.patterns.iter_mut().find(|pat| pat.id == pat_id) {
                if let Some(notes) = pat.piano_roll_notes.get_mut(&ch_id) {
                    for note in notes.iter_mut() {
                        let step = (note.start_tick as f64 / 24.0).round() as u64;
                        note.start_tick = step * 24;
                        let dur_steps = (note.duration_ticks as f64 / 24.0).round().max(1.0) as u64;
                        note.duration_ticks = dur_steps * 24;
                    }
                    let notes_clone = notes.clone();
                    let _ = tx_quant.send(UiToAudioMsg::UpdatePianoRollNotes {
                        pattern_id: pat_id,
                        channel_id: ch_id,
                        notes: notes_clone,
                    });
                }
            }
            canvas_quant.queue_draw();
        });

        // Strum (efeito dedilhado característico do FL Studio)
        let proj_strum = project.clone();
        let ch_strum = active_channel_id.clone();
        let tx_strum = tx.clone();
        let canvas_strum = canvas.clone();
        strum_btn.connect_clicked(move |_| {
            let ch_id = *ch_strum.borrow();
            let mut p = proj_strum.borrow_mut();
            let pat_id = p.active_pattern;
            if let Some(pat) = p.patterns.iter_mut().find(|pat| pat.id == pat_id) {
                if let Some(notes) = pat.piano_roll_notes.get_mut(&ch_id) {
                    notes.sort_by_key(|n| n.pitch);
                    for (i, note) in notes.iter_mut().enumerate() {
                        note.start_tick += (i as u64) * 6; // Pequeno delay de 6 ticks por nota
                    }
                    let notes_clone = notes.clone();
                    let _ = tx_strum.send(UiToAudioMsg::UpdatePianoRollNotes {
                        pattern_id: pat_id,
                        channel_id: ch_id,
                        notes: notes_clone,
                    });
                }
            }
            canvas_strum.queue_draw();
        });

        // Clear
        let proj_clear = project.clone();
        let ch_clear = active_channel_id.clone();
        let tx_clear = tx.clone();
        let canvas_clear = canvas.clone();
        clear_btn.connect_clicked(move |_| {
            let ch_id = *ch_clear.borrow();
            let mut p = proj_clear.borrow_mut();
            let pat_id = p.active_pattern;
            if let Some(pat) = p.patterns.iter_mut().find(|pat| pat.id == pat_id) {
                if let Some(notes) = pat.piano_roll_notes.get_mut(&ch_id) {
                    notes.clear();
                }
                let _ = tx_clear.send(UiToAudioMsg::UpdatePianoRollNotes {
                    pattern_id: pat_id,
                    channel_id: ch_id,
                    notes: Vec::new(),
                });
            }
            canvas_clear.queue_draw();
        });

        // --- GESTOS DE MOUSE E INTERAÇÃO EM TEMPO REAL ---
        let gesture = GestureClick::new();
        gesture.set_button(0); // Escuta botão esquerdo, meio e direito

        let proj_click = project.clone();
        let ch_click = active_channel_id.clone();
        let key_click = active_key_pressed.clone();
        let drag_click = drag_mode.clone();
        let tx_click = tx.clone();
        let canvas_click = canvas.clone();
        let playhead_click = playhead_tick.clone();

        gesture.connect_pressed(move |g, _n_press, x, y| {
            let button = g.current_button();
            let ch_id = *ch_click.borrow();
            let keys_bottom = RULER_HEIGHT + (TOTAL_KEYS as f64) * KEY_HEIGHT;

            if y < RULER_HEIGHT && x >= KEYBOARD_WIDTH {
                // 1. CLIQUE NA RÉGUA SUPERIOR: Posiciona o Playhead (Seek)
                if button == 1 {
                    let step = ((x - KEYBOARD_WIDTH) / STEP_WIDTH).max(0.0) as u64;
                    let tick = step * 24;
                    *playhead_click.borrow_mut() = tick;
                    let _ = tx_click.send(UiToAudioMsg::SeekTick(tick));
                    *drag_click.borrow_mut() = DragMode::Ruler;
                    canvas_click.queue_draw();
                }
            } else if x < KEYBOARD_WIDTH && y >= RULER_HEIGHT && y < keys_bottom {
                // 2. CLIQUE NO TECLADO DO PIANO (Coluna Esquerda com Audição Imediata)
                let row = ((y - RULER_HEIGHT) / KEY_HEIGHT) as i32;
                let pitch = (MAX_PITCH - row).clamp(MIN_PITCH, MAX_PITCH) as u8;

                *key_click.borrow_mut() = Some(pitch as i32);
                *drag_click.borrow_mut() = DragMode::Keyboard { pitch };

                let _ = tx_click.send(UiToAudioMsg::TriggerNoteOn {
                    channel_id: ch_id,
                    pitch,
                    velocity: 0.85,
                });
                canvas_click.queue_draw();
            } else if y >= keys_bottom && x >= KEYBOARD_WIDTH {
                // 3. CLIQUE NA PISTA DE VELOCIDADE (Bottom Velocity Lane)
                if button == 1 {
                    *drag_click.borrow_mut() = DragMode::EditingVelocity;
                    let mut p = proj_click.borrow_mut();
                    let pat_id = p.active_pattern;
                    if let Some(pat) = p.patterns.iter_mut().find(|pat| pat.id == pat_id) {
                        if let Some(notes) = pat.piano_roll_notes.get_mut(&ch_id) {
                            let click_step = ((x - KEYBOARD_WIDTH) / STEP_WIDTH).max(0.0) as u64;
                            let click_tick = click_step * 24;

                            // Localiza a nota mais próxima do ponto clicado
                            if let Some(note) = notes.iter_mut().find(|n| {
                                click_tick >= n.start_tick && click_tick < (n.start_tick + n.duration_ticks)
                            }) {
                                let vel = ((total_canvas_height - 6.0 - y) / (VELOCITY_HEIGHT - 12.0))
                                    .clamp(0.05, 1.0) as f32;
                                note.velocity = vel;

                                let notes_clone = notes.clone();
                                let _ = tx_click.send(UiToAudioMsg::UpdatePianoRollNotes {
                                    pattern_id: pat_id,
                                    channel_id: ch_id,
                                    notes: notes_clone,
                                });
                            }
                        }
                    }
                    canvas_click.queue_draw();
                }
            } else if x >= KEYBOARD_WIDTH && y >= RULER_HEIGHT && y < keys_bottom {
                // 4. CLIQUE NA GRADE DE NOTAS (Edição Principal)
                let row = ((y - RULER_HEIGHT) / KEY_HEIGHT) as i32;
                let pitch = (MAX_PITCH - row).clamp(MIN_PITCH, MAX_PITCH) as u8;
                let step = ((x - KEYBOARD_WIDTH) / STEP_WIDTH).max(0.0) as u64;
                let tick = step * 24;

                let mut p = proj_click.borrow_mut();
                let pat_id = p.active_pattern;

                if let Some(pat) = p.patterns.iter_mut().find(|pat| pat.id == pat_id) {
                    let notes = pat.piano_roll_notes.entry(ch_id).or_insert_with(Vec::new);

                    // Verifica se o clique atingiu alguma nota existente
                    let mut hit_idx = None;
                    let mut on_right_edge = false;

                    for (i, note) in notes.iter().enumerate() {
                        let n_row = (MAX_PITCH - note.pitch as i32) as f64;
                        let n_y = RULER_HEIGHT + n_row * KEY_HEIGHT;
                        let n_x = KEYBOARD_WIDTH + (note.start_tick as f64 / 24.0) * STEP_WIDTH;
                        let n_w = ((note.duration_ticks as f64 / 24.0) * STEP_WIDTH).max(6.0);

                        if x >= n_x && x <= n_x + n_w && y >= n_y && y <= n_y + KEY_HEIGHT {
                            hit_idx = Some(i);
                            if x >= n_x + n_w - 8.0 {
                                on_right_edge = true;
                            }
                            break;
                        }
                    }

                    if button == 1 {
                        if let Some(idx) = hit_idx {
                            let note = &notes[idx];
                            if on_right_edge {
                                // Redimensionar duração da nota arrastando a borda direita
                                *drag_click.borrow_mut() = DragMode::ResizingNote {
                                    note_id: note.id,
                                    initial_duration: note.duration_ticks,
                                    start_x: x,
                                };
                            } else {
                                // Mover / transpor a nota
                                *drag_click.borrow_mut() = DragMode::MovingNote {
                                    note_id: note.id,
                                    initial_pitch: note.pitch,
                                    initial_tick: note.start_tick,
                                    start_x: x,
                                    start_y: y,
                                };
                                // Audição imediata ao tocar na nota existente
                                let _ = tx_click.send(UiToAudioMsg::TriggerNoteOn {
                                    channel_id: ch_id,
                                    pitch: note.pitch,
                                    velocity: note.velocity,
                                });
                            }
                        } else {
                            // Criar nova nota com duração padrão e começar a arrastar
                            let new_id = (notes.len() as u64) + 1000;
                            let new_note = Note {
                                id: new_id,
                                pitch,
                                velocity: 0.82,
                                start_tick: tick,
                                duration_ticks: 24, // Começa com 1 passo (24 ticks)
                                pan: 0.0,
                                release: 0.5,
                                color_index: 0,
                                muted: false,
                            };
                            notes.push(new_note);

                            // Audição imediata em tempo real!
                            let _ = tx_click.send(UiToAudioMsg::TriggerNoteOn {
                                channel_id: ch_id,
                                pitch,
                                velocity: 0.82,
                            });

                            *drag_click.borrow_mut() = DragMode::DrawingNote {
                                note_id: new_id,
                                pitch,
                                start_tick: tick,
                            };

                            let notes_clone = notes.clone();
                            let _ = tx_click.send(UiToAudioMsg::UpdatePianoRollNotes {
                                pattern_id: pat_id,
                                channel_id: ch_id,
                                notes: notes_clone,
                            });
                        }
                    } else if button == 3 {
                        // BOTÃO DIREITO: Borracha instantânea (estilo FL Studio)
                        if let Some(idx) = hit_idx {
                            notes.remove(idx);
                            let notes_clone = notes.clone();
                            let _ = tx_click.send(UiToAudioMsg::UpdatePianoRollNotes {
                                pattern_id: pat_id,
                                channel_id: ch_id,
                                notes: notes_clone,
                            });
                        }
                    }
                }
                canvas_click.queue_draw();
            }
        });

        // Liberação do Mouse (Finaliza Operações e Silencia Teclas)
        let proj_release = project.clone();
        let ch_release = active_channel_id.clone();
        let key_release = active_key_pressed.clone();
        let drag_release = drag_mode.clone();
        let tx_release = tx.clone();
        let canvas_release = canvas.clone();

        gesture.connect_released(move |_g, _n_press, _x, _y| {
            let ch_id = *ch_release.borrow();
            let current_mode = *drag_release.borrow();

            match current_mode {
                DragMode::Keyboard { pitch } => {
                    let _ = tx_release.send(UiToAudioMsg::TriggerNoteOff {
                        channel_id: ch_id,
                        pitch,
                    });
                    *key_release.borrow_mut() = None;
                }
                DragMode::DrawingNote { pitch, .. } => {
                    let _ = tx_release.send(UiToAudioMsg::TriggerNoteOff {
                        channel_id: ch_id,
                        pitch,
                    });
                    let p = proj_release.borrow();
                    let pat_id = p.active_pattern;
                    if let Some(pat) = p.patterns.iter().find(|pat| pat.id == pat_id) {
                        if let Some(notes) = pat.piano_roll_notes.get(&ch_id) {
                            let _ = tx_release.send(UiToAudioMsg::UpdatePianoRollNotes {
                                pattern_id: pat_id,
                                channel_id: ch_id,
                                notes: notes.clone(),
                            });
                        }
                    }
                }
                DragMode::MovingNote { initial_pitch, .. } => {
                    let _ = tx_release.send(UiToAudioMsg::TriggerNoteOff {
                        channel_id: ch_id,
                        pitch: initial_pitch,
                    });
                    let p = proj_release.borrow();
                    let pat_id = p.active_pattern;
                    if let Some(pat) = p.patterns.iter().find(|pat| pat.id == pat_id) {
                        if let Some(notes) = pat.piano_roll_notes.get(&ch_id) {
                            let _ = tx_release.send(UiToAudioMsg::UpdatePianoRollNotes {
                                pattern_id: pat_id,
                                channel_id: ch_id,
                                notes: notes.clone(),
                            });
                        }
                    }
                }
                DragMode::ResizingNote { .. } | DragMode::EditingVelocity => {
                    let p = proj_release.borrow();
                    let pat_id = p.active_pattern;
                    if let Some(pat) = p.patterns.iter().find(|pat| pat.id == pat_id) {
                        if let Some(notes) = pat.piano_roll_notes.get(&ch_id) {
                            let _ = tx_release.send(UiToAudioMsg::UpdatePianoRollNotes {
                                pattern_id: pat_id,
                                channel_id: ch_id,
                                notes: notes.clone(),
                            });
                        }
                    }
                }
                _ => {}
            }

            *drag_release.borrow_mut() = DragMode::None;
            canvas_release.queue_draw();
        });

        canvas.add_controller(gesture);

        // Movimento do Mouse (Glissando no teclado, arrastar para esticar notas, etc.)
        let motion = EventControllerMotion::new();
        let proj_motion = project.clone();
        let ch_motion = active_channel_id.clone();
        let key_motion = active_key_pressed.clone();
        let drag_motion = drag_mode.clone();
        let tx_motion = tx.clone();
        let canvas_motion = canvas.clone();
        let playhead_motion = playhead_tick.clone();

        motion.connect_motion(move |_m, x, y| {
            let ch_id = *ch_motion.borrow();
            let mode = *drag_motion.borrow();

            match mode {
                DragMode::Ruler => {
                    let step = ((x - KEYBOARD_WIDTH) / STEP_WIDTH).max(0.0) as u64;
                    let tick = step * 24;
                    *playhead_motion.borrow_mut() = tick;
                    let _ = tx_motion.send(UiToAudioMsg::SeekTick(tick));
                    canvas_motion.queue_draw();
                }
                DragMode::Keyboard { pitch: prev_pitch } => {
                    let row = ((y - RULER_HEIGHT) / KEY_HEIGHT) as i32;
                    let new_pitch = (MAX_PITCH - row).clamp(MIN_PITCH, MAX_PITCH) as u8;
                    if new_pitch != prev_pitch {
                        let _ = tx_motion.send(UiToAudioMsg::TriggerNoteOff {
                            channel_id: ch_id,
                            pitch: prev_pitch,
                        });
                        let _ = tx_motion.send(UiToAudioMsg::TriggerNoteOn {
                            channel_id: ch_id,
                            pitch: new_pitch,
                            velocity: 0.85,
                        });
                        *key_motion.borrow_mut() = Some(new_pitch as i32);
                        *drag_motion.borrow_mut() = DragMode::Keyboard { pitch: new_pitch };
                        canvas_motion.queue_draw();
                    }
                }
                DragMode::DrawingNote {
                    note_id,
                    pitch: _,
                    start_tick,
                } => {
                    let cur_step = ((x - KEYBOARD_WIDTH) / STEP_WIDTH).max(0.0) as u64;
                    let cur_tick = cur_step * 24;
                    let new_dur = if cur_tick > start_tick {
                        (cur_tick - start_tick + 24).max(24)
                    } else {
                        24
                    };

                    let mut p = proj_motion.borrow_mut();
                    let pat_id = p.active_pattern;
                    if let Some(pat) = p.patterns.iter_mut().find(|pat| pat.id == pat_id) {
                        if let Some(notes) = pat.piano_roll_notes.get_mut(&ch_id) {
                            if let Some(n) = notes.iter_mut().find(|n| n.id == note_id) {
                                n.duration_ticks = new_dur;
                            }
                        }
                    }
                    canvas_motion.queue_draw();
                }
                DragMode::ResizingNote {
                    note_id,
                    initial_duration,
                    start_x,
                } => {
                    let delta_px = x - start_x;
                    let delta_ticks = ((delta_px / STEP_WIDTH) * 24.0).round() as i64;
                    let new_dur = (initial_duration as i64 + delta_ticks).max(24) as u64;

                    let mut p = proj_motion.borrow_mut();
                    let pat_id = p.active_pattern;
                    if let Some(pat) = p.patterns.iter_mut().find(|pat| pat.id == pat_id) {
                        if let Some(notes) = pat.piano_roll_notes.get_mut(&ch_id) {
                            if let Some(n) = notes.iter_mut().find(|n| n.id == note_id) {
                                n.duration_ticks = new_dur;
                            }
                        }
                    }
                    canvas_motion.queue_draw();
                }
                DragMode::MovingNote {
                    note_id,
                    initial_pitch,
                    initial_tick,
                    start_x,
                    start_y,
                } => {
                    let delta_x = x - start_x;
                    let delta_y = y - start_y;
                    let delta_step = (delta_x / STEP_WIDTH).round() as i64;
                    let delta_row = (delta_y / KEY_HEIGHT).round() as i32;

                    let new_tick = (initial_tick as i64 + delta_step * 24).max(0) as u64;
                    let new_pitch = (initial_pitch as i32 - delta_row).clamp(MIN_PITCH, MAX_PITCH) as u8;

                    let mut p = proj_motion.borrow_mut();
                    let pat_id = p.active_pattern;
                    if let Some(pat) = p.patterns.iter_mut().find(|pat| pat.id == pat_id) {
                        if let Some(notes) = pat.piano_roll_notes.get_mut(&ch_id) {
                            if let Some(n) = notes.iter_mut().find(|n| n.id == note_id) {
                                n.start_tick = new_tick;
                                n.pitch = new_pitch;
                            }
                        }
                    }
                    canvas_motion.queue_draw();
                }
                DragMode::EditingVelocity => {
                    let vel = ((total_canvas_height - 6.0 - y) / (VELOCITY_HEIGHT - 12.0))
                        .clamp(0.05, 1.0) as f32;
                    let click_step = ((x - KEYBOARD_WIDTH) / STEP_WIDTH).max(0.0) as u64;
                    let click_tick = click_step * 24;

                    let mut p = proj_motion.borrow_mut();
                    let pat_id = p.active_pattern;
                    if let Some(pat) = p.patterns.iter_mut().find(|pat| pat.id == pat_id) {
                        if let Some(notes) = pat.piano_roll_notes.get_mut(&ch_id) {
                            if let Some(note) = notes.iter_mut().find(|n| {
                                click_tick >= n.start_tick && click_tick < (n.start_tick + n.duration_ticks)
                            }) {
                                note.velocity = vel;
                            }
                        }
                    }
                    canvas_motion.queue_draw();
                }
                DragMode::None => {}
            }
        });

        canvas.add_controller(motion);

        // --- RENDERIZADOR CAIRO DE ALTA FIDELIDADE ESTILO FL STUDIO ---
        let proj_draw = project.clone();
        let ch_draw = active_channel_id.clone();
        let ghost_draw = ghost_notes_active.clone();
        let active_key_draw = active_key_pressed.clone();
        let playhead_draw = playhead_tick.clone();

        canvas.set_draw_func(move |_area, cr, width, height| {
            let active_ch = *ch_draw.borrow();
            let p = proj_draw.borrow();
            let pat_id = p.active_pattern;
            let current_pat = p.patterns.iter().find(|pat| pat.id == pat_id);

            let keys_bottom = RULER_HEIGHT + (TOTAL_KEYS as f64) * KEY_HEIGHT;

            // 1. Fundo Geral da Grade
            cr.set_source_rgb(0.12, 0.14, 0.17);
            let _ = cr.paint();

            // 2. Linhas Horizontais da Grade (Teclas Pretas são mais escuras)
            for i in 0..TOTAL_KEYS {
                let pitch = MAX_PITCH - i;
                let y = RULER_HEIGHT + (i as f64) * KEY_HEIGHT;

                let is_black = matches!(pitch.rem_euclid(12), 1 | 3 | 6 | 8 | 10);

                if is_black {
                    cr.set_source_rgb(0.09, 0.10, 0.13);
                    cr.rectangle(KEYBOARD_WIDTH, y, width as f64 - KEYBOARD_WIDTH, KEY_HEIGHT);
                    let _ = cr.fill();
                } else {
                    cr.set_source_rgb(0.13, 0.15, 0.18);
                    cr.rectangle(KEYBOARD_WIDTH, y, width as f64 - KEYBOARD_WIDTH, KEY_HEIGHT);
                    let _ = cr.fill();
                }

                // Divisória sutil entre semitons
                cr.set_source_rgba(0.20, 0.23, 0.28, 0.45);
                cr.set_line_width(0.6);
                cr.move_to(KEYBOARD_WIDTH, y);
                cr.line_to(width as f64, y);
                let _ = cr.stroke();
            }

            // 3. Linhas Verticais de Compassos e Batidas
            let num_steps = ((width as f64 - KEYBOARD_WIDTH) / STEP_WIDTH).ceil() as usize;
            for s in 0..num_steps {
                let x = KEYBOARD_WIDTH + (s as f64) * STEP_WIDTH;
                if s % 16 == 0 {
                    // Início do Compasso (Linha principal bem destacada)
                    cr.set_source_rgba(0.42, 0.48, 0.58, 0.85);
                    cr.set_line_width(1.5);
                } else if s % 4 == 0 {
                    // Batida (1/4 de compasso)
                    cr.set_source_rgba(0.28, 0.33, 0.40, 0.70);
                    cr.set_line_width(1.0);
                } else {
                    // Subdivisão de 1/16
                    cr.set_source_rgba(0.18, 0.21, 0.25, 0.40);
                    cr.set_line_width(0.5);
                }

                cr.move_to(x, RULER_HEIGHT);
                cr.line_to(x, keys_bottom);
                let _ = cr.stroke();
            }

            // 4. Ghost Notes (Notas translúcidas de outros canais do mesmo pattern)
            if *ghost_draw.borrow() {
                if let Some(pat) = current_pat {
                    for (&other_ch_id, notes) in &pat.piano_roll_notes {
                        if other_ch_id == active_ch {
                            continue;
                        }
                        for note in notes {
                            let row = (MAX_PITCH - note.pitch as i32) as f64;
                            let y = RULER_HEIGHT + row * KEY_HEIGHT + 1.5;
                            let x = KEYBOARD_WIDTH + (note.start_tick as f64 / 24.0) * STEP_WIDTH;
                            let w = ((note.duration_ticks as f64 / 24.0) * STEP_WIDTH - 1.0).max(4.0);

                            cr.set_source_rgba(0.65, 0.72, 0.80, 0.22);
                            cr.rectangle(x, y, w, KEY_HEIGHT - 3.0);
                            let _ = cr.fill();

                            cr.set_source_rgba(0.75, 0.82, 0.90, 0.35);
                            cr.set_line_width(0.8);
                            cr.rectangle(x, y, w, KEY_HEIGHT - 3.0);
                            let _ = cr.stroke();
                        }
                    }
                }
            }

            // 5. Notas Ativas do Canal Atual
            if let Some(pat) = current_pat {
                if let Some(notes) = pat.piano_roll_notes.get(&active_ch) {
                    for note in notes {
                        let row = (MAX_PITCH - note.pitch as i32) as f64;
                        let y = RULER_HEIGHT + row * KEY_HEIGHT + 1.5;
                        let x = KEYBOARD_WIDTH + (note.start_tick as f64 / 24.0) * STEP_WIDTH;
                        let w = ((note.duration_ticks as f64 / 24.0) * STEP_WIDTH - 1.0).max(6.0);
                        let h = KEY_HEIGHT - 3.0;

                        // Gradiente de preenchimento Verde Neon característico do FL Studio
                        cr.set_source_rgb(0.18, 0.82, 0.44);
                        cr.rectangle(x, y, w, h);
                        let _ = cr.fill();

                        // Borda em relevo 3D
                        cr.set_source_rgb(0.40, 0.98, 0.62);
                        cr.set_line_width(1.0);
                        cr.rectangle(x, y, w, h);
                        let _ = cr.stroke();

                        // Marcador de puxador de redimensionamento na borda direita
                        cr.set_source_rgba(0.0, 0.0, 0.0, 0.30);
                        cr.rectangle(x + w - 4.0, y, 3.0, h);
                        let _ = cr.fill();

                        // Rótulo da Nota no bloco (ex: "C5", "D#5") se houver espaço
                        if w >= 28.0 {
                            let note_names = [
                                "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
                            ];
                            let note_name = note_names[(note.pitch % 12) as usize];
                            let octave = (note.pitch / 12) as i32 - 1;
                            let label = format!("{}{}", note_name, octave);

                            cr.set_source_rgb(0.05, 0.15, 0.08);
                            cr.set_font_size(9.0);
                            cr.move_to(x + 4.0, y + h - 3.5);
                            let _ = cr.show_text(&label);
                        }
                    }
                }
            }

            // 6. TECLADO VERTICAL DO PIANO (Coluna da Esquerda com Relevo 3D Marfim e Ébano)
            cr.set_source_rgb(0.11, 0.13, 0.16);
            cr.rectangle(0.0, RULER_HEIGHT, KEYBOARD_WIDTH, keys_bottom - RULER_HEIGHT);
            let _ = cr.fill();

            let pressed_pitch = *active_key_draw.borrow();

            for i in 0..TOTAL_KEYS {
                let pitch = MAX_PITCH - i;
                let y = RULER_HEIGHT + (i as f64) * KEY_HEIGHT;

                let is_black = matches!(pitch.rem_euclid(12), 1 | 3 | 6 | 8 | 10);

                let is_pressed = pressed_pitch == Some(pitch);

                if is_pressed {
                    // Tecla pressionada iluminada em Azul Celeste vibrante
                    cr.set_source_rgb(0.20, 0.65, 1.0);
                    cr.rectangle(0.0, y, KEYBOARD_WIDTH, KEY_HEIGHT);
                    let _ = cr.fill();
                } else if is_black {
                    // Tecla preta (Ébano) saliente
                    cr.set_source_rgb(0.07, 0.08, 0.10);
                    cr.rectangle(0.0, y + 2.0, KEYBOARD_WIDTH * 0.62, KEY_HEIGHT - 4.0);
                    let _ = cr.fill();

                    cr.set_source_rgb(0.22, 0.25, 0.30);
                    cr.set_line_width(0.8);
                    cr.rectangle(0.0, y + 2.0, KEYBOARD_WIDTH * 0.62, KEY_HEIGHT - 4.0);
                    let _ = cr.stroke();
                } else {
                    // Tecla branca (Marfim) chanfrada
                    cr.set_source_rgb(0.85, 0.88, 0.92);
                    cr.rectangle(0.0, y + 0.5, KEYBOARD_WIDTH - 1.0, KEY_HEIGHT - 1.0);
                    let _ = cr.fill();

                    cr.set_source_rgb(0.40, 0.44, 0.50);
                    cr.set_line_width(0.8);
                    cr.rectangle(0.0, y + 0.5, KEYBOARD_WIDTH - 1.0, KEY_HEIGHT - 1.0);
                    let _ = cr.stroke();

                    // Rótulo clássico nas teclas C (C1, C2, C3, C4, C5, C6, C7)
                    if pitch.rem_euclid(12) == 0 {
                        let octave = (pitch / 12) - 1;
                        cr.set_source_rgb(0.20, 0.24, 0.30);
                        cr.set_font_size(9.5);
                        cr.move_to(KEYBOARD_WIDTH - 22.0, y + KEY_HEIGHT - 4.5);
                        let _ = cr.show_text(&format!("C{}", octave));
                    }
                }
            }

            // Sombra da coluna do teclado projetada sobre a grade
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.45);
            cr.rectangle(KEYBOARD_WIDTH, RULER_HEIGHT, 3.5, keys_bottom - RULER_HEIGHT);
            let _ = cr.fill();

            // 7. PISTA INFERIOR DE VELOCIDADE (Velocity Lane)
            cr.set_source_rgb(0.09, 0.10, 0.13);
            cr.rectangle(0.0, keys_bottom, width as f64, VELOCITY_HEIGHT);
            let _ = cr.fill();

            // Barra divisória superior
            cr.set_source_rgb(0.24, 0.28, 0.35);
            cr.set_line_width(1.2);
            cr.move_to(0.0, keys_bottom);
            cr.line_to(width as f64, keys_bottom);
            let _ = cr.stroke();

            // Rótulo da pista
            cr.set_source_rgba(0.6, 0.7, 0.8, 0.6);
            cr.set_font_size(10.0);
            cr.move_to(8.0, keys_bottom + 16.0);
            let _ = cr.show_text("Control: Velocity");

            // Hastes de velocidade para as notas ativas
            if let Some(pat) = current_pat {
                if let Some(notes) = pat.piano_roll_notes.get(&active_ch) {
                    for note in notes {
                        let x = KEYBOARD_WIDTH + (note.start_tick as f64 / 24.0) * STEP_WIDTH + 3.0;
                        let stem_h = (note.velocity as f64) * (VELOCITY_HEIGHT - 16.0);
                        let stem_top = (keys_bottom + VELOCITY_HEIGHT - 6.0) - stem_h;

                        // Haste vertical
                        cr.set_source_rgb(0.18, 0.82, 0.44);
                        cr.set_line_width(1.8);
                        cr.move_to(x, keys_bottom + VELOCITY_HEIGHT - 6.0);
                        cr.line_to(x, stem_top);
                        let _ = cr.stroke();

                        // Cabeça circular no topo da haste
                        cr.arc(x, stem_top, 2.5, 0.0, std::f64::consts::PI * 2.0);
                        cr.set_source_rgb(0.40, 0.98, 0.62);
                        let _ = cr.fill();
                    }
                }
            }

            // 8. RÉGUA SUPERIOR DE COMPASSOS (Timeline Ruler com Marcadores de Compassos)
            cr.set_source_rgb(0.13, 0.15, 0.18);
            cr.rectangle(0.0, 0.0, width as f64, RULER_HEIGHT);
            let _ = cr.fill();

            cr.set_source_rgb(0.25, 0.29, 0.35);
            cr.set_line_width(1.0);
            cr.move_to(0.0, RULER_HEIGHT);
            cr.line_to(width as f64, RULER_HEIGHT);
            let _ = cr.stroke();

            // Canto superior esquerdo do teclado
            cr.set_source_rgb(0.10, 0.12, 0.15);
            cr.rectangle(0.0, 0.0, KEYBOARD_WIDTH, RULER_HEIGHT);
            let _ = cr.fill();

            for s in 0..num_steps {
                let x = KEYBOARD_WIDTH + (s as f64) * STEP_WIDTH;
                if s % 16 == 0 {
                    let bar_num = (s / 16) + 1;
                    cr.set_source_rgb(0.85, 0.90, 0.95);
                    cr.set_font_size(10.0);
                    cr.move_to(x + 4.0, RULER_HEIGHT - 7.0);
                    let _ = cr.show_text(&format!("{}", bar_num));

                    cr.set_source_rgb(0.45, 0.50, 0.60);
                    cr.set_line_width(1.2);
                    cr.move_to(x, RULER_HEIGHT - 12.0);
                    cr.line_to(x, RULER_HEIGHT);
                    let _ = cr.stroke();
                } else if s % 4 == 0 {
                    cr.set_source_rgba(0.35, 0.40, 0.50, 0.6);
                    cr.set_line_width(0.8);
                    cr.move_to(x, RULER_HEIGHT - 6.0);
                    cr.line_to(x, RULER_HEIGHT);
                    let _ = cr.stroke();
                }
            }

            // 9. PLAYHEAD ANIMADO EM TEMPO REAL (Linha Verde Neon com Ponteiro Superior)
            let cur_tick = *playhead_draw.borrow();
            let pat_len_ticks = (current_pat.map(|p| p.length_steps).unwrap_or(16) as u64) * TICKS_PER_STEP;
            let rel_tick = if pat_len_ticks > 0 { cur_tick % pat_len_ticks } else { cur_tick };
            let playhead_x = KEYBOARD_WIDTH + (rel_tick as f64 / 24.0) * STEP_WIDTH;

            if playhead_x >= KEYBOARD_WIDTH && playhead_x <= width as f64 {
                // Linha vertical do playhead correndo por toda a extensão
                cr.set_source_rgba(0.18, 0.90, 0.45, 0.9);
                cr.set_line_width(1.8);
                cr.move_to(playhead_x, RULER_HEIGHT);
                cr.line_to(playhead_x, height as f64);
                let _ = cr.stroke();

                // Ponteiro triangular na régua superior
                cr.set_source_rgb(0.20, 0.95, 0.50);
                cr.move_to(playhead_x - 5.0, 2.0);
                cr.line_to(playhead_x + 5.0, 2.0);
                cr.line_to(playhead_x, RULER_HEIGHT - 2.0);
                cr.close_path();
                let _ = cr.fill();
            }
        });

        Self {
            container,
            canvas,
            scrolled,
            project,
            active_channel_id,
            ghost_notes_active,
            active_key_pressed,
            playhead_tick,
            ghost_toggle,
            channel_dropdown,
        }
    }

    /// Atualiza o playhead do Piano Roll sincronizado em tempo real (60 FPS)
    pub fn update_playhead(&self, tick: u64) {
        *self.playhead_tick.borrow_mut() = tick;
        self.canvas.queue_draw();
    }

    /// Alterna a visualização de Ghost Notes [Alt+V]
    pub fn toggle_ghost_notes(&self) {
        let active = !*self.ghost_notes_active.borrow();
        *self.ghost_notes_active.borrow_mut() = active;
        self.ghost_toggle.set_active(active);
        self.canvas.queue_draw();
    }

    /// Define o canal alvo para edição no Piano Roll
    pub fn set_active_channel(&self, channel_id: usize) {
        *self.active_channel_id.borrow_mut() = channel_id;
        self.channel_dropdown.set_selected(channel_id as u32);
        self.canvas.queue_draw();
    }
}
