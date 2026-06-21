<script>
  // Barra di stato in basso (stile VS Code): contesto del documento attivo,
  // pagina corrente, zoom e numero di documenti aperti.
  import { schede } from "../lib/schede.svelte.js";
  import Icona from "./Icona.svelte";

  const s = $derived(schede.schedaAttiva);
  const pdf = $derived(schede.pdfAttivo);
</script>

<footer class="stato">
  <div class="sx">
    {#if s}
      <span class="voce">
        <Icona nome={s.tipo === "creatore" ? "file-plus" : "file"} dim={13} />
        {s.nome}
      </span>
      {#if s.tipo === "creatore"}
        <span class="voce muto">Nuovo da modello</span>
      {/if}
    {:else}
      <span class="voce muto">Nessun documento aperto</span>
    {/if}
  </div>

  <div class="dx">
    {#if pdf}
      <span class="voce">Pag. {pdf.pagina + 1} / {pdf.pagine}</span>
      <span class="voce">{Math.round((pdf.zoom / 900) * 100)}%</span>
    {/if}
    <span class="voce muto">{schede.numeroPdf} PDF aperti</span>
  </div>
</footer>

<style>
  .stato {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 24px;
    flex: none;
    padding: 0 12px;
    background: var(--bg-stato);
    border-top: 1px solid var(--bordo-soft);
    color: var(--testo-soft);
    font-size: 12px;
  }
  .sx,
  .dx {
    display: flex;
    align-items: center;
    gap: 16px;
    min-width: 0;
  }
  .voce {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 360px;
  }
  .muto {
    color: var(--testo-debole);
  }
</style>
