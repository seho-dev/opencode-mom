import { SvelteMap } from 'svelte/reactivity';

type PaginationOptions = {
  key: () => string;
  query: () => string;
  total: () => number;
  ready: () => boolean;
  pageSize: number;
};

type SavedPage = { page: number; query: string };

const pagesByOwner = new WeakMap<object, SvelteMap<string, SavedPage>>();

export function createPagination(owner: object, options: PaginationOptions) {
  const pages = pagesByOwner.get(owner) ?? new SvelteMap<string, SavedPage>();

  pagesByOwner.set(owner, pages);

  $effect(() => {
    const key = options.key();
    const query = options.query();
    const saved = pages.get(key);
    let page = saved?.query === query ? saved.page : 1;

    // Only confirmed rows may lower a remembered page; loading and errors can temporarily expose empty lists.
    if (options.ready()) page = Math.max(1, Math.min(page, Math.ceil(options.total() / options.pageSize)));

    if (saved?.page !== page || saved.query !== query) pages.set(key, { page, query });
  });

  return {
    get page() {
      const saved = pages.get(options.key());

      return saved?.query === options.query() ? saved.page : 1;
    },
    set page(page: number) {
      pages.set(options.key(), { page, query: options.query() });
    },
  };
}
