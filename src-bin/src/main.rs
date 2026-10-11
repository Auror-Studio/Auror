use openstudio_gpu::detect_primary_gpu;
use openstudio_plugins::WindowsBridgeManager;
use openstudio_ui::OpenStudioApp;

fn main() {
    env_logger::init();

    println!("=======================================================");
    println!(" Aurora Studio - DAW de Alta Performance em Rust & GTK4");
    println!(" Compatível com o fluxo de trabalho do FL Studio       ");
    println!("=======================================================");

    // 1. Diagnóstico de Hardware e GPU
    let gpu = detect_primary_gpu();
    println!("[GPU] Fabricante: {}", gpu.vendor.name());
    println!("[GPU] Modelo: {}", gpu.model_name);
    println!("[GPU] Pipeline Recomendado: {}", gpu.recommended_pipeline);
    for hint in &gpu.optimization_hints {
        println!("  - {}", hint);
    }

    // 2. Diagnóstico de Compatibilidade de Plugins Windows
    println!("\n[PLUGINS] {}", WindowsBridgeManager::status_report());

    println!("\nIniciando interface gráfica GTK4...");

    let app = OpenStudioApp::new();
    app.run();
}
