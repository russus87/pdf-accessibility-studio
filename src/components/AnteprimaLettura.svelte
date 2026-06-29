<script>
  // Anteprima "screen reader": mostra il contenuto NELL'ORDINE in cui un lettore
  // di schermo lo legge (sequenza logica dei tag / MCID), con il ruolo di ogni
  // blocco. Cliccando un blocco si salta alla sua pagina. È la lente che PAC 2024
  // chiama "screen reader preview": serve a verificare ordine e struttura.
  import { schede } from "../lib/schede.svelte.js";
  import { blocchiLettura } from "../lib/api.js";

  const s = $derived(schede.schedaAttiva);
  let blocchi = $state(null);
  let caricamento = $state(false);
  let errore = $state(null);
  let soloTitoli = $state(false);

  $effect(() => {
    if (!s || s.tipo !== "pdf") { blocchi = null; return; }
    const id = s.id;
    let annullato = false;
    caricamento = true;
    errore = null;
    blocchi = null;
    blocchiLettura(id)
      .then((b) => !annullato && (blocchi = b))
      .catch((e) => !annullato && (errore = String(e)))
      .finally(() => !annullato && (caricamento = false));
    return () => (annullato = true);
  });

  const isTitolo = (r) => /^H[1-6]$/.test(r);
  const livello = (r) => (isTitolo(r) ? parseInt(r.slice(1), 10) : 0);
  const mostrati = $derived(
    !blocchi ? [] : soloTitoli ? blocchi.filter((b) => isTitolo(b.ruolo)) : blocchi,
  );
  const nTitoli = $derived(blocchi ? blocchi.filter((b) => isTitolo(b.ruolo)).length : 0);
</script>

<div class="pannello">
  <header>
    <h3>Anteprima lettura</h3>
    <p class="sub">Contenuto nell'ordine letto da uno screen reader. Clicca un blocco per saltare alla pagina.</p>
    {#if blocchi && blocchi.length}
      <label class="filtro"><input type="checkbox" bind:checked={soloTitoli} /> Solo titoli ({nTitoli})</label>
    {/if}
  </header>

  {#if caricamento}
    <p class="info">Lettura dell'ordine logico…</p>
  {:else if errore}
    <p class="err">{errore}</p>
  {:else if !blocchi || !blocchi.length}
    <p class="info">Nessun ordine di lettura ricavabile: il documento non è taggato. Usa <b>Auto-tag</b> per generare la struttura.</p>
  {:else}
    <ol class="lista">
      {#each mostrati as b, i}
        <li>
          <button class="blocco" class:titolo={isTitolo(b.ruolo)}
            style={`padding-left:${10 + (livello(b.ruolo) ? (livello(b.ruolo) - 1) * 12 : 0)}px`}
            disabled={b.pagina == null}
            onclick={() => b.pagina != null && schede.vaiAPagina(b.pagina)}>
            <span class="rb {isTitolo(b.ruolo) ? 'h' : ''}">{b.ruolo}</span>
            <span class="tx">{b.testo || "—"}</span>
            {#if b.pagina != null}<span class="pg">p.{b.pagina + 1}</span>{/if}
          </button>
        </li>
      {/each}
    </ol>
  {/if}
</div>

<style>
  .pannello { display: flex; flex-direction: column; height: 100%; overflow: hidden; }
  header { padding: 12px 14px; border-bottom: 1px solid var(--bordo); }
  h3 { margin: 0 0 4px; font-size: 15px; }
  .sub { margin: 0; font-size: 12px; color: var(--testo-soft); line-height: 1.4; }
  .filtro { display: inline-flex; align-items: center; gap: 6px; margin-top: 8px; font-size: 12px; color: var(--testo-soft); }
  .lista { list-style: none; margin: 0; padding: 6px 6px 16px; overflow-y: auto; counter-reset: ol; }
  .lista li { counter-increment: ol; }
  .blocco {
    width: 100%; display: flex; align-items: baseline; gap: 8px;
    background: transparent; border: none; border-radius: 6px;
    color: var(--testo); text-align: left; cursor: pointer; padding: 6px 10px; font: inherit;
  }
  .blocco::before {
    content: counter(ol); flex: none; min-width: 22px;
    color: var(--testo-soft); font-size: 11px; font-variant-numeric: tabular-nums;
  }
  .blocco:hover:not(:disabled) { background: var(--scheda); }
  .blocco:disabled { cursor: default; opacity: 0.7; }
  .blocco.titolo .tx { font-weight: 700; }
  .rb {
    flex: none; font-size: 10px; font-weight: 700; color: var(--testo-soft);
    border: 1px solid var(--bordo); border-radius: 5px; padding: 0 5px;
  }
  .rb.h { color: #fff; background: var(--accento); border-color: transparent; }
  .tx {
    flex: 1; font-size: 13px; line-height: 1.45;
    display: -webkit-box; -webkit-line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden;
  }
  .pg { flex: none; font-size: 11px; color: var(--testo-soft); }
  .info, .err { padding: 14px; color: var(--testo-soft); font-size: 13px; }
  .err { color: var(--errore); }
</style>
