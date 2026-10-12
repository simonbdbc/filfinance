import nodemailer from "nodemailer";
import { env } from "./env.js";

type AuthMail = {
  to: string;
  subject: string;
  text: string;
  html?: string;
};

export class MailDeliveryError extends Error {
  constructor() {
    super("SMTP delivery failed");
    this.name = "MailDeliveryError";
  }
}

const transporter = env.SMTP_HOST
  ? nodemailer.createTransport({
      host: env.SMTP_HOST,
      port: env.SMTP_PORT,
      secure: env.SMTP_SECURE,
      requireTLS: env.SMTP_REQUIRE_TLS,
      auth: env.SMTP_USER
        ? {
            user: env.SMTP_USER,
            pass: env.SMTP_PASSWORD
          }
        : undefined
    })
  : null;

export async function sendAuthMail(message: AuthMail): Promise<void> {
  if (!transporter) {
    console.info(developmentMailLog());
    throw new MailDeliveryError();
  }

  try {
    await transporter.sendMail({
      from: {
        name: env.SMTP_FROM_NAME,
        address: env.SMTP_FROM_EMAIL
      },
      ...message
    });
  } catch {
    // Transport exceptions can contain server diagnostics. Do not allow them
    // to flow into request logs, where they could expose SMTP configuration.
    throw new MailDeliveryError();
  }
}

export function developmentMailLog(): string {
  return "[auth-mail:development] SMTP transport is not configured";
}

export async function closeMailer(): Promise<void> {
  transporter?.close();
}
