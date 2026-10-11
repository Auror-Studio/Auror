use gtk4::prelude::*;
use gtk4::{Box, Button, Label, Orientation, ScrolledWindow};
use openstudio_core::UiToAudioMsg;
use openstudio_plugins::PluginDatabase;
use crossbeam_channel::Sender;

pub struct BrowserWidget {
    pub container: Box,
}

impl BrowserWidget {
    pub fn new(plugin_db: &PluginDatabase, tx: Sender<UiToAudioMsg>) -> Self {
        let container = Box::new(Orientation::Vertical, 4);
        container.add_css_class("browser-tree");
        container.set_size_request(200, -1);

        let title = Label::new(Some("📁 Browser [Alt+F8]"));
        title.add_css_class("lcd-bpm");
        container.append(&title);

        let scrolled = ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_hexpand(true);
        scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

        let list_box = Box::new(Orientation::Vertical, 2);

        // Seção Packs de Bateria com Audição Instantânea ao Clicar
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

        // Seção Plugins Detectados
        let plugins_hdr = Label::new(Some("▼ 🔌 Plugins VST3 / CLAP"));
        plugins_hdr.set_xalign(0.0);
        plugins_hdr.add_css_class("mixer-slot-row");
        list_box.append(&plugins_hdr);

        for p in &plugin_db.plugins {
            let label_text = format!("  • {} [{}]", p.name, p.category);
            let item = Label::new(Some(&label_text));
            item.set_xalign(0.0);
            list_box.append(&item);
        }

        // Seção Projeto Atual
        let proj_hdr = Label::new(Some("▼ 💾 Projeto Atual"));
        proj_hdr.set_xalign(0.0);
        proj_hdr.add_css_class("mixer-slot-row");
        list_box.append(&proj_hdr);

        let proj_items = ["  • Patterns (1)", "  • Canais (5)", "  • Histórico / Undo"];
        for pi in &proj_items {
            let item = Label::new(Some(pi));
            item.set_xalign(0.0);
            list_box.append(&item);
        }

        scrolled.set_child(Some(&list_box));
        container.append(&scrolled);

        Self { container }
    }
}
