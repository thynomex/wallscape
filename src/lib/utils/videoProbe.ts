import { convertFileSrc } from "@tauri-apps/api/core";
import type { ImportProbe } from "$lib/types/wallpaper";

interface ProbeVideoOptions {
  onStatus?: (status: string) => void;
}

export async function probeVideo(
  filePath: string,
  options: ProbeVideoOptions = {},
): Promise<ImportProbe> {
  const warnings: string[] = [];
  const video = document.createElement("video");
  video.preload = "metadata";
  video.muted = true;
  video.playsInline = true;
  video.crossOrigin = "anonymous";
  video.style.position = "fixed";
  video.style.width = "1px";
  video.style.height = "1px";
  video.style.opacity = "0";
  video.style.pointerEvents = "none";
  video.src = convertFileSrc(filePath);
  document.body.appendChild(video);

  try {
    options.onStatus?.("Reading video metadata...");
    await waitForVideoEvent(video, "loadedmetadata", 10_000);

    const duration = Number.isFinite(video.duration) ? video.duration : 0;
    const width = video.videoWidth || undefined;
    const height = video.videoHeight || undefined;
    const durationMs = duration > 0 ? Math.round(duration * 1000) : undefined;

    options.onStatus?.("Generating thumbnail...");
    const thumbnailDataUrl = await captureThumbnail(video, duration, warnings);

    options.onStatus?.("Estimating frame rate...");
    const fps = await estimateFps(video, duration, warnings);

    return {
      width,
      height,
      fps,
      durationMs,
      thumbnailDataUrl,
      warnings,
    };
  } catch (error) {
    warnings.push(`Video metadata probe failed: ${String(error)}`);
    return { warnings };
  } finally {
    video.pause();
    video.removeAttribute("src");
    video.load();
    video.remove();
  }
}

function waitForVideoEvent(
  video: HTMLVideoElement,
  eventName: keyof HTMLMediaElementEventMap,
  timeoutMs: number,
) {
  return new Promise<void>((resolve, reject) => {
    const timeout = window.setTimeout(() => {
      cleanup();
      reject(new Error(`${eventName} timed out`));
    }, timeoutMs);

    const onReady = () => {
      cleanup();
      resolve();
    };
    const onError = () => {
      cleanup();
      reject(new Error(video.error?.message || "video decode failed"));
    };
    const cleanup = () => {
      window.clearTimeout(timeout);
      video.removeEventListener(eventName, onReady);
      video.removeEventListener("error", onError);
    };

    video.addEventListener(eventName, onReady, { once: true });
    video.addEventListener("error", onError, { once: true });
  });
}

async function seekVideo(video: HTMLVideoElement, time: number) {
  if (!Number.isFinite(time) || time < 0) return;
  video.currentTime = time;
  await waitForVideoEvent(video, "seeked", 5_000);
}

async function captureThumbnail(
  video: HTMLVideoElement,
  duration: number,
  warnings: string[],
) {
  try {
    const seekTime = duration > 0 ? Math.min(Math.max(duration * 0.1, 0.5), 5) : 0;
    await seekVideo(video, seekTime);

    const sourceWidth = video.videoWidth || 1280;
    const sourceHeight = video.videoHeight || 720;
    const maxWidth = 720;
    const scale = Math.min(1, maxWidth / sourceWidth);
    const canvas = document.createElement("canvas");
    canvas.width = Math.max(1, Math.round(sourceWidth * scale));
    canvas.height = Math.max(1, Math.round(sourceHeight * scale));

    const ctx = canvas.getContext("2d");
    if (!ctx) throw new Error("canvas context unavailable");

    ctx.drawImage(video, 0, 0, canvas.width, canvas.height);
    return canvas.toDataURL("image/jpeg", 0.72);
  } catch (error) {
    warnings.push(`Thumbnail generation failed: ${String(error)}`);
    return undefined;
  }
}

async function estimateFps(
  video: HTMLVideoElement,
  duration: number,
  warnings: string[],
) {
  if (!("requestVideoFrameCallback" in video) || duration < 0.75) {
    warnings.push("Frame rate estimate was unavailable in this WebView.");
    return undefined;
  }

  try {
    const start = duration > 1.5 ? Math.min(0.5, duration - 1.25) : 0;
    await seekVideo(video, start);

    const fps = await new Promise<number | undefined>((resolve) => {
      let firstFrame: number | null = null;
      let firstTime: number | null = null;
      let lastFrame = 0;
      let lastTime = 0;
      let settled = false;

      const finish = () => {
        if (settled) return;
        settled = true;
        video.pause();

        if (firstFrame === null || firstTime === null || lastTime <= firstTime) {
          resolve(undefined);
          return;
        }

        const frames = Math.max(0, lastFrame - firstFrame);
        const seconds = lastTime - firstTime;
        const estimate = Math.round(frames / seconds);
        resolve(estimate >= 1 && estimate <= 240 ? estimate : undefined);
      };

      const timeout = window.setTimeout(finish, 1_300);
      const onFrame: VideoFrameRequestCallback = (_now, metadata) => {
        if (settled) return;
        firstFrame ??= metadata.presentedFrames;
        firstTime ??= metadata.mediaTime;
        lastFrame = metadata.presentedFrames;
        lastTime = metadata.mediaTime;

        if (firstTime !== null && metadata.mediaTime - firstTime >= 0.85) {
          window.clearTimeout(timeout);
          finish();
          return;
        }

        video.requestVideoFrameCallback(onFrame);
      };

      video.requestVideoFrameCallback(onFrame);
      video.play().catch(() => {
        window.clearTimeout(timeout);
        resolve(undefined);
      });
    });

    if (!fps) warnings.push("Frame rate estimate was unavailable.");
    return fps;
  } catch (error) {
    warnings.push(`Frame rate estimate failed: ${String(error)}`);
    return undefined;
  }
}
