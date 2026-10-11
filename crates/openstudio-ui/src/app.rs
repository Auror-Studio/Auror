use gtk4::gdk::Display;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box, CssProvider, Orientation, Stack,
    StackTransitionType,
};
use openstudio_audio::{start_audio_engine, AudioEngineHandle};
use openstudio_core::{AudioToUiMsg, PlaybackMode, Project, TICKS_PER_STEP};
use openstudio_gpu::{detect_primary_gpu, GpuDeviceInfo};
use openstudio_plugins::PluginDatabase;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::browser::BrowserWidget;
use crate::channel_rack::ChannelRackWidget;
use crate::gpu_dialog::show_gpu_dialog;
use crate::mixer::MixerWidget;
use crate::piano_roll::PianoRollWidget;
use crate::playlist::PlaylistWidget;
use crate::shortcuts::{install_fl_shortcuts, ShortcutContext};
use crate::styles::FL_STUDIO_CSS;
use crate::transport::TransportBar;

pub struct OpenStudioApp {
    pub project: Rc<RefCell<Project>>,
    pub gpu_info: GpuDeviceInfo,
    pub plugin_db: PluginDatabase,
}

impl OpenStudioApp {
    pub fn new() -> Self {
        let gpu_info = detect_primary_gpu();
        let mut plugin_db = PluginDatabase::default();
        plugin_db.scan_system_plugins();

        Self {
            project: Rc::new(RefCell::new(Project::default())),
            gpu_info,
            plugin_db,
        }
    }

    pub fn run(&self) {
        let app = Application::builder()
            .application_id("org.openstudio.daw")
            .build();

        let project_rc = self.project.clone();
        let gpu_info_clone = self.gpu_info.clone();
        let plugin_db_clone = PluginDatabase::default();

        app.connect_startup(|_| {
            load_css();
        });

        app.connect_activate(move |app| {
            build_ui(app, project_rc.clone(), &gpu_info_clone, &plugin_db_clone);
        });

        app.run();
    }
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_data(FL_STUDIO_CSS);

    if let Some(display) = Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn build_ui(
    app: &Application,
    project: Rc<RefCell<Project>>,
    gpu_info: &GpuDeviceInfo,
    plugin_db: &PluginDatabase,
) {
    let initial_project = project.borrow().clone();

    // 1. Inicializa o motor de áudio em tempo real e GARANTE QUE ELE NUNCA SEJA DESTRUÍDO
    let audio_handle = match start_audio_engine(initial_project) {
        Ok(handle) => handle,
        Err(e) => {
            eprintln!("Aviso: Não foi possível iniciar o motor de áudio: {}", e);
            AudioEngineHandle::dummy()
        }
    };

    let tx = audio_handle.ui_to_audio_tx.clone();
    let rx = audio_handle.audio_to_ui_rx.clone();
    // Guardamos o handle em um Rc para que a thread de áudio e o stream CPAL fiquem vivos permanentemente
    let audio_holder = Rc::new(RefCell::new(Some(audio_handle)));

    let window = ApplicationWindow::builder()
        .application(app)
        .title("OpenStudio - [Novo Projeto.ostudio]")
        .default_width(1280)
        .default_height(820)
        .build();

    window.add_css_class("main-window");

    let root_v_box = Box::new(Orientation::Vertical, 0);

    // 2. Barra de Transporte Superior
    let initial_bpm = project.borrow().tempo;
    let transport = TransportBar::new(tx.clone(), gpu_info, initial_bpm);
    root_v_box.append(&transport.container);

    // 3. Área Central de Trabalho (Navegador na Esquerda + Stack Modular estilo FL Studio)
    let work_area = Box::new(Orientation::Horizontal, 4);
    work_area.add_css_class("workspace-box");

    let browser = BrowserWidget::new(plugin_db);
    work_area.append(&browser.container);

    // Stack central sem abas feias - troca fluida entre Channel Rack, Piano Roll, Playlist e Mixer
    let stack = Stack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);
    stack.set_transition_type(StackTransitionType::Crossfade);

    let channel_rack = ChannelRackWidget::new(project.clone(), tx.clone());
    let piano_roll = PianoRollWidget::new(project.clone(), tx.clone());
    let playlist = PlaylistWidget::new(project.clone(), tx.clone());
    let mixer = MixerWidget::new(project.clone(), tx.clone());

    stack.add_named(&channel_rack.container, Some("channel_rack"));
    stack.add_named(&piano_roll.container, Some("piano_roll"));
    stack.add_named(&playlist.container, Some("playlist"));
    stack.add_named(&mixer.container, Some("mixer"));

    // Channel Rack como visão inicial
    stack.set_visible_child_name("channel_rack");

