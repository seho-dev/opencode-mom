import type { McpServer } from '$src/types/mcp.js';

const hidden = '••••••';
const credential = /secret|password|passwd|token|api[_-]?key|authorization/i;
// ponytail: explicit credential forms only; add named patterns for custom formats.
const credentialName =
  '(?:[\\w]+[_-])*(?:secret|password|passwd|token|api[_-]?key|authorization|access[_-]?token|refresh[_-]?token|client[_-]?secret|key)';
const explicitCredential = new RegExp(`^${credentialName}$`, 'i');
const assignment = new RegExp(`(^|\\s)((?:--?)?${credentialName}=)(?:"[^"]*"|'[^']*'|[^\\s]+)`, 'gi');
const flag = new RegExp(`(^|\\s)(--?${credentialName})(\\s+)(?:"[^"]*"|'[^']*'|[^\\s]+)`, 'gi');
const arrayAssignment = new RegExp(`^((?:--?)?${credentialName}=)[\\s\\S]*$`, 'i');
const arrayFlag = new RegExp(`^--?${credentialName}$`, 'i');

function maskText(value: string): string {
  return value
    .replace(/([a-z][a-z\d+.-]*:\/\/)[^/\s?#]*@/gi, `$1${hidden}@`)
    .replace(/([?&])([^=&#\s]+)=([^&#\s]*)/g, (match, separator, name) => {
      let decoded = name;
      try {
        decoded = decodeURIComponent(name);
      } catch {
        // Malformed query names still use the literal credential check.
      }
      return explicitCredential.test(decoded) ? `${separator}${name}=${hidden}` : match;
    })
    .replace(assignment, `$1$2${hidden}`)
    .replace(flag, `$1$2$3${hidden}`);
}

export function maskTarget(server: Pick<McpServer, 'type' | 'target' | 'config'>): string {
  const command = server.config.command;
  if (server.type === 'local' && Array.isArray(command) && command.every((entry) => typeof entry === 'string')) {
    return (maskConfig(command) as string[]).join(' ');
  }
  return maskText(server.target);
}

export function maskConfig(value: unknown, key = ''): unknown {
  if (/^(headers|env|environment)$/i.test(key) && value && typeof value === 'object') {
    return Object.fromEntries(Object.keys(value).map((name) => [name, hidden]));
  }
  if (credential.test(key)) return hidden;
  if (Array.isArray(value)) {
    return value.map((entry, index) => {
      const previous = value[index - 1];
      if (typeof previous === 'string' && arrayFlag.test(previous)) return hidden;
      if (typeof entry === 'string') return maskText(entry.replace(arrayAssignment, `$1${hidden}`));
      return maskConfig(entry);
    });
  }
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([name, entry]) => [name, maskConfig(entry, name)]));
  }
  return typeof value === 'string' ? maskText(value) : value;
}
