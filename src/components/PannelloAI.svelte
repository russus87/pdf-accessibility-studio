<script>
  // AI sul documento (Fase 37): riassunto, domande&risposte e traduzione, usando
  // la chiave Claude configurata (vedi pannello Correggi → Configura AI).
  import { schede } from "../lib/schede.svelte.js";
  import { statoAi, aiRiassumi, aiDomanda, aiTraduci } from "../lib/api.js";

  const s = $derived(schede.schedaAttiva);
  let ai = $state({ ha_chiave: false, modello: "" });
  let domanda = $state("");
  let lingua = $state("inglese");
  let risposta = $state("");
  let inCorso = $state(false);
  let errore = $state(null);

  $effect(() => { statoAi().then((r) => (ai = r)).catch(() => {}); });

  const lingue = ["inglese", "francese", "tedesco", "spagnolo", "portoghese", "italiano"];

  async function esegui(fn) {
    if (!s) return;
    errore = null;
    risposta = "";
    inCorso = true;
    try {
      risposta = await fn();
    } catch (e) {
      errore = String(e);
    } finally {
      inCorso = false;
    }
  }
  const riassumi = () => esegui(() => aiRiassumi(s.id));
  const chiedi = () => { if (domanda.trim()) esegui(() => aiDomanda(s.id, domanda.trim())); };
  const traduci = () => esegui(() => aiTraduci(s.id, lingua));
</script>

<div class="pannello">
  <header><h3>AI sul documento</h3></header>
  {#if !s}
    <p class="info">Apri un PDF.</p>
  {:else if !ai.ha_chiave}
    <p class="info">Configura prima la chiave AI nel pannello <b>Correggi → Configura AI</b>.</p>
  {:else}
    <div class="azioni">
      <button class="vai" onclick={riassumi} disabled={inCorso}>Riassumi</button>
    </div>

    <div class="sez">
      <label>Domanda sul documento
        <input type="text" bind:value={domanda} placeholder="Es. Qual è la scadenza?" onkeydown={(e) => e.key === "Enter" && chiedi()} />
      </label>
      <button class="vai" onclick={chiedi} disabled={inCorso || !domanda.trim()}>Chiedi</button>
    </div>

    <div class="sez">
      <label>Traduci in
        <select bind:value={lingua}>{#each lingue as l}<option value={l}>{l}</option>{/each}</select>
      </label>
      <button class="vai" onclick={traduci} disabled={inCorso}>Traduci</button>
    </div>

    {#if inCorso}<p class="info">Elaborazione AI…</p>{/if}
    {#if errore}<p class="err">{errore}</p>{/if}
    {#if risposta}
      <div class="risposta">{risposta}</div>
    {/if}
    <p class="nota">Il testo del documento viene inviato a Claude ({ai.modello}). Risultato indicativo.</p>
  {/if}
</div>

<style>
  .pannello { display: flex; flex-direction: column; gap: 10px; height: 100%; overflow: auto; padding-bottom: 16px; }
  header { padding: 12px 14px; border-bottom: 1px solid var(--bordo); }
  h3 { margin: 0; font-size: 15px; }
  .azioni, .sez { display: flex; flex-direction: column; gap: 6px; margin: 0 14px; }
  .sez { border-top: 1px solid var(--bordo); padding-top: 10px; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--testo-soft); }
  input[type="text"], select {
    background: var(--scheda); color: var(--testo); border: 1px solid var(--bordo);
    border-radius: 6px; padding: 7px 9px; font-size: 14px; width: 100%; box-sizing: border-box;
  }
  .vai { background: var(--accento); color: #fff; border: none; border-radius: 8px; padding: 8px 14px; cursor: pointer; font-size: 14px; }
  .vai:disabled { opacity: 0.5; cursor: default; }
  .risposta {
    margin: 0 14px; padding: 10px 12px; background: var(--scheda); border: 1px solid var(--bordo);
    border-radius: 8px; font-size: 13px; white-space: pre-wrap; line-height: 1.5;
  }
  .info, .err, .nota { margin: 0 14px; font-size: 12px; color: var(--testo-soft); }
  .err { color: var(--errore); }
  .nota { font-style: italic; opacity: 0.8; }
</style>
