<script>
  // Note di rilascio in-app: dopo un aggiornamento mostra le novità della
  // versione corrente, una sola volta (ricorda l'ultima versione vista).
  import { getVersion } from "@tauri-apps/api/app";

  const NOTE = {
    "1.13.0": [
      "Palette dei comandi: premi Ctrl+K per cercare ed eseguire qualsiasi azione",
      "Procedura guidata di remediation accessibilità",
      "Questa finestra di note di rilascio dopo gli aggiornamenti",
    ],
    "1.12.0": [
      "Ordine di lettura visuale: trascina i blocchi per riordinarli (pannello Tag)",
      "Validazione con correzione guidata: correggi e rivalida in un clic",
    ],
    "1.11.0": [
      "Assistente Alt-text con AI: genera i testi alternativi delle immagini, anche in blocco",
      "Sistema gerarchia titoli: riallinea i livelli H1–H6",
    ],
    "1.10.0": [
      "Annulla/Ripeti nell'editor (Ctrl+Z / Ctrl+Shift+Z)",
      "Snap, guide di allineamento, griglia e ridimensionamento degli oggetti",
    ],
    "1.9.2": [
      "Sposta gli oggetti inseriti nell'editor (testo, firme, immagini)",
      "Correzione del numero di pagina nel confronto",
    ],
  };

  let versione = $state("");
  let mostra = $state(false);
  let voci = $state([]);

  $effect(() => {
    getVersion()
      .then((v) => {
        versione = v;
        const vista = localStorage.getItem("ultimaVersioneVista");
        if (vista !== v && NOTE[v]) {
          voci = NOTE[v];
          mostra = true;
        } else if (!vista) {
          // primo avvio in assoluto: non disturbare, segna la versione corrente
          localStorage.setItem("ultimaVersioneVista", v);
        }
      })
      .catch(() => {});
  });

  function chiudi() {
    if (versione) localStorage.setItem("ultimaVersioneVista", versione);
    mostra = false;
  }
</script>

{#if mostra}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="velo" onclick={chiudi}>
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="box" onclick={(e) => e.stopPropagation()}>
      <h2>✨ Novità nella versione {versione}</h2>
      <ul>
        {#each voci as v}<li>{v}</li>{/each}
      </ul>
      <button onclick={chiudi}>Ho capito</button>
    </div>
  </div>
{/if}

<style>
  .velo {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    z-index: 1100;
    display: flex;
    justify-content: center;
    align-items: center;
  }
  .box {
    width: 480px;
    max-width: 90vw;
    background: var(--barra, #1e1e1e);
    border: 1px solid var(--bordo);
    border-radius: 12px;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.5);
    padding: 22px 24px;
  }
  h2 {
    margin: 0 0 12px;
    font-size: 17px;
    color: var(--testo);
  }
  ul {
    margin: 0 0 18px;
    padding-left: 20px;
    color: var(--testo);
    line-height: 1.6;
    font-size: 14px;
  }
  button {
    background: var(--accento);
    color: #fff;
    border: none;
    border-radius: 8px;
    padding: 9px 18px;
    cursor: pointer;
    font-size: 14px;
  }
</style>
