//! Libreria firme (Fase 25): salva le immagini di firma (PNG) nella cartella dati
//! dell'app, così possono essere riapplicate ai PDF senza reimportarle ogni volta.

use std::path::PathBuf;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::Serialize;
use tauri::Manager;

/// Una firma salvata: nome e immagine PNG in base64 (per anteprima e riuso).
#[derive(Serialize)]
pub struct Firma {
    pub nome: String,
    pub dati_base64: String,
}

/// Cartella delle firme, dentro la config dir dell'app.
fn cartella(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("firme"))
}

/// Rende un nome sicuro per il filesystem (solo lettere, cifre, spazi, - e _).
fn nome_sicuro(nome: &str) -> String {
    let pulito: String = nome
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let t = pulito.trim();
    if t.is_empty() { "firma".to_string() } else { t.to_string() }
}

/// Elenca le firme salvate (nome + PNG base64).
#[tauri::command]
pub fn firme_elenco(app: tauri::AppHandle) -> Vec<Firma> {
    let Some(dir) = cartella(&app) else { return Vec::new() };
    let Ok(voci) = std::fs::read_dir(&dir) else { return Vec::new() };
    let mut out = Vec::new();
    for v in voci.flatten() {
        let p = v.path();
        if p.extension().and_then(|e| e.to_str()) != Some("png") {
            continue;
        }
        let Some(nome) = p.file_stem().and_then(|s| s.to_str()) else { continue };
        if let Ok(bytes) = std::fs::read(&p) {
            out.push(Firma { nome: nome.to_string(), dati_base64: STANDARD.encode(&bytes) });
        }
    }
    out.sort_by(|a, b| a.nome.cmp(&b.nome));
    out
}

/// Salva (o sovrascrive) una firma dato il PNG in base64.
#[tauri::command]
pub fn firma_salva(app: tauri::AppHandle, nome: String, dati_base64: String) -> Result<(), String> {
    let dir = cartella(&app).ok_or("config dir non disponibile")?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let bytes = STANDARD.decode(dati_base64.trim()).map_err(|e| format!("PNG non valido: {e}"))?;
    let file = dir.join(format!("{}.png", nome_sicuro(&nome)));
    std::fs::write(file, bytes).map_err(|e| e.to_string())
}

/// Elimina una firma per nome.
#[tauri::command]
pub fn firma_elimina(app: tauri::AppHandle, nome: String) -> Result<(), String> {
    let dir = cartella(&app).ok_or("config dir non disponibile")?;
    let file = dir.join(format!("{}.png", nome_sicuro(&nome)));
    if file.exists() {
        std::fs::remove_file(file).map_err(|e| e.to_string())?;
    }
    Ok(())
}
