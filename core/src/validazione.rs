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
    heading: usize,
    link: usize,
    link_senza_testo: usize,
}

/// Valida un PDF e restituisce il report.
pub fn valida(percorso: &std::path::Path) -> Risultato<Report> {
    let info = analizza(percorso)?;
    Ok(report_da_info(&info))
}

/// Costruisce il report dalle informazioni strutturali (separato per i test).
pub fn report_da_info(info: &InfoStruttura) -> Report {
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
        }
        "Link" => {
            c.link += 1;
            // Un link "ha testo" se contiene un nodo di testo o ha Alt.
            let ha_alt = nodo.alt.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
            if !ha_alt && nodo.figli.is_empty() {
                c.link_senza_testo += 1;
            }
        }
        "H" | "H1" | "H2" | "H3" | "H4" | "H5" | "H6" => c.heading += 1,
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

fn ok(regola: &str, msg: &str) -> Esito {
    Esito { regola: regola.into(), gravita: Gravita::Ok, messaggio: msg.into() }
}
fn avviso(regola: &str, msg: &str) -> Esito {
    Esito { regola: regola.into(), gravita: Gravita::Avviso, messaggio: msg.into() }
}
fn errore(regola: &str, msg: &str) -> Esito {
    Esito { regola: regola.into(), gravita: Gravita::Errore, messaggio: msg.into() }
}
