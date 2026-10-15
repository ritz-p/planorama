function lifecycle() {
  const adapters = [];
  return {
    register(adapter) { adapters.push(adapter); },
    reset() { for (const adapter of adapters) adapter.reset(); }
  };

}
