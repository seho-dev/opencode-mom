## Anti-patterns

- Do not fetch large usage pages through CLI stdout: CLI `process.exit` can truncate responses; stdout is reserved for the small bootstrap health result.
- Do not deduplicate fork usage by message IDs or timestamp cutoffs: copied messages receive new IDs, and owned child messages may have older timestamps.
