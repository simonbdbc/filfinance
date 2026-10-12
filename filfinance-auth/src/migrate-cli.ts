import { closePools } from "./db.js";
import { migrateDatabases } from "./migrations.js";

try {
  await migrateDatabases();
  console.info("FilFinance auth and app migrations are up to date.");
} finally {
  await closePools();
}
