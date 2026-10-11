use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Box, Button, Entry, Label, Orientation, Window};
use openstudio_core::Project;
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

/// Diálogo para Salvar Projeto no formato .ostudio
pub fn show_save_project_dialog(parent: &ApplicationWindow, project: Rc<RefCell<Project>>) {
    let dialog = Window::builder()
        .transient_for(parent)
        .modal(true)
        .title("Salvar Projeto - Auror Studio")
        .default_width(450)
        .default_height(200)
        .build();

    dialog.add_css_class("main-window");

    let content = Box::new(Orientation::Vertical, 12);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(16);
    content.set_margin_end(16);

    let title_lbl = Label::new(Some("💾 Salvar Projeto (.ostudio)"));
    title_lbl.add_css_class("lcd-bpm");
    content.append(&title_lbl);

    let entry = Entry::new();
    entry.set_text("Novo_Projeto.ostudio");
    entry.set_placeholder_text(Some("Nome ou caminho do arquivo"));
    content.append(&entry);

    let status_lbl = Label::new(None);
    content.append(&status_lbl);

    let btn_box = Box::new(Orientation::Horizontal, 8);
    let save_btn = Button::with_label("Salvar");
    save_btn.add_css_class("transport-btn");
    save_btn.add_css_class("play-btn");

    let cancel_btn = Button::with_label("Cancelar");
    cancel_btn.add_css_class("transport-btn");

    btn_box.append(&save_btn);
    btn_box.append(&cancel_btn);
    content.append(&btn_box);

    let dialog_close = dialog.clone();
    cancel_btn.connect_clicked(move |_| {
        dialog_close.close();
    });

    let dialog_save = dialog.clone();
    let parent_win = parent.clone();
    save_btn.connect_clicked(move |_| {
        let path_str = entry.text();
        let path = Path::new(path_str.as_str());
        let p = project.borrow();
        match openstudio_project::save_project_to_file(&p, path) {
            Ok(()) => {
                status_lbl.set_text("✔ Projeto salvo com sucesso!");
                parent_win.set_title(Some(&format!("Auror Studio - [{}]", path_str)));
                let d = dialog_save.clone();
                glib::timeout_add_local_once(std::time::Duration::from_millis(700), move || {
                    d.close();
                });
            }
            Err(e) => {
                status_lbl.set_text(&format!("❌ Erro: {}", e));
            }
        }
    });

    dialog.set_child(Some(&content));
    dialog.present();
}

/// Diálogo para Exportar Áudio no formato WAV
pub fn show_export_audio_dialog(parent: &ApplicationWindow, project: Rc<RefCell<Project>>) {
    let dialog = Window::builder()
        .transient_for(parent)
        .modal(true)
        .title("Exportar Áudio WAV - Auror Studio")
        .default_width(450)
        .default_height(200)
        .build();

    dialog.add_css_class("main-window");

    let content = Box::new(Orientation::Vertical, 12);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(16);
    content.set_margin_end(16);

    let title_lbl = Label::new(Some("🎵 Exportar Áudio (WAV 16-bit PCM)"));
    title_lbl.add_css_class("lcd-bpm");
    content.append(&title_lbl);

    let entry = Entry::new();
    entry.set_text("Auror_Export.wav");
    entry.set_placeholder_text(Some("Nome do arquivo WAV de saída"));
    content.append(&entry);

    let status_lbl = Label::new(Some("Qualidade de estúdio: 44.1 kHz, 16-bit Stereo"));
    content.append(&status_lbl);

    let btn_box = Box::new(Orientation::Horizontal, 8);
    let export_btn = Button::with_label("Renderizar / Exportar");
    export_btn.add_css_class("transport-btn");
    export_btn.add_css_class("record-btn");

    let cancel_btn = Button::with_label("Cancelar");
    cancel_btn.add_css_class("transport-btn");

    btn_box.append(&export_btn);
    btn_box.append(&cancel_btn);
    content.append(&btn_box);

    let dialog_close = dialog.clone();
    cancel_btn.connect_clicked(move |_| {
        dialog_close.close();
    });

    let dialog_exp = dialog.clone();
    export_btn.connect_clicked(move |btn| {
        btn.set_sensitive(false);
        let path_str = entry.text();
        let path = Path::new(path_str.as_str());
        let p = project.borrow();
        match openstudio_audio::export_project_to_wav(&p, path) {
            Ok(()) => {
                status_lbl.set_text("✔ Áudio renderizado e salvo com sucesso!");
                let d = dialog_exp.clone();
                glib::timeout_add_local_once(std::time::Duration::from_millis(1000), move || {
                    d.close();
                });
            }
            Err(e) => {
                btn.set_sensitive(true);
                status_lbl.set_text(&format!("❌ Erro ao exportar: {}", e));
            }
        }
    });

    dialog.set_child(Some(&content));
    dialog.present();
}
