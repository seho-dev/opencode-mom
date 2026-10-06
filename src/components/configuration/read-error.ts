export function readError(cause: unknown): string {
  if (cause && typeof cause === 'object' && 'message' in cause) return String(cause.message);
  return String(cause);
}

export function isNotFound(cause: unknown): boolean {
  return !!cause && typeof cause === 'object' && 'code' in cause && cause.code === 'not_found';
}
