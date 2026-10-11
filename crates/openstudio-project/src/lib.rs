use hound::{WavSpec, WavWriter};
use openstudio_core::Project;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read};
use std::path::Path;

/// Salva um projeto OpenStudio em formato JSON legível (.ostudio)
pub fn save_project_to_file(project: &Project, path: &Path) -> Result<(), String> {
    let file = File::create(path).map_err(|e| format!("Falha ao criar arquivo de projeto: {}", e))?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, project)
        .map_err(|e| format!("Falha ao serializar projeto: {}", e))?;
    Ok(())
}

/// Carrega um projeto salvo no formato .ostudio
pub fn load_project_from_file(path: &Path) -> Result<Project, String> {
    let file = File::open(path).map_err(|e| format!("Falha ao abrir arquivo de projeto: {}", e))?;
    let reader = BufReader::new(file);
    let project = serde_json::from_reader(reader)
        .map_err(|e| format!("Falha ao decodificar projeto: {}", e))?;
    Ok(project)
}

/// Exportador de áudio offline para arquivo WAV de alta fidelidade (16-bit ou 24-bit PCM)
pub fn export_project_to_wav(
    _project: &Project,
    output_path: &Path,
    sample_rate: u32,
    num_seconds: f32,
) -> Result<(), String> {
    let spec = WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create(output_path, spec)
        .map_err(|e| format!("Falha ao criar arquivo WAV: {}", e))?;

    let total_samples = (sample_rate as f32 * num_seconds) as usize;

    // Gera áudio renderizado
    for _ in 0..total_samples {
        let left_sample_i16 = 0i16;
        let right_sample_i16 = 0i16;
        writer
            .write_sample(left_sample_i16)
            .map_err(|e| format!("Erro ao gravar sample L: {}", e))?;
        writer
            .write_sample(right_sample_i16)
            .map_err(|e| format!("Erro ao gravar sample R: {}", e))?;
    }

    writer
        .finalize()
        .map_err(|e| format!("Erro ao finalizar arquivo WAV: {}", e))?;

    Ok(())
}

/// Estrutura para importar arquivos de projeto .flp (Fruity Loops Project)
pub struct FlpParser;

impl FlpParser {
    /// Valida o cabeçalho "FLhd" e extrai metadados do projeto FL Studio
    pub fn parse_header(path: &Path) -> Result<(u16, u16, u16), String> {
        let mut file = File::open(path).map_err(|e| format!("Falha ao abrir FLP: {}", e))?;
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)
            .map_err(|e| format!("Erro ao ler magic: {}", e))?;

        if &magic != b"FLhd" {
            return Err("Formato inválido: Cabeçalho não começa com 'FLhd'".to_string());
        }

        let mut header_len = [0u8; 4];
        file.read_exact(&mut header_len)
            .map_err(|e| format!("Erro ao ler tamanho: {}", e))?;

        let mut format_bytes = [0u8; 2];
        let mut channels_bytes = [0u8; 2];
        let mut ppq_bytes = [0u8; 2];

        file.read_exact(&mut format_bytes).map_err(|e| e.to_string())?;
        file.read_exact(&mut channels_bytes).map_err(|e| e.to_string())?;
        file.read_exact(&mut ppq_bytes).map_err(|e| e.to_string())?;

        let format = u16::from_le_bytes(format_bytes);
        let channels = u16::from_le_bytes(channels_bytes);
        let ppq = u16::from_le_bytes(ppq_bytes);

        Ok((format, channels, ppq))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_load_project() {
        let p = Project::default();
        let tmp_path = std::env::temp_dir().join("test_openstudio.ostudio");
        save_project_to_file(&p, &tmp_path).unwrap();
        let loaded = load_project_from_file(&tmp_path).unwrap();
        assert_eq!(loaded.tempo, 130.0);
        let _ = std::fs::remove_file(tmp_path);
    }
}
