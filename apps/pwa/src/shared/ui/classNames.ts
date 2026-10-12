type ClassNameValue = string | false | null | undefined;

export function cx(...classNames: ClassNameValue[]) {
  return classNames.filter(Boolean).join(" ");
}
