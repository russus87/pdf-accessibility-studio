//! Lettura dei segnalibri (outline) del PDF tramite Pdfium, per un indice
//! navigabile: ogni voce ha titolo, pagina di destinazione e livello di
//! annidamento.

use std::collections::HashMap;
use std::path::Path;

use lopdf::{Dictionary, Document, Object, ObjectId, StringFormat};
use pdfium_render::prelude::PdfBookmark;
use serde::Serialize;

use crate::documento::con_documento;
use crate::errore::{Errore, Risultato};

/// Sostituisce l'outline del documento con le voci date (titolo, livello 1.., pagina
/// 0-based) e salva una copia. Ritorna il numero di voci. Usato dall'editor segnalibri.
pub fn imposta(origine: &Path, destinazione: &Path, voci: &[(String, u8, i32)]) -> Risultato<usize> {
    let mut doc = Document::load(origine).map_err(|e| Errore::Pdfium(format!("lopdf: {e}")))?;

    let mut page_ids: HashMap<i32, ObjectId> = HashMap::new();
    for (num, id) in doc.get_pages() {
        page_ids.insert(num as i32 - 1, id);
    }

    let catalog_id = doc.trailer.get(b"Root").and_then(|o| o.as_reference())
        .map_err(|_| Errore::Pdfium("catalogo non trovato".into()))?;

    if voci.is_empty() {
        if let Ok(cat) = doc.get_object_mut(catalog_id).and_then(|o| o.as_dict_mut()) {
            cat.remove(b"Outlines");
        }
        doc.save(destinazione).map_err(|e| Errore::Io(format!("salvataggio: {e}")))?;
        return Ok(0);
    }

    let outlines_id = doc.new_object_id();
    let item_ids: Vec<ObjectId> = (0..voci.len()).map(|_| doc.new_object_id()).collect();

    let mut parent_of: Vec<Option<usize>> = vec![None; voci.len()];
    let mut stack: Vec<usize> = Vec::new();
    for i in 0..voci.len() {
        let lvl = voci[i].1;
        while let Some(&top) = stack.last() {
            if voci[top].1 >= lvl { stack.pop(); } else { break; }
        }
        parent_of[i] = stack.last().copied();
        stack.push(i);
    }
    let mut figli: HashMap<Option<usize>, Vec<usize>> = HashMap::new();
    for i in 0..voci.len() {
        figli.entry(parent_of[i]).or_default().push(i);
    }

    for i in 0..voci.len() {
        let (titolo, _, pagina) = &voci[i];
        let mut d = Dictionary::new();
        d.set("Title", stringa_utf16(titolo));
        d.set("Parent", match parent_of[i] {
            Some(p) => Object::Reference(item_ids[p]),
            None => Object::Reference(outlines_id),
        });
        let fratelli = &figli[&parent_of[i]];
        let pos = fratelli.iter().position(|&x| x == i).unwrap();
        if pos > 0 { d.set("Prev", Object::Reference(item_ids[fratelli[pos - 1]])); }
        if pos + 1 < fratelli.len() { d.set("Next", Object::Reference(item_ids[fratelli[pos + 1]])); }
        if let Some(ch) = figli.get(&Some(i)) {
            d.set("First", Object::Reference(item_ids[ch[0]]));
            d.set("Last", Object::Reference(item_ids[*ch.last().unwrap()]));
            d.set("Count", ch.len() as i64);
        }
        if let Some(pid) = page_ids.get(pagina) {
            d.set("Dest", Object::Array(vec![Object::Reference(*pid), Object::Name(b"Fit".to_vec())]));
        }
        doc.set_object(item_ids[i], d);
    }

    let top = &figli[&None];
    let mut root = Dictionary::new();
    root.set("Type", Object::Name(b"Outlines".to_vec()));
    root.set("First", Object::Reference(item_ids[top[0]]));
    root.set("Last", Object::Reference(item_ids[*top.last().unwrap()]));
    root.set("Count", voci.len() as i64);
    doc.set_object(outlines_id, root);

    doc.get_object_mut(catalog_id).and_then(|o| o.as_dict_mut())
        .map_err(|e| Errore::Pdfium(format!("catalogo: {e}")))?
        .set("Outlines", Object::Reference(outlines_id));

    doc.save(destinazione).map_err(|e| Errore::Io(format!("salvataggio: {e}")))?;
    Ok(voci.len())
}

fn stringa_utf16(s: &str) -> Object {
    let mut bytes = vec![0xFE, 0xFF];
    for u in s.encode_utf16() { bytes.extend_from_slice(&u.to_be_bytes()); }
    Object::String(bytes, StringFormat::Hexadecimal)
}

/// Una voce dell'indice/outline.
#[derive(Debug, Clone, Serialize)]
pub struct Segnalibro {
    pub titolo: String,
    /// Pagina di destinazione (0-based), se disponibile.
    pub pagina: Option<i32>,
    /// Livello di annidamento (0 = primo livello).
    pub livello: usize,
}

/// Restituisce l'elenco dei segnalibri in ordine di lettura (prefisso).
pub fn segnalibri(percorso: &Path) -> Risultato<Vec<Segnalibro>> {
    con_documento(percorso, |doc| {
        let mut out = Vec::new();
        let mut nodo = doc.bookmarks().root();
        while let Some(b) = nodo {
            visita(&b, 0, &mut out);
            nodo = b.next_sibling();
        }
        Ok(out)
    })
}

fn visita(b: &PdfBookmark, livello: usize, out: &mut Vec<Segnalibro>) {
    let titolo = b.title().unwrap_or_default();
    let pagina = b
        .destination()
        .and_then(|d| d.page_index().ok())
        .map(|p| p as i32);
    out.push(Segnalibro { titolo, pagina, livello });

    for figlio in b.iter_direct_children() {
        visita(&figlio, livello + 1, out);
    }
}
