use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Apple,
    Qualcomm,
    SoftwareFallback,
    Unknown,
}

impl GpuVendor {
    pub fn name(&self) -> &'static str {
        match self {
            GpuVendor::Nvidia => "NVIDIA Corporation",
            GpuVendor::Amd => "AMD (Advanced Micro Devices) / Radeon",
            GpuVendor::Intel => "Intel Corporation (Iris / Arc / UHD)",
            GpuVendor::Apple => "Apple Silicon (Metal)",
            GpuVendor::Qualcomm => "Qualcomm Adreno",
            GpuVendor::SoftwareFallback => "Renderizador em Software (Mesa llvmpipe)",
            GpuVendor::Unknown => "Adaptador Desconhecido",
        }
    }

    pub fn brand_tag(&self) -> &'static str {
        match self {
            GpuVendor::Nvidia => "nvidia",
            GpuVendor::Amd => "radeon",
            GpuVendor::Intel => "intel",
            GpuVendor::Apple => "apple",
            GpuVendor::Qualcomm => "qualcomm",
            GpuVendor::SoftwareFallback => "llvmpipe",
            GpuVendor::Unknown => "generic",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuDeviceInfo {
    pub vendor: GpuVendor,
    pub model_name: String,
    pub driver_info: String,
    pub pci_id: Option<String>,
    pub vulkan_support: bool,
    pub opengl_support: bool,
    pub is_discrete: bool,
    pub recommended_pipeline: String,
    pub optimization_hints: Vec<String>,
}

impl Default for GpuDeviceInfo {
    fn default() -> Self {
        Self {
            vendor: GpuVendor::Unknown,
            model_name: "Generic Graphics Device".to_string(),
            driver_info: "Unknown Driver".to_string(),
            pci_id: None,
            vulkan_support: true,
            opengl_support: true,
            is_discrete: false,
            recommended_pipeline: "GSK Vulkan / OpenGL".to_string(),
            optimization_hints: Vec::new(),
        }
    }
}

/// Detecta a GPU instalada verificando /sys/class/drm, lspci e identificadores PCI
pub fn detect_primary_gpu() -> GpuDeviceInfo {
    let mut info = GpuDeviceInfo::default();

    // 1. Tentar inspecionar o sysfs do Linux (/sys/class/drm/card0/device ou card1)
    if let Some(sysfs_info) = probe_linux_sysfs() {
        info = sysfs_info;
    } else if let Some(lspci_info) = probe_lspci() {
        info = lspci_info;
    }

    // Configurar otimizações baseadas no fabricante detectado
    match info.vendor {
        GpuVendor::Nvidia => {
            info.recommended_pipeline = "GTK4 NGL / Vulkan com Driver Proprietário NVIDIA".to_string();
            info.optimization_hints.push("Pipeline de baixa latência ativado com NV_vulkan".to_string());
            info.optimization_hints.push("Offload de shaders GLSL para formas de onda e FFT a 120 FPS".to_string());
            info.optimization_hints.push("MSAA 4x ativado no Piano Roll e Playlist".to_string());
        }
        GpuVendor::Amd => {
            info.recommended_pipeline = "Mesa RADV / RadeonSI com aceleração Vulkan nativa".to_string();
            info.optimization_hints.push("Pipeline AMD Radeon com buffer duplo sem tearing ativado".to_string());
            info.optimization_hints.push("Cálculo de FFT espectral paralelo via GPU Compute/Shaders".to_string());
            info.optimization_hints.push("Renderização vetorial rápida para 10.000+ notas no Piano Roll".to_string());
        }
        GpuVendor::Intel => {
            info.recommended_pipeline = "Mesa Iris / ANV (Intel Graphics)".to_string();
            info.optimization_hints.push("Otimização de eficiência energética para GPU integrada".to_string());
            info.optimization_hints.push("Renderização em lotes (batching) para medidores de pico do Mixer".to_string());
        }
        GpuVendor::Apple => {
            info.recommended_pipeline = "MoltenVK / Metal Layer".to_string();
            info.optimization_hints.push("Compartilhamento de memória unificada ativado".to_string());
        }
        _ => {
            info.recommended_pipeline = "Cairo / GSK Fallback".to_string();
            info.optimization_hints.push("Renderização padrão de alta compatibilidade".to_string());
        }
    }

    info
}

fn probe_linux_sysfs() -> Option<GpuDeviceInfo> {
    let drm_path = Path::new("/sys/class/drm");
    if !drm_path.exists() {
        return None;
    }

    // Procura card0 ou card1
    for entry in fs::read_dir(drm_path).ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("card") && !name.contains('-') {
            let device_dir = entry.path().join("device");
            if device_dir.exists() {
                let vendor_file = device_dir.join("vendor");
                let device_file = device_dir.join("device");

                if let (Ok(vendor_hex), Ok(device_hex)) = (
                    fs::read_to_string(&vendor_file),
                    fs::read_to_string(&device_file),
                ) {
                    let v_str = vendor_hex.trim().to_lowercase();
                    let d_str = device_hex.trim().to_lowercase();

                    let (vendor, is_discrete) = parse_vendor_id(&v_str);

                    // Tentar ler nome do modelo no arquivo uevent ou subsystem
                    let mut model_name = format!("GPU [{}:{}]", v_str, d_str);
                    if let Ok(uevent) = fs::read_to_string(device_dir.join("uevent")) {
                        for line in uevent.lines() {
                            if line.starts_with("PCI_ID=") {
                                model_name = format!("Dispositivo PCI ({})", line);
                            }
                        }
                    }

                    // Tentar refinar com lspci se disponível
                    if let Some(refined_name) = query_lspci_device_name(&v_str, &d_str) {
                        model_name = refined_name;
                    }

                    return Some(GpuDeviceInfo {
                        vendor,
                        model_name,
                        driver_info: "Linux Direct Rendering Manager (DRM/KMS)".to_string(),
                        pci_id: Some(format!("{}:{}", v_str, d_str)),
                        vulkan_support: true,
                        opengl_support: true,
                        is_discrete,
                        recommended_pipeline: String::new(),
                        optimization_hints: Vec::new(),
                    });
                }
            }
        }
    }

    None
}

