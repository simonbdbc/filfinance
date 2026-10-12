import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import type { Pool } from "pg";
import { appPool, authPool } from "./db.js";

export async function migrateDatabases(): Promise<void> {
  await migrateDirectory({
    pool: authPool,
    schema: "auth",
    directory: "auth-migrations",
    lockName: "filfinance-auth-migrations"
  });
  await migrateDirectory({
    pool: appPool,
    schema: "app",
    directory: "app-migrations",
    lockName: "filfinance-app-migrations"
  });
}

type MigrationTarget = {
  pool: Pool;
  schema: "auth" | "app";
  directory: string;
  lockName: string;
};

async function migrateDirectory(target: MigrationTarget): Promise<void> {
  const migrationsDirectory = path.join(process.cwd(), "db", target.directory);
  const migrationNames = (await readdir(migrationsDirectory))
    .filter((name) => /^\d+.*\.sql$/.test(name))
    .sort();
  const client = await target.pool.connect();

  try {
    await client.query("SELECT pg_advisory_lock(hashtext($1))", [target.lockName]);
    await client.query(`
      CREATE TABLE IF NOT EXISTS ${target.schema}.schema_migrations (
        name text PRIMARY KEY,
        applied_at timestamptz NOT NULL DEFAULT now()
      )
    `);

    for (const name of migrationNames) {
      const alreadyApplied = await client.query(
        `SELECT 1 FROM ${target.schema}.schema_migrations WHERE name = $1`,
        [name]
      );
      if (alreadyApplied.rowCount) {
        continue;
      }

      const sql = await readFile(path.join(migrationsDirectory, name), "utf8");
      await client.query("BEGIN");
      try {
        await client.query(sql);
        await client.query(
          `INSERT INTO ${target.schema}.schema_migrations (name) VALUES ($1)`,
          [name]
        );
        await client.query("COMMIT");
        console.info(`Applied ${target.schema} migration ${name}`);
      } catch (error) {
        await client.query("ROLLBACK");
        throw error;
      }
    }
  } finally {
    await client.query("SELECT pg_advisory_unlock(hashtext($1))", [target.lockName]).catch(() => undefined);
    client.release();
  }
}
