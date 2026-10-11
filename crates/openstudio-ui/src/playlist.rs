use gtk4::prelude::*;
use gtk4::{
    Box, Button, DrawingArea, DropDown, EventControllerMotion, GestureClick, Label,
    Orientation, ScrolledWindow, StringList,
};
use openstudio_core::{ClipPayload, PlaylistClip, Project, UiToAudioMsg};
use crossbeam_channel::Sender;
use std::cell::RefCell;
use std::rc::Rc;

pub const TOTAL_TRACKS: usize = 24;
pub const TOTAL_BARS: usize = 64;
pub const RULER_HEIGHT: f64 = 24.0;
pub const TRACK_HEIGHT: f64 = 30.0;
pub const HEADER_WIDTH: f64 = 90.0;
pub const BAR_WIDTH: f64 = 96.0;
pub const TICKS_PER_BAR: u64 = 384; // 16 passos * 24 ticks

#[derive(Debug, Clone, Copy, PartialEq)]
enum DragMode {
    None,
    Ruler,
    DrawingClip {
        clip_id: u64,
        track_index: usize,
        start_tick: u64,
    },
    ResizingClip {
        clip_id: u64,
        initial_length: u64,
        start_x: f64,
    },
    MovingClip {
        clip_id: u64,
        initial_track: usize,
        initial_tick: u64,
        start_x: f64,
        start_y: f64,
    },
}

#[derive(Clone)]
pub struct PlaylistWidget {
    pub container: Box,
    pub canvas: DrawingArea,
    pub scrolled: ScrolledWindow,
    pub project: Rc<RefCell<Project>>,
    pub playhead_tick: Rc<RefCell<u64>>,
    pub track_mutes: Rc<RefCell<Vec<bool>>>,
}

impl PlaylistWidget {
    pub fn new(
        project: Rc<RefCell<Project>>,
        tx: Sender<UiToAudioMsg>,
    ) -> Self {
        let container = Box::new(Orientation::Vertical, 2);
        container.add_css_class("channel-rack-box");

        let playhead_tick = Rc::new(RefCell::new(0));
        let track_mutes = Rc::new(RefCell::new(vec![false; TOTAL_TRACKS]));
        let drag_mode = Rc::new(RefCell::new(DragMode::None));

        // 1. Barra de Ferramentas Superior da Playlist
        let toolbar = Box::new(Orientation::Horizontal, 6);
        toolbar.add_css_class("piano-roll-toolbar");

        let title = Label::new(Some("🎼 Playlist - Arrangement [F5]"));
        title.add_css_class("lcd-bpm");
        toolbar.append(&title);

        // Seletor de Pattern para carimbar
        let pat_str_list = StringList::new(&["Pattern 1", "Pattern 2", "Pattern 3"]);
        let pat_dropdown = DropDown::new(Some(pat_str_list), None::<gtk4::Expression>);
        pat_dropdown.add_css_class("piano-roll-dropdown");
        toolbar.append(&pat_dropdown);

        // Ferramentas
        let p_btn = Button::with_label("✏ Draw [P]");
        p_btn.add_css_class("transport-btn");
        let b_btn = Button::with_label("🖌 Paint [B]");
        b_btn.add_css_class("transport-btn");
        let d_btn = Button::with_label("🗑 Erase [D]");
        d_btn.add_css_class("transport-btn");
        let c_btn = Button::with_label("✂ Slice [C]");
        c_btn.add_css_class("transport-btn");
        let clear_btn = Button::with_label("✖ Clear All");
        clear_btn.add_css_class("transport-btn");

        toolbar.append(&p_btn);
        toolbar.append(&b_btn);
        toolbar.append(&d_btn);
        toolbar.append(&c_btn);
        toolbar.append(&clear_btn);

        container.append(&toolbar);

        // 2. Área de Rolagem Completa da Playlist
        let scrolled = ScrolledWindow::new();
        scrolled.set_hexpand(true);
        scrolled.set_vexpand(true);
        scrolled.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic);

        let canvas = DrawingArea::new();
        let total_w = HEADER_WIDTH + (TOTAL_BARS as f64) * BAR_WIDTH;
        let total_h = RULER_HEIGHT + (TOTAL_TRACKS as f64) * TRACK_HEIGHT;

        canvas.set_content_width(total_w as i32);
        canvas.set_content_height(total_h as i32);

        scrolled.set_child(Some(&canvas));
        container.append(&scrolled);

