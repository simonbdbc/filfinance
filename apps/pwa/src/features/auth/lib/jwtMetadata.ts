const BASE64_ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

export type JwtMetadata = {
  expiresAt: number | null;
  subject: string;
};

export function readJwtMetadata(token: string): JwtMetadata | null {
  try {
    const encodedPayload = token.split(".")[1];
    if (!encodedPayload) {
      return null;
    }

    const payload = JSON.parse(decodeBase64UrlUtf8(encodedPayload)) as {
      exp?: unknown;
      sub?: unknown;
    };
    if (typeof payload.sub !== "string" || !payload.sub) {
      return null;
    }

    return {
      expiresAt: typeof payload.exp === "number" ? payload.exp * 1_000 : null,
      subject: payload.sub,
    };
  } catch {
    return null;
  }
}

function decodeBase64UrlUtf8(value: string) {
  const normalized = value.replaceAll("-", "+").replaceAll("_", "/").replace(/=+$/, "");
  if (!normalized || normalized.length % 4 === 1) {
    throw new Error("Invalid base64url payload");
  }

  const bytes: number[] = [];
  let accumulator = 0;
  let bitCount = 0;

  for (const character of normalized) {
    const index = BASE64_ALPHABET.indexOf(character);
    if (index < 0) {
      throw new Error("Invalid base64url character");
    }
    accumulator = (accumulator << 6) | index;
    bitCount += 6;

    if (bitCount >= 8) {
      bitCount -= 8;
      bytes.push((accumulator >> bitCount) & 0xff);
      accumulator &= (1 << bitCount) - 1;
    }
  }

  if (bitCount > 0 && accumulator !== 0) {
    throw new Error("Invalid base64url padding bits");
  }

  const percentEncoded = bytes
    .map((byte) => `%${byte.toString(16).padStart(2, "0")}`)
    .join("");
  return decodeURIComponent(percentEncoded);
}
