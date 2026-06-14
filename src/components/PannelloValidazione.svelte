<script>
  // Pannello validazione accessibilita': mostra gli esiti delle regole.
  import { schede } from "../lib/schede.svelte.js";
  import { valida } from "../lib/api.js";

  const s = $derived(schede.schedaAttiva);
  let report = $state(null);
  let caricamento = $state(false);
  let errore = $state(null);

  // Rivalida quando cambia la scheda attiva.
  $effect(() => {
    if (!s) return;
    const id = s.id;
    let annullato = false;
    caricamento = true;
    errore = null;
    report = null;
    valida(id)
      .then((r) => !annullato && (report = r))
      .catch((e) => !annullato && (errore = String(e)))
      .finally(() => !annullato && (caricamento = false));
    return () => (annullato = true);
  });

  const icona = { errore: "✕", avviso: "!", ok: "✓" };
</script>

<div class="pannello">
  <header><h3>Validazione accessibilità</h3></header>

  {#if caricamento}
    <p class="info">Analisi in corso…</p>
  {:else if errore}
    <p class="err">{errore}</p>
  {:else if report}
    <div class="riepilogo">
      <span class="badge err">{report.errori} errori</span>
      <span class="badge avv">{report.avvisi} avvisi</span>
    </div>
    <ul>
      {#each report.esiti as e}
        <li class={e.gravita}>
          <span class="segno {e.gravita}">{icona[e.gravita]}</span>
          <div>
            <div class="regola">{e.regola}</div>
            <div class="msg">{e.messaggio}</div>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .pannello {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: auto;
  }
  header {
    padding: 12px 14px;
    border-bottom: 1px solid var(--bordo);
  }
  h3 {
    margin: 0;
    font-size: 15px;
  }
  .riepilogo {
    display: flex;
    gap: 8px;
    padding: 12px 14px;
  }
  .badge {
    padding: 3px 10px;
    border-radius: 12px;
    font-size: 12px;
    font-weight: bold;
  }
  .badge.err {
    background: #4a2326;
    color: #ff9a9a;
  }
  .badge.avv {
    background: #4a3a1f;
    color: #ffcf7a;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0 8px 16px;
  }
  li {
    display: flex;
    gap: 10px;
    padding: 10px;
    border-radius: 8px;
    align-items: flex-start;
  }
  li + li {
    margin-top: 4px;
  }
  .segno {
    flex: none;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    font-size: 13px;
    font-weight: bold;
    color: #fff;
  }
  .segno.errore {
    background: #c0392b;
  }
  .segno.avviso {
    background: #d68910;
  }
  .segno.ok {
    background: #27885a;
  }
  .regola {
    font-weight: 600;
  }
  .msg {
    color: var(--testo-soft);
    font-size: 13px;
    margin-top: 2px;
  }
  .info,
  .err {
    padding: 14px;
    color: var(--testo-soft);
  }
  .err {
    color: var(--errore);
  }
</style>
