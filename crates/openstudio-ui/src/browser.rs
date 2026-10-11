use gtk4::prelude::*;
use gtk4::{Box, Label, Orientation, ScrolledWindow};
use openstudio_plugins::PluginDatabase;

pub struct BrowserWidget {
    pub container: Box,
}

impl BrowserWidget {
    pub fn new(plugin_db: &PluginDatabase) -> Self {
        let container = Box::new(Orientation::Vertical, 4);
        container.add_css_class("browser-tree");
        container.set_size_request(180, -1);

        let title = Label::new(Some("Browser [Alt+F8]"));
        title.add_css_class("lcd-bpm");
        container.append(&title);

        let scrolled = ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_hexpand(true);

        let list_box = Box::new(Orientation::Vertical, 2);

        // Seção Packs
        let packs_hdr = Label::new(Some("▼ 📁 Packs (Samples)"));
        packs_hdr.set_xalign(0.0);
        packs_hdr.add_css_class("mixer-slot-row");
        list_box.append(&packs_hdr);

        let sample_items = ["  • Drums (808 / Kick / Snare)", "  • Instruments", "  • Vocals & FX", "  • Legacy"];
        for s in &sample_items {
            let item = Label::new(Some(s));
            item.set_xalign(0.0);
            list_box.append(&item);
        }

        // Seção Plugins
        let plugins_hdr = Label::new(Some("▼ 🔌 Plugins (VST3 / CLAP / Native)"));
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

        let proj_items = ["  • Patterns (1)", "  • Canais (4)", "  • Histórico / Undo"];
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
