// Gestione del tema chiaro/scuro, persistito in localStorage.

export function applicaTema(t) {
  document.documentElement.dataset.tema = t;
  try {
    localStorage.setItem("tema", t);
  } catch (_) {
    /* localStorage non disponibile: ignora */
  }
}

/** Imposta il tema salvato (default scuro) e lo ritorna. */
export function temaIniziale() {
  let t = "scuro";
  try {
    t = localStorage.getItem("tema") || "scuro";
  } catch (_) {
    /* ignora */
  }
  applicaTema(t);
  return t;
}
