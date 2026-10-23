# Defensive Coding & Simplicity Rules - Code Examples

> Reference code examples for [../rules/defensive.md](../rules/defensive.md).

## Patterns & Examples

### 1. Zero Speculative Abstraction

```ts
// ❌ Bad: Thin wrapper functions that merely forward native expressions
const getActiveUsers = (users) => users.filter((u) => u.isActive);
const getUserDisplayName = (user) => user.name ?? 'Anonymous';
const isPositive = (count) => count > 0;

// ✅ Good: Use direct, idiomatic expressions where intent is obvious
const activeUsers = users.filter((u) => u.isActive);
const displayName = user.name ?? 'Anonymous';
if (count > 0) { /* ... */ }

// ✅ Good: Extract ONLY when genuine domain policy or multi-step logic exists
function canPublishArticle(article: Article): boolean {
  return article.status === 'approved' && !article.isArchived && article.wordCount >= 300;
}
```

### 2. Normalization at Ingestion Boundary

```ts
// ❌ Bad: Scattering ad-hoc parsing and case checks across multiple UI components
function OrderBadge({ status }: { status: string }) {
  if (status === 'PAID' || status === 'paid' || status === '1') {
    return <span className="badge-success">Paid</span>;
  }
  return <span className="badge-default">Pending</span>;
}

// ✅ Good: Normalize at data receipt; downstream logic uses consistent enum/map lookup
type OrderStatus = 'paid' | 'pending' | 'canceled';

const STATUS_LABEL_MAP: Record<OrderStatus, string> = {
  paid: 'Paid',
  pending: 'Pending',
  canceled: 'Canceled',
};

function normalizeOrderStatus(raw: unknown): OrderStatus {
  const normalized = String(raw ?? '').trim().toLowerCase();
  return normalized in STATUS_LABEL_MAP ? (normalized as OrderStatus) : 'pending';
}

function OrderBadge({ status }: { status: OrderStatus }) {
  return <span className="badge">{STATUS_LABEL_MAP[status]}</span>;
}
```

### 3. Safe Fallbacks vs Guessing Chains

```ts
// ❌ Bad: Speculative fallback chains guessing where data might hide
const title = item.title ?? item.name ?? item.label ?? item.header ?? 'Untitled';

// ❌ Bad: Unintentional truthy check breaking valid zero values
const count = response.count || 10; // When count is 0, incorrectly defaults to 10!

// ✅ Good: Explicit nullish coalescing with guaranteed boundary contract
const count = response.count ?? 10;
const title = item.title ?? 'Untitled';
```
