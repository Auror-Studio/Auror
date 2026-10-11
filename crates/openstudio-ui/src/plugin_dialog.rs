use gtk4::prelude::*;
use gtk4::{Box, Button, Label, Orientation, Scale, Window};
use openstudio_core::UiToAudioMsg;
use openstudio_plugins::{PluginDescriptor, PluginFormat};
use crossbeam_channel::Sender;
use std::rc::Rc;

pub fn show_plugin_dialog(
    parent: &impl IsA<Window>,
    plugin: &PluginDescriptor,
    tx: Sender<UiToAudioMsg>,
    on_open_piano_roll: Option<Rc<dyn Fn()>>,
) {
    let dialog = Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(format!("Plugin: {} [{}]", plugin.name, plugin.category))
        .default_width(460)
        .default_height(400)
        .build();

    dialog.add_css_class("main-window");

    let content = Box::new(Orientation::Vertical, 12);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(16);
    content.set_margin_end(16);

    // Cabeçalho com Ícone e Nome
    let header_box = Box::new(Orientation::Horizontal, 8);
    let icon_label = match plugin.category.as_str() {
        "Synth" => "🎹",
        "Dynamics" => "🎛",
        "EQ" => "📊",
        "Delay" => "⏳",
        _ => "🔌",
    };
    let title_lbl = Label::new(Some(&format!("{} {}", icon_label, plugin.name)));
    title_lbl.add_css_class("lcd-bpm");
    header_box.append(&title_lbl);

    let format_badge = match plugin.format {
        PluginFormat::NativeBuiltin => "Nativo FL Core",
        PluginFormat::VST3 => "VST3 Standard",
        PluginFormat::CLAP => "CLAP 1.0",
        PluginFormat::LV2 => "LV2 Audio",
        PluginFormat::WindowsBridgedVST3 => "Windows Bridge",
    };
    let badge_lbl = Label::new(Some(&format!("[{}]", format_badge)));
    badge_lbl.add_css_class("gpu-badge");
    header_box.append(&badge_lbl);
    content.append(&header_box);

    // Metadados
    let meta_box = Box::new(Orientation::Vertical, 4);
    let vendor_lbl = Label::new(Some(&format!("Desenvolvedor: {} | Versão: {}", plugin.vendor, plugin.version)));
    vendor_lbl.set_xalign(0.0);
    meta_box.append(&vendor_lbl);

    let cat_lbl = Label::new(Some(&format!("Categoria: {} | Arquitetura: 64-bit Real-time DSP", plugin.category)));
    cat_lbl.set_xalign(0.0);
    meta_box.append(&cat_lbl);
    content.append(&meta_box);

    // Seção de Parâmetros e Controles Rápidos
    let params_title = Label::new(Some("Controles e Parâmetros em Tempo Real:"));
    params_title.set_xalign(0.0);
    params_title.add_css_class("lcd-time");
    content.append(&params_title);

    let params_box = Box::new(Orientation::Vertical, 8);
    if plugin.parameters.is_empty() {
        let p1_box = Box::new(Orientation::Horizontal, 8);
        let p1_lbl = Label::new(Some("Mix Wet / Dry:"));
        p1_lbl.set_xalign(0.0);
        let p1_scale = Scale::with_range(Orientation::Horizontal, 0.0, 1.0, 0.01);
        p1_scale.set_value(1.0);
        p1_scale.set_hexpand(true);
        p1_box.append(&p1_lbl);
        p1_box.append(&p1_scale);
        params_box.append(&p1_box);

        let p2_box = Box::new(Orientation::Horizontal, 8);
        let p2_lbl = Label::new(Some("Ganho / Input Drive:"));
        p2_lbl.set_xalign(0.0);
        let p2_scale = Scale::with_range(Orientation::Horizontal, 0.0, 2.0, 0.05);
        p2_scale.set_value(1.0);
        p2_scale.set_hexpand(true);
        p2_box.append(&p2_lbl);
        p2_box.append(&p2_scale);
        params_box.append(&p2_box);
    } else {
        for param in &plugin.parameters {
            let p_box = Box::new(Orientation::Horizontal, 8);
            let p_lbl = Label::new(Some(&format!("{}:", param.name)));
            p_lbl.set_xalign(0.0);
            let p_scale = Scale::with_range(Orientation::Horizontal, param.min as f64, param.max as f64, 0.05);
            p_scale.set_value(param.current_val as f64);
            p_scale.set_hexpand(true);
            p_box.append(&p_lbl);
            p_box.append(&p_scale);
            params_box.append(&p_box);
        }
    }
    content.append(&params_box);

    // Ações Rápidas (Audition / Inserir no Mixer / Abrir Piano Roll)
    let actions_box = Box::new(Orientation::Horizontal, 8);
    actions_box.set_hexpand(true);

    let tx_audition = tx.clone();
    let is_synth = plugin.category == "Synth";
    let audition_btn = Button::with_label("🔊 Testar Som (Audition)");
    audition_btn.add_css_class("transport-btn");
    audition_btn.connect_clicked(move |_| {
        let ch_id = if is_synth { 4 } else { 0 };
        let _ = tx_audition.send(UiToAudioMsg::TriggerNoteOn {
            channel_id: ch_id,
            pitch: 60, // C5
            velocity: 0.9,
        });
    });
    actions_box.append(&audition_btn);

    if plugin.category == "Synth" {
        if let Some(on_pr) = on_open_piano_roll {
            let pr_btn = Button::with_label("🎹 Abrir no Piano Roll");
            pr_btn.add_css_class("transport-btn");
            let dlg_pr = dialog.clone();
            pr_btn.connect_clicked(move |_| {
                on_pr();
                dlg_pr.close();
            });
            actions_box.append(&pr_btn);
        }
    } else {
        let tx_insert = tx.clone();
        let insert_btn = Button::with_label("🎛 Inserir no Master");
        insert_btn.add_css_class("transport-btn");
        let name_clone = plugin.name.clone();
        insert_btn.connect_clicked(move |btn| {
            if name_clone.contains("Soundgoodizer") {
                let _ = tx_insert.send(UiToAudioMsg::SetMixerSlotBypass {
                    track_id: 0,
                    slot_index: 0,
                    enabled: true,
                });
                let _ = tx_insert.send(UiToAudioMsg::SetMixerSlotWet {
                    track_id: 0,
                    slot_index: 0,
                    wet: 0.7,
                });
            } else if name_clone.contains("Delay") {
                let _ = tx_insert.send(UiToAudioMsg::SetMixerSlotBypass {
                    track_id: 0,
                    slot_index: 2,
                    enabled: true,
                });
                let _ = tx_insert.send(UiToAudioMsg::SetMixerSlotWet {
                    track_id: 0,
                    slot_index: 2,
                    wet: 0.4,
                });
            }
            btn.set_label("✔ Inserido no Master!");
        });
        actions_box.append(&insert_btn);
    }

    let close_btn = Button::with_label("Fechar");
    close_btn.add_css_class("transport-btn");
    let dlg_close = dialog.clone();
    close_btn.connect_clicked(move |_| {
        dlg_close.close();
    });
    actions_box.append(&close_btn);

    content.append(&actions_box);

    dialog.set_child(Some(&content));
    dialog.present();
}
