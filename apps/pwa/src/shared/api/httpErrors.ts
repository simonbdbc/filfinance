function readErrorField(payload: unknown) {
  if (!payload || typeof payload !== "object") {
    return null;
  }

  const fields = payload as Record<string, unknown>;
  for (const key of ["error", "message"]) {
    const value = fields[key];
    if (typeof value === "string" && value.trim().length > 0) {
      return value.trim();
    }
  }

  return null;
}

export async function buildHttpErrorMessage(response: Response, context: string) {
  let detail: string | null = null;

  try {
    const rawBody = await response.text();
    const trimmedBody = rawBody.trim();

    if (trimmedBody.length > 0) {
      try {
        detail = readErrorField(JSON.parse(trimmedBody));
      } catch {
        detail = trimmedBody;
      }
    }
  } catch {
    detail = null;
  }

  return detail
    ? `${context} HTTP ${response.status}: ${detail}`
    : `${context} HTTP ${response.status}`;
}

export async function throwHttpError(
  response: Response,
  context: string,
): Promise<never> {
  throw new Error(await buildHttpErrorMessage(response, context));
}
