<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { dismissToast, type Toast } from '../state.svelte';
  import Icon from './Icon.svelte';

  let { t }: { t: Toast } = $props();

  // Timed toasts pause while the pointer or keyboard focus is on them (WCAG 2.2.1); errors never time out.
  let remaining = 0;
  let startedAt = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  function resume() {
    if (!t.ttl || timer) return;
    startedAt = Date.now();
    timer = setTimeout(() => dismissToast(t.id), Math.max(remaining, 1500));
  }
  function pause() {
    if (!timer) return;
    clearTimeout(timer);
    timer = undefined;
    remaining -= Date.now() - startedAt;
  }
  onMount(() => {
    remaining = t.ttl;
    resume();
  });
  onDestroy(() => clearTimeout(timer));
</script>

<div
  class="toast {t.kind}"
  role={t.kind === 'err' ? 'alert' : 'status'}
  onpointerenter={pause}
  onpointerleave={resume}
  onfocusin={pause}
  onfocusout={resume}
>
  <span class="toast-text">{t.text}</span>
  <button class="icon-btn toast-close" aria-label="Dismiss notification" onclick={() => dismissToast(t.id)}><Icon name="x" size={16} /></button>
</div>
