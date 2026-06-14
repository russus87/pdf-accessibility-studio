//! Correzione assistita: applica correzioni di accessibilita' a livello
//! documento e salva una copia corretta (senza toccare l'originale).
//!
//! Per ora: lingua (/Lang), titolo (Info/Title) e DisplayDocTitle. Questi
//! risolvono diversi esiti della validazione. L'aggiunta di Alt alle singole
//! figure richiede un'interfaccia dedicata e arrivera' in seguito.

use std::collections::HashMap;
use std::path::Path;

use lopdf::{Dictionary, Document, Object, ObjectId, StringFormat};

use crate::errore::{Errore, Risultato};

/// Correzioni richieste dall'utente.
#[derive(Debug, Default)]
pub struct Correzioni {
    pub lang: Option<String>,
    pub titolo: Option<String>,
    pub display_doc_title: bool,
    /// Metadati documento (Info): autore, soggetto, parole chiave.
    pub autore: Option<String>,
    pub soggetto: Option<String>,
    pub parole_chiave: Option<String>,
    /// Testo alternativo da impostare su singoli elementi: (riferimento
    /// "numero_generazione", testo Alt).
    pub alt: Vec<(String, String)>,
    /// Nuovi ruoli (/S) da impostare su elementi: (riferimento, ruolo).
    pub ruoli: Vec<(String, String)>,
}

/// Applica le correzioni a `origine` e salva il risultato in `destinazione`.
pub fn applica(origine: &Path, destinazione: &Path, c: &Correzioni) -> Risultato<()> {
    let mut doc = Document::load(origine).map_err(|e| Errore::Pdfium(format!("lopdf: {e}")))?;

    let catalog_id = doc
        .trailer
        .get(b"Root")
        .and_then(|o| o.as_reference())
        .map_err(|_| Errore::Pdfium("catalogo (Root) non trovato".into()))?;

    // Legge le ViewerPreferences esistenti (per non perderne altre chiavi).
    let mut vp = leggi_viewer_preferences(&doc, catalog_id).unwrap_or_default();

    // --- Catalogo: Lang + ViewerPreferences/DisplayDocTitle ---
    {
        let catalog = doc
            .get_object_mut(catalog_id)
            .and_then(|o| o.as_dict_mut())
            .map_err(|e| Errore::Pdfium(format!("catalogo: {e}")))?;

        if let Some(l) = &c.lang {
            catalog.set("Lang", Object::string_literal(l.as_str()));
        }
        if c.display_doc_title {
            vp.set("DisplayDocTitle", Object::Boolean(true));
            catalog.set("ViewerPreferences", Object::Dictionary(vp));
        }
    }

    // --- Info: titolo, autore, soggetto, parole chiave ---
    if c.titolo.is_some() || c.autore.is_some() || c.soggetto.is_some() || c.parole_chiave.is_some() {
        let info_id = match doc.trailer.get(b"Info").and_then(|o| o.as_reference()) {
            Ok(id) => id,
            Err(_) => {
                let id = doc.add_object(Dictionary::new());
                doc.trailer.set("Info", Object::Reference(id));
                id
            }
        };
        let info = doc
            .get_object_mut(info_id)
            .and_then(|o| o.as_dict_mut())
            .map_err(|e| Errore::Pdfium(format!("Info: {e}")))?;
        if let Some(t) = &c.titolo {
            info.set("Title", stringa_utf16(t));
        }
        if let Some(a) = &c.autore {
            info.set("Author", stringa_utf16(a));
        }
        if let Some(s) = &c.soggetto {
            info.set("Subject", stringa_utf16(s));
        }
        if let Some(k) = &c.parole_chiave {
            info.set("Keywords", stringa_utf16(k));
        }
    }

    // --- Alt su singoli elementi (figure) ---
    for (riferimento, testo) in &c.alt {
        if testo.trim().is_empty() {
            continue;
        }
        if let Some(id) = parse_riferimento(riferimento) {
            if let Ok(dict) = doc.get_object_mut(id).and_then(|o| o.as_dict_mut()) {
                dict.set("Alt", stringa_utf16(testo));
            }
        }
    }

    // --- Cambio ruolo (/S) su singoli elementi ---
    for (riferimento, ruolo) in &c.ruoli {
        if ruolo.trim().is_empty() {
            continue;
        }
        if let Some(id) = parse_riferimento(riferimento) {
            if let Ok(dict) = doc.get_object_mut(id).and_then(|o| o.as_dict_mut()) {
                dict.set("S", Object::Name(ruolo.as_bytes().to_vec()));
            }
        }
    }

    doc.save(destinazione)
        .map_err(|e| Errore::Io(format!("salvataggio: {e}")))?;
    Ok(())
}

