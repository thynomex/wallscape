import type { MotionBgsSearchArgs } from "$lib/types/wallpaper";

export interface MotionBgsSearchFields {
  query: string;
  category: string;
}

export function motionBgsSearchArgs(
  fields: MotionBgsSearchFields,
  page: number,
): MotionBgsSearchArgs {
  return {
    query: fields.query.trim() || undefined,
    category: fields.category || "latest",
    page,
  };
}
