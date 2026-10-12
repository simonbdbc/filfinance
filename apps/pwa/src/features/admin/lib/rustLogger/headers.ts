/** Combines caller headers with the reader's mandatory authenticated headers. */
export function mergeReaderHeaders(
  extra: HeadersInit | undefined,
  required: Record<string, string | undefined>,
) {
  const headers = new Headers(extra);
  for (const [name, value] of Object.entries(required)) {
    if (value) headers.set(name, value);
  }
  return headers;
}
