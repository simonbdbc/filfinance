import { Pool } from "pg";
import { env } from "./env.js";

function ssl() {
  return env.POSTGRES_SSL ? { rejectUnauthorized: false } : undefined;
}

export const authPool = new Pool({
  connectionString: env.AUTH_DATABASE_URL,
  options: "-c search_path=auth",
  ssl: ssl()
});

export const appPool = new Pool({
  connectionString: env.DATABASE_URL,
  options: "-c search_path=app,public",
  ssl: ssl()
});

export async function closePools(): Promise<void> {
  await Promise.all([authPool.end(), appPool.end()]);
}