        // Ação de Limpar Todos os Clipes
        let proj_clear = project.clone();
        let tx_clear = tx.clone();
        let canvas_clear = canvas.clone();
        clear_btn.connect_clicked(move |_| {
            let mut p = proj_clear.borrow_mut();
            p.playlist_clips.clear();
            let _ = tx_clear.send(UiToAudioMsg::UpdatePlaylistClips(Vec::new()));
            canvas_clear.queue_draw();
        });

        // 3. Gestos de Mouse Interativos na Playlist
        let gesture = GestureClick::new();
        gesture.set_button(0); // Escuta botão esquerdo (1) e direito (3)

        let proj_click = project.clone();
        let drag_click = drag_mode.clone();
        let mutes_click = track_mutes.clone();
        let tx_click = tx.clone();
        let canvas_click = canvas.clone();
        let playhead_click = playhead_tick.clone();

        gesture.connect_pressed(move |g, _n_press, x, y| {
            let button = g.current_button();

            if y < RULER_HEIGHT && x >= HEADER_WIDTH {
                // CLIQUE NA RÉGUA SUPERIOR: Posiciona o Playhead (Seek)
                if button == 1 {
                    let bar_f = ((x - HEADER_WIDTH) / BAR_WIDTH).max(0.0);
                    let tick = (bar_f * (TICKS_PER_BAR as f64)) as u64;
                    *playhead_click.borrow_mut() = tick;
                    let _ = tx_click.send(UiToAudioMsg::SeekTick(tick));
                    *drag_click.borrow_mut() = DragMode::Ruler;
                    canvas_click.queue_draw();
                }
            } else if x < HEADER_WIDTH && y >= RULER_HEIGHT {
                // CLIQUE NO CABEÇALHO DA FAIXA: Muta / Desmuta a Track
                let trk_idx = ((y - RULER_HEIGHT) / TRACK_HEIGHT) as usize;
                if trk_idx < TOTAL_TRACKS {
                    let mut mutes = mutes_click.borrow_mut();
                    mutes[trk_idx] = !mutes[trk_idx];
                    canvas_click.queue_draw();
                }
            } else if x >= HEADER_WIDTH && y >= RULER_HEIGHT {
                // CLIQUE NA GRADE DE ARRANJO DA PLAYLIST
                let trk_idx = ((y - RULER_HEIGHT) / TRACK_HEIGHT) as usize;
                let bar_idx = ((x - HEADER_WIDTH) / BAR_WIDTH).max(0.0) as u64;
                let snapped_tick = bar_idx * TICKS_PER_BAR;

                let mut p = proj_click.borrow_mut();

                // Procura se o clique atingiu algum clipe existente
                let mut hit_idx = None;
                let mut on_right_edge = false;

                for (i, clip) in p.playlist_clips.iter().enumerate() {
                    if clip.track_index == trk_idx {
                        let c_x = HEADER_WIDTH + (clip.start_tick as f64 / TICKS_PER_BAR as f64) * BAR_WIDTH;
                        let c_w = ((clip.length_ticks as f64 / TICKS_PER_BAR as f64) * BAR_WIDTH).max(12.0);
                        let c_y = RULER_HEIGHT + (trk_idx as f64) * TRACK_HEIGHT;

                        if x >= c_x && x <= c_x + c_w && y >= c_y && y <= c_y + TRACK_HEIGHT {
                            hit_idx = Some(i);
                            if x >= c_x + c_w - 10.0 {
                                on_right_edge = true;
                            }
                            break;
                        }
                    }
                }

                if button == 1 {
                    if let Some(idx) = hit_idx {
                        let clip = &p.playlist_clips[idx];
                        if on_right_edge {
                            // Redimensionar clipe esticando para novos compassos
                            *drag_click.borrow_mut() = DragMode::ResizingClip {
                                clip_id: clip.id,
                                initial_length: clip.length_ticks,
                                start_x: x,
                            };
                        } else {
                            // Mover clipe livremente entre faixas e tempos
                            *drag_click.borrow_mut() = DragMode::MovingClip {
                                clip_id: clip.id,
                                initial_track: clip.track_index,
                                initial_tick: clip.start_tick,
                                start_x: x,
                                start_y: y,
                            };
                        }
                    } else {
                        // Inserir novo clipe de Pattern na faixa e compasso clicado!
                        let new_id = (p.playlist_clips.len() as u64) + 100;
                        let new_clip = PlaylistClip {
                            id: new_id,
                            track_index: trk_idx,
                            start_tick: snapped_tick,
                            length_ticks: TICKS_PER_BAR, // 1 compasso
                            payload: ClipPayload::Pattern(p.active_pattern),
                            muted: false,
                        };
                        p.playlist_clips.push(new_clip);

                        *drag_click.borrow_mut() = DragMode::DrawingClip {
                            clip_id: new_id,
                            track_index: trk_idx,
                            start_tick: snapped_tick,
                        };

                        let clips_clone = p.playlist_clips.clone();
                        let _ = tx_click.send(UiToAudioMsg::UpdatePlaylistClips(clips_clone));
                    }
                } else if button == 3 {
                    // BOTÃO DIREITO: Apagar o clipe instantaneamente (Borracha do FL Studio)
                    if let Some(idx) = hit_idx {
                        p.playlist_clips.remove(idx);
                        let clips_clone = p.playlist_clips.clone();
                        let _ = tx_click.send(UiToAudioMsg::UpdatePlaylistClips(clips_clone));
                    }
                }
                canvas_click.queue_draw();
            }
        });

