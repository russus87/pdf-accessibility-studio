//! Operazioni sulle pagine: ruota, elimina, estrai, riordina, unisci.
//! Tutte caricano il/i PDF, modificano e salvano una copia (l'originale resta).

use std::path::Path;

use lopdf::{Document, Object, ObjectId};

use crate::errore::{Errore, Risultato};

fn carica(p: &Path) -> Risultato<Document> {
    Document::load(p).map_err(|e| Errore::Pdfium(format!("lopdf: {e}")))
}

fn salva(doc: &mut Document, dest: &Path) -> Risultato<()> {
    doc.save(dest).map_err(|e| Errore::Io(format!("salvataggio: {e}")))?;
    Ok(())
}

/// ObjectId del nodo Pages radice.
fn pages_root(doc: &Document) -> Risultato<ObjectId> {
    let catalog_id = doc
        .trailer
        .get(b"Root")
        .and_then(|o| o.as_reference())
        .map_err(|_| Errore::Pdfium("catalogo non trovato".into()))?;
    doc.get_object(catalog_id)
        .and_then(|o| o.as_dict())
        .and_then(|d| d.get(b"Pages"))
        .and_then(|o| o.as_reference())
        .map_err(|_| Errore::Pdfium("nodo Pages non trovato".into()))
}

/// Ruota le pagine indicate (1-based) di `gradi` (multipli di 90, additivo).
pub fn ruota(origine: &Path, dest: &Path, pagine: &[u32], gradi: i64) -> Risultato<()> {
    let mut doc = carica(origine)?;
    let mappa = doc.get_pages();
    for n in pagine {
        if let Some(id) = mappa.get(n).copied() {
            if let Ok(d) = doc.get_object_mut(id).and_then(|o| o.as_dict_mut()) {
                let attuale = d.get(b"Rotate").and_then(|o| o.as_i64()).unwrap_or(0);
                let nuovo = ((attuale + gradi) % 360 + 360) % 360;
                d.set("Rotate", Object::Integer(nuovo));
            }
        }
    }
    salva(&mut doc, dest)
}

/// Elimina le pagine indicate (1-based) e salva.
pub fn elimina(origine: &Path, dest: &Path, pagine: &[u32]) -> Risultato<()> {
    let mut doc = carica(origine)?;
    doc.delete_pages(pagine);
    doc.prune_objects();
    salva(&mut doc, dest)
}

/// Tiene solo le pagine indicate (1-based), eliminando le altre.
pub fn estrai(origine: &Path, dest: &Path, pagine: &[u32]) -> Risultato<()> {
    let doc = carica(origine)?;
    let totali: Vec<u32> = doc.get_pages().keys().copied().collect();
    let da_eliminare: Vec<u32> = totali.into_iter().filter(|n| !pagine.contains(n)).collect();
    // Riusa elimina ricaricando per coerenza.
    let mut doc = doc;
    doc.delete_pages(&da_eliminare);
    doc.prune_objects();
    salva(&mut doc, dest)
}

/// Riordina le pagine secondo `ordine` (permutazione di numeri 1-based).
pub fn riordina(origine: &Path, dest: &Path, ordine: &[u32]) -> Risultato<()> {
    let mut doc = carica(origine)?;
    let mappa = doc.get_pages();
    let ids: Vec<ObjectId> = ordine.iter().filter_map(|n| mappa.get(n).copied()).collect();
    if ids.is_empty() {
        return Err(Errore::Pdfium("ordine pagine non valido".into()));
    }
    let root = pages_root(&doc)?;

    // Ogni pagina punta alla radice e la radice elenca le pagine nel nuovo ordine.
    for id in &ids {
        if let Ok(d) = doc.get_object_mut(*id).and_then(|o| o.as_dict_mut()) {
            d.set("Parent", Object::Reference(root));
        }
    }
    let kids: Vec<Object> = ids.iter().map(|id| Object::Reference(*id)).collect();
    if let Ok(d) = doc.get_object_mut(root).and_then(|o| o.as_dict_mut()) {
        d.set("Count", Object::Integer(ids.len() as i64));
        d.set("Kids", Object::Array(kids));
    }
    doc.prune_objects();
    salva(&mut doc, dest)
}

/// Unisce più PDF (in ordine) in un unico file.
pub fn unisci(origini: &[std::path::PathBuf], dest: &Path) -> Risultato<()> {
    if origini.is_empty() {
        return Err(Errore::Io("nessun file da unire".into()));
    }
    let mut base = carica(&origini[0])?;
    let root = pages_root(&base)?;

    for p in &origini[1..] {
        let mut altro = carica(p)?;
        // Sposta gli id di "altro" oltre il massimo di "base" per evitare collisioni.
        altro.renumber_objects_with(base.max_id + 1);
        base.max_id = altro.max_id;

        let pagine_altro: Vec<ObjectId> = altro.get_pages().values().copied().collect();
        base.objects.extend(altro.objects);

        // Aggiorna Parent delle nuove pagine e accodale alla radice.
        for id in &pagine_altro {
            if let Ok(d) = base.get_object_mut(*id).and_then(|o| o.as_dict_mut()) {
                d.set("Parent", Object::Reference(root));
            }
        }
        if let Ok(d) = base.get_object_mut(root).and_then(|o| o.as_dict_mut()) {
            let mut kids = d.get(b"Kids").and_then(|o| o.as_array()).cloned().unwrap_or_default();
            let count = d.get(b"Count").and_then(|o| o.as_i64()).unwrap_or(0);
            kids.extend(pagine_altro.iter().map(|id| Object::Reference(*id)));
            d.set("Count", Object::Integer(count + pagine_altro.len() as i64));
            d.set("Kids", Object::Array(kids));
        }
    }
    salva(&mut base, dest)
}
