import sitemap from "@astrojs/sitemap";
import { defineConfig } from "astro/config";

import { siteConfig } from "./src/config/site.config.ts";

export default defineConfig({
  integrations: [sitemap()],
  output: "static",
  site: siteConfig.origin,
  trailingSlash: "always",
});
