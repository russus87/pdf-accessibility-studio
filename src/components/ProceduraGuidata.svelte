<script>
  // Procedura guidata di remediation accessibilità: accompagna l'utente nei
  // passi tipici (apri → valida → correggi → esporta) aprendo lo strumento
  // giusto a ogni passo. Non esegue magie: orchestra i pannelli esistenti.
  import { schede } from "../lib/schede.svelte.js";

  let passo = $state(0);

  const passi = [
    {
      tit: "1 · Apri un PDF",
      txt: "Apri il documento da rendere accessibile. Se ne hai già uno aperto, vai avanti.",
      azione: { eti: "Apri PDF…", run: () => schede.apriDaDialogo() },
    },
    {
      tit: "2 · Controlla i problemi",
      txt: "Apri la Validazione: vedrai errori e avvisi WCAG/PDF-UA, con il riepilogo Matterhorn.",
      azione: { eti: "Apri Validazione", run: () => schede.apriPannello("valida") },
    },
    {
      tit: "3 · Correzione automatica",
      txt: "Nella Validazione, usa «Correzione guidata» per impostare lingua, titolo e id PDF/UA: la copia viene rivalidata in automatico.",
      azione: { eti: "Apri Validazione", run: () => schede.apriPannello("valida") },
    },
    {
      tit: "4 · Testi alternativi",
      txt: "Aggiungi gli alt alle immagini (anche con l'AI, in blocco) dal pannello Tag.",
      azione: { eti: "Apri Tag → Alt (AI)", run: () => schede.apriPannello("tag", "alt") },
    },
    {
      tit: "5 · Titoli e ordine di lettura",
      txt: "Sistema la gerarchia dei titoli (modo Ruoli) e l'ordine di lettura (modo Riordina) dal pannello Tag.",
      azione: { eti: "Apri Tag → Riordina", run: () => schede.apriPannello("tag", "riordina") },
    },
    {
      tit: "6 · Esporta il report",
      txt: "Salva il report di accessibilità (HTML/PDF) dalla Validazione per documentare la conformità.",
      azione: { eti: "Apri Validazione", run: () => schede.apriPannello("valida") },
    },
  ];

  const corrente = $derived(passi[passo]);
  function chiudi() { schede.guida = false; passo = 0; }
  function fai() { corrente.azione?.run(); }
</script>

{#if schede.guida}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="velo" onclick={chiudi}>
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="box" onclick={(e) => e.stopPropagation()}>
      <div class="cap">
        <span class="tag">Remediation guidata</span>
        <button class="x" onclick={chiudi} aria-label="Chiudi">×</button>
      </div>
      <div class="prog">
        {#each passi as _, i}<span class="punto" class:on={i <= passo}></span>{/each}
      </div>
      <h2>{corrente.tit}</h2>
      <p>{corrente.txt}</p>
      <div class="azioni">
        <button class="sec" onclick={() => (passo = Math.max(0, passo - 1))} disabled={passo === 0}>Indietro</button>
        {#if corrente.azione}<button class="pri" onclick={fai}>{corrente.azione.eti}</button>{/if}
        {#if passo < passi.length - 1}
          <button class="sec" onclick={() => (passo = Math.min(passi.length - 1, passo + 1))}>Avanti →</button>
        {:else}
          <button class="sec" onclick={chiudi}>Fine</button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .velo {
    position: fixed; inset: 0; background: rgba(0, 0, 0, 0.45); z-index: 1050;
    display: flex; justify-content: center; align-items: center;
  }
  .box {
    width: 460px; max-width: 90vw; background: var(--barra, #1e1e1e);
    border: 1px solid var(--bordo); border-radius: 12px;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.5); padding: 18px 22px 22px;
  }
  .cap { display: flex; justify-content: space-between; align-items: center; }
  .tag {
    font-size: 11px; text-transform: uppercase; letter-spacing: 0.06em;
    color: var(--accento); font-weight: 700;
  }
  .x { background: transparent; border: none; color: var(--testo-soft); font-size: 20px; cursor: pointer; line-height: 1; }
  .prog { display: flex; gap: 6px; margin: 12px 0; }
  .punto { flex: 1; height: 4px; border-radius: 2px; background: var(--bordo); }
  .punto.on { background: var(--accento); }
  h2 { margin: 4px 0 6px; font-size: 17px; color: var(--testo); }
  p { margin: 0 0 18px; color: var(--testo); line-height: 1.5; font-size: 14px; }
  .azioni { display: flex; gap: 8px; justify-content: flex-end; align-items: center; }
  .azioni .pri { background: var(--accento); color: #fff; border: none; border-radius: 8px; padding: 8px 14px; cursor: pointer; font-size: 13px; }
  .azioni .sec { background: var(--scheda); color: var(--testo); border: 1px solid var(--bordo); border-radius: 8px; padding: 8px 14px; cursor: pointer; font-size: 13px; }
  .azioni .sec:disabled { opacity: 0.5; cursor: default; }
  .pri { margin-right: auto; }
</style>
