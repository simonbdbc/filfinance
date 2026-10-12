type AuthEmailContent = {
  text: string;
  html: string;
};

export function verificationEmailContent(url: string): AuthEmailContent {
  const escapedUrl = escapeHtml(url);

  return {
    text: `Ouvrez ce lien pour verifier votre adresse Fil Finance:\n\n<${url}>`,
    html: [
      '<!doctype html><html lang="fr"><body style="margin:0;padding:24px;background:#f4f5f7;color:#17202a;font-family:Arial,sans-serif">',
      '<div style="max-width:560px;margin:0 auto;background:#ffffff;border:1px solid #d9dee5;border-radius:8px;padding:24px">',
      '<h1 style="margin:0 0 12px;font-size:22px;line-height:28px">Confirmez votre adresse email</h1>',
      '<p style="margin:0 0 20px;font-size:15px;line-height:22px">Activez votre compte Fil Finance en utilisant le bouton ci-dessous.</p>',
      `<a href="${escapedUrl}" style="display:inline-block;padding:12px 18px;border-radius:6px;background:#176b4d;color:#ffffff;text-decoration:none;font-size:15px;font-weight:700">Verifier mon adresse</a>`,
      '<p style="margin:20px 0 0;font-size:12px;line-height:18px;color:#5f6b76">Si vous n\'etes pas a l\'origine de cette demande, ignorez cet email.</p>',
      "</div></body></html>",
    ].join(""),
  };
}

function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}
