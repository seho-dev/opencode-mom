# Engineering Conventions & Project Structure - Code Examples

> Reference code examples for [../rules/conventions.md](../rules/conventions.md).

## Patterns & Examples

### 1. API Layer Separation

```tsx
// ❌ Bad: Embedding direct network requests and endpoints in UI components
function UserCard({ userId }: { userId: string }) {
  const [user, setUser] = useState(null);

  useEffect(() => {
    fetch(`https://api.example.com/v1/users/${userId}`, {
      headers: { Authorization: `Bearer ${getToken()}` },
    })
      .then((res) => res.json())
      .then((data) => setUser(data));
  }, [userId]);

  return <div>{user?.name}</div>;
}

// ✅ Good: Centralized API definition; component consumes clean function
// src/api/user.ts
export async function getUserProfile(userId: string): Promise<UserProfile> {
  return httpClient.get(`/users/${userId}`);
}

// src/components/UserCard.tsx
import { getUserProfile } from '@/api/user';

function UserCard({ userId }: { userId: string }) {
  const { data: user } = useQuery(['user', userId], () => getUserProfile(userId));
  return <div>{user?.name}</div>;
}
```

### 2. Path Aliases vs Fragile Traversal

```ts
// ❌ Bad: Fragile, deep relative path navigation
import { Button } from '../../../../components/common/Button';
import { formatDate } from '../../../utils/date';
import { USER_ROLES } from '../../constants';

// ✅ Good: Stable, project-configured path alias
import { Button } from '@/components/common/Button';
import { formatDate } from '@/utils/date';
import { USER_ROLES } from '@/constants/roles';
```

### 3. Centralized Constants vs Hardcoded Literals

```ts
// ❌ Bad: Hardcoded string literals and magic numbers scattered in code
if (user.role === 'admin' && status === 2) {
  expireInDays = 30;
}

// ✅ Good: Typed constants imported from domain constants module
export const USER_ROLES = {
  ADMIN: 'admin',
  MEMBER: 'member',
  GUEST: 'guest',
} as const;

export const ORDER_STATUS = {
  PENDING: 1,
  ACTIVE: 2,
  EXPIRED: 3,
} as const;

export const DEFAULT_SESSION_EXPIRATION_DAYS = 30;

// Call site:
if (user.role === USER_ROLES.ADMIN && status === ORDER_STATUS.ACTIVE) {
  expireInDays = DEFAULT_SESSION_EXPIRATION_DAYS;
}
```

### 4. File & Directory Layout Guideline (Illustrative)

```text
src/
├── api/                  # API endpoint functions (*.api.ts or domain modules)
├── assets/               # Static assets (images, icons, fonts)
├── components/           # Shared, domain-agnostic UI components
├── features/ (or pages/) # Feature or domain modules
│   └── order-management/
│       ├── components/   # Domain-specific components
│       ├── hooks/        # Feature business logic
│       ├── constants.ts  # Feature-specific constants
│       └── index.tsx     # Feature entry
├── hooks/                # Global / shared hooks
├── services/             # Core HTTP client instances, interceptors, telemetry
├── styles/               # Global tokens, resets, theme variables
├── types/                # Global TypeScript definitions
└── utils/                # Pure shared utility functions
```
