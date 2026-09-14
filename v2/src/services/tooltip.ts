/**
 * Replaces the browser's delayed native `title` tooltip with an instant,
 * custom-styled tooltip. Every element carrying a `title` attribute gets a
 * hover hint on `mouseover` with zero delay (the native tooltip is also
 * suppressed by removing the attribute while hovered).
 */
export function initInstantTooltips() {
  if (typeof document === 'undefined') return

  let tooltip: HTMLDivElement | null = null
  let activeTarget: HTMLElement | null = null

  function ensureTooltip(): HTMLDivElement {
    if (tooltip) return tooltip
    tooltip = document.createElement('div')
    tooltip.className = 'vb-tooltip'
    document.body.appendChild(tooltip)
    return tooltip
  }

  function position(target: HTMLElement) {
    if (!tooltip) return
    const rect = target.getBoundingClientRect()
    const tipRect = tooltip.getBoundingClientRect()
    const padding = 8
    let top = rect.top - tipRect.height - 8
    if (top < padding) top = rect.bottom + 8
    let left = rect.left + rect.width / 2 - tipRect.width / 2
    left = Math.max(padding, Math.min(left, window.innerWidth - tipRect.width - padding))
    tooltip.style.left = `${left}px`
    tooltip.style.top = `${top}px`
  }

  function show(target: HTMLElement) {
    const title = target.getAttribute('title')
    if (!title) return
    if (activeTarget && activeTarget !== target) hide(activeTarget)
    const tip = ensureTooltip()
    tip.textContent = title
    target.dataset.vbTitle = title
    target.removeAttribute('title')
    tip.style.opacity = '1'
    activeTarget = target
    position(target)
  }

  function hide(target: HTMLElement) {
    if (activeTarget !== target) return
    if (tooltip) tooltip.style.opacity = '0'
    const saved = target.dataset.vbTitle
    if (saved) {
      target.setAttribute('title', saved)
      delete target.dataset.vbTitle
    }
    activeTarget = null
  }

  function relatedInside(related: EventTarget | null): boolean {
    return !!(related instanceof Node && activeTarget?.contains(related))
  }

  document.addEventListener('mouseover', (e) => {
    const target = e.target as HTMLElement
    if (!target || typeof target.closest !== 'function') return
    if (activeTarget && activeTarget.contains(target)) return
    const tipEl = target.closest('[title]') as HTMLElement | null
    if (tipEl) show(tipEl)
  })

  document.addEventListener(
    'mouseleave',
    (e) => {
      if (!activeTarget) return
      const target = e.target as Node
      if (!activeTarget.contains(target)) return
      if (relatedInside(e.relatedTarget)) return
      hide(activeTarget)
    },
    true
  )
}
