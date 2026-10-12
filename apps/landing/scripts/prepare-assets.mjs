import { mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";

import sharp from "sharp";

const root = fileURLToPath(new URL("../", import.meta.url));
const sourceDir = fileURLToPath(new URL("../src/assets/source/", import.meta.url));
const imageDir = fileURLToPath(new URL("../public/images/", import.meta.url));
const iconDir = fileURLToPath(new URL("../public/icons/", import.meta.url));
const socialDir = fileURLToPath(new URL("../public/social/", import.meta.url));

const responsiveSets = [
  {
    input: `${sourceDir}hero-source.png`,
    name: "hero",
    ratio: 16 / 9,
    position: "centre",
  },
  {
    input: `${sourceDir}final-cta-source.png`,
    name: "final-cta",
    ratio: 2,
    position: "centre",
  },
];

const widths = [640, 1024, 1600];

await Promise.all([imageDir, iconDir, socialDir].map((directory) => mkdir(directory, { recursive: true })));

for (const set of responsiveSets) {
  for (const width of widths) {
    const height = Math.round(width / set.ratio);
    const resized = sharp(set.input).resize({
      width,
      height,
      fit: "cover",
      position: set.position,
      withoutEnlargement: false,
    });

    await Promise.all([
      resized.clone().avif({ effort: 7, quality: 58 }).toFile(`${imageDir}${set.name}-${width}.avif`),
      resized.clone().webp({ effort: 6, quality: 78 }).toFile(`${imageDir}${set.name}-${width}.webp`),
    ]);
  }
}

await sharp(`${sourceDir}hero-source.png`)
  .resize({ width: 1200, height: 630, fit: "cover", position: "centre" })
  .jpeg({ chromaSubsampling: "4:4:4", mozjpeg: true, quality: 82 })
  .toFile(`${socialDir}fil-finance-social.jpg`);

const iconSource = fileURLToPath(new URL("../../pwa/assets/favicon.png", import.meta.url));
const icons = [
  { name: "favicon-32.png", size: 32 },
  { name: "favicon-64.png", size: 64 },
  { name: "apple-touch-icon.png", size: 180 },
  { name: "icon-192.png", size: 192 },
];

await Promise.all(
  icons.map(({ name, size }) =>
    sharp(iconSource)
      .resize({ width: size, height: size, fit: "cover" })
      .png({ compressionLevel: 9, palette: true })
      .toFile(`${iconDir}${name}`),
  ),
);

console.log(`[assets] Images et icônes préparées dans ${root}public.`);
