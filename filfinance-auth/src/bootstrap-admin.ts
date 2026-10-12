import { appPool, authPool } from "./db.js";
import { env } from "./env.js";

let bootstrapComplete = false;

function normalizeEmail(value: string): string {
  return value.trim().toLowerCase();
}

export async function promoteInitialAdminIfReady(): Promise<void> {
  if (bootstrapComplete) {
    return;
  }

  const email = normalizeEmail(env.ADMIN_EMAIL_TO_PROMOTE);
  if (!email) {
    bootstrapComplete = true;
    return;
  }

  const client = await appPool.connect();
  try {
    await client.query("BEGIN");
    await client.query("SELECT pg_advisory_xact_lock(hashtext($1))", [
      "filfinance-initial-admin"
    ]);
    await client.query(`
      CREATE TABLE IF NOT EXISTS app.auth_bootstrap (
        bootstrap_key text PRIMARY KEY,
        promoted_auth_user_id text,
        promoted_email text,
        promoted_at timestamptz NOT NULL DEFAULT now()
      )
    `);

    const existing = await client.query(
      "SELECT bootstrap_key FROM app.auth_bootstrap WHERE bootstrap_key = $1 FOR UPDATE",
      ["initial_admin"]
    );
    if (existing.rowCount && existing.rowCount > 0) {
      await client.query("COMMIT");
      bootstrapComplete = true;
      return;
    }

    const user = await authPool.query<{
      id: string;
      email: string;
      emailVerified: boolean;
    }>(
      'SELECT id, email, "emailVerified" FROM auth."user" WHERE lower(email) = $1 LIMIT 1',
      [email]
    );

    if (user.rowCount !== 1) {
      await client.query("COMMIT");
      console.log(`Initial admin ${email} is not present yet; bootstrap deferred.`);
      return;
    }

    const row = user.rows[0];
    if (!row.emailVerified) {
      await client.query("COMMIT");
      console.log(`Initial admin ${email} has not verified email yet; bootstrap deferred.`);
      return;
    }

    await authPool.query('UPDATE auth."user" SET role = $1 WHERE id = $2', ["admin", row.id]);
    await client.query(
      `
        INSERT INTO app.auth_bootstrap (bootstrap_key, promoted_auth_user_id, promoted_email)
        VALUES ($1, $2, $3)
      `,
      ["initial_admin", row.id, row.email]
    );
    await client.query("COMMIT");
    bootstrapComplete = true;
    console.log(`Initial admin ${email} promoted.`);
  } catch (error) {
    await client.query("ROLLBACK");
    console.warn("Initial admin bootstrap skipped:", error);
  } finally {
    client.release();
  }
}