/// Riordina gli elementi di primo livello dello structure tree secondo l'ordine
/// di riferimenti dato, e salva una copia. Cambia l'ordine di lettura logico.
pub fn riordina(origine: &Path, destinazione: &Path, ordine: &[String]) -> Risultato<()> {
    let mut doc = Document::load(origine).map_err(|e| Errore::Pdfium(format!("lopdf: {e}")))?;

    let catalog_id = doc
        .trailer
        .get(b"Root")
        .and_then(|o| o.as_reference())
        .map_err(|_| Errore::Pdfium("catalogo non trovato".into()))?;

    let sr = doc
        .get_object(catalog_id)
        .and_then(|o| o.as_dict())
        .and_then(|d| d.get(b"StructTreeRoot"))
        .map_err(|_| Errore::Pdfium("StructTreeRoot non trovato".into()))?
        .clone();

    let nuovo_k: Vec<Object> = ordine
        .iter()
        .filter_map(|s| parse_riferimento(s))
        .map(Object::Reference)
        .collect();

    match sr {
        Object::Reference(id) => {
            let d = doc
                .get_object_mut(id)
                .and_then(|o| o.as_dict_mut())
                .map_err(|e| Errore::Pdfium(format!("StructTreeRoot: {e}")))?;
            d.set("K", Object::Array(nuovo_k));
        }
        Object::Dictionary(mut d) => {
            d.set("K", Object::Array(nuovo_k));
            let cat = doc
                .get_object_mut(catalog_id)
                .and_then(|o| o.as_dict_mut())
                .map_err(|e| Errore::Pdfium(format!("catalogo: {e}")))?;
            cat.set("StructTreeRoot", Object::Dictionary(d));
        }
        _ => return Err(Errore::Pdfium("StructTreeRoot non valido".into())),
    }

    doc.save(destinazione).map_err(|e| Errore::Io(format!("salvataggio: {e}")))?;
    Ok(())
}

