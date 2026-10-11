use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Box, Button, Label, Orientation, ScrolledWindow};
use openstudio_core::UiToAudioMsg;
use openstudio_plugins::PluginDatabase;
use crossbeam_channel::Sender;
use std::rc::Rc;
use crate::plugin_dialog::show_plugin_dialog;

pub struct BrowserWidget {
    pub container: Box,
}

pub type ViewSwitchCallback = Option<Rc<dyn Fn(&str)>>;

impl BrowserWidget {
    pub fn new(
        plugin_db: &PluginDatabase,
        tx: Sender<UiToAudioMsg>,
        window: Option<ApplicationWindow>,
        on_switch_view: ViewSwitchCallback,
    ) -> Self {
        let container = Box::new(Orientation::Vertical, 4);
        container.add_css_class("browser-tree");
        container.set_size_request(220, -1);

        let title = Label::new(Some("📁 Browser [Alt+F8]"));
        title.add_css_class("lcd-bpm");
        container.append(&title);

        let scrolled = ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_hexpand(true);
        scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

        let list_box = Box::new(Orientation::Vertical, 4);

        // Seção 1: Packs de Bateria com Audição Instantânea ao Clicar
        let packs_hdr = Label::new(Some("▼ 📁 Packs (Samples & Drums)"));
        packs_hdr.set_xalign(0.0);
        packs_hdr.add_css_class("mixer-slot-row");
        list_box.append(&packs_hdr);

        let drum_auditions = [
            ("  🔊 808 Sub Kick", 0, 48),
            ("  🔊 Studio Clap", 1, 60),
            ("  🔊 Trap Closed Hat", 2, 60),
            ("  🔊 Punchy Snare", 3, 60),
            ("  🔊 3x Osc Synth Lead", 4, 60),
        ];

        for (name, ch_id, pitch) in drum_auditions {
            let btn = Button::with_label(name);
            btn.add_css_class("transport-btn");
            let tx_clone = tx.clone();
            btn.connect_clicked(move |_| {
                let _ = tx_clone.send(UiToAudioMsg::TriggerNoteOn {
                    channel_id: ch_id,
                    pitch,
                    velocity: 0.9,
                });
            });
            list_box.append(&btn);
        }

        // Seção 2: Plugins VST3 / CLAP / Nativos (100% Clicáveis com Diálogo Modal)
        let plugins_hdr = Label::new(Some("▼ 🔌 Plugins VST3 / CLAP"));
        plugins_hdr.set_xalign(0.0);
        plugins_hdr.add_css_class("mixer-slot-row");
        list_box.append(&plugins_hdr);

        for p in &plugin_db.plugins {
            let icon = match p.category.as_str() {
                "Synth" => "🎹",
                "Dynamics" => "🎛",
                "EQ" => "📊",
                "Delay" => "⏳",
                _ => "🔌",
            };
            let btn = Button::with_label(&format!("  {} {} [{}]", icon, p.name, p.category));
            btn.add_css_class("transport-btn");
            let p_clone = p.clone();
            let win_clone = window.clone();
            let tx_clone = tx.clone();
            let switch_pr = on_switch_view.clone();
            btn.connect_clicked(move |_| {
                if let Some(ref win) = win_clone {
                    let on_pr = switch_pr.clone().map(|sw| {
                        let sw_clone = sw.clone();
                        Rc::new(move || sw_clone("piano_roll")) as Rc<dyn Fn()>
                    });
                    show_plugin_dialog(win, &p_clone, tx_clone.clone(), on_pr);
                }
            });
            list_box.append(&btn);
        }

        // Seção 3: Projeto Atual (Navegação Interativa entre Views)
        let proj_hdr = Label::new(Some("▼ 💾 Projeto Atual"));
        proj_hdr.set_xalign(0.0);
        proj_hdr.add_css_class("mixer-slot-row");
        list_box.append(&proj_hdr);

        // Patterns -> Playlist
        let pat_btn = Button::with_label("  🎼 Patterns (Pattern 1)");
        pat_btn.add_css_class("transport-btn");
        let sw_pat = on_switch_view.clone();
        pat_btn.connect_clicked(move |_| {
            if let Some(ref sw) = sw_pat {
                sw("playlist");
            }
        });
        list_box.append(&pat_btn);

        // Canais -> Channel Rack
        let ch_btn = Button::with_label("  🎚 Canais (5 Canais)");
        ch_btn.add_css_class("transport-btn");
        let sw_ch = on_switch_view.clone();
        ch_btn.connect_clicked(move |_| {
            if let Some(ref sw) = sw_ch {
                sw("channel_rack");
            }
        });
        list_box.append(&ch_btn);

        // Piano Roll
        let pr_btn = Button::with_label("  🎹 Piano Roll [F7]");
        pr_btn.add_css_class("transport-btn");
        let sw_pr = on_switch_view.clone();
        pr_btn.connect_clicked(move |_| {
            if let Some(ref sw) = sw_pr {
                sw("piano_roll");
            }
        });
        list_box.append(&pr_btn);

        // Mixer
        let mx_btn = Button::with_label("  🎛 Mixer Master [F9]");
        mx_btn.add_css_class("transport-btn");
        let sw_mx = on_switch_view.clone();
        mx_btn.connect_clicked(move |_| {
            if let Some(ref sw) = sw_mx {
                sw("mixer");
            }
        });
        list_box.append(&mx_btn);

        // Histórico / Desfazer
        let undo_btn = Button::with_label("  ↩ Histórico / Desfazer");
        undo_btn.add_css_class("transport-btn");
        undo_btn.connect_clicked(move |btn| {
            btn.set_label("  ✔ Nenhuma ação pendente");
            glib::timeout_add_local_once(std::time::Duration::from_millis(1500), {
                let b = btn.clone();
                move || b.set_label("  ↩ Histórico / Desfazer")
            });
        });
        list_box.append(&undo_btn);

        scrolled.set_child(Some(&list_box));
        container.append(&scrolled);

        Self { container }
    }
}
