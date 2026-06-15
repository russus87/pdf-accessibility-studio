//! Motore vocale neurale Piper (Fase B del lettore vocale).
//!
//! Piper non e' incorporato nel binario: viene scaricato a runtime nella
//! cartella dati dell'app (engine self-contained con onnxruntime incluso), e
//! cosi' anche i singoli modelli vocali. La sintesi avviene richiamando il
//! binario, che produce un WAV restituito alla UI in base64.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use serde::Serialize;
use tauri::{AppHandle, Manager};

/// Bundle dell'engine (binario + onnxruntime + espeak-ng-data) per piattaforma.
/// Tutti gli archivi si estraggono in una cartella `piper/`.
const ENGINE_BASE: &str = "https://github.com/rhasspy/piper/releases/download/2023.11.14-2";
/// Base dei modelli vocali su Hugging Face.
const VOCI_BASE: &str = "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0";

/// URL dell'engine per la piattaforma corrente (None se non supportata).
fn engine_url() -> Option<String> {
    let nome = if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "piper_linux_x86_64.tar.gz"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "piper_macos_x64.tar.gz"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "piper_macos_aarch64.tar.gz"
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "piper_windows_amd64.zip"
    } else {
        return None;
    };
    Some(format!("{ENGINE_BASE}/{nome}"))
}

/// Nome del binario piper (con estensione su Windows).
fn nome_bin() -> &'static str {
    if cfg!(windows) {
        "piper.exe"
    } else {
        "piper"
    }
}

/// Una voce del catalogo scaricabile.
struct VoceCat {
    id: &'static str,
    nome: &'static str,
    lingua: &'static str,
    qualita: &'static str,
    mb: u32,
    percorso: &'static str,
}

const CATALOGO: &[VoceCat] = &[
    VoceCat { id: "it_IT-paola-medium", nome: "Paola", lingua: "it", qualita: "medium", mb: 64, percorso: "it/it_IT/paola/medium" },
    VoceCat { id: "it_IT-riccardo-x_low", nome: "Riccardo", lingua: "it", qualita: "x_low", mb: 28, percorso: "it/it_IT/riccardo/x_low" },
    VoceCat { id: "en_US-amy-medium", nome: "Amy", lingua: "en", qualita: "medium", mb: 64, percorso: "en/en_US/amy/medium" },
];

#[derive(Serialize)]
pub struct VoceInfo {
    pub id: String,
    pub nome: String,
    pub lingua: String,
    pub qualita: String,
    pub mb: u32,
    pub installata: bool,
}

#[derive(Serialize)]
pub struct StatoPiper {
    /// Piattaforma supportata per il download automatico dell'engine.
    pub supportato: bool,
    /// Engine scaricato e pronto.
    pub engine_pronto: bool,
    /// Catalogo voci con stato di installazione.
    pub voci: Vec<VoceInfo>,
}

fn supportato() -> bool {
    engine_url().is_some()
}

fn dir_piper(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|d| d.join("piper"))
        .map_err(|e| format!("cartella dati non disponibile: {e}"))
}

fn dir_engine(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(dir_piper(app)?.join("engine"))
}

fn dir_voci(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(dir_piper(app)?.join("voci"))
}

/// Percorso atteso del binario piper (l'archivio si estrae in `engine/piper/`).
fn bin_piper(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(dir_engine(app)?.join("piper").join(nome_bin()))
}

fn modello_voce(app: &AppHandle, id: &str) -> Result<PathBuf, String> {
    Ok(dir_voci(app)?.join(format!("{id}.onnx")))
}

/// Stato di Piper: piattaforma, engine, voci installate.
#[tauri::command]
pub fn piper_stato(app: AppHandle) -> StatoPiper {
    let engine_pronto = bin_piper(&app).map(|p| p.exists()).unwrap_or(false);
    let voci = CATALOGO
        .iter()
        .map(|v| VoceInfo {
            id: v.id.to_string(),
            nome: v.nome.to_string(),
            lingua: v.lingua.to_string(),
            qualita: v.qualita.to_string(),
            mb: v.mb,
            installata: modello_voce(&app, v.id).map(|p| p.exists()).unwrap_or(false),
        })
        .collect();
    StatoPiper { supportato: supportato(), engine_pronto, voci }
}

