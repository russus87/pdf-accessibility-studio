<script>
  // Strumento di misura avanzato sopra la pagina renderizzata.
  //
  // Tre modalità:
  //  - "righello": un righello semitrasparente trascinabile, orientabile in
  //    orizzontale o verticale (icone), con scala in cm/mm per misure precise;
  //  - "distanza": misura la distanza (e l'angolo) tra due punti cliccati;
  //  - "area": misura larghezza, altezza e area di un rettangolo.
  //
  // Tutto è in frazioni 0..1 della pagina (indipendente dallo zoom). Le dimensioni
  // in punti PDF (1 pt = 1/72") arrivano dal backend; i pixel sono quelli reali
  // del rendering corrente. Unità selezionabile: cm, mm, pollici, punti.

  let { imgEl = null, larghezzaPt, altezzaPt } = $props();

  const PT_PER_CM = 72 / 2.54; // ≈ 28.3465

  let box = $state(null);
  let modo = $state("righello"); // righello | distanza | area
  let unita = $state("cm"); // cm | mm | in | pt
  let orientamento = $state("orizz"); // orizz | vert (righello)
  let rigPos = $state(0.5); // posizione del righello lungo l'asse perpendicolare (0..1)
  let trascina = $state(false);
  let punti = $state([]); // per distanza/area
  let cursore = $state(null); // {fx, fy}

  // Azzera misura e ricentra il righello quando cambia pagina.
  $effect(() => {
    void larghezzaPt;
    void altezzaPt;
    punti = [];
    rigPos = 0.5;
  });

  const cmLarghezza = $derived(larghezzaPt / PT_PER_CM);
  const cmAltezza = $derived(altezzaPt / PT_PER_CM);

  // Converte una lunghezza in punti nell'unità scelta.
  function conv(distPt) {
    switch (unita) {
      case "mm": return distPt / PT_PER_CM * 10;
      case "in": return distPt / 72;
      case "pt": return distPt;
      default: return distPt / PT_PER_CM;
    }
  }
  const decimali = $derived(unita === "pt" ? 1 : unita === "mm" ? 1 : 2);
  const fmt = (n, d = decimali) => n.toLocaleString("it-IT", { minimumFractionDigits: d, maximumFractionDigits: d });
  const fmtU = (distPt) => `${fmt(conv(distPt))} ${unita}`;

  // --- Righello: tacche ogni mm lungo l'asse, numeri ai cm. ---
  function tacche(totPt) {
    const totCm = totPt / PT_PER_CM;
    const out = [];
    const nMm = Math.floor(totCm * 10 + 1e-4);
    for (let mm = 0; mm <= nMm; mm++) {
      const cm = mm / 10;
      out.push({ pos: (cm * PT_PER_CM / totPt) * 100, major: mm % 10 === 0, mid: mm % 5 === 0, label: mm % 10 === 0 ? mm / 10 : null });
    }
    return out;
  }
  const taccheRig = $derived(tacche(orientamento === "orizz" ? larghezzaPt : altezzaPt));
  // Posizione del righello nell'unità scelta (distanza dal bordo).
  const posRigPt = $derived(rigPos * (orientamento === "orizz" ? altezzaPt : larghezzaPt));

  function frazioni(e) {
    const r = box.getBoundingClientRect();
    return {
      fx: Math.min(Math.max((e.clientX - r.left) / r.width, 0), 1),
      fy: Math.min(Math.max((e.clientY - r.top) / r.height, 0), 1),
    };
  }

  function suMove(e) {
    cursore = frazioni(e);
    if (trascina) {
      rigPos = orientamento === "orizz" ? cursore.fy : cursore.fx;
    }
  }
  function rigDown(e) {
    trascina = true;
    e.currentTarget.setPointerCapture?.(e.pointerId);
    e.stopPropagation();
  }
  function rigUp() {
    trascina = false;
  }

  function suClic(e) {
    if (modo === "righello") return; // il righello si trascina, non si clicca
    const { fx, fy } = frazioni(e);
    punti = punti.length >= 2 ? [{ x: fx, y: fy }] : [...punti, { x: fx, y: fy }];
  }

  function azzera() {
    punti = [];
  }

  const distanza = $derived.by(() => {
    if (modo !== "distanza" || punti.length < 2) return null;
    const [a, b] = punti;
    const dxPt = (b.x - a.x) * larghezzaPt;
    const dyPt = (b.y - a.y) * altezzaPt;
    const distPt = Math.hypot(dxPt, dyPt);
    const angolo = (Math.atan2(-dyPt, dxPt) * 180) / Math.PI; // 0 = orizzontale verso destra
    const pw = imgEl?.naturalWidth || larghezzaPt;
    const distPx = Math.hypot((b.x - a.x) * pw, (b.y - a.y) * (imgEl?.naturalHeight || altezzaPt));
    const dpi = larghezzaPt > 0 ? pw / (larghezzaPt / 72) : 0;
    return { distPt, angolo: ((angolo % 360) + 360) % 360, distPx, dpi };
  });

  const area = $derived.by(() => {
    if (modo !== "area" || punti.length < 2) return null;
    const [a, b] = punti;
    const wPt = Math.abs(b.x - a.x) * larghezzaPt;
    const hPt = Math.abs(b.y - a.y) * altezzaPt;
    const areaCm2 = (wPt / PT_PER_CM) * (hPt / PT_PER_CM);
    return { wPt, hPt, areaCm2,
      left: Math.min(a.x, b.x) * 100, top: Math.min(a.y, b.y) * 100,
      w: Math.abs(b.x - a.x) * 100, h: Math.abs(b.y - a.y) * 100 };
  });
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_click_events_have_key_events a11y_no_noninteractive_tabindex -->
<div
  class="misura"
  class:trascinando={trascina}
  bind:this={box}
  role="application"
  aria-label="Strumento di misura"
  tabindex="0"
  onpointermove={suMove}
  onclick={suClic}
  onkeydown={(e) => { if (e.key === "Escape") azzera(); }}
