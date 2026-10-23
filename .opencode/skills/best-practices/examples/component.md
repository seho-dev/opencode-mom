# Component Architecture & Design Rules - Code Examples

> Reference code examples for [../rules/component.md](../rules/component.md).

## Patterns & Examples

### 1. State Ownership & Props Flow

```tsx
// ❌ Bad: Child mirrors parent state and attempts to two-way sync
function UserProfile({ user, onUpdate }) {
  const [localName, setLocalName] = useState(user.name);

  useEffect(() => {
    setLocalName(user.name);
  }, [user.name]);

  const handleChange = (e) => {
    setLocalName(e.target.value);
    onUpdate({ ...user, name: e.target.value });
  };

  return <input value={localName} onChange={handleChange} />;
}

// ✅ Good: Single owner; child renders value and emits change intent
function UserProfile({ user, onNameChange }) {
  return (
    <input
      value={user.name}
      onChange={(e) => onNameChange(e.target.value)}
    />
  );
}
```

### 2. Derived State Computation

```tsx
// ❌ Bad: Mirroring derived state and syncing with effects
function ItemList({ items }) {
  const [activeItems, setActiveItems] = useState([]);

  useEffect(() => {
    setActiveItems(items.filter((item) => item.isActive));
  }, [items]);

  return <ul>{activeItems.map((item) => <li key={item.id}>{item.name}</li>)}</ul>;
}

// ✅ Good: Derive directly in render (memoize only if computation is heavy)
function ItemList({ items }) {
  const activeItems = items.filter((item) => item.isActive);

  return <ul>{activeItems.map((item) => <li key={item.id}>{item.name}</li>)}</ul>;
}
```

### 3. Imperative Handles (Behavior vs Raw Setter)

```tsx
// ❌ Bad: Leaking raw state setters through an imperative handle
useImperativeHandle(ref, () => ({
  setIsOpen: (open) => setIsOpen(open),
  setData: (data) => setData(data),
}));

// ✅ Good: Exposing explicit behavioral actions
useImperativeHandle(ref, () => ({
  focusInput: () => inputRef.current?.focus(),
  resetForm: () => setFormState(initialState),
}));
```

### 4. Prop Destructuring & Ordering

```tsx
// ❌ Bad: Unstructured props access and mixed callback placement
function OrderCard(props) {
  return <div onClick={props.onSelect}>{props.order.id}</div>;
}

// ✅ Good: Destructured, default values explicit, callbacks grouped at end
interface OrderCardProps {
  orderId: string;
  status?: string;
  isExpanded?: boolean;
  onSelect: (id: string) => void;
  onCancel?: (id: string) => void;
}

function OrderCard({
  orderId,
  status = 'pending',
  isExpanded = false,
  onSelect,
  onCancel,
}: OrderCardProps) {
  return (
    <div onClick={() => onSelect(orderId)}>
      <span>{orderId} ({status})</span>
    </div>
  );
}
```

### 5. Accessible Tab Navigation

```text
Container: <div role="tablist" aria-label="Section title">
Tab buttons:
  <button
    role="tab"
    id="tab-{id}"
    aria-selected={activeTab === id}
    aria-controls="tabpanel-{id}"
    tabindex={activeTab === id ? 0 : -1}
    onclick={() => selectTab(id)}
    onkeydown={handleTabKeyNavigation}
  >
Tab panel:
  <div id="tabpanel-{id}" role="tabpanel" aria-labelledby="tab-{id}">

Keyboard cycling (ArrowRight, ArrowLeft, Home, End):
  ArrowRight: next = tabs[(current + 1) % tabs.length]
  ArrowLeft:  next = tabs[(current - 1 + tabs.length) % tabs.length]
  Home:       next = tabs[0]
  End:        next = tabs[tabs.length - 1]
  selectTab(next); focusTab(next);
```

### 6. Asynchronous Action Feedback and Guards

```text
// ✅ Async action button with disabled guard, aria-busy, and motion-safe indicator
<button
  disabled={isLoading || isSaving}
  aria-busy={isLoading}
  onclick={handleRefresh}
>
  <SpinnerIcon
    class={isLoading ? "animate-spin motion-reduce:animate-none" : ""}
    aria-hidden="true"
  />
  <span>Refresh</span>
</button>

// ✅ Context-aware header action in multi-tab or scoped views
onclick={() => {
  if (activeTab === "tabA") refreshTabA();
  else if (activeTab === "tabB") refreshTabB();
}}
```

### 7. Standardized Table Empty and Loading States

```text
// ✅ Standardized empty row primitive rather than ad-hoc <tr><td colspan>
<table>
  <thead>...</thead>
  <tbody>
    {#if isLoading}
      <EmptyTableRow colspan={colCount} message="Loading..." />
    {:else if !items.length}
      <EmptyTableRow colspan={colCount} message="No items found" />
    {:else}
      {#each items as item}
        <tr>...</tr>
      {/each}
    {/if}
  </tbody>
</table>
```

### 8. Bounded Raw Content Containers

```text
// ✅ Bounded preformatted text, code, JSON, logs with overflow containment
<pre
  class="max-h-96 overflow-y-auto whitespace-pre-wrap break-words [overflow-wrap:anywhere]"
>{rawContent}</pre>
```

