//! Motore di validazione dell'accessibilita': applica un insieme di regole
//! (sottoinsieme di PDF/UA e WCAG) alle informazioni strutturali del PDF.

use serde::Serialize;

use crate::errore::Risultato;
use crate::struttura::{analizza, InfoStruttura, NodoTag};

/// Gravita' di un esito.
#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Gravita {
    Errore,
    Avviso,
    Ok,
}

/// Esito di una singola regola di validazione.
#[derive(Debug, Clone, Serialize)]
pub struct Esito {
    pub regola: String,
    pub gravita: Gravita,
    pub messaggio: String,
}

/// Report complessivo della validazione.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub esiti: Vec<Esito>,
    pub errori: usize,
    pub avvisi: usize,
}

/// Conteggi ricavati visitando l'albero dei tag.
#[derive(Default)]
struct Conteggi {
    figure: usize,
    figure_senza_alt: usize,
    tabelle: usize,
    tabelle_senza_th: usize,
    tabelle_senza_tr: usize,
    heading: usize,
    /// Livelli dei titoli numerati (Hn) in ordine di documento.
    livelli_heading: Vec<u8>,
    liste: usize,
    liste_malformate: usize,
    link: usize,
    link_senza_testo: usize,
}

/// Valida un PDF e restituisce il report.
pub fn valida(percorso: &std::path::Path) -> Risultato<Report> {
    let info = analizza(percorso)?;
    // Il testo per pagina serve a riconoscere i PDF scansionati; se Pdfium non
    // e' disponibile la regola viene semplicemente saltata.
    let testo = crate::documento::testo_pagine(percorso).ok();
    Ok(costruisci(&info, testo.as_deref()))
}

