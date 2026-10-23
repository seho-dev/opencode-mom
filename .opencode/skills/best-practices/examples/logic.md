# Business Logic & Async Rules - Code Examples

> Reference code examples for [../rules/logic.md](../rules/logic.md).

## Patterns & Examples

### 1. Presentation & Logic Separation

```tsx
// ❌ Bad: Bloating the UI component with business calculations and async logic
function UserStatusWidget({ userId }) {
  const [data, setData] = useState(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    let active = true;
    setLoading(true);
    fetch(`/api/users/${userId}/metrics`)
      .then((res) => res.json())
      .then((res) => {
        if (!active) return;
        // Complex domain scoring calculation mixed in view layer
        const score = res.points * 1.5 + (res.tier === 'VIP' ? 100 : 0);
        setData({ ...res, score });
      })
      .finally(() => active && setLoading(false));
    return () => { active = false; };
  }, [userId]);

  if (loading) return <div>Loading...</div>;
  return <div>Score: {data?.score}</div>;
}

// ✅ Good: Domain logic extracted into a reusable hook or utility function
function UserStatusWidget({ userId }) {
  const { data, isLoading } = useUserMetrics(userId);

  if (isLoading) return <div>Loading...</div>;
  return <div>Score: {data?.score}</div>;
}
```

### 2. Side Effect Cleanup & Cancellation

```tsx
// ❌ Bad: Uncleaned listeners and fire-and-forget requests that leak or set state after unmount
useEffect(() => {
  window.addEventListener('resize', handleResize);
  fetchData(id).then((result) => setData(result));
  // Missing cleanup listener and missing abort signal
}, [id]);

// ✅ Good: Clean up listeners and cancel pending requests
useEffect(() => {
  const controller = new AbortController();

  const handleResize = () => setDimensions(window.innerWidth);
  window.addEventListener('resize', handleResize);

  fetchData(id, { signal: controller.signal })
    .then((result) => setData(result))
    .catch((error) => {
      if (error.name !== 'AbortError') {
        showErrorMessage(error);
      }
    });

  return () => {
    window.removeEventListener('resize', handleResize);
    controller.abort();
  };
}, [id]);
```

### 3. Predictable Async State Machine

```tsx
// ✅ Good: Explicit status representation prevents impossible states (e.g., loading && error)
type AsyncState<T> =
  | { status: 'idle'; data: null; error: null }
  | { status: 'loading'; data: T | null; error: null }
  | { status: 'success'; data: T; error: null }
  | { status: 'error'; data: T | null; error: Error };
```
