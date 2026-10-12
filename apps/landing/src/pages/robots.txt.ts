import type { APIRoute } from "astro";

import { siteConfig } from "../config/site.config";

export const prerender = true;

export const GET: APIRoute = () => new Response(
  `User-agent: *\nAllow: /\n\nSitemap: ${siteConfig.origin}sitemap-index.xml\n`,
  {
    headers: {
      "content-type": "text/plain; charset=utf-8",
    },
  },
);
