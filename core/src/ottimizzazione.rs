//! Ottimizzazione/compressione dei PDF (Fase 31).
//!
//! Riduce la dimensione del file: rimuove gli oggetti non referenziati, elimina
//! gli stream vuoti, comprime i content stream (Flate) e risalva usando object
//! stream + xref stream. Ritorna le dimensioni prima/dopo.

use std::path::Path;

use image::ImageEncoder;
use lopdf::{Document, Object, ObjectId, SaveOptions};

use crate::errore::{Errore, Risultato};

/// Ottimizza `origine` e salva in `destinazione`. Ritorna (byte prima, byte dopo).
pub fn ottimizza(origine: &Path, destinazione: &Path) -> Risultato<(u64, u64)> {
    let prima = std::fs::metadata(origine).map(|m| m.len()).unwrap_or(0);

    let mut doc = Document::load(origine).map_err(|e| Errore::Pdfium(format!("lopdf: {e}")))?;
    doc.prune_objects();
    doc.delete_zero_length_streams();
    doc.compress();

    let opzioni = SaveOptions::builder()
        .use_object_streams(true)
        .use_xref_streams(true)
        .compression_level(9)
        .build();

    let mut file = std::fs::File::create(destinazione).map_err(|e| Errore::Io(format!("creazione: {e}")))?;
    doc.save_with_options(&mut file, opzioni)
        .map_err(|e| Errore::Io(format!("salvataggio: {e}")))?;

    let dopo = std::fs::metadata(destinazione).map(|m| m.len()).unwrap_or(0);
    Ok((prima, dopo))
}

/// Ottimizza riducendo anche la risoluzione delle immagini JPEG (DCTDecode) più
/// larghe di `max_larghezza` px, ricomprimendole a `qualita` (0-100). Poi applica
/// prune+compress come `ottimizza`. Ritorna (byte prima, byte dopo).
pub fn comprimi_immagini(origine: &Path, destinazione: &Path, max_larghezza: u32, qualita: u8) -> Risultato<(u64, u64)> {
    let prima = std::fs::metadata(origine).map(|m| m.len()).unwrap_or(0);
    let mut doc = Document::load(origine).map_err(|e| Errore::Pdfium(format!("lopdf: {e}")))?;

    // Individua le immagini JPEG da ridurre e calcola il nuovo contenuto.
    let ids: Vec<ObjectId> = doc.objects.keys().copied().collect();
    let mut sostituzioni: Vec<(ObjectId, Vec<u8>, i64, i64)> = Vec::new();
    for id in ids {
        let Ok(Object::Stream(st)) = doc.get_object(id) else { continue };
        let d = &st.dict;
        let is_image = d.get(b"Subtype").and_then(|o| o.as_name()).map(|n| n == b"Image").unwrap_or(false);
        if !is_image || !ha_filtro(d, b"DCTDecode") {
            continue;
        }
        let larghezza = d.get(b"Width").and_then(|o| o.as_i64()).unwrap_or(0);
        if larghezza <= max_larghezza as i64 {
            continue;
        }
        // st.content è il JPEG grezzo (DCTDecode).
        let Ok(img) = image::load_from_memory(&st.content) else { continue };
        let nuova_l = max_larghezza;
        let nuova_a = ((img.height() as u64 * nuova_l as u64) / img.width().max(1) as u64) as u32;
        let ridotta = img.resize(nuova_l, nuova_a.max(1), image::imageops::FilterType::Triangle).to_rgb8();
        let (w, h) = (ridotta.width(), ridotta.height());
        let mut out = Vec::new();
        if image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, qualita)
            .write_image(ridotta.as_raw(), w, h, image::ExtendedColorType::Rgb8)
            .is_ok()
        {
            sostituzioni.push((id, out, w as i64, h as i64));
        }
    }

    for (id, contenuto, w, h) in sostituzioni {
        if let Ok(Object::Stream(st)) = doc.get_object_mut(id) {
            st.content = contenuto;
            st.allows_compression = false; // è già JPEG: non ri-comprimere
            st.dict.set("Length", Object::Integer(st.content.len() as i64));
            st.dict.set("Width", Object::Integer(w));
            st.dict.set("Height", Object::Integer(h));
            st.dict.set("ColorSpace", Object::Name(b"DeviceRGB".to_vec()));
            st.dict.set("BitsPerComponent", Object::Integer(8));
            st.dict.set("Filter", Object::Name(b"DCTDecode".to_vec()));
            st.dict.remove(b"DecodeParms");
            st.dict.remove(b"SMask"); // la trasparenza non sopravvive al JPEG
        }
    }

    doc.prune_objects();
    doc.delete_zero_length_streams();
    doc.compress();
    let opzioni = SaveOptions::builder().use_object_streams(true).use_xref_streams(true).compression_level(9).build();
    let mut file = std::fs::File::create(destinazione).map_err(|e| Errore::Io(format!("creazione: {e}")))?;
    doc.save_with_options(&mut file, opzioni).map_err(|e| Errore::Io(format!("salvataggio: {e}")))?;

    let dopo = std::fs::metadata(destinazione).map(|m| m.len()).unwrap_or(0);
    Ok((prima, dopo))
}

/// Vero se il /Filter (nome o array) contiene il filtro dato.
fn ha_filtro(d: &lopdf::Dictionary, filtro: &[u8]) -> bool {
    match d.get(b"Filter") {
        Ok(Object::Name(n)) => n.as_slice() == filtro,
        Ok(Object::Array(a)) => a.iter().any(|o| o.as_name().map(|n| n == filtro).unwrap_or(false)),
        _ => false,
    }
}