/// Genera l'outline (segnalibri) dai titoli Hn del documento e salva una copia.
/// Ritorna il numero di segnalibri creati.
pub fn genera_segnalibri(origine: &Path, destinazione: &Path) -> Risultato<usize> {
    // I titoli (con testo e pagina) vengono dall'ordine logico dei tag.
    let blocchi = crate::lettura::blocchi(origine)?;
    let titoli: Vec<(u8, String, Option<i32>)> = blocchi
        .iter()
        .filter_map(|b| livello_heading(&b.ruolo).map(|l| (l, b.testo.clone(), b.pagina)))
        .collect();
    if titoli.is_empty() {
        return Err(Errore::Pdfium("nessun titolo (H1..H6) trovato nei tag".into()));
    }

    let mut doc = Document::load(origine).map_err(|e| Errore::Pdfium(format!("lopdf: {e}")))?;

    // Indice pagina -> ObjectId.
    let mut page_ids: HashMap<i32, ObjectId> = HashMap::new();
    for (num, id) in doc.get_pages() {
        page_ids.insert(num as i32 - 1, id);
    }

    // Riserva gli id: radice outline + un item per titolo.
    let outlines_id = doc.new_object_id();
    let item_ids: Vec<ObjectId> = (0..titoli.len()).map(|_| doc.new_object_id()).collect();

    // Annidamento per livello: per ogni titolo, il genitore e' l'ultimo titolo
    // con livello inferiore ancora aperto.
    let mut parent_of: Vec<Option<usize>> = vec![None; titoli.len()];
    let mut stack: Vec<usize> = Vec::new();
    for i in 0..titoli.len() {
        let lvl = titoli[i].0;
        while let Some(&top) = stack.last() {
            if titoli[top].0 >= lvl {
                stack.pop();
            } else {
                break;
            }
        }
        parent_of[i] = stack.last().copied();
        stack.push(i);
    }

    // Liste dei figli per ogni genitore (None = radice).
    let mut figli: HashMap<Option<usize>, Vec<usize>> = HashMap::new();
    for i in 0..titoli.len() {
        figli.entry(parent_of[i]).or_default().push(i);
    }

    // Crea gli oggetti item con i collegamenti dell'outline.
    for i in 0..titoli.len() {
        let (_, titolo, pagina) = &titoli[i];
        let mut d = Dictionary::new();
        d.set("Title", stringa_utf16(titolo));
        d.set(
            "Parent",
            match parent_of[i] {
                Some(p) => Object::Reference(item_ids[p]),
                None => Object::Reference(outlines_id),
            },
        );
        let fratelli = &figli[&parent_of[i]];
        let pos = fratelli.iter().position(|&x| x == i).unwrap();
        if pos > 0 {
            d.set("Prev", Object::Reference(item_ids[fratelli[pos - 1]]));
        }
        if pos + 1 < fratelli.len() {
            d.set("Next", Object::Reference(item_ids[fratelli[pos + 1]]));
        }
        if let Some(ch) = figli.get(&Some(i)) {
            d.set("First", Object::Reference(item_ids[ch[0]]));
            d.set("Last", Object::Reference(item_ids[*ch.last().unwrap()]));
            d.set("Count", ch.len() as i64);
        }
        if let Some(pid) = pagina.and_then(|p| page_ids.get(&p)) {
            d.set("Dest", Object::Array(vec![Object::Reference(*pid), Object::Name(b"Fit".to_vec())]));
        }
        doc.set_object(item_ids[i], d);
    }

    // Radice /Outlines.
    let top = &figli[&None];
    let mut root = Dictionary::new();
    root.set("Type", Object::Name(b"Outlines".to_vec()));
    root.set("First", Object::Reference(item_ids[top[0]]));
    root.set("Last", Object::Reference(item_ids[*top.last().unwrap()]));
    root.set("Count", titoli.len() as i64);
    doc.set_object(outlines_id, root);

    // Collega l'outline al catalogo.
    let catalog_id = doc
        .trailer
        .get(b"Root")
        .and_then(|o| o.as_reference())
        .map_err(|_| Errore::Pdfium("catalogo non trovato".into()))?;
    doc.get_object_mut(catalog_id)
        .and_then(|o| o.as_dict_mut())
        .map_err(|e| Errore::Pdfium(format!("catalogo: {e}")))?
        .set("Outlines", Object::Reference(outlines_id));

    doc.save(destinazione).map_err(|e| Errore::Io(format!("salvataggio: {e}")))?;
    Ok(titoli.len())
}

/// Livello numerico di un ruolo Hn (None per ruoli non titolo).
fn livello_heading(ruolo: &str) -> Option<u8> {
    ruolo.strip_prefix('H').and_then(|s| s.parse::<u8>().ok()).filter(|n| (1..=6).contains(n))
}

/// Restituisce una copia del dizionario ViewerPreferences, se presente.
fn leggi_viewer_preferences(doc: &Document, catalog_id: lopdf::ObjectId) -> Option<Dictionary> {
    let catalog = doc.get_object(catalog_id).ok()?.as_dict().ok()?;
    match catalog.get(b"ViewerPreferences").ok()? {
        Object::Reference(id) => doc.get_object(*id).ok()?.as_dict().ok().cloned(),
        Object::Dictionary(d) => Some(d.clone()),
        _ => None,
    }
}

/// Converte un riferimento "numero_generazione" in un ObjectId.
fn parse_riferimento(s: &str) -> Option<lopdf::ObjectId> {
    let (n, g) = s.split_once('_')?;
    Some((n.parse().ok()?, g.parse().ok()?))
}

/// Codifica una stringa come testo PDF UTF-16BE (gestisce gli accenti).
fn stringa_utf16(s: &str) -> Object {
    let mut bytes = vec![0xFE, 0xFF];
    for u in s.encode_utf16() {
        bytes.extend_from_slice(&u.to_be_bytes());
    }
    Object::String(bytes, StringFormat::Hexadecimal)
}
