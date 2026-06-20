//! Integrazione con l'API Claude per il suggerimento del testo alternativo
//! delle immagini (Claude vision). La chiave API e il modello sono salvati nelle
//! impostazioni locali dell'app.

use std::path::PathBuf;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use tauri::Manager;

/// Impostazioni persistenti dell'app (file JSON nella config dir).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Impostazioni {
    #[serde(default)]
    pub anthropic_api_key: Option<String>,
    #[serde(default = "modello_default")]
    pub modello: String,
    /// File aperti di recente (più recenti in testa, max 10).
    #[serde(default)]
    pub recenti: Vec<String>,
}

impl Default for Impostazioni {
    fn default() -> Self {
        Impostazioni { anthropic_api_key: None, modello: modello_default(), recenti: Vec::new() }
    }
}

/// Registra un file tra i recenti (in testa, senza duplicati, max 10).
pub fn aggiungi_recente(app: &tauri::AppHandle, percorso: &str) {
    let mut i = carica(app);
    i.recenti.retain(|p| p != percorso);
    i.recenti.insert(0, percorso.to_string());
    i.recenti.truncate(10);
    let _ = salva(app, &i);
}

fn modello_default() -> String {
    // Modello capace per impostazione predefinita; modificabile dall'utente
    // (es. claude-haiku-4-5 costa meno per i testi alternativi).
    "claude-opus-4-8".to_string()
}

fn percorso(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("impostazioni.json"))
}

/// Carica le impostazioni (o i default se il file non esiste/è illeggibile).
pub fn carica(app: &tauri::AppHandle) -> Impostazioni {
    percorso(app)
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// Salva le impostazioni nella config dir dell'app.
pub fn salva(app: &tauri::AppHandle, imp: &Impostazioni) -> Result<(), String> {
    let p = percorso(app).ok_or("config dir non disponibile")?;
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let dati = serde_json::to_vec_pretty(imp).map_err(|e| e.to_string())?;
    std::fs::write(p, dati).map_err(|e| e.to_string())
}

/// Invia a Claude un prompt testuale (sistema + messaggio utente) e ritorna la
/// risposta. Usato per riassunto, domande&risposte e traduzione sul documento.
pub async fn chiedi(modello: &str, chiave: &str, sistema: &str, utente: &str) -> Result<String, String> {
    let corpo = serde_json::json!({
        "model": modello,
        "max_tokens": 2048,
        "system": sistema,
        "messages": [{ "role": "user", "content": utente }]
    });

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", chiave)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&corpo)
        .send()
        .await
        .map_err(|e| format!("rete: {e}"))?;

    let stato = resp.status();
    let val: serde_json::Value = resp.json().await.map_err(|e| format!("risposta non valida: {e}"))?;
    if !stato.is_success() {
        let msg = val["error"]["message"].as_str().unwrap_or("errore sconosciuto");
        return Err(format!("API Claude {stato}: {msg}"));
    }
    let testo = val["content"][0]["text"].as_str().unwrap_or("").trim().to_string();
    if testo.is_empty() {
        return Err("Risposta vuota dall'AI.".into());
    }
    Ok(testo)
}

/// Chiede a Claude un testo alternativo per l'immagine PNG fornita.
pub async fn alt_da_immagine(modello: &str, chiave: &str, png: Vec<u8>) -> Result<String, String> {
    let b64 = STANDARD.encode(&png);
    let corpo = serde_json::json!({
        "model": modello,
        "max_tokens": 256,
        "messages": [{
            "role": "user",
            "content": [
                {
                    "type": "image",
                    "source": { "type": "base64", "media_type": "image/png", "data": b64 }
                },
                {
                    "type": "text",
                    "text": "Sei un esperto di accessibilità. Scrivi un testo alternativo conciso (una frase, in italiano) per l'immagine principale di questa pagina di un documento PDF, adatto a uno screen reader. Rispondi SOLO con il testo alternativo, senza virgolette né preamboli."
                }
            ]
        }]
    });

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", chiave)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&corpo)
        .send()
        .await
        .map_err(|e| format!("rete: {e}"))?;

    let stato = resp.status();
    let val: serde_json::Value = resp.json().await.map_err(|e| format!("risposta non valida: {e}"))?;

    if !stato.is_success() {
        let msg = val["error"]["message"].as_str().unwrap_or("errore sconosciuto");
        return Err(format!("API Claude {stato}: {msg}"));
    }
    if val["stop_reason"] == "refusal" {
        return Err("La richiesta è stata rifiutata dai filtri di sicurezza.".into());
    }

    let testo = val["content"][0]["text"].as_str().unwrap_or("").trim().to_string();
    if testo.is_empty() {
        return Err("Risposta vuota dall'AI.".into());
    }
    Ok(testo)
}
