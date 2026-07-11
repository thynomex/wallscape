<script lang="ts">
  import { toastStore, type Toast } from "$lib/stores/toasts.svelte";
  import { resolveMediaSrc } from "$lib/utils/media";
  import Icon from "./Icon.svelte";

  function getIcon(type: string) {
    switch (type) {
      case "success":
        return "check";
      case "error":
        return "x";
      case "warning":
        return "alert-triangle";
      case "info":
        return "info";
      default:
        return "info";
    }
  }
</script>

<div class="toast-container">
  {#each toastStore.toasts as toast (toast.id)}
    <div class="toast toast-{toast.type}" class:dismissing={toast.dismissing}>
      {#if resolveMediaSrc(toast.thumbnail)}
        <div class="toast-thumbnail">
          <img
            src={resolveMediaSrc(toast.thumbnail) ?? ""}
            alt=""
            draggable={false}
            ondragstart={(event) => event.preventDefault()}
          />
        </div>
      {:else}
        <div class="toast-icon">
          <Icon name={getIcon(toast.type)} size={18} />
        </div>
      {/if}
      <div class="toast-content">
        <span class="toast-message">{toast.message}</span>
        <div class="toast-progress" style="animation-duration: {toast.duration}ms;"></div>
      </div>
    </div>
  {/each}
</div>

<style>
  .toast-container {
    position: fixed;
    bottom: 20px;
    right: 20px;
    z-index: 9999;
    display: flex;
    flex-direction: column;
    gap: 8px;
    pointer-events: none;
  }

  .toast {
    pointer-events: auto;
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 240px;
    max-width: 320px;
    padding: 12px 16px;
    background: rgba(10, 12, 16, 0.75);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 20px;
    box-shadow:
      0 8px 32px rgba(0, 0, 0, 0.7),
      0 4px 16px rgba(0, 0, 0, 0.5),
      inset 0 1px 0 rgba(255, 255, 255, 0.08);
    backdrop-filter: blur(40px) saturate(180%);
    -webkit-backdrop-filter: blur(40px) saturate(180%);
    transform-origin: bottom right;
    animation: toast-slide-in 360ms var(--ease-emphasized) both;
  }

  @keyframes toast-slide-in {
    from {
      opacity: 0;
      transform: translate3d(18px, 10px, 0) scale(0.94);
      filter: blur(10px);
    }
    to {
      opacity: 1;
      transform: translate3d(0, 0, 0) scale(1);
      filter: blur(0);
    }
  }

  .toast.dismissing {
    animation: toast-slide-out 260ms var(--ease-standard) forwards;
  }

  @keyframes toast-slide-out {
    from {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
    to {
      opacity: 0;
      transform: translate3d(10px, -8px, 0) scale(0.95);
      filter: blur(6px);
    }
  }

  .toast-thumbnail {
    flex-shrink: 0;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    overflow: hidden;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .toast-thumbnail img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    animation: toast-thumb-in 520ms var(--ease-emphasized) both;
  }

  .toast-icon {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: rgba(52, 211, 153, 0.1);
    animation: toast-icon-in 420ms var(--ease-spring) both;
  }

  .toast-success .toast-icon {
    background: rgba(52, 211, 153, 0.1);
    color: #10b981;
  }

  .toast-error .toast-icon {
    background: rgba(248, 113, 113, 0.1);
    color: #ef4444;
  }

  .toast-warning .toast-icon {
    background: rgba(251, 191, 36, 0.1);
    color: #f59e0b;
  }

  .toast-info .toast-icon {
    background: rgba(96, 165, 250, 0.1);
    color: #3b82f6;
  }

  .toast-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .toast-message {
    font-size: 14px;
    font-weight: 600;
    color: #ffffff;
    line-height: 1.4;
    letter-spacing: -0.01em;
  }

  .toast-progress {
    height: 2px;
    background: rgba(255, 255, 255, 0.85);
    border-radius: 1px;
    transform-origin: left;
    animation: toast-progress linear forwards;
  }

  @keyframes toast-thumb-in {
    from {
      transform: scale(1.2);
      filter: blur(6px);
    }
    to {
      transform: scale(1);
      filter: blur(0);
    }
  }

  @keyframes toast-icon-in {
    from {
      transform: scale(0.72);
      opacity: 0;
    }
    to {
      transform: scale(1);
      opacity: 1;
    }
  }

  @keyframes toast-progress {
    from {
      transform: scaleX(1);
    }
    to {
      transform: scaleX(0);
    }
  }

  @media (max-width: 640px) {
    .toast-container {
      bottom: 16px;
      right: 12px;
      left: 12px;
    }

    .toast {
      min-width: unset;
      max-width: unset;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .toast,
    .toast.dismissing,
    .toast-thumbnail img,
    .toast-icon {
      animation: none !important;
    }
  }
</style>
