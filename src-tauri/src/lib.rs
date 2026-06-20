//! Bootstrap dell'app Tauri: plugin, stato condiviso, caricamento di Pdfium e
//! registrazione dei comandi.

mod comandi;
mod docling;
mod firme;
mod ia;
mod piper;
mod stato;

use std::path::PathBuf;

use tauri::Manager;

use stato::StatoApp;

/// Costruisce l'elenco di cartelle in cui cercare la libreria nativa Pdfium,
/// in ordine di priorita': variabile d'ambiente, cartella "pdfium" accanto
/// all'eseguibile, risorse del bundle, cartella di lavoro (utile in sviluppo).
fn cartelle_pdfium(app: &tauri::App) -> Vec<PathBuf> {
    let mut cartelle = Vec::new();

    if let Ok(env) = std::env::var("PDFIUM_LIB_PATH") {
        cartelle.push(PathBuf::from(env));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            cartelle.push(dir.join("pdfium"));
        }
    }
    if let Ok(risorse) = app.path().resource_dir() {
        cartelle.push(risorse.join("pdfium"));
    }
    cartelle.push(PathBuf::from("pdfium"));

    cartelle
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(StatoApp::default())
        .setup(|app| {
            // Carica Pdfium una volta sola. Se manca, l'errore verra' mostrato
            // alla prima apertura di un PDF (messaggio chiaro nella UI).
            let cartelle = cartelle_pdfium(app);
            if let Err(e) = pdfa_core::pdfium::inizializza(&cartelle) {
                log::warn!("Pdfium non caricato all'avvio: {e}");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            comandi::apri_pdf,
            comandi::file_recenti,
            comandi::render_pagina,
            comandi::dimensioni_pagina,
            comandi::chiudi_documento,
            comandi::valida,
            comandi::salva_report_validazione,
            comandi::albero_tag,
            comandi::segnalibri,
            comandi::testo_documento,
            comandi::blocchi_lettura,
            comandi::confronta,
            comandi::confronta_immagine,
            comandi::report_html,
            comandi::salva_report,
            comandi::esporta_tag_stringa,
            comandi::salva_tag,
            comandi::correggi,
            comandi::genera_segnalibri,
            comandi::marca_artifact,
            comandi::applica_tabella,
            comandi::leggi_form,
            comandi::applica_form,
            comandi::modello_esempio,
            comandi::anteprima_modello,
            comandi::genera_da_modello,
            comandi::sovrapponi,
            comandi::aggiungi_campi,
            comandi::salva_editor,
            firme::firme_elenco,
            firme::firma_salva,
            firme::firma_elimina,
            comandi::riordina,
            comandi::stato_ai,
            comandi::imposta_ai,
            comandi::suggerisci_alt,
            comandi::ocr_info,
            comandi::esegui_ocr,
            comandi::contrasto,
            comandi::cerca,
            comandi::evidenzia_ricerca,
            comandi::riquadro_tag,
            comandi::metadati,
            comandi::salva_metadati,
            comandi::tts_info,
            comandi::tts_sintesi,
            piper::piper_stato,
            piper::piper_scarica_engine,
            piper::piper_scarica_voce,
            piper::piper_sintesi,
            comandi::ruota_pagine,
            comandi::elimina_pagine,
            comandi::estrai_pagine,
            comandi::riordina_pagine,
            comandi::unisci_pdf,
            docling::docling_stato,
            docling::docling_prepara,
            docling::analizza_struttura_ai,
            docling::genera_pdf_accessibile,
        ])
        .run(tauri::generate_context!())
        .expect("errore nell'avvio di PDF Accessibility Studio");
}