/// Costruisce il report dalle informazioni strutturali (e dal testo per pagina,
/// se disponibile). Separato da `valida` per i test.
pub fn costruisci(info: &InfoStruttura, testo_pagine: Option<&[String]>) -> Report {
    let mut c = Conteggi::default();
    for nodo in &info.radice {
        visita(nodo, &mut c);
    }

    let mut esiti = Vec::new();

    // 1. Documento taggato.
    if info.taggato && info.ha_struct_tree {
        esiti.push(ok("Documento taggato", "Il PDF e' marcato come taggato e ha un albero dei tag."));
    } else if info.ha_struct_tree {
        esiti.push(avviso(
            "Documento taggato",
            "C'e' un albero dei tag ma manca MarkInfo/Marked = true.",
        ));
    } else {
        esiti.push(errore(
            "Documento taggato",
            "Il PDF non e' taggato: manca lo StructTreeRoot. Gli screen reader non possono leggerlo in ordine logico.",
        ));
    }

    // 2. Lingua del documento.
    match &info.lang {
        Some(l) if !l.trim().is_empty() => {
            esiti.push(ok("Lingua del documento", &format!("Lingua impostata: {l}.")))
        }
        _ => esiti.push(errore(
            "Lingua del documento",
            "Manca /Lang nel catalogo: la sintesi vocale non sa in che lingua leggere.",
        )),
    }

    // 3. Titolo nei metadati.
    match &info.titolo {
        Some(t) if !t.trim().is_empty() => {
            esiti.push(ok("Titolo del documento", &format!("Titolo: \u{201c}{t}\u{201d}.")))
        }
        _ => esiti.push(avviso(
            "Titolo del documento",
            "Manca il titolo nei metadati (Info/Title).",
        )),
    }

    // 4. DisplayDocTitle.
    match info.display_doc_title {
        Some(true) => esiti.push(ok(
            "Mostra titolo",
            "ViewerPreferences/DisplayDocTitle = true: il visore mostra il titolo, non il nome file.",
        )),
        _ => esiti.push(avviso(
            "Mostra titolo",
            "DisplayDocTitle non e' true: i lettori mostreranno il nome del file invece del titolo.",
        )),
    }

    // 5. Testo alternativo delle figure.
    if c.figure == 0 {
        esiti.push(ok("Testo alternativo immagini", "Nessuna figura taggata da verificare."));
    } else if c.figure_senza_alt == 0 {
        esiti.push(ok(
            "Testo alternativo immagini",
            &format!("Tutte le {} figure hanno un testo alternativo.", c.figure),
        ));
    } else {
        esiti.push(errore(
            "Testo alternativo immagini",
            &format!(
                "{} figure su {} sono prive di Alt: saranno invisibili agli screen reader.",
                c.figure_senza_alt, c.figure
            ),
        ));
    }

    // 6. Intestazioni di tabella.
    if c.tabelle == 0 {
        esiti.push(ok("Intestazioni tabelle", "Nessuna tabella taggata da verificare."));
    } else if c.tabelle_senza_th == 0 {
        esiti.push(ok(
            "Intestazioni tabelle",
            &format!("Tutte le {} tabelle hanno celle di intestazione (TH).", c.tabelle),
        ));
    } else {
        esiti.push(avviso(
            "Intestazioni tabelle",
            &format!("{} tabelle su {} non hanno celle TH di intestazione.", c.tabelle_senza_th, c.tabelle),
        ));
    }

    // 7. Link con testo.
    if c.link_senza_testo > 0 {
        esiti.push(avviso(
            "Testo dei link",
            &format!("{} link su {} sembrano privi di testo/Alt descrittivo.", c.link_senza_testo, c.link),
        ));
    } else if c.link > 0 {
        esiti.push(ok("Testo dei link", &format!("Tutti i {} link hanno testo.", c.link)));
    }

    // 8. Presenza di intestazioni (struttura dei titoli).
    if info.ha_struct_tree {
        if c.heading > 0 {
            esiti.push(ok("Struttura dei titoli", &format!("Trovati {} elementi di intestazione (Hn).", c.heading)));
        } else {
            esiti.push(avviso(
                "Struttura dei titoli",
                "Nessun elemento di intestazione (H1..H6): la navigazione per titoli non funzionera'.",
            ));
        }
    }

    // 9. Ordine dei titoli: deve iniziare da H1 e non saltare livelli (H1->H3).
    if !c.livelli_heading.is_empty() {
        let mut problemi = Vec::new();
        if c.livelli_heading[0] != 1 {
            problemi.push(format!("il primo titolo e' H{}, non H1", c.livelli_heading[0]));
        }
        for coppia in c.livelli_heading.windows(2) {
            if coppia[1] > coppia[0] + 1 {
                problemi.push(format!("salto da H{} a H{}", coppia[0], coppia[1]));
            }
        }
        if problemi.is_empty() {
            esiti.push(ok("Ordine dei titoli", "I livelli dei titoli sono coerenti (nessun salto)."));
        } else {
            esiti.push(avviso("Ordine dei titoli", &format!("Gerarchia dei titoli incoerente: {}.", problemi.join("; "))));
        }
    }

    // 10. Struttura delle liste (L deve contenere LI).
    if c.liste > 0 {
        if c.liste_malformate == 0 {
            esiti.push(ok("Struttura liste", &format!("Tutte le {} liste contengono elementi LI.", c.liste)));
        } else {
            esiti.push(avviso("Struttura liste", &format!("{} liste su {} non contengono elementi LI.", c.liste_malformate, c.liste)));
        }
    }

    // 11. Struttura tabelle (Table deve contenere righe TR).
    if c.tabelle > 0 && c.tabelle_senza_tr > 0 {
        esiti.push(avviso("Righe delle tabelle", &format!("{} tabelle su {} non contengono righe TR.", c.tabelle_senza_tr, c.tabelle)));
    }

    // 12. Mappatura a Unicode dei font (ToUnicode).
    if info.font_totali > 0 {
        if info.font_senza_tounicode == 0 {
            esiti.push(ok("Testo mappabile (ToUnicode)", "Tutti i font hanno la mappa ToUnicode."));
        } else {
            esiti.push(avviso(
                "Testo mappabile (ToUnicode)",
                &format!(
                    "{} font su {} senza ToUnicode: il testo potrebbe risultare illeggibile a copia/incolla e screen reader.",
                    info.font_senza_tounicode, info.font_totali
                ),
            ));
        }
    }

    // 13. Identificatore PDF/UA nei metadati XMP.
    if info.ha_pdfua_id {
        esiti.push(ok("Identificatore PDF/UA", "I metadati XMP dichiarano la conformita' PDF/UA."));
    } else {
        esiti.push(avviso(
            "Identificatore PDF/UA",
            "Manca l'identificatore PDF/UA (pdfuaid) nei metadati XMP.",
        ));
    }

    // 14. PDF scansionato (immagini senza testo estraibile).
    if let Some(testo) = testo_pagine {
        if !testo.is_empty() {
            let vuote = testo.iter().filter(|t| t.trim().len() < 5).count();
            if vuote == testo.len() {
                esiti.push(errore(
                    "PDF scansionato",
                    "Nessun testo estraibile in tutte le pagine: il PDF e' probabilmente scansionato. Serve l'OCR.",
                ));
            } else if vuote * 2 > testo.len() {
                esiti.push(avviso(
                    "PDF scansionato",
                    &format!("{vuote} pagine su {} non hanno testo estraibile (forse scansionate).", testo.len()),
                ));
            }
        }
    }

    let errori = esiti.iter().filter(|e| e.gravita == Gravita::Errore).count();
    let avvisi = esiti.iter().filter(|e| e.gravita == Gravita::Avviso).count();
    Report { esiti, errori, avvisi }
}

