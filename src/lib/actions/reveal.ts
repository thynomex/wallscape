interface RevealOptions {
  delay?: number;
  distance?: string;
  once?: boolean;
  rootMargin?: string;
  threshold?: number;
}

// Matches --motion-slow, plus headroom for a missed transitionend.
const REVEAL_SETTLE_MS = 700;

const reduceMotion = () =>
  typeof window !== "undefined" &&
  window.matchMedia("(prefers-reduced-motion: reduce)").matches;

export function reveal(node: HTMLElement, options: RevealOptions = {}) {
  let observer: IntersectionObserver | null = null;
  let initialVisibilityFrame: number | null = null;
  let settleTimer: ReturnType<typeof setTimeout> | null = null;
  let settled = false;

  // Without motion there is nothing to reveal; leave the node untouched so no
  // will-change/filter lingers on it.
  if (reduceMotion() || typeof IntersectionObserver === "undefined") {
    return {
      update() {},
      destroy() {},
    };
  }

  function setOptions(next: RevealOptions) {
    if (settled) return;
    node.style.setProperty("--reveal-delay", `${next.delay ?? 0}ms`);
    node.style.setProperty("--reveal-distance", next.distance ?? "18px");
  }

  function clearSettleTimer() {
    if (settleTimer !== null) {
      clearTimeout(settleTimer);
      settleTimer = null;
    }
  }

  // Once revealed, drop every reveal hook so the element is left with its
  // own resting styles. A finished reveal otherwise keeps filter and
  // will-change: filter, which blocks descendants' backdrop-filter.
  function settle() {
    if (settled) return;
    settled = true;
    clearSettleTimer();
    node.removeEventListener("transitionend", handleTransitionEnd);
    node.classList.remove("motion-reveal", "is-visible");
    node.style.removeProperty("--reveal-delay");
    node.style.removeProperty("--reveal-distance");
  }

  function handleTransitionEnd(event: TransitionEvent) {
    if (event.target === node) settle();
  }

  function show() {
    if (node.classList.contains("is-visible")) return;
    node.classList.add("is-visible");

    if (options.once === false) return;

    observer?.unobserve(node);
    node.addEventListener("transitionend", handleTransitionEnd);
    clearSettleTimer();
    settleTimer = setTimeout(settle, (options.delay ?? 0) + REVEAL_SETTLE_MS);
  }

  function isInitiallyVisible() {
    const rect = node.getBoundingClientRect();
    const viewportHeight = window.innerHeight || document.documentElement.clientHeight;
    const viewportWidth = window.innerWidth || document.documentElement.clientWidth;

    return (
      rect.bottom >= 0 &&
      rect.right >= 0 &&
      rect.top <= viewportHeight &&
      rect.left <= viewportWidth
    );
  }

  setOptions(options);
  node.classList.add("motion-reveal");

  observer = new IntersectionObserver(
    ([entry]) => {
      if (entry?.isIntersecting) {
        show();
      } else if (options.once === false) {
        node.classList.remove("is-visible");
      }
    },
    {
      rootMargin: options.rootMargin ?? "0px 0px -8% 0px",
      threshold: options.threshold ?? 0.16,
    },
  );

  observer.observe(node);
  initialVisibilityFrame = requestAnimationFrame(() => {
    if (isInitiallyVisible()) {
      show();
    }
    initialVisibilityFrame = null;
  });

  return {
    update(next: RevealOptions) {
      options = next;
      setOptions(next);
    },
    destroy() {
      if (initialVisibilityFrame !== null) {
        cancelAnimationFrame(initialVisibilityFrame);
      }
      clearSettleTimer();
      node.removeEventListener("transitionend", handleTransitionEnd);
      observer?.disconnect();
      node.classList.remove("motion-reveal", "is-visible");
    },
  };
}
