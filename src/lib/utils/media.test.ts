import { expect, it, vi } from "vitest";
import {
  formatDimensions,
  formatFileSizeMb,
  formatResolution,
  wallpaperMediaLabel,
} from "./media";

vi.mock("@tauri-apps/api/core", () => ({ convertFileSrc: (path: string) => path }));

it("labels videos by frame rate, then duration, and never as 0fps", () => {
  expect(wallpaperMediaLabel({ media_type: "image", fps: 0, duration_ms: 0 })).toBe("Image");
  expect(wallpaperMediaLabel({ media_type: "video", fps: 29.97, duration_ms: 12_000 })).toBe(
    "30fps video",
  );
  expect(wallpaperMediaLabel({ media_type: "video", fps: 0, duration_ms: 12_400 })).toBe(
    "12s video",
  );
  expect(wallpaperMediaLabel({ media_type: "video", fps: 0, duration_ms: 0 })).toBe("Video");
  expect(wallpaperMediaLabel(null)).toBe("");
});

it("formats dimensions with × and hides unknown ones", () => {
  expect(formatDimensions(1920, 1080)).toBe("1920×1080");
  expect(formatDimensions(0, 1080)).toBe("");
  expect(formatDimensions(undefined, undefined)).toBe("");
  expect(formatResolution("3840x2160")).toBe("3840×2160");
  expect(formatResolution(null)).toBe("");
});

it("formats file sizes as whole megabytes and hides unknown ones", () => {
  expect(formatFileSizeMb(0)).toBe("");
  expect(formatFileSizeMb(undefined)).toBe("");
  expect(formatFileSizeMb(200 * 1024)).toBe("1MB");
  expect(formatFileSizeMb(48.6 * 1024 * 1024)).toBe("49MB");
});
