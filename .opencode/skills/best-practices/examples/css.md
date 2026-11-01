# CSS & Styling Rules - Code Examples

> Reference code examples for [../rules/css.md](../rules/css.md).

## Patterns & Examples

### 1. Scoped Third-Party UI Overrides

```css
/* ❌ Bad: Global override polluting other pages and features */
.ant-btn-primary, .el-button--primary {
  border-radius: 20px;
  background-color: #ff5722;
}

/* ✅ Good: Scoped strictly under a feature container */
.marketing-banner .ant-btn-primary,
.marketing-banner .el-button--primary {
  border-radius: 20px;
  background-color: var(--color-marketing-accent);
}
```

### 2. Design Tokens vs Magic Numbers

```css
/* ❌ Bad: Hardcoded magic colors and arbitrary pixel spacings */
.user-card {
  color: #1890ff;
  border: 1px solid #d9d9d9;
  border-radius: 4px;
  padding: 12px 16px;
}

/* ✅ Good: Using design tokens / CSS custom properties */
.user-card {
  color: var(--color-primary);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  padding: var(--spacing-md) var(--spacing-lg);
}
```

### 3. Dynamic Inline Styles vs Class Toggles

```tsx
// ❌ Bad: Static styling dumped into inline style object
function Badge({ isActive, label }) {
  return (
    <span
      style={{
        display: 'inline-block',
        padding: '4px 8px',
        borderRadius: '4px',
        color: isActive ? '#52c41a' : '#bfbfbf',
      }}
    >
      {label}
    </span>
  );
}

// ✅ Good: Static rules in CSS; dynamic states toggled via classes or CSS variables
function Badge({ isActive, label }) {
  return (
    <span className={`badge ${isActive ? 'badge--active' : 'badge--inactive'}`}>
      {label}
    </span>
  );
}

// ✅ Good: Inline style reserved for genuine dynamic calculations
function ProgressBar({ percent }: { percent: number }) {
  return (
    <div className="progress-track">
      <div className="progress-fill" style={{ width: `${percent}%` }} />
    </div>
  );
}
```

### 4. Motion Reduction Compliance

```css
/* ✅ Provide fallback when user enables reduced motion */
@media (prefers-reduced-motion: reduce) {
  .animate-spin,
  .animate-pulse {
    animation: none !important;
    transition: none !important;
  }
}
```

### 5. Centralized Scrollbar Utility

```css
/* ✅ Ensure scrollable viewports share unified scrollbar styling */
.scrollable-container,
.form-body,
.table-wrap,
pre {
  scrollbar-gutter: stable;
  scrollbar-width: thin;
  scrollbar-color: var(--color-scrollbar-thumb) transparent;
}
```

