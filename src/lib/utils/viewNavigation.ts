export const VIEW_NAMES = ["home", "discover", "explore", "library"] as const;

export type ViewName = (typeof VIEW_NAMES)[number];

export function isViewName(view: string): view is ViewName {
  return VIEW_NAMES.includes(view as ViewName);
}
