use gtk4::gdk::{Key, ModifierType};
use gtk4::prelude::*;
use gtk4::EventControllerKey;
use openstudio_core::{PlaybackMode, UiToAudioMsg};
use crossbeam_channel::Sender;
use std::rc::Rc;
use std::cell::RefCell;

pub struct ShortcutContext {
    pub is_playing: Rc<RefCell<bool>>,
    pub playback_mode: Rc<RefCell<PlaybackMode>>,
    pub tx: Sender<UiToAudioMsg>,
    pub on_toggle_playlist: Rc<dyn Fn()>,
    pub on_toggle_channel_rack: Rc<dyn Fn()>,
    pub on_toggle_piano_roll: Rc<dyn Fn()>,
    pub on_toggle_mixer: Rc<dyn Fn()>,
    pub on_toggle_browser: Rc<dyn Fn()>,
    pub on_toggle_ghost_notes: Rc<dyn Fn()>,
}

pub fn install_fl_shortcuts(window: &impl IsA<gtk4::Widget>, ctx: ShortcutContext) {
    let controller = EventControllerKey::new();

    let is_playing = ctx.is_playing;
    let playback_mode = ctx.playback_mode;
    let tx = ctx.tx;
    let on_playlist = ctx.on_toggle_playlist;
    let on_channel_rack = ctx.on_toggle_channel_rack;
    let on_piano_roll = ctx.on_toggle_piano_roll;
    let on_mixer = ctx.on_toggle_mixer;
    let on_browser = ctx.on_toggle_browser;
    let on_ghost = ctx.on_toggle_ghost_notes;

    controller.connect_key_pressed(move |_controller, key, _keycode, modifier| {
        let is_ctrl = modifier.contains(ModifierType::CONTROL_MASK);
        let is_alt = modifier.contains(ModifierType::ALT_MASK);

        match key {
            // Space: Play / Stop clássico do FL Studio
            Key::space => {
                let mut playing = is_playing.borrow_mut();
                if *playing {
                    *playing = false;
                    let _ = tx.send(UiToAudioMsg::Stop);
                } else {
                    *playing = true;
                    let _ = tx.send(UiToAudioMsg::Play);
                }
                return glib::Propagation::Stop;
            }

            // L: Alternar modo Song / Pattern (ícone e comportamento lendário do FL Studio)
            Key::L | Key::l if !is_ctrl && !is_alt => {
                let mut mode = playback_mode.borrow_mut();
                let new_mode = match *mode {
                    PlaybackMode::Pattern => PlaybackMode::Song,
                    PlaybackMode::Song => PlaybackMode::Pattern,
                };
                *mode = new_mode;
                let _ = tx.send(UiToAudioMsg::SetPlaybackMode(new_mode));
                return glib::Propagation::Stop;
            }

            // R: Alternar Gravação
            Key::R | Key::r if !is_ctrl && !is_alt => {
                let _ = tx.send(UiToAudioMsg::Record);
                return glib::Propagation::Stop;
            }

            // F5: Alternar Playlist / Arrangement
            Key::F5 => {
                on_playlist();
                return glib::Propagation::Stop;
            }

            // F6: Alternar Channel Rack / Step Sequencer
            Key::F6 => {
                on_channel_rack();
                return glib::Propagation::Stop;
            }

            // F7: Alternar Piano Roll
            Key::F7 => {
                on_piano_roll();
                return glib::Propagation::Stop;
            }

            // F9: Alternar Mixer
            Key::F9 => {
                on_mixer();
                return glib::Propagation::Stop;
            }

            // Alt + F8: Alternar Navegador / Browser
            Key::F8 if is_alt => {
                on_browser();
                return glib::Propagation::Stop;
            }

            // Alt + V: Alternar Ghost Notes no Piano Roll
            Key::V | Key::v if is_alt => {
                on_ghost();
                return glib::Propagation::Stop;
            }

            // Ctrl + B: Duplicar seleção para a direita (estilo FL)
            Key::B | Key::b if is_ctrl => {
                // Duplica notas ou padrões selecionados
                return glib::Propagation::Stop;
            }

            // Atalhos de Ferramentas: P (Pencil), B (Brush), D (Delete), C (Slice), E (Select), T (Mute), Z (Zoom)
            Key::P | Key::p if !is_ctrl && !is_alt => {
                return glib::Propagation::Stop;
            }
            Key::B | Key::b if !is_ctrl && !is_alt => {
                return glib::Propagation::Stop;
            }
            Key::D | Key::d if !is_ctrl && !is_alt => {
                return glib::Propagation::Stop;
            }
            Key::C | Key::c if !is_ctrl && !is_alt => {
                return glib::Propagation::Stop;
            }

            _ => {}
        }

        glib::Propagation::Proceed
    });

    window.add_controller(controller);
}
