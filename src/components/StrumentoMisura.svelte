<script>
  // Overlay di misura sopra la pagina renderizzata: righelli metrici (cm) lungo
  // il bordo superiore (larghezza) e sinistro (altezza), e misura della distanza
  // tra due punti cliccati, espressa in centimetri, punti PDF e pixel.
  //
  // Tutto è espresso in frazioni 0..1 della pagina, così funziona a qualsiasi
  // zoom. Le dimensioni in punti (1 pt = 1/72") arrivano dal backend; i pixel
  // sono quelli reali del PNG renderizzato (risoluzione corrente).

  let { imgEl = null, larghezzaPt, altezzaPt } = $props();

  const PT_PER_CM = 72 / 2.54; // ≈ 28.3465 punti per centimetro

  let box = $state(null);
  // Punti cliccati, frazione 0..1 con origine in alto a sinistra.
  let punti = $state([]);

  // Cambiando pagina (cambiano le dimensioni) si azzera la misura corrente.
  $effect(() => {
    void larghezzaPt;
    void altezzaPt;
    punti = [];
  });

  const cmLarghezza = $derived(larghezzaPt / PT_PER_CM);
  const cmAltezza = $derived(altezzaPt / PT_PER_CM);

  // Una tacca per centimetro; posizione in % = punti_del_cm / punti_totali.
  function tacche(totCm, totPt) {
    const out = [];
    for (let cm = 0; cm <= Math.floor(totCm + 1e-4); cm++) {
      out.push({ cm, pos: ((cm * PT_PER_CM) / totPt) * 100 });
    }
    return out;
  }
  const taccheX = $derived(tacche(cmLarghezza, larghezzaPt));
  const taccheY = $derived(tacche(cmAltezza, altezzaPt));

  function aggiungiPunto(e) {
    if (!box) return;
    const r = box.getBoundingClientRect();
    const fx = Math.min(Math.max((e.clientX - r.left) / r.width, 0), 1);
    const fy = Math.min(Math.max((e.clientY - r.top) / r.height, 0), 1);
    // Il terzo clic ricomincia una nuova misura.
    punti = punti.length >= 2 ? [{ x: fx, y: fy }] : [...punti, { x: fx, y: fy }];
  }

  function azzera() {
    punti = [];
  }

  const misura = $derived.by(() => {
    if (punti.length < 2) return null;
    const [a, b] = punti;
    const dxPt = (b.x - a.x) * larghezzaPt;
    const dyPt = (b.y - a.y) * altezzaPt;
    const distPt = Math.hypot(dxPt, dyPt);
    const distCm = distPt / PT_PER_CM;

    // Pixel reali del rendering corrente (fallback alla dimensione mostrata).
    const pw = imgEl?.naturalWidth || box?.clientWidth || larghezzaPt;
    const ph = imgEl?.naturalHeight || box?.clientHeight || altezzaPt;
    const distPx = Math.hypot((b.x - a.x) * pw, (b.y - a.y) * ph);
    const dpi = larghezzaPt > 0 ? pw / (larghezzaPt / 72) : 0;

    return { distPt, distCm, distPx, dpi };
  });

  const fmt = (n, d = 2) =>
    n.toLocaleString("it-IT", { minimumFractionDigits: d, maximumFractionDigits: d });
</script>

<div
  class="misura"
  bind:this={box}
  role="application"
  aria-label="Strumento di misura: clicca due punti sulla pagina per misurarne la distanza. Premi Esc per azzerare."
  tabindex="0"
  onclick={aggiungiPunto}
  onkeydown={(e) => {
    if (e.key === "Escape") azzera();
  }}
