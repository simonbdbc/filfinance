import { mkdir, writeFile } from "node:fs/promises";
import { dirname } from "node:path";
import { SMTPServer } from "smtp-server";

const outputFile = process.env.FILFINANCE_SMTP_CAPTURE_FILE;
const readyFile = process.env.FILFINANCE_SMTP_READY_FILE;
const port = Number.parseInt(process.env.FILFINANCE_SMTP_CAPTURE_PORT || "2525", 10);
const maxMessageBytes = 1024 * 1024;

if (!outputFile || !readyFile) {
  throw new Error("FILFINANCE_SMTP_CAPTURE_FILE and FILFINANCE_SMTP_READY_FILE are required");
}
if (!Number.isInteger(port) || port < 1 || port > 65535) {
  throw new Error("FILFINANCE_SMTP_CAPTURE_PORT must be a valid TCP port");
}

const server = new SMTPServer({
  authOptional: true,
  disabledCommands: ["AUTH", "STARTTLS"],
  logger: false,
  size: maxMessageBytes,
  onData(stream, _session, callback) {
    const chunks = [];
    let byteCount = 0;
    let completed = false;

    const finish = (error) => {
      if (completed) return;
      completed = true;
      callback(error);
    };

    stream.on("data", (chunk) => {
      byteCount += chunk.length;
      if (byteCount <= maxMessageBytes) {
        chunks.push(chunk);
      }
    });
    stream.on("error", finish);
    stream.on("end", async () => {
      if (stream.sizeExceeded || byteCount > maxMessageBytes) {
        finish(new Error("Captured SMTP message exceeds the one-megabyte test limit"));
        return;
      }

      try {
        await mkdir(dirname(outputFile), { recursive: true, mode: 0o700 });
        await writeFile(outputFile, Buffer.concat(chunks), { flag: "wx", mode: 0o600 });
        finish();
      } catch (error) {
        finish(error instanceof Error ? error : new Error("Unable to persist SMTP capture"));
      }
    });
  },
});

server.on("error", (error) => {
  console.error(`SMTP capture server failed: ${error.message}`);
  process.exitCode = 1;
});

server.listen(port, "127.0.0.1", async () => {
  await mkdir(dirname(readyFile), { recursive: true, mode: 0o700 });
  await writeFile(readyFile, "ready\n", { flag: "wx", mode: 0o600 });
  console.log(`SMTP capture server ready on 127.0.0.1:${port}`);
});

function shutdown() {
  server.close(() => process.exit(0));
}

process.on("SIGINT", shutdown);
process.on("SIGTERM", shutdown);
