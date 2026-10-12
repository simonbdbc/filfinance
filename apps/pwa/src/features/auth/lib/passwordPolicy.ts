export const PASSWORD_MIN_LENGTH = 10;
export const PASSWORD_MAX_LENGTH = 128;

export type PasswordValidity = "empty" | "too-short" | "too-long" | "valid";

export function passwordValidity(password: string): PasswordValidity {
  if (password.length === 0) return "empty";
  if (password.length < PASSWORD_MIN_LENGTH) return "too-short";
  if (password.length > PASSWORD_MAX_LENGTH) return "too-long";
  return "valid";
}

export function isValidNewPassword(password: string): boolean {
  return passwordValidity(password) === "valid";
}
