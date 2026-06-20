//! Report di conformità basato sul **Matterhorn Protocol** (PDF/UA, ISO 14289-1).
//!
//! Organizza i controlli per checkpoint, collega gli esiti automatici della
//! validazione (PDF/UA + WCAG) e indica come "verifica manuale" le condizioni che
//! richiedono un controllo umano, riportando i criteri WCAG corrispondenti.

use serde::Serialize;

use crate::errore::Risultato;
use crate::validazione::{self, Gravita};

/// Esito di una voce del checklist.
#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StatoMh {
    Superato,
    Fallito,
    Avviso,
    Manuale,
}

/// Una voce del report Matterhorn.
#[derive(Debug, Clone, Serialize)]
pub struct VoceMatterhorn {
    pub checkpoint: String,
    pub titolo: String,
    pub wcag: String,
    pub stato: StatoMh,
    pub dettaglio: String,
}

/// Report Matterhorn completo, con conteggi.
#[derive(Debug, Clone, Serialize)]
pub struct ReportMatterhorn {
    pub voci: Vec<VoceMatterhorn>,
    pub superati: usize,
    pub falliti: usize,
    pub avvisi: usize,
    pub manuali: usize,
    pub conforme: bool,
}

/// Tabella statica: (checkpoint, titolo, regola automatica collegata, WCAG, nota manuale).
const CHECKLIST: &[(&str, &str, Option<&str>, &str, &str)] = &[
    ("01", "Contenuto reale taggato (Tagged PDF)", Some("Documento taggato"), "1.3.1", ""),
    ("06-a", "Titolo nei metadati", Some("Titolo del documento"), "2.4.2", ""),
    ("06-b", "Identificatore PDF/UA in XMP", Some("Identificatore PDF/UA"), "—", ""),
    ("07", "ViewerPreferences/DisplayDocTitle", Some("Mostra titolo"), "2.4.2", ""),
    ("09", "Artifact vs contenuto taggato", None, "1.3.1", "Decori, intestazioni e piè di pagina ripetuti devono essere artifact; il contenuto reale dev'essere taggato. (Usa lo strumento Tag → Artifact.)"),
    ("10", "Mappatura caratteri (ToUnicode)", Some("Testo mappabile (ToUnicode)"), "1.1.1", ""),
    ("11", "Lingua naturale dichiarata", Some("Lingua del documento"), "3.1.1", ""),
    ("13-a", "Grafica: testo alternativo", Some("Testo alternativo immagini"), "1.1.1", ""),
    ("13-b", "Documento non solo scansione", Some("PDF scansionato"), "1.4.5", ""),
    ("14-a", "Titoli presenti", Some("Struttura dei titoli"), "2.4.6", ""),
    ("14-b", "Gerarchia titoli corretta", Some("Ordine dei titoli"), "1.3.1", ""),
    ("15-a", "Tabelle: intestazioni (TH)", Some("Intestazioni tabelle"), "1.3.1", ""),
    ("15-b", "Tabelle: righe (TR)", Some("Righe delle tabelle"), "1.3.1", ""),
    ("16", "Liste strutturate (L/LI)", Some("Struttura liste"), "1.3.1", ""),
    ("17", "Espressioni matematiche", None, "1.1.1", "Le formule devono avere un testo alternativo (Alt) che ne descrive il significato."),
    ("18", "Intestazioni/piè come artifact di paginazione", None, "1.3.1", "I numeri di pagina e gli header/footer ripetuti vanno marcati come artifact."),
    ("19", "Note e riferimenti", None, "1.3.1", "Note a piè di pagina e riferimenti devono usare i ruoli Note/Reference."),
    ("20", "Contenuto opzionale (layer)", None, "1.3.1", "Il contenuto nascosto nei layer non deve celare informazioni necessarie."),
    ("21", "File incorporati", None, "—", "Eventuali allegati devono essere accessibili o non essenziali."),
    ("23", "Firme digitali", None, "—", "I campi firma devono avere un'etichetta accessibile."),
    ("24", "Moduli", Some("__moduli__"), "3.3.2", "Ogni campo modulo deve avere un'etichetta (TU)."),
    ("26", "Sicurezza non blocca le tecnologie assistive", None, "—", "Se cifrato, deve consentire l'estrazione per le AT (lo strumento Protezione lo garantisce)."),
    ("27", "Navigazione (segnalibri)", None, "2.4.5", "I documenti lunghi dovrebbero avere segnalibri."),
    ("28", "Annotazioni/link con testo", Some("Testo dei link"), "2.4.4", ""),
    ("30", "Contrasto del colore", None, "1.4.3", "Verifica il contrasto testo/sfondo (usa lo strumento Contrasto)."),
    ("31", "Font incorporati e mappabili", Some("Testo mappabile (ToUnicode)"), "1.1.1", ""),
];