        // Liberação do Mouse
        let proj_release = project.clone();
        let drag_release = drag_mode.clone();
        let tx_release = tx.clone();
        let canvas_release = canvas.clone();

        gesture.connect_released(move |_g, _n_press, _x, _y| {
            let current_mode = *drag_release.borrow();
            if let DragMode::DrawingClip { .. } | DragMode::ResizingClip { .. } | DragMode::MovingClip { .. } = current_mode {
                let p = proj_release.borrow();
                let _ = tx_release.send(UiToAudioMsg::UpdatePlaylistClips(p.playlist_clips.clone()));
            }
            *drag_release.borrow_mut() = DragMode::None;
            canvas_release.queue_draw();
        });

        canvas.add_controller(gesture);

        // Movimento do Mouse (arrastar para esticar duração ou mover)
        let motion = EventControllerMotion::new();
        let proj_motion = project.clone();
        let drag_motion = drag_mode.clone();
        let tx_motion = tx.clone();
        let canvas_motion = canvas.clone();
        let playhead_motion = playhead_tick.clone();

        motion.connect_motion(move |_m, x, y| {
            let mode = *drag_motion.borrow();
            match mode {
                DragMode::Ruler => {
                    let bar_f = ((x - HEADER_WIDTH) / BAR_WIDTH).max(0.0);
                    let tick = (bar_f * (TICKS_PER_BAR as f64)) as u64;
                    *playhead_motion.borrow_mut() = tick;
                    let _ = tx_motion.send(UiToAudioMsg::SeekTick(tick));
                    canvas_motion.queue_draw();
                }
                DragMode::DrawingClip { clip_id, start_tick, .. } => {
                    let cur_bar = ((x - HEADER_WIDTH) / BAR_WIDTH).max(0.0) as u64;
                    let cur_tick = cur_bar * TICKS_PER_BAR;
                    let new_len = if cur_tick > start_tick {
                        (cur_tick - start_tick + TICKS_PER_BAR).max(TICKS_PER_BAR)
                    } else {
                        TICKS_PER_BAR
                    };

                    let mut p = proj_motion.borrow_mut();
                    if let Some(c) = p.playlist_clips.iter_mut().find(|c| c.id == clip_id) {
                        c.length_ticks = new_len;
                    }
                    canvas_motion.queue_draw();
                }
                DragMode::ResizingClip { clip_id, initial_length, start_x } => {
                    let delta_px = x - start_x;
                    let delta_bars = (delta_px / BAR_WIDTH).round() as i64;
                    let delta_ticks = delta_bars * (TICKS_PER_BAR as i64);
                    let new_len = (initial_length as i64 + delta_ticks).max(TICKS_PER_BAR as i64) as u64;

                    let mut p = proj_motion.borrow_mut();
                    if let Some(c) = p.playlist_clips.iter_mut().find(|c| c.id == clip_id) {
                        c.length_ticks = new_len;
                    }
                    canvas_motion.queue_draw();
                }
                DragMode::MovingClip { clip_id, initial_track, initial_tick, start_x, start_y } => {
                    let delta_x = x - start_x;
                    let delta_y = y - start_y;
                    let delta_bars = (delta_x / BAR_WIDTH).round() as i64;
                    let delta_tracks = (delta_y / TRACK_HEIGHT).round() as i64;

                    let new_tick = (initial_tick as i64 + delta_bars * (TICKS_PER_BAR as i64)).max(0) as u64;
                    let new_track = (initial_track as i64 + delta_tracks).clamp(0, (TOTAL_TRACKS - 1) as i64) as usize;

                    let mut p = proj_motion.borrow_mut();
                    if let Some(c) = p.playlist_clips.iter_mut().find(|c| c.id == clip_id) {
                        c.start_tick = new_tick;
                        c.track_index = new_track;
                    }
                    canvas_motion.queue_draw();
                }
                DragMode::None => {}
            }
        });

