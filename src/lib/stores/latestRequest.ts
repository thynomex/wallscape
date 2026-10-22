/** Independent request sequence for each store instance and operation. */
export function latestRequest() {
  const generations = new WeakMap<object, number>();

  return (owner: object) => {
    const generation = (generations.get(owner) ?? 0) + 1;
    generations.set(owner, generation);
    return () => generations.get(owner) === generation;
  };
}
