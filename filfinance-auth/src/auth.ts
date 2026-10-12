import { expo } from "@better-auth/expo";
import { betterAuth } from "better-auth";
import { admin as adminPlugin, jwt } from "better-auth/plugins";
import { authPool } from "./db.js";
import { verificationEmailContent } from "./email-templates.js";
import { env } from "./env.js";
import { sendAuthMail } from "./mailer.js";

export const auth = betterAuth({
  appName: "FilFinance",
  baseURL: env.BETTER_AUTH_URL,
  secret: env.BETTER_AUTH_SECRET,
  database: authPool,
  emailAndPassword: {
    enabled: true,
    requireEmailVerification: env.BETTER_AUTH_REQUIRE_EMAIL_VERIFICATION,
    minPasswordLength: 10,
    maxPasswordLength: 128,
    revokeSessionsOnPasswordReset: true,
    customSyntheticUser: ({ coreFields, additionalFields, id }) => ({
      ...coreFields,
      role: "listener",
      banned: false,
      banReason: null,
      banExpires: null,
      ...additionalFields,
      id
    }),
    sendResetPassword: async ({ user, url }) => {
      await sendAuthMail({
        to: user.email,
        subject: "Reinitialisation de votre mot de passe Fil Finance",
        text: `Ouvrez ce lien pour choisir un nouveau mot de passe: ${url}`
      });
    }
  },
  emailVerification: {
    // The client calls /send-verification-email explicitly after sign-up so an
    // SMTP failure reaches the UI instead of being swallowed by background work.
    sendOnSignUp: false,
    sendOnSignIn: false,
    autoSignInAfterVerification: true,
    sendVerificationEmail: async ({ user, url }) => {
      const content = verificationEmailContent(url);
      await sendAuthMail({
        to: user.email,
        subject: "Verification de votre adresse Fil Finance",
        ...content
      });
    }
  },
  trustedOrigins: env.TRUSTED_ORIGINS,
  plugins: [
    expo(),
    adminPlugin({
      defaultRole: "listener",
      adminRoles: ["admin"]
    }),
    jwt({
      jwks: {
        keyPairConfig: {
          alg: "RS256",
          modulusLength: 2048
        },
        rotationInterval: 60 * 60 * 24 * 30,
        gracePeriod: 60 * 60 * 24 * 30
      },
      jwt: {
        issuer: env.BETTER_AUTH_URL,
        audience: env.BETTER_AUTH_JWT_AUDIENCE,
        expirationTime: env.BETTER_AUTH_JWT_EXPIRATION,
        definePayload: ({ user }) => ({
          email: user.email,
          email_verified: user.emailVerified,
          name: user.name,
          role: typeof user.role === "string" ? user.role : "listener"
        })
      }
    })
  ]
});