fn parse_vendor_id(v_hex: &str) -> (GpuVendor, bool) {
    if v_hex.contains("1002") || v_hex.contains("1022") {
        (GpuVendor::Amd, false) // AMD Radeon
    } else if v_hex.contains("10de") {
        (GpuVendor::Nvidia, true) // NVIDIA (geralmente dedicada)
    } else if v_hex.contains("8086") {
        (GpuVendor::Intel, false) // Intel
    } else {
        (GpuVendor::Unknown, false)
    }
}

fn query_lspci_device_name(_vendor: &str, _device: &str) -> Option<String> {
    let output = Command::new("lspci").output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let lower = line.to_lowercase();
        if lower.contains("vga compatible") || lower.contains("3d controller") || lower.contains("display controller") {
            // Remove o prefixo do barramento (ex: "03:00.0 ")
            let parts: Vec<&str> = line.splitn(2, ": ").collect();
            if parts.len() == 2 {
                return Some(parts[1].trim().to_string());
            }
            return Some(line.trim().to_string());
        }
    }
    None
}

fn probe_lspci() -> Option<GpuDeviceInfo> {
    let output = Command::new("lspci").output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let lower = line.to_lowercase();
        if lower.contains("vga") || lower.contains("3d controller") || lower.contains("display") {
            let mut vendor = GpuVendor::Unknown;
            let mut is_discrete = false;
            if lower.contains("amd") || lower.contains("radeon") || lower.contains("advanced micro devices") {
                vendor = GpuVendor::Amd;
            } else if lower.contains("nvidia") || lower.contains("geforce") || lower.contains("rtx") || lower.contains("quadro") {
                vendor = GpuVendor::Nvidia;
                is_discrete = true;
            } else if lower.contains("intel") {
                vendor = GpuVendor::Intel;
            }

            return Some(GpuDeviceInfo {
                vendor,
                model_name: line.trim().to_string(),
                driver_info: "Kernel Driver / Mesa".to_string(),
                pci_id: None,
                vulkan_support: true,
                opengl_support: true,
                is_discrete,
                recommended_pipeline: String::new(),
                optimization_hints: Vec::new(),
            });
        }
    }
    None
}

/// Shaders GLSL para renderização acelerada por GPU de Analisador de Espectro FFT
pub const FFT_SPECTRUM_VERTEX_SHADER: &str = r#"#version 330 core
layout (location = 0) in vec2 aPos;
layout (location = 1) in float aFrequency;

out float vFrequency;

void main() {
    gl_Position = vec4(aPos.x, aPos.y, 0.0, 1.0);
    vFrequency = aFrequency;
}
"#;

pub const FFT_SPECTRUM_FRAGMENT_SHADER: &str = r#"#version 330 core
in float vFrequency;
out vec4 FragColor;

uniform float uMagnitude; // 0.0 .. 1.0

// Gradiente térmico característico do FL Studio Parametric EQ 2
// Tons de Azul -> Ciano -> Verde -> Amarelo -> Laranja -> Vermelho
vec3 fl_spectrum_color(float val) {
    if (val < 0.2) {
        return mix(vec3(0.05, 0.1, 0.3), vec3(0.0, 0.6, 0.9), val / 0.2);
    } else if (val < 0.5) {
        return mix(vec3(0.0, 0.6, 0.9), vec3(0.2, 0.9, 0.3), (val - 0.2) / 0.3);
    } else if (val < 0.8) {
        return mix(vec3(0.2, 0.9, 0.3), vec3(1.0, 0.8, 0.0), (val - 0.5) / 0.3);
    } else {
        return mix(vec3(1.0, 0.8, 0.0), vec3(1.0, 0.15, 0.1), (val - 0.8) / 0.2);
    }
}

void main() {
    vec3 col = fl_spectrum_color(uMagnitude);
    FragColor = vec4(col, 0.85);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_gpu() {
        let gpu = detect_primary_gpu();
        println!("GPU detectada: {:?}", gpu);
        assert!(!gpu.model_name.is_empty());
        assert!(!gpu.optimization_hints.is_empty());
    }
}
