export function initial(value: string): string {
  return value.trim().charAt(0).toUpperCase() || 'S';
}

export function excerpt(value: string, limit = 118): string {
  const normalized = value.replace(/\s+/g, ' ').trim();
  return normalized.length > limit ? `${normalized.slice(0, limit)}…` : normalized;
}

export function issueLabel(type: string): string {
  return type.replaceAll('_', ' ').replace(/^./, (value) => value.toUpperCase());
}
