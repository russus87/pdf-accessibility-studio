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

/// Ritorna il percorso del file associato a un id di scheda.
fn percorso(stato: &State<StatoApp>, id: &str) -> Result<PathBuf, String> {
    stato
        .documenti
        .lock()
        .unwrap()
        .get(id)
        .cloned()
        .ok_or_else(|| "documento non aperto".to_string())
}

// --- Fase 2: validazione accessibilita' --------------------------------------

/// Valida l'accessibilita' del PDF e ritorna il report (regole PDF/UA + WCAG).
#[tauri::command]
pub fn valida(id: String, stato: State<StatoApp>) -> Result<pdfa_core::Report, String> {
    let path = percorso(&stato, &id)?;
    pdfa_core::valida(&path).map_err(|e| e.to_string())
}

/// Ritorna l'albero dei tag e le info strutturali del PDF.
#[tauri::command]
pub fn albero_tag(id: String, stato: State<StatoApp>) -> Result<pdfa_core::InfoStruttura, String> {
    let path = percorso(&stato, &id)?;
    pdfa_core::analizza(&path).map_err(|e| e.to_string())
}

// --- Fase 3: testo per la sintesi vocale --------------------------------------

/// Ritorna il testo di ogni pagina, in ordine: la UI lo legge con il sintetizzatore.
#[tauri::command]
pub fn testo_documento(id: String, stato: State<StatoApp>) -> Result<Vec<String>, String> {
    let path = percorso(&stato, &id)?;
    pdfa_core::testo_pagine(&path).map_err(|e| e.to_string())
}

// --- Fase 4: confronto tra PDF ------------------------------------------------

/// Confronto combinato testo + tag tra due schede.
#[derive(Serialize)]
pub struct ConfrontoCombinato {
    pub testo: pdfa_core::confronto::Diff,
    pub tag: pdfa_core::confronto::Diff,
}

#[tauri::command]
pub fn confronta(id_a: String, id_b: String, stato: State<StatoApp>) -> Result<ConfrontoCombinato, String> {
    let a = percorso(&stato, &id_a)?;
    let b = percorso(&stato, &id_b)?;
    let testo = pdfa_core::confronto::confronta_testo(&a, &b).map_err(|e| e.to_string())?;
    let tag = pdfa_core::confronto::confronta_tag(&a, &b).map_err(|e| e.to_string())?;
    Ok(ConfrontoCombinato { testo, tag })
}

#[tauri::command]
pub fn confronta_immagine(
    id_a: String,
    id_b: String,
    pagina: i32,
    larghezza: i32,
    stato: State<StatoApp>,
) -> Result<pdfa_core::confronto::ConfrontoImmagine, String> {
    let a = percorso(&stato, &id_a)?;
    let b = percorso(&stato, &id_b)?;
    pdfa_core::confronto::confronta_immagine(&a, &b, pagina, larghezza).map_err(|e| e.to_string())
}

/// Genera il report HTML di confronto (stringa, per anteprima in-app).
#[tauri::command]
pub fn report_html(id_a: String, id_b: String, stato: State<StatoApp>) -> Result<String, String> {
    let a = percorso(&stato, &id_a)?;
    let b = percorso(&stato, &id_b)?;
    let testo = pdfa_core::confronto::confronta_testo(&a, &b).map_err(|e| e.to_string())?;
    let tag = pdfa_core::confronto::confronta_tag(&a, &b).map_err(|e| e.to_string())?;
    Ok(pdfa_core::confronto::report_html(
        &nome(&a),
        &nome(&b),
        &testo,
        &tag,
    ))
}

/// Salva il report di confronto su disco in formato "html" o "pdf".
#[tauri::command]
pub fn salva_report(
    id_a: String,
    id_b: String,
    formato: String,
    destinazione: String,
    stato: State<StatoApp>,
) -> Result<(), String> {
    let a = percorso(&stato, &id_a)?;
    let b = percorso(&stato, &id_b)?;
    let testo = pdfa_core::confronto::confronta_testo(&a, &b).map_err(|e| e.to_string())?;
    let tag = pdfa_core::confronto::confronta_tag(&a, &b).map_err(|e| e.to_string())?;
    let (na, nb) = (nome(&a), nome(&b));

    match formato.as_str() {
        "pdf" => {
            let bytes = pdfa_core::confronto::report_pdf(&na, &nb, &testo, &tag).map_err(|e| e.to_string())?;
            std::fs::write(&destinazione, bytes).map_err(|e| e.to_string())
        }
        _ => {
            let html = pdfa_core::confronto::report_html(&na, &nb, &testo, &tag);
            std::fs::write(&destinazione, html).map_err(|e| e.to_string())
        }
    }
}

// --- Fase 5: export dei tag ---------------------------------------------------

/// Ritorna i tag esportati come stringa nel formato "json" o "xml" (anteprima).
#[tauri::command]
pub fn esporta_tag_stringa(id: String, formato: String, stato: State<StatoApp>) -> Result<String, String> {
    let path = percorso(&stato, &id)?;
    match formato.as_str() {
        "xml" => pdfa_core::export::esporta_xml(&path),
        _ => pdfa_core::export::esporta_json(&path),
    }
    .map_err(|e| e.to_string())
}

/// Salva i tag esportati su disco (json o xml).
#[tauri::command]
pub fn salva_tag(
    id: String,
    formato: String,
    destinazione: String,
    stato: State<StatoApp>,
) -> Result<(), String> {
    let contenuto = esporta_tag_stringa(id, formato, stato)?;
    std::fs::write(&destinazione, contenuto).map_err(|e| e.to_string())
}

/// Nome file leggibile da un percorso.
fn nome(p: &std::path::Path) -> String {
    p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}