/// Genera il report Matterhorn per un PDF.
pub fn analizza(percorso: &std::path::Path) -> Risultato<ReportMatterhorn> {
    let report = validazione::valida(percorso)?;
    let info = crate::analizza(percorso)?;

    // Indicizza gli esiti automatici per nome regola.
    let mut per_regola: std::collections::HashMap<&str, &validazione::Esito> = std::collections::HashMap::new();
    for e in &report.esiti {
        per_regola.insert(e.regola.as_str(), e);
    }

    // Controllo aggiuntivo per i moduli: campi senza /TU.
    let campi = crate::form::leggi(percorso).unwrap_or_default();
    let campi_senza_tu = campi.iter().filter(|c| c.tooltip.is_none()).count();

    let mut voci = Vec::new();
    for (cp, titolo, regola, wcag, nota) in CHECKLIST {
        let (stato, dettaglio) = match *regola {
            Some("__moduli__") => {
                if campi.is_empty() {
                    (StatoMh::Manuale, "Nessun campo modulo nel documento.".to_string())
                } else if campi_senza_tu == 0 {
                    (StatoMh::Superato, format!("Tutti i {} campi hanno un'etichetta (TU).", campi.len()))
                } else {
                    (StatoMh::Fallito, format!("{campi_senza_tu} campi su {} senza etichetta (TU).", campi.len()))
                }
            }
            Some(r) => match per_regola.get(r) {
                Some(e) => (
                    match e.gravita {
                        Gravita::Ok => StatoMh::Superato,
                        Gravita::Avviso => StatoMh::Avviso,
                        Gravita::Errore => StatoMh::Fallito,
                    },
                    e.messaggio.clone(),
                ),
                None => (StatoMh::Manuale, "Non valutato automaticamente.".to_string()),
            },
            None => (StatoMh::Manuale, nota.to_string()),
        };
        voci.push(VoceMatterhorn {
            checkpoint: cp.to_string(),
            titolo: titolo.to_string(),
            wcag: wcag.to_string(),
            stato,
            dettaglio,
        });
    }
    let _ = info; // riservato a futuri controlli

    let superati = voci.iter().filter(|v| v.stato == StatoMh::Superato).count();
    let falliti = voci.iter().filter(|v| v.stato == StatoMh::Fallito).count();
    let avvisi = voci.iter().filter(|v| v.stato == StatoMh::Avviso).count();
    let manuali = voci.iter().filter(|v| v.stato == StatoMh::Manuale).count();

    Ok(ReportMatterhorn { voci, superati, falliti, avvisi, manuali, conforme: falliti == 0 })
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Genera il report Matterhorn in HTML.
pub fn report_html(nome: &str, r: &ReportMatterhorn) -> String {
    let badge = |s: StatoMh| match s {
        StatoMh::Superato => ("#1a7f37", "Superato"),
        StatoMh::Fallito => ("#cf222e", "Fallito"),
        StatoMh::Avviso => ("#9a6700", "Avviso"),
        StatoMh::Manuale => ("#57606a", "Verifica manuale"),
    };
    let mut righe = String::new();
    for v in &r.voci {
        let (col, txt) = badge(v.stato);
        righe.push_str(&format!(
            "<tr><td class=\"cp\">{}</td><td>{}</td><td class=\"wc\">{}</td><td><span class=\"b\" style=\"background:{col}\">{txt}</span></td><td class=\"dt\">{}</td></tr>",
            esc(&v.checkpoint), esc(&v.titolo), esc(&v.wcag), esc(&v.dettaglio)
        ));
    }
    format!(
        "<!doctype html><html lang=\"it\"><head><meta charset=\"utf-8\"><title>Report Matterhorn — {nome}</title><style>\
        body{{font-family:sans-serif;margin:24px;color:#111}}h1{{font-size:20px}}\
        .som{{margin:12px 0;font-size:14px}}.som b{{font-size:16px}}\
        table{{border-collapse:collapse;width:100%;font-size:13px;margin-top:12px}}\
        th,td{{border:1px solid #ddd;padding:6px 8px;text-align:left;vertical-align:top}}\
        th{{background:#f3f4f6}}.cp{{font-family:monospace;white-space:nowrap}}.wc{{white-space:nowrap;color:#444}}\
        .b{{color:#fff;border-radius:10px;padding:1px 8px;font-size:11px;white-space:nowrap}}.dt{{color:#444}}</style></head><body>\
        <h1>Report di conformità Matterhorn (PDF/UA)</h1><p>Documento: <b>{nome}</b></p>\
        <p class=\"som\">Esito: {} · <b style=\"color:#1a7f37\">{} superati</b> · \
        <b style=\"color:#cf222e\">{} falliti</b> · <span style=\"color:#9a6700\">{} avvisi</span> · \
        <span style=\"color:#57606a\">{} da verificare manualmente</span></p>\
        <table><thead><tr><th>Checkpoint</th><th>Requisito</th><th>WCAG</th><th>Stato</th><th>Dettaglio</th></tr></thead><tbody>{righe}</tbody></table>\
        <p style=\"margin-top:16px;color:#666;font-size:12px\">Report basato sul Matterhorn Protocol del PDF Association. I checkpoint “verifica manuale” richiedono un controllo umano.</p>\
        </body></html>",
        if r.conforme { "nessun controllo automatico fallito" } else { "controlli falliti presenti" },
        r.superati, r.falliti, r.avvisi, r.manuali, nome = esc(nome)
    )
}