/// Visita ricorsiva dell'albero per i conteggi.
fn visita(nodo: &NodoTag, c: &mut Conteggi) {
    let ruolo = nodo.ruolo.as_str();
    match ruolo {
        "Figure" => {
            c.figure += 1;
            let descritta = nodo.alt.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false)
                || nodo.actual_text.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
            if !descritta {
                c.figure_senza_alt += 1;
            }
        }
        "Table" => {
            c.tabelle += 1;
            if !contiene_ruolo(nodo, "TH") {
                c.tabelle_senza_th += 1;
            }
            if !contiene_ruolo(nodo, "TR") {
                c.tabelle_senza_tr += 1;
            }
        }
        "L" => {
            c.liste += 1;
            if !contiene_ruolo(nodo, "LI") {
                c.liste_malformate += 1;
            }
        }
        "Link" => {
            c.link += 1;
            // Un link "ha testo" se contiene un nodo di testo o ha Alt.
            let ha_alt = nodo.alt.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
            if !ha_alt && nodo.figli.is_empty() {
                c.link_senza_testo += 1;
            }
        }
        "H" | "H1" | "H2" | "H3" | "H4" | "H5" | "H6" => {
            c.heading += 1;
            // Registra il livello numerico (H -> trattato come non numerato).
            if let Some(n) = ruolo.strip_prefix('H').and_then(|s| s.parse::<u8>().ok()) {
                c.livelli_heading.push(n);
            }
        }
        _ => {}
    }
    for figlio in &nodo.figli {
        visita(figlio, c);
    }
}

/// Verifica (in profondita') se un nodo contiene un certo ruolo tra i discendenti.
fn contiene_ruolo(nodo: &NodoTag, ruolo: &str) -> bool {
    nodo.figli.iter().any(|f| f.ruolo == ruolo || contiene_ruolo(f, ruolo))
}

// --- Report esportabile ------------------------------------------------------

