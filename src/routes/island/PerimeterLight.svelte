<script lang="ts">
import type { IslandView } from '$src/types/island.js';

let {
  attention,
  view,
  notchedCompact,
  paused = false,
}: { attention: boolean; view: IslandView; notchedCompact: boolean; paused?: boolean } = $props();
</script>

{#snippet trace()}
  <rect class="light-track" width="100%" height="100%" rx="12" pathLength="100" />
  <rect class="light-halo" width="100%" height="100%" rx="12" pathLength="100" />
  <rect class="light-trail" width="100%" height="100%" rx="12" pathLength="100" />
  <rect class="light-head" width="100%" height="100%" rx="12" pathLength="100" />
{/snippet}

<div
  class="perimeter-light"
  class:attention={attention}
  class:paused={paused}
  class:notched={notchedCompact}
  class:expanded={view === 'expanded'}
  style:--light-top={view === 'compact' ? '0px' : 'var(--top-inset)'}
  aria-hidden="true"
>
  <svg class="light-single" aria-hidden="true">{@render trace()}</svg>
  <svg class="light-wing light-left" aria-hidden="true">{@render trace()}</svg>
  <svg class="light-wing light-right" aria-hidden="true">{@render trace()}</svg>
</div>

<style>
.perimeter-light {
  --light-color: #a4e8ce;
  position: absolute;
  z-index: 5;
  inset: 0;
  overflow: hidden;
  clip-path: inset(var(--light-top) 0 0 0);
  border-radius: inherit;
  color: var(--light-color);
  pointer-events: none;
  animation: light-lifetime 3.6s ease both;
}
.perimeter-light.attention {
  --light-color: var(--island-amber, #edc786);
  animation: none;
}
.perimeter-light.paused {
  visibility: hidden;
}
.perimeter-light.paused,
.paused :is(.light-halo, .light-trail, .light-head) {
  animation-play-state: paused;
}
/* The rect hangs 14px above the clip so only the left, bottom, and right outer edges show. */
svg {
  position: absolute;
  top: calc(var(--light-top) - 14px);
  left: 2.5px;
  width: calc(100% - 5px);
  height: calc(100% - var(--light-top) + 11.5px);
  overflow: visible;
  pointer-events: none;
}
.light-wing {
  display: none;
  width: calc((100% - var(--notch-width)) / 2 + 14px);
}
.light-left {
  clip-path: inset(0 14px 0 0);
}
.light-right {
  right: 2.5px;
  left: auto;
  clip-path: inset(0 0 0 14px);
}
.notched .light-single {
  display: none;
}
.notched .light-wing {
  display: block;
}
rect {
  fill: none;
  rx: 12px;
  stroke: currentColor;
  stroke-linecap: round;
}
.notched rect {
  rx: 12px;
}
.expanded rect {
  rx: 21px;
}
.light-track {
  stroke-width: 1;
  opacity: 0.38;
}
.light-halo,
.light-trail {
  stroke-dasharray: 16 84;
  animation: light-travel 3.2s linear both;
}
.light-halo {
  stroke-width: 5;
  opacity: 0.13;
}
.light-trail {
  stroke-width: 1.75;
  opacity: 0.7;
}
.light-head {
  stroke-width: 2;
  stroke-dasharray: 2 98;
  animation: light-head-travel 3.2s linear both;
}
.attention :is(.light-halo, .light-trail, .light-head) {
  animation-duration: 4s;
  animation-iteration-count: infinite;
}
@keyframes light-travel {
  from {
    stroke-dashoffset: 0;
  }
  to {
    stroke-dashoffset: -100;
  }
}
@keyframes light-head-travel {
  from {
    stroke-dashoffset: -16;
  }
  to {
    stroke-dashoffset: -116;
  }
}
@keyframes light-lifetime {
  0% {
    opacity: 0;
  }
  10%,
  85% {
    opacity: 1;
  }
  100% {
    opacity: 0;
  }
}
@media (prefers-reduced-motion: reduce) {
  .perimeter-light,
  .light-halo,
  .light-trail,
  .light-head {
    animation: none;
  }
  .light-track {
    stroke-width: 1.75;
    opacity: 0.85;
  }
  .light-halo,
  .light-trail,
  .light-head {
    display: none;
  }
}
</style>
