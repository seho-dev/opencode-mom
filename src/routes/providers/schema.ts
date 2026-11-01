import { z } from 'zod';

export function providerSchema(
  messages: { required: string; json: string; object: string; strings: string },
  requirePackage = true,
) {
  return z.object({
    name: z.string().refine((value) => !!value.trim(), messages.required),
    package: z.string().refine((value) => !requirePackage || !!value.trim(), messages.required),
    baseURL: z.string(),
    apiKey: z.string(),
    headers: z.string().transform((value, ctx) => {
      if (!value.trim()) return undefined;
      let parsed: unknown;
      try {
        parsed = JSON.parse(value);
      } catch {
        ctx.addIssue({ code: 'custom', message: messages.json });
        return z.NEVER;
      }
      if (!parsed || Array.isArray(parsed) || typeof parsed !== 'object') {
        ctx.addIssue({ code: 'custom', message: messages.object });
        return z.NEVER;
      }
      const result = z.record(z.string(), z.string()).safeParse(parsed);
      if (!result.success) {
        ctx.addIssue({ code: 'custom', message: messages.strings });
        return z.NEVER;
      }
      return result.data;
    }),
  });
}
