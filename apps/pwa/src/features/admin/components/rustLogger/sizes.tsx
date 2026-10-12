import type { FileSize } from "../../lib/rustLogger/protocol";

export function formatBytes(value: string | null | undefined) {
  if (value == null) return "indisponible";
  const bytes = BigInt(value);
  const units = ["o", "Kio", "Mio", "Gio", "Tio", "Pio", "Eio"];
  let divisor = 1n;
  let unit = 0;
  while (bytes >= divisor * 1024n && unit < units.length - 1) {
    divisor *= 1024n;
    unit++;
  }
  return unit === 0
    ? `${bytes} o`
    : `${bytes / divisor},${((bytes % divisor) * 10n) / divisor} ${units[unit]}`;
}

export function Sizes({
  size,
  loading = false,
}: {
  size: FileSize | undefined;
  loading?: boolean;
}) {
  return (
    <span
      className="entry-sizes"
      title={
        size
          ? `Contenu : ${size.content} octets · Alloué : ${size.allocated ?? "indisponible"}${size.partial ? " · Résultat partiel" : ""}`
          : loading
            ? "Calcul en cours"
            : "Taille indisponible"
      }
    >
      {size ? (
        <>
          Contenu {formatBytes(size.content)} · Alloué {formatBytes(size.allocated)}
          {size.partial ? " · partiel" : ""}
        </>
      ) : loading ? (
        "Calcul en cours…"
      ) : (
        "Taille indisponible"
      )}
    </span>
  );
}