>
  <!-- Righello orizzontale: larghezza -->
  <div class="righello orizz" aria-hidden="true">
    {#each taccheX as t}
      <div class="tacca" style={`left:${t.pos}%`}><span class="num">{t.cm}</span></div>
    {/each}
  </div>

  <!-- Righello verticale: altezza -->
  <div class="righello vert" aria-hidden="true">
    {#each taccheY as t}
      <div class="tacca" style={`top:${t.pos}%`}><span class="num">{t.cm}</span></div>
    {/each}
  </div>

  <!-- Segmento di misura -->
  <svg class="disegno" viewBox="0 0 1000 1000" preserveAspectRatio="none" aria-hidden="true">
    {#if punti.length === 2}
      <line
        x1={punti[0].x * 1000}
        y1={punti[0].y * 1000}
        x2={punti[1].x * 1000}
        y2={punti[1].y * 1000}
        vector-effect="non-scaling-stroke"
      />
    {/if}
  </svg>
  {#each punti as p}
    <div class="punto" style={`left:${p.x * 100}%;top:${p.y * 100}%`} aria-hidden="true"></div>
  {/each}

  <!-- Pannello con i valori -->
  <div
    class="pannello"
    role="status"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    <div class="tit">📏 Misura</div>
    {#if misura}
      <div class="val"><b>{fmt(misura.distCm)}</b><span class="u">cm</span></div>
      <div class="val"><b>{fmt(misura.distPt, 1)}</b><span class="u">pt</span></div>
      <div class="val">
        <b>{fmt(misura.distPx, 0)}</b><span class="u">px</span>
        <span class="dpi">@ {fmt(misura.dpi, 0)} dpi</span>
      </div>
      <button onclick={azzera}>Azzera</button>
    {:else}
      <div class="hint">Clicca due punti sulla pagina.</div>
    {/if}
    <div class="dim">Pagina {fmt(cmLarghezza, 1)} × {fmt(cmAltezza, 1)} cm</div>
  </div>
</div>

<style>
  .misura {
    position: absolute;
    inset: 0;
    cursor: crosshair;
    z-index: 5;
  }

  .righello {
    position: absolute;
    background: rgba(20, 20, 24, 0.55);
    pointer-events: none;
    font-size: 9px;
    color: #fff;
  }
  .righello.orizz {
    top: 0;
    left: 0;
    right: 0;
    height: 16px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.35);
  }
  .righello.vert {
    top: 0;
    left: 0;
    bottom: 0;
    width: 16px;
    border-right: 1px solid rgba(255, 255, 255, 0.35);
  }
  .orizz .tacca {
    position: absolute;
    top: 0;
    bottom: 0;
    border-left: 1px solid rgba(255, 255, 255, 0.5);
  }
  .vert .tacca {
    position: absolute;
    left: 0;
    right: 0;
    border-top: 1px solid rgba(255, 255, 255, 0.5);
  }
  .orizz .num {
    position: absolute;
    left: 2px;
    top: 1px;
    line-height: 1;
  }
  .vert .num {
    position: absolute;
    left: 2px;
    top: 1px;
    line-height: 1;
  }

  .disegno {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }
  .disegno line {
    stroke: #ff3b30;
    stroke-width: 2;
    stroke-dasharray: 5 4;
  }
  .punto {
    position: absolute;
    width: 10px;
    height: 10px;
    margin: -5px 0 0 -5px;
    border-radius: 50%;
    background: #ff3b30;
    border: 2px solid #fff;
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.4);
    pointer-events: none;
  }

  .pannello {
    position: absolute;
    top: 24px;
    right: 8px;
    min-width: 132px;
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 9px 11px;
    background: rgba(20, 20, 24, 0.88);
    color: #fff;
    border-radius: 8px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.45);
    font-size: 13px;
    pointer-events: auto;
    cursor: default;
  }
  .pannello .tit {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    opacity: 0.7;
    margin-bottom: 2px;
  }
  .pannello .val {
    display: flex;
    align-items: baseline;
    gap: 5px;
  }
  .pannello .val b {
    font-variant-numeric: tabular-nums;
    font-size: 15px;
  }
  .pannello .u {
    opacity: 0.7;
  }
  .pannello .dpi {
    margin-left: auto;
    font-size: 10px;
    opacity: 0.5;
  }
  .pannello .hint {
    opacity: 0.75;
    font-size: 12px;
  }
  .pannello .dim {
    margin-top: 4px;
    padding-top: 4px;
    border-top: 1px solid rgba(255, 255, 255, 0.18);
    font-size: 10px;
    opacity: 0.6;
  }
  .pannello button {
    margin-top: 4px;
    background: rgba(255, 255, 255, 0.14);
    color: #fff;
    border: 1px solid rgba(255, 255, 255, 0.25);
    border-radius: 6px;
    padding: 4px 8px;
    cursor: pointer;
    font-size: 12px;
  }
  .pannello button:hover {
    background: rgba(255, 255, 255, 0.24);
  }
</style>
