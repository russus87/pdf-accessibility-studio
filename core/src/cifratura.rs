//! Protezione dei PDF con password e permessi (Fase 27).
//!
//! - `proteggi`: cifra il documento (RC4/AES 128 bit) con una password di
//!   apertura (utente) e/o una password proprietario, limitando i permessi
//!   (stampa, copia, modifica, annotazioni). La copia per l'accessibilità resta
//!   sempre consentita.
//! - `rimuovi_protezione`: decifra un PDF protetto data la password e salva una
//!   copia senza cifratura.

use std::path::Path;

use lopdf::encryption::{EncryptionState, EncryptionVersion, Permissions};
use lopdf::Document;

use crate::errore::{Errore, Risultato};

/// Permessi da concedere nel PDF protetto.
#[derive(Debug, Clone, Copy)]
pub struct Permessi {
    pub stampa: bool,
    pub copia: bool,
    pub modifica: bool,
    pub annotazioni: bool,
}

impl Default for Permessi {
    fn default() -> Self {
        Permessi { stampa: true, copia: false, modifica: false, annotazioni: true }
    }
}

fn costruisci_permessi(p: &Permessi) -> Permissions {
    let mut perm = Permissions::empty();
    // Sempre concesso: copia per tecnologie assistive (coerente con un tool di accessibilità).
    perm |= Permissions::COPYABLE_FOR_ACCESSIBILITY;
    if p.stampa {
        perm |= Permissions::PRINTABLE | Permissions::PRINTABLE_IN_HIGH_QUALITY;
    }
    if p.copia {
        perm |= Permissions::COPYABLE;
    }
    if p.modifica {
        perm |= Permissions::MODIFIABLE | Permissions::ASSEMBLABLE;
    }
    if p.annotazioni {
        perm |= Permissions::ANNOTABLE | Permissions::FILLABLE;
    }
    perm
}

/// Cifra `origine` con le password indicate e salva in `destinazione`.
/// `password_utente` serve per aprire il file; `password_proprietario` per
/// cambiare permessi/togliere la protezione (se vuota, usa quella utente).
pub fn proteggi(
    origine: &Path,
    destinazione: &Path,
    password_utente: &str,
    password_proprietario: &str,
    permessi: &Permessi,
) -> Risultato<()> {
    let mut doc = Document::load(origine).map_err(|e| Errore::Pdfium(format!("lopdf: {e}")))?;

    // La cifratura richiede un /ID nel trailer: se manca, ne generiamo uno
    // deterministico (presente e stabile è sufficiente).
    if doc.trailer.get(b"ID").is_err() {
        let seed = doc.objects.len() as u64;
        let id: Vec<u8> = (0..16u64)
            .map(|i| (seed.wrapping_mul(2654435761).wrapping_add(i).rotate_left(i as u32 % 8) & 0xff) as u8)
            .collect();
        let s = lopdf::Object::String(id, lopdf::StringFormat::Hexadecimal);
        doc.trailer.set("ID", lopdf::Object::Array(vec![s.clone(), s]));
    }

    let owner = if password_proprietario.is_empty() {
        password_utente
    } else {
        password_proprietario
    };

    let versione = EncryptionVersion::V2 {
        document: &doc,
        owner_password: owner,
        user_password: password_utente,
        key_length: 128, // bit
        permissions: costruisci_permessi(permessi),
    };
    let stato = EncryptionState::try_from(versione)
        .map_err(|e| Errore::Pdfium(format!("cifratura: {e}")))?;
    doc.encrypt(&stato).map_err(|e| Errore::Pdfium(format!("encrypt: {e}")))?;

    doc.save(destinazione).map_err(|e| Errore::Io(format!("salvataggio: {e}")))?;
    Ok(())
}

/// Rimuove la protezione da un PDF cifrato (data la password) e salva una copia.
pub fn rimuovi_protezione(origine: &Path, destinazione: &Path, password: &str) -> Risultato<()> {
    // load_with_password apre e decifra in un colpo solo (rimuovendo /Encrypt).
    let mut doc = Document::load_with_password(origine, password)
        .map_err(|_| Errore::Pdfium("password errata o documento non decifrabile".into()))?;
    doc.save(destinazione).map_err(|e| Errore::Io(format!("salvataggio: {e}")))?;
    Ok(())
}

/// Indica se il PDF è protetto da password.
pub fn e_protetto(origine: &Path) -> Risultato<bool> {
    let doc = Document::load(origine).map_err(|e| Errore::Pdfium(format!("lopdf: {e}")))?;
    Ok(doc.is_encrypted())
}
