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

/// Chiude una scheda: dimentica il documento e, se nessun'altra scheda usa lo
/// stesso file, lo rimuove dalla cache di rendering.
#[tauri::command]
pub fn chiudi_documento(id: String, stato: State<StatoApp>) {
    let mut docs = stato.documenti.lock().unwrap();
    if let Some(path) = docs.remove(&id) {
        let ancora_usato = docs.values().any(|p| p == &path);
        if !ancora_usato {
            pdfa_core::rimuovi_dalla_cache(&path);
        }
    }
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

/// Salva il report di validazione su disco in formato "html" o "pdf".
#[tauri::command]
pub fn salva_report_validazione(
    id: String,
    formato: String,
    destinazione: String,
    stato: State<StatoApp>,
) -> Result<(), String> {
    let path = percorso(&stato, &id)?;
    let report = pdfa_core::valida(&path).map_err(|e| e.to_string())?;
    let nome = nome(&path);
    match formato.as_str() {
        "pdf" => {
            let bytes = pdfa_core::validazione::report_pdf(&nome, &report).map_err(|e| e.to_string())?;
            std::fs::write(&destinazione, bytes).map_err(|e| e.to_string())
        }
        _ => {
            let html = pdfa_core::validazione::report_html(&nome, &report);
            std::fs::write(&destinazione, html).map_err(|e| e.to_string())
        }
    }
}

/// Ritorna l'albero dei tag e le info strutturali del PDF.
#[tauri::command]
pub fn albero_tag(id: String, stato: State<StatoApp>) -> Result<pdfa_core::InfoStruttura, String> {
    let path = percorso(&stato, &id)?;
    pdfa_core::analizza(&path).map_err(|e| e.to_string())
}

/// Ritorna l'indice/outline (segnalibri) del PDF.
#[tauri::command]
pub fn segnalibri(id: String, stato: State<StatoApp>) -> Result<Vec<pdfa_core::segnalibri::Segnalibro>, String> {
    let path = percorso(&stato, &id)?;
    pdfa_core::segnalibri::segnalibri(&path).map_err(|e| e.to_string())
}

// --- Fase 3: testo per la sintesi vocale --------------------------------------

/// Ritorna il testo di ogni pagina, in ordine: la UI lo legge con il sintetizzatore.
#[tauri::command]
pub fn testo_documento(id: String, stato: State<StatoApp>) -> Result<Vec<String>, String> {
    let path = percorso(&stato, &id)?;
    pdfa_core::testo_pagine(&path).map_err(|e| e.to_string())
}

/// Ritorna i blocchi di lettura nell'ordine logico dei tag (ordine MCID).
/// Vuoto se il PDF non e' taggato: la UI ripiega sull'ordine di pagina.
#[tauri::command]
pub fn blocchi_lettura(id: String, stato: State<StatoApp>) -> Result<Vec<pdfa_core::lettura::BloccoLettura>, String> {
    let path = percorso(&stato, &id)?;
    pdfa_core::lettura::blocchi(&path).map_err(|e| e.to_string())
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

// --- Suggerimento Alt con AI (Claude vision) ---------------------------------

/// Stato della configurazione AI da mostrare nella UI.
#[derive(Serialize)]
pub struct StatoAi {
    pub ha_chiave: bool,
    pub modello: String,
}

/// Ritorna se la chiave API è impostata e quale modello è configurato.
#[tauri::command]
pub fn stato_ai(app: tauri::AppHandle) -> StatoAi {
    let i = crate::ia::carica(&app);
    StatoAi {
        ha_chiave: i.anthropic_api_key.map(|k| !k.trim().is_empty()).unwrap_or(false),
        modello: i.modello,
    }
}

/// Salva la chiave API Anthropic (e, opzionalmente, il modello).
#[tauri::command]
pub fn imposta_ai(app: tauri::AppHandle, chiave: Option<String>, modello: Option<String>) -> Result<(), String> {
    let mut i = crate::ia::carica(&app);
    if let Some(c) = chiave {
        i.anthropic_api_key = Some(c).filter(|s| !s.trim().is_empty());
    }
    if let Some(m) = modello.filter(|s| !s.trim().is_empty()) {
        i.modello = m;
    }
    crate::ia::salva(&app, &i)
}

/// Suggerisce un testo alternativo per una figura, inviando l'immagine della sua
/// pagina a Claude (vision).
#[tauri::command]
pub async fn suggerisci_alt(
    app: tauri::AppHandle,
    id: String,
    pagina: i32,
    stato: State<'_, StatoApp>,
) -> Result<String, String> {
    let i = crate::ia::carica(&app);
    let chiave = i
        .anthropic_api_key
        .filter(|k| !k.trim().is_empty())
        .ok_or("Chiave API Anthropic non impostata (vedi impostazioni AI)")?;
    let path = percorso(&stato, &id)?;
    let png = pdfa_core::render_pagina(&path, pagina, 1024).map_err(|e| e.to_string())?;
    crate::ia::alt_da_immagine(&i.modello, &chiave, png).await
}

/// Stima il contrasto colori (WCAG) di una pagina.
#[tauri::command]
pub fn contrasto(id: String, pagina: i32, stato: State<StatoApp>) -> Result<pdfa_core::contrasto::ContrastoPagina, String> {
    let path = percorso(&stato, &id)?;
    pdfa_core::contrasto::analizza_pagina(&path, pagina).map_err(|e| e.to_string())
}

// --- OCR (PDF scansionati) ---------------------------------------------------

#[derive(Serialize)]
pub struct InfoOcr {
    pub disponibile: bool,
    pub lingue: Vec<String>,
}

/// Indica se tesseract è installato e quali lingue offre.
#[tauri::command]
pub fn ocr_info() -> InfoOcr {
    InfoOcr {
        disponibile: pdfa_core::ocr::disponibile(),
        lingue: pdfa_core::ocr::lingue(),
    }
}

/// Esegue l'OCR del PDF e salva una copia ricercabile in `destinazione`.
#[tauri::command]
pub fn esegui_ocr(
    id: String,
    lingua: String,
    destinazione: String,
    stato: State<StatoApp>,
) -> Result<(), String> {
    let path = percorso(&stato, &id)?;
    pdfa_core::ocr::ocr_a_pdf(&path, std::path::Path::new(&destinazione), &lingua)
        .map_err(|e| e.to_string())
}

/// Nome file leggibile da un percorso.
fn nome(p: &std::path::Path) -> String {
    p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

// --- Correzione assistita -----------------------------------------------------

/// Un Alt da impostare su un elemento (dal pannello correzione).
#[derive(serde::Deserialize)]
pub struct AltInput {
    pub riferimento: String,
    pub testo: String,
}

/// Un cambio di ruolo da applicare a un elemento.
#[derive(serde::Deserialize)]
pub struct RuoloInput {
    pub riferimento: String,
    pub ruolo: String,
}

/// Applica le correzioni di accessibilita' (lingua/titolo/DisplayDocTitle e Alt
/// sulle figure) e salva una copia corretta. Non tocca l'originale.
#[tauri::command]
pub fn correggi(
    id: String,
    lang: Option<String>,
    titolo: Option<String>,
    display_doc_title: bool,
    autore: Option<String>,
    soggetto: Option<String>,
    parole_chiave: Option<String>,
    alt: Option<Vec<AltInput>>,
    ruoli: Option<Vec<RuoloInput>>,
    destinazione: String,
    stato: State<StatoApp>,
) -> Result<(), String> {
    let path = percorso(&stato, &id)?;
    let pulisci = |o: Option<String>| o.filter(|s| !s.trim().is_empty());
    let correzioni = pdfa_core::correzione::Correzioni {
        lang: pulisci(lang),
        titolo: pulisci(titolo),
        display_doc_title,
        autore: pulisci(autore),
        soggetto: pulisci(soggetto),
        parole_chiave: pulisci(parole_chiave),
        alt: alt
            .unwrap_or_default()
            .into_iter()
            .map(|a| (a.riferimento, a.testo))
            .collect(),
        ruoli: ruoli
            .unwrap_or_default()
            .into_iter()
            .map(|r| (r.riferimento, r.ruolo))
            .collect(),
    };
    pdfa_core::correzione::applica(&path, std::path::Path::new(&destinazione), &correzioni)
        .map_err(|e| e.to_string())
}

/// Genera i segnalibri dai titoli (Hn) e salva una copia. Ritorna il numero
/// di segnalibri creati.
#[tauri::command]
pub fn genera_segnalibri(
    id: String,
    destinazione: String,
    stato: State<StatoApp>,
) -> Result<usize, String> {
    let path = percorso(&stato, &id)?;
    pdfa_core::correzione::genera_segnalibri(&path, std::path::Path::new(&destinazione))
        .map_err(|e| e.to_string())
}

/// Riordina gli elementi di primo livello (ordine di lettura) e salva una copia.
#[tauri::command]
pub fn riordina(
    id: String,
    ordine: Vec<String>,
    destinazione: String,
    stato: State<StatoApp>,
) -> Result<(), String> {
    let path = percorso(&stato, &id)?;
    pdfa_core::correzione::riordina(&path, std::path::Path::new(&destinazione), &ordine)
        .map_err(|e| e.to_string())
}