>
  <!-- Righello trascinabile -->
  {#if modo === "righello"}
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      class="righello-band {orientamento}"
      style={orientamento === "orizz" ? `top:${rigPos * 100}%` : `left:${rigPos * 100}%`}
      role="slider"
      tabindex="0"
      aria-label="Righello (trascina per spostare)"
      aria-valuenow={Math.round(conv(posRigPt))}
      onpointerdown={rigDown}
      onpointerup={rigUp}
      onpointermove={suMove}
      onkeydown={(e) => {
        const step = e.shiftKey ? 0.05 : 0.005;
        if (e.key === "ArrowUp" || e.key === "ArrowLeft") rigPos = Math.max(0, rigPos - step);
        if (e.key === "ArrowDown" || e.key === "ArrowRight") rigPos = Math.min(1, rigPos + step);
      }}
    >
      <div class="linea-rif"></div>
      {#each taccheRig as t}
        <div class="rt" class:maj={t.major} class:mid={t.mid} style={orientamento === "orizz" ? `left:${t.pos}%` : `top:${t.pos}%`}>
          {#if t.label != null}<span class="rn">{t.label}</span>{/if}
        </div>
      {/each}
      <div class="rig-pos">{fmt(conv(posRigPt))} {unita}</div>
    </div>
  {/if}

  <!-- Distanza -->
  {#if modo === "distanza" && punti.length === 2}
    <svg class="disegno" viewBox="0 0 1000 1000" preserveAspectRatio="none" aria-hidden="true">
      <line x1={punti[0].x * 1000} y1={punti[0].y * 1000} x2={punti[1].x * 1000} y2={punti[1].y * 1000} vector-effect="non-scaling-stroke" />
    </svg>
  {/if}

  <!-- Area -->
  {#if area}
    <div class="area-box" style={`left:${area.left}%;top:${area.top}%;width:${area.w}%;height:${area.h}%`} aria-hidden="true"></div>
  {/if}

  {#if modo !== "righello"}
    {#each punti as p}
      <div class="punto" style={`left:${p.x * 100}%;top:${p.y * 100}%`} aria-hidden="true"></div>
    {/each}
  {/if}

  <!-- Pannello comandi + valori -->
  <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
  <div class="pannello" onpointerdown={(e) => e.stopPropagation()} onclick={(e) => e.stopPropagation()}>
    <div class="tit">📏 Misura</div>

    <div class="modi">
      <button class:on={modo === "righello"} onclick={() => (modo = "righello")}>Righello</button>
      <button class:on={modo === "distanza"} onclick={() => { modo = "distanza"; punti = []; }}>Distanza</button>
      <button class:on={modo === "area"} onclick={() => { modo = "area"; punti = []; }}>Area</button>
    </div>

    <div class="riga2">
      <select bind:value={unita} aria-label="Unità">
        <option value="cm">cm</option>
        <option value="mm">mm</option>
        <option value="in">in</option>
        <option value="pt">pt</option>
      </select>
      {#if modo === "righello"}
        <div class="orient">
          <button class:on={orientamento === "orizz"} title="Orizzontale (parallelo)" aria-label="Righello orizzontale" onclick={() => (orientamento = "orizz")}>━</button>
          <button class:on={orientamento === "vert"} title="Verticale (90°)" aria-label="Righello verticale" onclick={() => (orientamento = "vert")}>┃</button>
        </div>
      {/if}
    </div>

    {#if modo === "righello"}
      <div class="val"><span class="k">Posizione</span><b>{fmt(conv(posRigPt))}</b><span class="u">{unita}</span></div>
      <div class="hint">Trascina il righello. Usa le icone per orientarlo.</div>
    {:else if modo === "distanza"}
      {#if distanza}
        <div class="val"><span class="k">Distanza</span><b>{fmtU(distanza.distPt)}</b></div>
        <div class="val"><span class="k">Angolo</span><b>{fmt(distanza.angolo, 1)}</b><span class="u">°</span></div>
        <div class="val small"><span class="k">Pixel</span>{fmt(distanza.distPx, 0)} px @ {fmt(distanza.dpi, 0)} dpi</div>
        <button onclick={azzera}>Azzera</button>
      {:else}
        <div class="hint">Clicca due punti sulla pagina.</div>
      {/if}
    {:else if modo === "area"}
      {#if area}
        <div class="val"><span class="k">Largh.</span><b>{fmtU(area.wPt)}</b></div>
        <div class="val"><span class="k">Alt.</span><b>{fmtU(area.hPt)}</b></div>
        <div class="val"><span class="k">Area</span><b>{fmt(area.areaCm2)}</b><span class="u">cm²</span></div>
        <button onclick={azzera}>Azzera</button>
      {:else}
        <div class="hint">Clicca due angoli opposti.</div>
      {/if}
    {/if}

    <div class="dim">
      {#if cursore}Cursore {fmt(conv(cursore.fx * larghezzaPt))} × {fmt(conv(cursore.fy * altezzaPt))} {unita} · {/if}
      Pagina {fmt(cmLarghezza, 1)} × {fmt(cmAltezza, 1)} cm
    </div>
  </div>
</div>

<style>
  .misura { position: absolute; inset: 0; cursor: crosshair; z-index: 5; }
  .misura.trascinando { cursor: grabbing; }

  /* Righello trascinabile semitrasparente */
  .righello-band {
    position: absolute;
    background: linear-gradient(rgba(255, 214, 102, 0.30), rgba(255, 214, 102, 0.16));
    border: 1px solid rgba(180, 120, 0, 0.7);
    box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.25);
    cursor: grab;
    touch-action: none;
  }
  .righello-band:active { cursor: grabbing; }
  .righello-band.orizz { left: 0; right: 0; height: 30px; transform: translateY(-50%); }
  .righello-band.vert { top: 0; bottom: 0; width: 30px; transform: translateX(-50%); }
  /* Linea di riferimento centrale (la misura) */
  .linea-rif { position: absolute; background: #d11; }
  .orizz .linea-rif { left: 0; right: 0; top: 50%; height: 1px; }
  .vert .linea-rif { top: 0; bottom: 0; left: 50%; width: 1px; }
  /* Tacche */
  .rt { position: absolute; background: rgba(90, 60, 0, 0.85); }
  .orizz .rt { bottom: 0; width: 1px; height: 5px; }
  .orizz .rt.mid { height: 8px; }
  .orizz .rt.maj { height: 13px; background: rgba(60, 40, 0, 1); }
  .vert .rt { right: 0; height: 1px; width: 5px; }
  .vert .rt.mid { width: 8px; }
  .vert .rt.maj { width: 13px; background: rgba(60, 40, 0, 1); }
  .rn { position: absolute; font-size: 8px; color: #3a2800; font-weight: 600; line-height: 1; }
  .orizz .rn { top: 1px; left: 2px; }
  .vert .rn { left: 2px; top: 1px; }
  .rig-pos {
    position: absolute; background: #d11; color: #fff; font-size: 10px; padding: 1px 5px;
    border-radius: 4px; font-variant-numeric: tabular-nums; white-space: nowrap;
  }
  .orizz .rig-pos { left: 4px; top: 50%; transform: translateY(-50%); }
  .vert .rig-pos { top: 4px; left: 50%; transform: translateX(-50%); }

  .disegno { position: absolute; inset: 0; width: 100%; height: 100%; pointer-events: none; }
  .disegno line { stroke: #ff3b30; stroke-width: 2; }
  .area-box { position: absolute; border: 2px solid #ff3b30; background: rgba(255, 59, 48, 0.12); pointer-events: none; }
  .punto {
    position: absolute; width: 10px; height: 10px; margin: -5px 0 0 -5px; border-radius: 50%;
    background: #ff3b30; border: 2px solid #fff; box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.4); pointer-events: none;
  }

  /* Pannello — layout pulito, niente sovrapposizioni */
  .pannello {
    position: absolute; top: 12px; right: 12px; width: 184px;
    display: flex; flex-direction: column; gap: 6px;
    padding: 10px 12px; background: rgba(22, 22, 26, 0.92); color: #fff;
    border-radius: 10px; box-shadow: 0 6px 20px rgba(0, 0, 0, 0.5);
    font-size: 13px; pointer-events: auto; cursor: default;
  }
  .pannello .tit { font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em; opacity: 0.7; }
  .modi { display: flex; gap: 4px; }
  .modi button {
    flex: 1; background: rgba(255, 255, 255, 0.1); color: #fff; border: 1px solid rgba(255, 255, 255, 0.18);
    border-radius: 6px; padding: 4px 2px; cursor: pointer; font-size: 11px;
  }
  .modi button.on { background: var(--accento, #3b82f6); border-color: transparent; }
  .riga2 { display: flex; align-items: center; gap: 8px; }
  .riga2 select {
    flex: 1; background: rgba(255, 255, 255, 0.1); color: #fff;
    border: 1px solid rgba(255, 255, 255, 0.2); border-radius: 6px; padding: 4px 6px; font-size: 12px;
  }
  .orient { display: flex; gap: 4px; }
  .orient button {
    width: 28px; background: rgba(255, 255, 255, 0.1); color: #fff;
    border: 1px solid rgba(255, 255, 255, 0.2); border-radius: 6px; padding: 4px 0; cursor: pointer; font-size: 13px;
  }
  .orient button.on { background: var(--accento, #3b82f6); border-color: transparent; }
  .val { display: flex; align-items: baseline; gap: 6px; }
  .val .k { opacity: 0.6; font-size: 11px; min-width: 56px; }
  .val b { font-variant-numeric: tabular-nums; font-size: 15px; }
  .val .u { opacity: 0.7; font-size: 12px; }
  .val.small { font-size: 11px; opacity: 0.75; }
  .val.small b { font-size: 11px; }
  .hint { opacity: 0.75; font-size: 12px; }
  .dim {
    margin-top: 2px; padding-top: 6px; border-top: 1px solid rgba(255, 255, 255, 0.18);
    font-size: 10px; opacity: 0.65; line-height: 1.4;
  }
  .pannello button:not(.modi button):not(.orient button) {
    margin-top: 2px; background: rgba(255, 255, 255, 0.14); color: #fff;
    border: 1px solid rgba(255, 255, 255, 0.25); border-radius: 6px; padding: 5px 8px; cursor: pointer; font-size: 12px;
  }
</style>