        canvas.add_controller(motion);

        // 4. Renderização Cairo da Playlist (100% FL Studio Dark Theme)
        let proj_draw = project.clone();
        let playhead_draw = playhead_tick.clone();
        let mutes_draw = track_mutes.clone();

        canvas.set_draw_func(move |_area, cr, width, height| {
            let p = proj_draw.borrow();
            let mutes = mutes_draw.borrow();

            // Fundo da área de trabalho
            cr.set_source_rgb(0.12, 0.14, 0.17);
            let _ = cr.paint();

            // 1. Linhas e Faixas Horizontais (Tracks)
            for t in 0..TOTAL_TRACKS {
                let y = RULER_HEIGHT + (t as f64) * TRACK_HEIGHT;

                // Fundo da faixa alternando suavemente
                if t % 2 == 0 {
                    cr.set_source_rgb(0.14, 0.16, 0.19);
                } else {
                    cr.set_source_rgb(0.12, 0.14, 0.17);
                }
                cr.rectangle(HEADER_WIDTH, y, width as f64 - HEADER_WIDTH, TRACK_HEIGHT);
                let _ = cr.fill();

                // Linha divisória horizontal
                cr.set_source_rgba(0.22, 0.26, 0.32, 0.5);
                cr.set_line_width(0.8);
                cr.move_to(HEADER_WIDTH, y + TRACK_HEIGHT);
                cr.line_to(width as f64, y + TRACK_HEIGHT);
                let _ = cr.stroke();
            }

            // 2. Linhas Verticais de Compassos e Batidas
            let num_bars = ((width as f64 - HEADER_WIDTH) / BAR_WIDTH).ceil() as usize;
            for b in 0..num_bars {
                let x = HEADER_WIDTH + (b as f64) * BAR_WIDTH;

                // Linha principal do compasso (mais destacada)
                cr.set_source_rgba(0.38, 0.44, 0.54, 0.8);
                cr.set_line_width(1.2);
                cr.move_to(x, RULER_HEIGHT);
                cr.line_to(x, height as f64);
                let _ = cr.stroke();

                // 4 Batidas internas (subdivisões 1/4)
                for beat in 1..4 {
                    let beat_x = x + (beat as f64) * (BAR_WIDTH / 4.0);
                    cr.set_source_rgba(0.20, 0.23, 0.28, 0.4);
                    cr.set_line_width(0.6);
                    cr.move_to(beat_x, RULER_HEIGHT);
                    cr.line_to(beat_x, height as f64);
                    let _ = cr.stroke();
                }
            }

            // 3. Desenho dos Clipes da Playlist (Com Cabeçalho Laranja e Relevo FL Studio)
            for clip in &p.playlist_clips {
                let c_x = HEADER_WIDTH + (clip.start_tick as f64 / TICKS_PER_BAR as f64) * BAR_WIDTH;
                let c_w = ((clip.length_ticks as f64 / TICKS_PER_BAR as f64) * BAR_WIDTH - 2.0).max(8.0);
                let c_y = RULER_HEIGHT + (clip.track_index as f64) * TRACK_HEIGHT + 2.0;
                let c_h = TRACK_HEIGHT - 4.0;

                // Fundo do corpo do clipe (Gradiente Âmbar Escuro)
                cr.set_source_rgb(0.28, 0.20, 0.12);
                cr.rectangle(c_x, c_y, c_w, c_h);
                let _ = cr.fill();

                // Cabeçalho do clipe em Laranja Neon característico
                cr.set_source_rgb(0.95, 0.55, 0.15);
                cr.rectangle(c_x, c_y, c_w, 10.0);
                let _ = cr.fill();

                // Borda em relevo 3D
                cr.set_source_rgb(1.0, 0.75, 0.35);
                cr.set_line_width(1.0);
                cr.rectangle(c_x, c_y, c_w, c_h);
                let _ = cr.stroke();

                // Puxador de redimensionamento na borda direita
                cr.set_source_rgba(0.0, 0.0, 0.0, 0.35);
                cr.rectangle(c_x + c_w - 4.0, c_y, 3.0, c_h);
                let _ = cr.fill();

                // Título do clipe
                let clip_title = match clip.payload {
                    ClipPayload::Pattern(id) => format!("Pattern {}", id + 1),
                    ClipPayload::Audio(ref name) => name.clone(),
                    ClipPayload::Automation(_) => "Automation".to_string(),
                };

                cr.set_source_rgb(0.12, 0.06, 0.02);
                cr.set_font_size(8.5);
                cr.move_to(c_x + 4.0, c_y + 8.5);
                let _ = cr.show_text(&clip_title);
            }

            // 4. Cabeçalho das Faixas (Coluna da Esquerda com Nomes e LEDs de Mute)
            cr.set_source_rgb(0.15, 0.17, 0.21);
            cr.rectangle(0.0, RULER_HEIGHT, HEADER_WIDTH, height as f64 - RULER_HEIGHT);
            let _ = cr.fill();

            for t in 0..TOTAL_TRACKS {
                let y = RULER_HEIGHT + (t as f64) * TRACK_HEIGHT;

                // Linha divisória horizontal do cabeçalho
                cr.set_source_rgba(0.24, 0.28, 0.35, 0.8);
                cr.set_line_width(0.8);
                cr.move_to(0.0, y + TRACK_HEIGHT);
                cr.line_to(HEADER_WIDTH, y + TRACK_HEIGHT);
                let _ = cr.stroke();

                // LED de Mute (Verde se ativo, cinza se mutado)
                let is_muted = mutes.get(t).copied().unwrap_or(false);
                if is_muted {
                    cr.set_source_rgb(0.35, 0.38, 0.42);
                } else {
                    cr.set_source_rgb(0.20, 0.85, 0.40);
                }
                cr.arc(10.0, y + TRACK_HEIGHT / 2.0, 3.5, 0.0, std::f64::consts::PI * 2.0);
                let _ = cr.fill();

                // Nome da faixa
                cr.set_source_rgb(0.85, 0.90, 0.95);
                cr.set_font_size(10.0);
                cr.move_to(22.0, y + TRACK_HEIGHT / 2.0 + 3.5);
                let _ = cr.show_text(&format!("Track {}", t + 1));
            }

            // Borda vertical separando cabeçalho da grade
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.4);
            cr.rectangle(HEADER_WIDTH, RULER_HEIGHT, 3.0, height as f64 - RULER_HEIGHT);
            let _ = cr.fill();

            // 5. Régua de Compassos Superior (Timeline Ruler)
            cr.set_source_rgb(0.14, 0.16, 0.20);
            cr.rectangle(0.0, 0.0, width as f64, RULER_HEIGHT);
            let _ = cr.fill();

            cr.set_source_rgb(0.26, 0.30, 0.38);
            cr.set_line_width(1.0);
            cr.move_to(0.0, RULER_HEIGHT);
            cr.line_to(width as f64, RULER_HEIGHT);
            let _ = cr.stroke();

            // Canto superior esquerdo
            cr.set_source_rgb(0.12, 0.14, 0.17);
            cr.rectangle(0.0, 0.0, HEADER_WIDTH, RULER_HEIGHT);
            let _ = cr.fill();

            for b in 0..num_bars {
                let x = HEADER_WIDTH + (b as f64) * BAR_WIDTH;
                let bar_num = b + 1;

                cr.set_source_rgb(0.85, 0.90, 0.95);
                cr.set_font_size(10.0);
                cr.move_to(x + 4.0, RULER_HEIGHT - 7.0);
                let _ = cr.show_text(&format!("{}", bar_num));

                cr.set_source_rgb(0.45, 0.50, 0.60);
                cr.set_line_width(1.2);
                cr.move_to(x, RULER_HEIGHT - 12.0);
                cr.line_to(x, RULER_HEIGHT);
                let _ = cr.stroke();
            }

            // 6. Playhead Animado em Tempo Real (Linha Verde Neon varrendo a timeline da música)
            let cur_tick = *playhead_draw.borrow();
            let playhead_x = HEADER_WIDTH + (cur_tick as f64 / TICKS_PER_BAR as f64) * BAR_WIDTH;

            if playhead_x >= HEADER_WIDTH && playhead_x <= width as f64 {
                cr.set_source_rgba(0.20, 0.95, 0.50, 0.9);
                cr.set_line_width(1.8);
                cr.move_to(playhead_x, RULER_HEIGHT);
                cr.line_to(playhead_x, height as f64);
                let _ = cr.stroke();

                // Ponteiro triangular na régua superior
                cr.set_source_rgb(0.25, 1.0, 0.55);
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
            playhead_tick,
            track_mutes,
        }
    }

    /// Atualiza a posição do playhead animado em tempo real (60 FPS)
    pub fn update_playhead(&self, tick: u64) {
        *self.playhead_tick.borrow_mut() = tick;
        self.canvas.queue_draw();
    }
}