    work_area.append(&stack);
    root_v_box.append(&work_area);

    window.set_child(Some(&root_v_box));

    // 4. Conexão dos botões da barra superior com destaque visual ativo
    let f5_btn = transport.f5_btn.clone();
    let f6_btn = transport.f6_btn.clone();
    let f7_btn = transport.f7_btn.clone();
    let f9_btn = transport.f9_btn.clone();

    let switch_to_view = {
        let stack = stack.clone();
        let f5 = f5_btn.clone();
        let f6 = f6_btn.clone();
        let f7 = f7_btn.clone();
        let f9 = f9_btn.clone();

        Rc::new(move |view_name: &str| {
            stack.set_visible_child_name(view_name);
            f5.set_active(view_name == "playlist");
            f6.set_active(view_name == "channel_rack");
            f7.set_active(view_name == "piano_roll");
            f9.set_active(view_name == "mixer");
        })
    };

    let sw_f5 = switch_to_view.clone();
    f5_btn.connect_clicked(move |_| sw_f5("playlist"));

    let sw_f6 = switch_to_view.clone();
    f6_btn.connect_clicked(move |_| sw_f6("channel_rack"));

    let sw_f7 = switch_to_view.clone();
    f7_btn.connect_clicked(move |_| sw_f7("piano_roll"));

    let sw_f9 = switch_to_view.clone();
    f9_btn.connect_clicked(move |_| sw_f9("mixer"));

    // Botão de GPU abre diálogo de aceleração de hardware
    let win_gpu = window.clone();
    let gpu_clone = gpu_info.clone();
    transport.gpu_btn.connect_clicked(move |_| {
        show_gpu_dialog(&win_gpu, &gpu_clone);
    });

    // 5. Instalação dos Atalhos Globais 100% FL Studio
    let is_playing = Rc::new(RefCell::new(false));
    let playback_mode = Rc::new(RefCell::new(PlaybackMode::Pattern));

    let sw_short_f5 = switch_to_view.clone();
    let sw_short_f6 = switch_to_view.clone();
    let sw_short_f7 = switch_to_view.clone();
    let sw_short_f9 = switch_to_view.clone();
    let browser_box = browser.container.clone();

    let shortcut_ctx = ShortcutContext {
        is_playing: is_playing.clone(),
        playback_mode: playback_mode.clone(),
        tx: tx.clone(),
        on_toggle_playlist: Rc::new(move || sw_short_f5("playlist")),
        on_toggle_channel_rack: Rc::new(move || sw_short_f6("channel_rack")),
        on_toggle_piano_roll: Rc::new(move || sw_short_f7("piano_roll")),
        on_toggle_mixer: Rc::new(move || sw_short_f9("mixer")),
        on_toggle_browser: Rc::new(move || {
            let visible = browser_box.is_visible();
            browser_box.set_visible(!visible);
        }),
        on_toggle_ghost_notes: Rc::new(move || {}),
    };

    install_fl_shortcuts(&window, shortcut_ctx);

    // 6. Loop de Telemetria de Áudio (Mantém o Stream Vivo e Atualiza a UI a 60 FPS)
    let time_label = transport.time_label.clone();
    let mixer_ref = mixer;
    let channel_rack_ref = channel_rack;
    let _audio_guard = audio_holder.clone();

    glib::timeout_add_local(Duration::from_millis(16), move || {
        let _keep_alive = &_audio_guard; // Previne que o stream CPAL seja coletado

        while let Ok(msg) = rx.try_recv() {
            match msg {
                AudioToUiMsg::PlayheadTick(tick) => {
                    let total_steps = tick / TICKS_PER_STEP;
                    let bar = (total_steps / 16) + 1;
                    let beat = ((total_steps % 16) / 4) + 1;
                    let step = (total_steps % 4) + 1;
                    time_label.set_text(&format!("{:03}:{:02}:{:02}", bar, beat, step));
                }
                AudioToUiMsg::CurrentStep(step) => {
                    // Animação luminosa do Playhead correndo pelos 16 passos
                    channel_rack_ref.update_playhead(step);
                }
                AudioToUiMsg::IsPlaying(_playing) => {}
                AudioToUiMsg::MixerMeters(meters) => {
                    for (track_id, peak_l, peak_r) in meters {
                        if track_id == 0 {
                            let max_peak = peak_l.max(peak_r);
                            mixer_ref.update_master_meter(max_peak);
                        }
                    }
                }
                AudioToUiMsg::ActiveVoices(_) => {}
                AudioToUiMsg::CpuLoad(_) => {}
            }
        }
        glib::ControlFlow::Continue
    });

    window.present();
}
