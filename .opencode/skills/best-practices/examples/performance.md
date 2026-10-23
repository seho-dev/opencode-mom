# Performance Optimization Rules - Code Examples

> Reference code examples for [../rules/performance.md](../rules/performance.md).

## Patterns & Examples

### 1. Lazy Loading Routes and Heavy Components

```tsx
// ❌ Bad: Statically bundling massive libraries into the initial page bundle
import HeavyDataChart from './HeavyDataChart';

function Dashboard() {
  const [showChart, setShowChart] = useState(false);
  return (
    <div>
      <button onClick={() => setShowChart(true)}>View Analytics</button>
      {showChart && <HeavyDataChart />}
    </div>
  );
}

// ✅ Good: Dynamically import heavy component on demand
import { lazy, Suspense } from 'react';

const HeavyDataChart = lazy(() => import('./HeavyDataChart'));

function Dashboard() {
  const [showChart, setShowChart] = useState(false);
  return (
    <div>
      <button onClick={() => setShowChart(true)}>View Analytics</button>
      {showChart && (
        <Suspense fallback={<div>Loading chart...</div>}>
          <HeavyDataChart />
        </Suspense>
      )}
    </div>
  );
}
```

### 2. Expensive Derivations & Callback Stability

```tsx
// ❌ Bad: Running expensive multi-step filter/sort on every unrelated re-render
function AnalyticsTable({ rawLogs, filterText, onSelectLog }) {
  // Re-runs even if rawLogs and filterText did not change!
  const processedLogs = rawLogs
    .filter((log) => log.message.includes(filterText))
    .sort((a, b) => b.timestamp - a.timestamp);

  // Re-creates new function reference on every parent render
  const handleSelect = (id: string) => onSelectLog(id);

  return <VirtualizedList items={processedLogs} onSelect={handleSelect} />;
}

// ✅ Good: Memoize expensive data derivations and stabilize handler references
function AnalyticsTable({ rawLogs, filterText, onSelectLog }) {
  const processedLogs = useMemo(() => {
    return rawLogs
      .filter((log) => log.message.includes(filterText))
      .sort((a, b) => b.timestamp - a.timestamp);
  }, [rawLogs, filterText]);

  const handleSelect = useCallback((id: string) => {
    onSelectLog(id);
  }, [onSelectLog]);

  return <VirtualizedList items={processedLogs} onSelect={handleSelect} />;
}
```