/// Scarica un URL e ritorna i byte (segue i redirect).
async fn scarica(url: &str) -> Result<Vec<u8>, String> {
    let resp = reqwest::get(url).await.map_err(|e| format!("download: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("download fallito: HTTP {}", resp.status()));
    }
    Ok(resp.bytes().await.map_err(|e| format!("lettura: {e}"))?.to_vec())
}

/// Estrae un archivio ZIP (engine di Windows) nella cartella `dest`.
#[cfg(target_os = "windows")]
fn estrai_zip(bytes: &[u8], dest: &std::path::Path) -> Result<(), String> {
    let reader = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(reader).map_err(|e| format!("zip: {e}"))?;
    zip.extract(dest).map_err(|e| format!("estrazione zip: {e}"))?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn estrai_zip(_bytes: &[u8], _dest: &std::path::Path) -> Result<(), String> {
    Err("estrazione ZIP non prevista su questa piattaforma".into())
}

/// Scarica ed estrae l'engine Piper nella cartella dati.
#[tauri::command]
pub async fn piper_scarica_engine(app: AppHandle) -> Result<(), String> {
    let url = engine_url().ok_or("download automatico di Piper non supportato su questa piattaforma")?;
    let dest = dir_engine(&app)?;
    let bytes = scarica(&url).await?;

    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    if url.ends_with(".zip") {
        estrai_zip(&bytes, &dest)?;
    } else {
        let dec = flate2::read::GzDecoder::new(&bytes[..]);
        let mut ar = tar::Archive::new(dec);
        ar.unpack(&dest).map_err(|e| format!("estrazione: {e}"))?;
    }

    // Assicura il bit di esecuzione sul binario.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(bin) = bin_piper(&app) {
            if let Ok(meta) = std::fs::metadata(&bin) {
                let mut perm = meta.permissions();
                perm.set_mode(0o755);
                let _ = std::fs::set_permissions(&bin, perm);
            }
        }
    }
    Ok(())
}

/// Scarica un modello vocale (file .onnx e .onnx.json).
#[tauri::command]
pub async fn piper_scarica_voce(app: AppHandle, voce: String) -> Result<(), String> {
    let cat = CATALOGO
        .iter()
        .find(|v| v.id == voce)
        .ok_or_else(|| "voce sconosciuta".to_string())?;
    let dir = dir_voci(&app)?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let url_onnx = format!("{VOCI_BASE}/{}/{}.onnx?download=true", cat.percorso, cat.id);
    let url_json = format!("{VOCI_BASE}/{}/{}.onnx.json?download=true", cat.percorso, cat.id);

    let onnx = scarica(&url_onnx).await?;
    let json = scarica(&url_json).await?;
    std::fs::write(dir.join(format!("{}.onnx", cat.id)), onnx).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(format!("{}.onnx.json", cat.id)), json).map_err(|e| e.to_string())?;
    Ok(())
}

/// Contatore per nomi di file temporanei univoci tra chiamate concorrenti.
static SEQ: AtomicU64 = AtomicU64::new(0);

/// Sintetizza `testo` con la voce data e ritorna un WAV in base64.
#[tauri::command]
pub fn piper_sintesi(app: AppHandle, testo: String, voce: String, velocita: f32) -> Result<String, String> {
    let bin = bin_piper(&app)?;
    if !bin.exists() {
        return Err("motore Piper non installato".into());
    }
    let modello = modello_voce(&app, &voce)?;
    if !modello.exists() {
        return Err("voce Piper non installata".into());
    }
    let engine_dir = bin.parent().ok_or("percorso engine non valido")?.to_path_buf();

    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let out = std::env::temp_dir().join(format!("piper_{}_{}.wav", std::process::id(), n));

    // length_scale > 1 = piu' lento; lo deriviamo dalla velocita' relativa.
    let length_scale = (1.0 / velocita.max(0.1)).clamp(0.3, 3.0);

    let mut cmd = Command::new(&bin);
    cmd.current_dir(&engine_dir);
    // Le librerie native (onnxruntime, espeak) stanno accanto al binario.
    #[cfg(target_os = "linux")]
    cmd.env("LD_LIBRARY_PATH", &engine_dir);
    #[cfg(target_os = "macos")]
    cmd.env("DYLD_LIBRARY_PATH", &engine_dir);
    // Su Windows le DLL vengono trovate nella cartella dell'eseguibile.

    let mut child = cmd
        .arg("--model")
        .arg(&modello)
        .arg("--output_file")
        .arg(&out)
        .arg("--length_scale")
        .arg(format!("{length_scale:.3}"))
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("avvio piper: {e}"))?;

    child
        .stdin
        .take()
        .ok_or("stdin piper non disponibile")?
        .write_all(testo.as_bytes())
        .map_err(|e| format!("scrittura testo: {e}"))?;

    let status = child.wait().map_err(|e| format!("attesa piper: {e}"))?;
    if !status.success() {
        let _ = std::fs::remove_file(&out);
        return Err("piper ha restituito un errore".into());
    }

    let wav = std::fs::read(&out).map_err(|e| format!("lettura WAV: {e}"))?;
    let _ = std::fs::remove_file(&out);
    Ok(STANDARD.encode(wav))
}
