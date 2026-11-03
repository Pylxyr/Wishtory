/**
 * Accessible tooltip action: `use:tip={'text'}` or `use:tip={{ text, placement }}`.
 * Shows on pointer hover and on keyboard focus, stays open while the pointer is over the bubble,
 * closes with Escape, and links the trigger to a hidden description through aria-describedby so
 * screen readers announce it when the trigger gets focus (WCAG 1.4.13).
 */
export type TipParam = string | { text: string; placement?: 'below' | 'right' };

let bubble: HTMLDivElement | null = null;
let descs: HTMLDivElement | null = null;
let owner: HTMLElement | null = null;
let hideTimer: ReturnType<typeof setTimeout> | undefined;
let nextId = 1;

function setup() {
  if (bubble) return;
  bubble = document.createElement('div');
  bubble.className = 'tip';
  bubble.setAttribute('aria-hidden', 'true'); // the hidden description below is what assistive tech reads
  bubble.hidden = true;
  bubble.addEventListener('pointerenter', () => clearTimeout(hideTimer));
  bubble.addEventListener('pointerleave', () => scheduleHide());
  document.body.appendChild(bubble);
  descs = document.createElement('div');
  descs.className = 'sr';
  descs.setAttribute('aria-hidden', 'true'); // still read through aria-describedby, but not a page region of its own
  document.body.appendChild(descs);
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && owner && bubble && !bubble.hidden) {
      e.stopPropagation(); // dismiss the tooltip only, not a dialog behind it
      hide();
    }
  }, true);
}

function scheduleHide() {
  clearTimeout(hideTimer);
  hideTimer = setTimeout(hide, 140);
}
function hide() {
  clearTimeout(hideTimer);
  if (bubble) bubble.hidden = true;
  owner = null;
}
function show(node: HTMLElement, text: string, placement: 'below' | 'right') {
  setup();
  if (!bubble) return;
  clearTimeout(hideTimer);
  owner = node;
  bubble.textContent = text;
  bubble.hidden = false;
  const r = node.getBoundingClientRect();
  const b = bubble.getBoundingClientRect();
  const margin = 8;
  let left: number, top: number;
  if (placement === 'right') {
    left = r.right + margin;
    top = r.top + r.height / 2 - b.height / 2;
  } else {
    left = r.right - b.width; // right-aligned under the trigger; most triggers sit near the right edge
    top = r.bottom + margin;
  }
  left = Math.min(Math.max(left, margin), window.innerWidth - b.width - margin);
  if (top + b.height > window.innerHeight - margin) top = Math.max(margin, r.top - b.height - margin);
  top = Math.max(top, margin);
  bubble.style.left = `${Math.round(left)}px`;
  bubble.style.top = `${Math.round(top)}px`;
}

export function tip(node: HTMLElement, param: TipParam) {
  setup();
  const id = `tipdesc-${nextId++}`;
  const desc = document.createElement('div');
  desc.id = id;
  descs?.appendChild(desc);
  const previous = node.getAttribute('aria-describedby');
  node.setAttribute('aria-describedby', previous ? `${previous} ${id}` : id);

  let text = '';
  let placement: 'below' | 'right' = 'below';
  const apply = (p: TipParam) => {
    text = typeof p === 'string' ? p : p.text;
    placement = typeof p === 'string' ? 'below' : p.placement ?? 'below';
    desc.textContent = text;
    if (owner === node && bubble && !bubble.hidden) show(node, text, placement);
  };
  apply(param);

  const open = () => show(node, text, placement);
  const close = () => scheduleHide();
  node.addEventListener('pointerenter', open);
  node.addEventListener('pointerleave', close);
  node.addEventListener('focus', open);
  node.addEventListener('blur', hide);
  // A trigger that is not itself a control still has to be reachable by keyboard.
  const added = !node.matches('button, a, input, select, textarea, [tabindex]');
  if (added) node.tabIndex = 0;

  return {
    update: apply,
    destroy() {
      node.removeEventListener('pointerenter', open);
      node.removeEventListener('pointerleave', close);
      node.removeEventListener('focus', open);
      node.removeEventListener('blur', hide);
      if (owner === node) hide();
      desc.remove();
      if (previous) node.setAttribute('aria-describedby', previous); else node.removeAttribute('aria-describedby');
      if (added) node.removeAttribute('tabindex');
    }
  };
}
