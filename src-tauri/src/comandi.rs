//! Comandi Tauri invocati dalla UI Svelte.

use std::path::PathBuf;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::Serialize;
use tauri::State;

use crate::stato::StatoApp;

/// Risposta all'apertura di un PDF.
#[derive(Serialize)]
pub struct RispostaApri {
    pub id: String,
    pub percorso: String,
    pub pagine: u16,
    pub titolo: Option<String>,
}

/// Apre un PDF dato il percorso (scelto dalla UI con il dialogo di sistema) e
/// registra una nuova scheda. Restituisce id, numero di pagine e titolo.
#[tauri::command]
pub fn apri_pdf(percorso: String, stato: State<StatoApp>) -> Result<RispostaApri, String> {
    let path = PathBuf::from(&percorso);
    let info = pdfa_core::apri(&path).map_err(|e| e.to_string())?;

    let id = stato.nuovo_id();
    stato
        .documenti
        .lock()
        .unwrap()
        .insert(id.clone(), path);

    Ok(RispostaApri {
        id,
        percorso,
        pagine: info.pagine,
        titolo: info.titolo,
    })
}

/// Renderizza una pagina (indice 0-based) alla larghezza richiesta e la
/// restituisce come PNG codificato in base64 (la UI lo usa come data URL).
#[tauri::command]
pub fn render_pagina(
    id: String,
    indice: i32,
    larghezza: i32,
    stato: State<StatoApp>,
) -> Result<String, String> {
    let path = stato
        .documenti
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .ok_or_else(|| "documento non aperto".to_string())?;

    let png = pdfa_core::render_pagina(&path, indice, larghezza).map_err(|e| e.to_string())?;
    Ok(STANDARD.encode(png))
}

/// Chiude una scheda: dimentica il documento associato all'id.
#[tauri::command]
pub fn chiudi_documento(id: String, stato: State<StatoApp>) {
    stato.documenti.lock().unwrap().remove(&id);
}
