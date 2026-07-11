import type { Collection, StoreActionResult, Wallpaper } from "$lib/types/wallpaper";
import { collectionMembershipMessage } from "$lib/utils/toastMessages";

interface CollectionActions {
  createCollection(name: string): Promise<StoreActionResult<Collection>>;
  deleteCollection(collection: Collection): Promise<StoreActionResult>;
  setCollectionMembership(
    collection: Collection,
    wallpaper: Wallpaper,
    inCollection: boolean,
  ): Promise<StoreActionResult<Collection>>;
}

export interface CollectionWorkflowOutcome<T = void> {
  value: T;
  message: string;
}

export async function createCollection(
  name: string,
  actions: Pick<CollectionActions, "createCollection">,
): Promise<StoreActionResult<CollectionWorkflowOutcome<Collection>>> {
  const result = await actions.createCollection(name);
  if (!result.ok) return result;

  return {
    ok: true,
    value: {
      value: result.value,
      message: "Collection saved",
    },
  };
}

export async function deleteCollection(
  collection: Collection,
  actions: Pick<CollectionActions, "deleteCollection">,
): Promise<StoreActionResult<CollectionWorkflowOutcome>> {
  const result = await actions.deleteCollection(collection);
  if (!result.ok) return result;

  return {
    ok: true,
    value: {
      value: undefined,
      message: "Collection deleted",
    },
  };
}

export async function setCollectionMembership(
  collection: Collection,
  wallpaper: Wallpaper,
  inCollection: boolean,
  actions: Pick<CollectionActions, "setCollectionMembership">,
): Promise<StoreActionResult<CollectionWorkflowOutcome<Collection>>> {
  const result = await actions.setCollectionMembership(
    collection,
    wallpaper,
    inCollection,
  );
  if (!result.ok) return result;

  return {
    ok: true,
    value: {
      value: result.value,
      message: collectionMembershipMessage(inCollection),
    },
  };
}
