use gtk4::prelude::*;
use gtk4::{Box, Button, Label, Orientation, Scale, ToggleButton};
use openstudio_core::{ChannelParam, Project, UiToAudioMsg};
use crossbeam_channel::Sender;
use std::cell::RefCell;
use std::rc::Rc;

pub type OpenPianoRollCallback = Rc<RefCell<Option<Rc<dyn Fn(usize)>>>>;

pub struct ChannelRackWidget {
    pub container: Box,
    pub step_buttons: Vec<Vec<ToggleButton>>,
    pub current_playhead: Rc<RefCell<usize>>,
    pub on_open_piano_roll: OpenPianoRollCallback,
}

impl ChannelRackWidget {
    pub fn new(
        project: Rc<RefCell<Project>>,
        tx: Sender<UiToAudioMsg>,
    ) -> Self {
        let container = Box::new(Orientation::Vertical, 4);
        container.add_css_class("channel-rack-box");

        let on_open_piano_roll = Rc::new(RefCell::new(None::<Rc<dyn Fn(usize)>>));

        // Cabeçalho do Channel Rack
        let header = Box::new(Orientation::Horizontal, 8);
        header.add_css_class("channel-rack-header");

        let title_label = Label::new(Some("🥁 Channel Rack"));
        title_label.add_css_class("lcd-bpm");
        header.append(&title_label);

        let pattern_label = Label::new(Some("Pattern 1"));
        pattern_label.add_css_class("lcd-display");
        header.append(&pattern_label);

        // Controle Interativo de Swing (Balanço do FL Studio)
        let swing_box = Box::new(Orientation::Horizontal, 4);
        swing_box.add_css_class("lcd-display");
        let swing_title = Label::new(Some("Swing:"));
        swing_title.add_css_class("lcd-bpm");
        swing_box.append(&swing_title);

        let swing_scale = Scale::with_range(Orientation::Horizontal, 0.0, 1.0, 0.05);
        swing_scale.set_value(0.0);
        swing_scale.set_size_request(60, -1);
        swing_scale.set_draw_value(false);
        let tx_swing = tx.clone();
        swing_scale.connect_value_changed(move |s| {
            let _ = tx_swing.send(UiToAudioMsg::SetSwing(s.value() as f32));
        });
        swing_box.append(&swing_scale);
        header.append(&swing_box);

        container.append(&header);

        let mut step_buttons = Vec::new();
        let proj = project.borrow();
        let active_pattern_id = proj.active_pattern;
        let current_playhead = Rc::new(RefCell::new(0));

        for channel in &proj.channels {
            let row = Box::new(Orientation::Horizontal, 6);
            row.add_css_class("channel-strip");

            // Mute / Solo LED
            let mute_btn = ToggleButton::with_label("●");
            mute_btn.add_css_class("transport-btn");
            mute_btn.set_active(true);
            row.append(&mute_btn);

            // Mini Controle de Volume (0..125%)
            let vol_scale = Scale::with_range(Orientation::Horizontal, 0.0, 1.25, 0.05);
            vol_scale.set_value(channel.volume as f64);
            vol_scale.set_size_request(45, -1);
            vol_scale.set_draw_value(false);
            let tx_vol = tx.clone();
            let ch_vol_id = channel.id;
            vol_scale.connect_value_changed(move |s| {
                let _ = tx_vol.send(UiToAudioMsg::SetChannelParam {
                    channel_id: ch_vol_id,
                    param_type: ChannelParam::Volume,
                    value: s.value() as f32,
                });
            });
            row.append(&vol_scale);

            // Mini Controle de Pan (-1.0..+1.0)
            let pan_scale = Scale::with_range(Orientation::Horizontal, -1.0, 1.0, 0.1);
            pan_scale.set_value(channel.pan as f64);
            pan_scale.set_size_request(35, -1);
            pan_scale.set_draw_value(false);
            let tx_pan = tx.clone();
            let ch_pan_id = channel.id;
            pan_scale.connect_value_changed(move |s| {
                let _ = tx_pan.send(UiToAudioMsg::SetChannelParam {
                    channel_id: ch_pan_id,
                    param_type: ChannelParam::Pan,
                    value: s.value() as f32,
                });
            });
            row.append(&pan_scale);

            // Botão com Nome do Canal (Audição imediata ao clicar!)
            let name_btn = Button::with_label(&channel.name);
            name_btn.add_css_class("channel-name-btn");
            let tx_audition = tx.clone();
            let ch_id = channel.id;
            name_btn.connect_clicked(move |_| {
                let _ = tx_audition.send(UiToAudioMsg::TriggerNoteOn {
                    channel_id: ch_id,
                    pitch: 60,
                    velocity: 0.9,
                });
            });
            row.append(&name_btn);

            // Botão de Atalho para Piano Roll deste canal (🎹)
            let pr_jump_btn = Button::with_label("🎹");
            pr_jump_btn.add_css_class("transport-btn");
            let cb_ref = on_open_piano_roll.clone();
            pr_jump_btn.connect_clicked(move |_| {
                if let Some(ref cb) = *cb_ref.borrow() {
                    cb(ch_id);
                }
            });
            row.append(&pr_jump_btn);

            // Badge com número da rota do Mixer
            let trk_badge = Label::new(Some(&format!("{:>2}", channel.target_mixer_track)));
            trk_badge.add_css_class("channel-trk-badge");
            row.append(&trk_badge);

            // Grade de 16 passos com relevo e agrupamento de 4 batidas
            let mut ch_steps = Vec::new();
            for step_idx in 0..16 {
                let btn = ToggleButton::new();
                btn.add_css_class("step-button");

                if (step_idx / 4) % 2 == 0 {
                    btn.add_css_class("step-dark");
                } else {
                    btn.add_css_class("step-light");
                }

                // Verifica se já está ativo no projeto
                if let Some(pattern) = proj.patterns.iter().find(|p| p.id == active_pattern_id) {
                    if let Some(steps) = pattern.step_data.get(&channel.id) {
                        if let Some(s) = steps.get(step_idx) {
                            if s.active {
                                btn.set_active(true);
                                btn.add_css_class("step-active");
                            }
                        }
                    }
                }

                let tx_step = tx.clone();
                let ch_idx = channel.id;
                let pat_id = active_pattern_id;
                let btn_clone = btn.clone();

                btn.connect_toggled(move |b| {
                    let active = b.is_active();
                    if active {
                        btn_clone.add_css_class("step-active");
                        // Audição instantânea ao acionar o passo!
                        let _ = tx_step.send(UiToAudioMsg::TriggerNoteOn {
                            channel_id: ch_idx,
                            pitch: 60,
                            velocity: 0.85,
                        });
                    } else {
                        btn_clone.remove_css_class("step-active");
                    }

                    let _ = tx_step.send(UiToAudioMsg::UpdateStep {
                        pattern_id: pat_id,
                        channel_id: ch_idx,
                        step_index: step_idx,
                        active,
                        velocity: 0.85,
                    });
                });

                row.append(&btn);
                ch_steps.push(btn);
            }

            container.append(&row);
            step_buttons.push(ch_steps);
        }

        Self {
            container,
            step_buttons,
            current_playhead,
            on_open_piano_roll,
        }
    }

    /// Atualiza o indicador do Playhead animado (varredura luminosa dos 16 passos)
    pub fn update_playhead(&self, step: usize) {
        let prev = *self.current_playhead.borrow();
        if prev != step {
            // Remove destaque do passo anterior
            for ch_steps in &self.step_buttons {
                if let Some(btn) = ch_steps.get(prev) {
                    btn.remove_css_class("step-playhead");
                }
                // Adiciona destaque ao novo passo
                if let Some(btn) = ch_steps.get(step) {
                    btn.add_css_class("step-playhead");
                }
            }
            *self.current_playhead.borrow_mut() = step;
        }
    }
}
