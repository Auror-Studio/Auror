use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginFormat {
    NativeBuiltin,
    VST3,
    CLAP,
    LV2,
    WindowsBridgedVST3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginParameter {
    pub id: u32,
    pub name: String,
    pub min: f32,
    pub max: f32,
    pub default_val: f32,
    pub current_val: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDescriptor {
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub version: String,
    pub category: String, // "Synth", "Effect", "Dynamics", "EQ", "Delay"
    pub format: PluginFormat,
    pub path: Option<PathBuf>,
    pub parameters: Vec<PluginParameter>,
}

pub trait AudioPlugin: Send {
    fn info(&self) -> PluginDescriptor;
    fn process_stereo(&mut self, sample_rate: f32, in_l: f32, in_r: f32) -> (f32, f32);
    fn set_parameter(&mut self, id: u32, value: f32);
    fn get_parameter(&self, id: u32) -> f32;
    fn reset(&mut self);
}

/// Registry que cataloga todos os plugins disponíveis (nativos e de terceiros)
pub struct PluginDatabase {
    pub plugins: Vec<PluginDescriptor>,
}

impl Default for PluginDatabase {
    fn default() -> Self {
        let mut db = Self {
            plugins: Vec::new(),
        };
        db.register_builtins();
        db
    }
}

impl PluginDatabase {
    fn register_builtins(&mut self) {
        // Clones dos clássicos do FL Studio
        self.plugins.push(PluginDescriptor {
            id: "org.openstudio.soundgoodizer".to_string(),
            name: "Soundgoodizer".to_string(),
            vendor: "OpenStudio Team".to_string(),
            version: "1.0.0".to_string(),
            category: "Dynamics".to_string(),
            format: PluginFormat::NativeBuiltin,
            path: None,
            parameters: vec![
                PluginParameter {
                    id: 0,
                    name: "Mode (A/B/C/D)".to_string(),
                    min: 0.0,
                    max: 3.0,
                    default_val: 0.0,
                    current_val: 0.0,
                },
                PluginParameter {
                    id: 1,
                    name: "Amount".to_string(),
                    min: 0.0,
                    max: 1.0,
                    default_val: 0.5,
                    current_val: 0.5,
                },
            ],
        });

        self.plugins.push(PluginDescriptor {
            id: "org.openstudio.parametriceq2".to_string(),
            name: "Fruity Parametric EQ 2".to_string(),
            vendor: "OpenStudio Team".to_string(),
            version: "2.0.0".to_string(),
            category: "EQ".to_string(),
            format: PluginFormat::NativeBuiltin,
            path: None,
            parameters: vec![],
        });

        self.plugins.push(PluginDescriptor {
            id: "org.openstudio.3xosc".to_string(),
            name: "3x Osc".to_string(),
            vendor: "OpenStudio Team".to_string(),
            version: "1.0.0".to_string(),
            category: "Synth".to_string(),
            format: PluginFormat::NativeBuiltin,
            path: None,
            parameters: vec![],
        });

        self.plugins.push(PluginDescriptor {
            id: "org.openstudio.fruitydelay3".to_string(),
            name: "Fruity Delay 3".to_string(),
            vendor: "OpenStudio Team".to_string(),
            version: "3.0.0".to_string(),
            category: "Delay".to_string(),
            format: PluginFormat::NativeBuiltin,
            path: None,
            parameters: vec![],
        });
    }

    /// Varre diretórios padrão do sistema operacional em busca de plugins VST3 e CLAP
    pub fn scan_system_plugins(&mut self) {
        let mut paths_to_scan = Vec::new();

        // Diretórios padrão de VST3 no Linux
        if let Some(home) = std::env::var_os("HOME") {
            let home_path = PathBuf::from(home);
            paths_to_scan.push(home_path.join(".vst3"));
            paths_to_scan.push(home_path.join(".clap"));
        }
        paths_to_scan.push(PathBuf::from("/usr/lib/vst3"));
        paths_to_scan.push(PathBuf::from("/usr/local/lib/vst3"));
        paths_to_scan.push(PathBuf::from("/usr/lib/clap"));

        for base_path in paths_to_scan {
            if base_path.exists() {
                self.scan_folder(&base_path);
            }
        }
    }

    fn scan_folder(&mut self, folder: &Path) {
        if let Ok(entries) = std::fs::read_dir(folder) {
            for entry in entries.flatten() {
                let path = entry.path();
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let file_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Plugin");

                if ext.eq_ignore_ascii_case("vst3") {
                    self.plugins.push(PluginDescriptor {
                        id: format!("vst3.{}", file_name),
                        name: file_name.to_string(),
                        vendor: "External VST3".to_string(),
                        version: "1.0".to_string(),
                        category: "Effect/Instrument".to_string(),
                        format: PluginFormat::VST3,
                        path: Some(path),
                        parameters: Vec::new(),
                    });
                } else if ext.eq_ignore_ascii_case("clap") {
                    self.plugins.push(PluginDescriptor {
                        id: format!("clap.{}", file_name),
                        name: file_name.to_string(),
                        vendor: "External CLAP".to_string(),
                        version: "1.0".to_string(),
                        category: "Effect/Instrument".to_string(),
                        format: PluginFormat::CLAP,
                        path: Some(path),
                        parameters: Vec::new(),
                    });
                }
            }
        }
    }
}

/// Gerenciador de compatibilidade com plugins Windows (Wine / yabridge)
pub struct WindowsBridgeManager;

impl WindowsBridgeManager {
    /// Verifica se o yabridge e Wine estão instalados no sistema operacional
    pub fn is_yabridge_available() -> bool {
        Command::new("which")
            .arg("yabridgectl")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub fn is_wine_available() -> bool {
        Command::new("which")
            .arg("wine")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub fn status_report() -> String {
        let yabridge = Self::is_yabridge_available();
        let wine = Self::is_wine_available();

        format!(
            "Suporte a Plugins Windows no Linux:\n- Wine: {}\n- Yabridge: {}\n- Status: {}",
            if wine { "Instalado" } else { "Não encontrado" },
            if yabridge { "Instalado" } else { "Não encontrado" },
            if wine && yabridge {
                "Pronto para hospedar plugins Windows VST3 de 64-bit!"
            } else {
                "Instale wine e yabridge para rodar plugins Windows nativamente."
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_database() {
        let db = PluginDatabase::default();
        assert!(db.plugins.iter().any(|p| p.name == "Soundgoodizer"));
        assert!(db.plugins.iter().any(|p| p.name == "Fruity Parametric EQ 2"));
    }
}