/// Genera il report di validazione in HTML autoconsistente.
pub fn report_html(nome: &str, r: &Report) -> String {
    let mut h = String::from("<!doctype html><html lang=\"it\"><head><meta charset=\"utf-8\">");
    h.push_str("<title>Report accessibilita'</title><style>");
    h.push_str(
        "body{font-family:Segoe UI,system-ui,sans-serif;margin:32px;color:#222}
         h1{font-size:22px} .meta{color:#555}
         .riepilogo span{display:inline-block;margin-right:14px;font-weight:bold}
         .err{color:#a11} .avv{color:#a60} .ok{color:#063}
         ul{list-style:none;padding:0} li{padding:8px 10px;border-radius:6px;margin:4px 0;border:1px solid #eee}
         li.errore{background:#fdf0f0} li.avviso{background:#fcf7ea} li.ok{background:#f1f8f3}
         .regola{font-weight:600} .msg{color:#444;font-size:14px}",
    );
    h.push_str("</style></head><body>");
    h.push_str("<h1>Report di accessibilita' PDF</h1>");
    h.push_str(&format!("<p class=\"meta\">Documento: <b>{}</b></p>", esc(nome)));
    h.push_str(&format!(
        "<p class=\"riepilogo\"><span class=\"err\">Errori: {}</span><span class=\"avv\">Avvisi: {}</span></p>",
        r.errori, r.avvisi
    ));
    h.push_str("<ul>");
    for e in &r.esiti {
        let cls = match e.gravita {
            Gravita::Errore => "errore",
            Gravita::Avviso => "avviso",
            Gravita::Ok => "ok",
        };
        h.push_str(&format!(
            "<li class=\"{cls}\"><div class=\"regola\">{}</div><div class=\"msg\">{}</div></li>",
            esc(&e.regola),
            esc(&e.messaggio)
        ));
    }
    h.push_str("</ul></body></html>");
    h
}

/// Genera il report di validazione in PDF (dallo stesso HTML, via printpdf).
pub fn report_pdf(nome: &str, r: &Report) -> Risultato<Vec<u8>> {
    use std::collections::BTreeMap;
    let html = report_html(nome, r);
    let imm = BTreeMap::new();
    let font = BTreeMap::new();
    let opt = printpdf::GeneratePdfOptions::default();
    let mut w = Vec::new();
    let doc = printpdf::PdfDocument::from_html(&html, &imm, &font, &opt, &mut w)
        .map_err(|e| crate::errore::Errore::Io(format!("report PDF: {e}")))?;
    let mut w2 = Vec::new();
    Ok(doc.save(&printpdf::PdfSaveOptions::default(), &mut w2))
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn ok(regola: &str, msg: &str) -> Esito {
    Esito { regola: regola.into(), gravita: Gravita::Ok, messaggio: msg.into() }
}
fn avviso(regola: &str, msg: &str) -> Esito {
    Esito { regola: regola.into(), gravita: Gravita::Avviso, messaggio: msg.into() }
}
fn errore(regola: &str, msg: &str) -> Esito {
    Esito { regola: regola.into(), gravita: Gravita::Errore, messaggio: msg.into() }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::struttura::NodoTag;

    fn nodo(ruolo: &str, figli: Vec<NodoTag>) -> NodoTag {
        NodoTag { ruolo: ruolo.into(), figli, ..Default::default() }
    }

    fn info(radice: Vec<NodoTag>) -> InfoStruttura {
        InfoStruttura {
            taggato: true,
            ha_struct_tree: true,
            lang: Some("it".into()),
            titolo: Some("T".into()),
            display_doc_title: Some(true),
            font_totali: 0,
            font_senza_tounicode: 0,
            ha_pdfua_id: true,
            radice,
        }
    }

    fn trova<'a>(r: &'a Report, regola: &str) -> Option<&'a Esito> {
        r.esiti.iter().find(|e| e.regola == regola)
    }

    #[test]
    fn ordine_titoli_segnala_salti() {
        // H1 poi H3: salto di livello -> avviso.
        let r = costruisci(&info(vec![nodo("H1", vec![]), nodo("H3", vec![])]), None);
        let e = trova(&r, "Ordine dei titoli").expect("regola presente");
        assert_eq!(e.gravita, Gravita::Avviso);
        assert!(e.messaggio.contains("H1 a H3"));
    }

    #[test]
    fn ordine_titoli_ok() {
        let r = costruisci(&info(vec![nodo("H1", vec![]), nodo("H2", vec![]), nodo("H3", vec![])]), None);
        assert_eq!(trova(&r, "Ordine dei titoli").unwrap().gravita, Gravita::Ok);
    }

    #[test]
    fn lista_senza_li_segnalata() {
        let r = costruisci(&info(vec![nodo("L", vec![nodo("P", vec![])])]), None);
        assert_eq!(trova(&r, "Struttura liste").unwrap().gravita, Gravita::Avviso);
    }

    #[test]
    fn pdf_scansionato_rilevato() {
        let pagine = vec![String::new(), "   ".into()];
        let r = costruisci(&info(vec![]), Some(&pagine));
        assert_eq!(trova(&r, "PDF scansionato").unwrap().gravita, Gravita::Errore);
    }
}
