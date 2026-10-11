use gtk4::prelude::*;
use gtk4::{Box, Button, Label, Orientation, Window};
use openstudio_gpu::GpuDeviceInfo;

pub fn show_gpu_dialog(parent: &impl IsA<Window>, gpu_info: &GpuDeviceInfo) {
    let dialog = Window::builder()
        .transient_for(parent)
        .modal(true)
        .title("Diagnóstico de Aceleração por GPU")
        .default_width(450)
        .default_height(320)
        .build();

    dialog.add_css_class("main-window");

    let content = Box::new(Orientation::Vertical, 10);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(16);
    content.set_margin_end(16);

    let title = Label::new(Some("⚡ Aceleração por GPU Ativada"));
    title.add_css_class("lcd-bpm");
    content.append(&title);

    let vendor_lbl = Label::new(Some(&format!("Fabricante: {}", gpu_info.vendor.name())));
    vendor_lbl.set_xalign(0.0);
    content.append(&vendor_lbl);

    let model_lbl = Label::new(Some(&format!("Dispositivo: {}", gpu_info.model_name)));
    model_lbl.set_xalign(0.0);
    content.append(&model_lbl);

    let pipeline_lbl = Label::new(Some(&format!("Pipeline de Renderização: {}", gpu_info.recommended_pipeline)));
    pipeline_lbl.set_xalign(0.0);
    content.append(&pipeline_lbl);

    let hints_title = Label::new(Some("Otimizações de Hardware Aplicadas:"));
    hints_title.set_xalign(0.0);
    hints_title.add_css_class("lcd-time");
    content.append(&hints_title);

    for hint in &gpu_info.optimization_hints {
        let hint_lbl = Label::new(Some(&format!("  ✔ {}", hint)));
        hint_lbl.set_xalign(0.0);
        content.append(&hint_lbl);
    }

    let close_btn = Button::with_label("Fechar");
    close_btn.add_css_class("transport-btn");
    let dialog_close = dialog.clone();
    close_btn.connect_clicked(move |_| {
        dialog_close.close();
    });
    content.append(&close_btn);

    dialog.set_child(Some(&content));
    dialog.present();
}
