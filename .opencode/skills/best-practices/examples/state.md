# State Management Rules - Code Examples

> Reference code examples for [../rules/state.md](../rules/state.md).

## Patterns & Examples

### 1. Minimal State vs Redundant Derived State

```tsx
// ❌ Bad: Redundant state variables that must be synchronized manually
function TodoApp() {
  const [todos, setTodos] = useState<Todo[]>([]);
  const [completedCount, setCompletedCount] = useState(0); // Redundant

  const handleToggle = (id: string) => {
    const updated = todos.map((t) => (t.id === id ? { ...t, done: !t.done } : t));
    setTodos(updated);
    setCompletedCount(updated.filter((t) => t.done).length); // Prone to drift
  };
  // ...
}

// ✅ Good: Derive counts directly from the primary state
function TodoApp() {
  const [todos, setTodos] = useState<Todo[]>([]);
  const completedCount = todos.filter((t) => t.done).length;

  const handleToggle = (id: string) => {
    setTodos((prev) => prev.map((t) => (t.id === id ? { ...t, done: !t.done } : t)));
  };
  // ...
}
```

### 2. Prop Drilling vs Composition / Context

```tsx
// ❌ Bad: Drilling currentUser through unrelated intermediate layout components
function App({ currentUser }) {
  return <Layout currentUser={currentUser} />;
}
function Layout({ currentUser }) {
  return <Sidebar currentUser={currentUser} />;
}
function Sidebar({ currentUser }) {
  return <UserProfile user={currentUser} />;
}

// ✅ Good (Option A - Composition): Pass the leaf component directly via children
function App({ currentUser }) {
  return (
    <Layout>
      <Sidebar>
        <UserProfile user={currentUser} />
      </Sidebar>
    </Layout>
  );
}

// ✅ Good (Option B - Shared Context/Store): Access state directly at the consumer
function UserProfile() {
  const currentUser = useCurrentUser();
  return <div>{currentUser.name}</div>;
}
```

### 3. Isolated Editing Buffer vs State Mirroring

```tsx
// ✅ Good: Copying to local state is ONLY valid when creating an isolated draft buffer
function EditDialog({ initialData, onSave, onCancel }) {
  // Deliberate draft copy: edits do NOT reflect into parent until confirmed
  const [draft, setDraft] = useState(initialData);

  return (
    <div>
      <input
        value={draft.title}
        onChange={(e) => setDraft({ ...draft, title: e.target.value })}
      />
      <button onClick={() => onSave(draft)}>Save</button>
      <button onClick={onCancel}>Cancel</button>
    </div>
  );
}
```
