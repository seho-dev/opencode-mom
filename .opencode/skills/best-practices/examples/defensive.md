# Boundary Safety & Simplicity Examples

> Concrete patterns and examples supporting [../rules/defensive.md](../rules/defensive.md).

---

## Language-Independent Baseline (Illustrative Pseudocode)

These examples describe contracts, not a required language, library, or architecture. Runtime predicates perform real checks; record construction alone is not validation.

### 1. Validate Once; Default Only Legitimate Absence

Example contract: an order is an object with a mandatory exact status of `paid`, `pending`, or `canceled`. `retry_count` may be absent, meaning `0`; when present it must be a non-negative integer. No coercion or case normalization is agreed.

```text
function parse_order(raw):
    if not is_object(raw) or not has_field(raw, "status"):
        fail InvalidInput("order requires status")

    status = raw["status"]
    if not is_string(status) or status not in {"paid", "pending", "canceled"}:
        fail InvalidInput("invalid status")

    if has_field(raw, "retry_count"):
        retry_count = raw["retry_count"]
        if not is_integer(retry_count) or retry_count < 0:
            fail InvalidInput("invalid retry_count")
    else:
        retry_count = 0

    return ValidatedOrder(status, retry_count)

function status_label(order):
    # Internal contract already guarantees a known status.
    return {"paid": "Paid", "pending": "Pending", "canceled": "Canceled"}[order.status]
```

Missing `retry_count` defaults to `0`; a supplied `0` remains valid. A supplied null, negative number, string, or boolean fails the integer contract. Missing, unknown, or uppercase status fails rather than becoming `pending`. Internal consumers do not repeat these checks while the validated contract remains intact.

### 2. Domain Policy vs Pass-Through Helper

```text
# Unnecessary alias: no meaning or responsibility beyond the existing operation.
function filter_items(items, predicate):
    return filter(items, predicate)

# Prefer the existing operation directly.
active_users = filter(users, user -> user.active)

# Useful even with one caller: names an agreed shipment-release policy.
function can_release_shipment(shipment):
    return shipment.payment_status equals "paid"
        and shipment.inventory_reserved
        and not shipment.on_hold
```

The policy assumes validated shipment data and an agreed release contract. Its value is domain meaning, not its length or how many tests or callers use it.

### 3. Meaningful Error Ownership and Cleanup

Here `using` represents structured resource management that closes the file on every exit. The named errors illustrate expected failures, not a required exception hierarchy.

```text
function load_order(path):
    using source = open_read_only(path):
        return parse_order(decode_json(source.read()))
    # Read, decode, and validation failures propagate; no catch-log-rethrow.

function import_command(path):
    try:
        order = load_order(path)
        save_order(order)
    catch ReadFailure, DecodeFailure, InvalidInput, SaveFailure as error:
        report_import_failure(error)
        return Failure
    return Success
```

The command boundary can report failure and return an unsuccessful outcome. The loader cannot recover, so it propagates failures while still releasing the file. No layer substitutes an empty order or reports successful work after a failed save.

---

## Frontend & TypeScript Patterns

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

### 2. Parse and Validate at Ingestion Boundary

```ts
// ❌ Bad: Scattering ad-hoc parsing and case checks across multiple UI components
function OrderBadge({ status }: { status: string }) {
  if (status === 'PAID' || status === 'paid' || status === '1') {
    return <span className="badge-success">Paid</span>;
  }
  return <span className="badge-default">Pending</span>;
}

// ✅ Good: Validate exact allowed values at receipt; downstream logic uses consistent enum/map lookup
type OrderStatus = 'paid' | 'pending' | 'canceled';
const ALLOWED_ORDER_STATUSES: readonly OrderStatus[] = ['paid', 'pending', 'canceled'];

const STATUS_LABEL_MAP: Record<OrderStatus, string> = {
  paid: 'Paid',
  pending: 'Pending',
  canceled: 'Canceled',
};

function parseOrderStatus(raw: unknown): OrderStatus {
  if (typeof raw !== 'string') {
    throw new TypeError('order status must be a string');
  }
  if (!ALLOWED_ORDER_STATUSES.includes(raw as OrderStatus)) {
    throw new RangeError(`invalid order status: ${raw}`);
  }
  return raw as OrderStatus;
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
