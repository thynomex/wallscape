interface RevealOptions {
  delay?: number;
  distance?: string;
  once?: boolean;
  rootMargin?: string;
  threshold?: number;
}

const reduceMotion = () =>
  typeof window !== "undefined" &&
  window.matchMedia("(prefers-reduced-motion: reduce)").matches;

export function reveal(node: HTMLElement, options: RevealOptions = {}) {
  let observer: IntersectionObserver | null = null;
  let initialVisibilityFrame: number | null = null;

  function setOptions(next: RevealOptions) {
    node.style.setProperty("--reveal-delay", `${next.delay ?? 0}ms`);
    node.style.setProperty("--reveal-distance", next.distance ?? "18px");
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

  if (reduceMotion() || typeof IntersectionObserver === "undefined") {
    node.classList.add("is-visible");
    return {
      update: setOptions,
      destroy: () => node.classList.remove("motion-reveal", "is-visible"),
    };
  }

  observer = new IntersectionObserver(
    ([entry]) => {
      if (entry?.isIntersecting) {
        node.classList.add("is-visible");
        if (options.once !== false) observer?.unobserve(node);
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
      node.classList.add("is-visible");
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
      observer?.disconnect();
      node.classList.remove("motion-reveal", "is-visible");
    },
  };
}
