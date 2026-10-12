# Messagerie de production FilFinance

Ce document separe strictement la reception des demandes utilisateurs de
l'envoi transactionnel de l'application :

- `mail.sbcenter.fr` (Stalwart) recoit le courrier de `filfinance.fr` ;
- Better Auth envoie directement par le relais SMTP Brevo ;
- Stalwart envoie aussi ses messages externes par Brevo, avec une seconde cle
  SMTP dediee ;
- aucune cle SMTP ne doit etre ajoutee a Git, a une variable de build publique,
  a GitLab CI, ni aux logs.

## Etat DNS inventorie le 2026-09-02

Ne pas modifier la zone avant de conserver cet etat dans le ticket de
bascule :

| Enregistrement | Valeur actuelle |
| --- | --- |
| MX `filfinance.fr` | `1 mx1.mail.ovh.net.`, `5 mx2.mail.ovh.net.`, `100 mx3.mail.ovh.net.` (TTL 300) |
| SPF `filfinance.fr` | `v=spf1 include:mx.ovh.com -all` |
| DMARC `_dmarc.filfinance.fr` | `v=DMARC1; p=none; rua=mailto:rua@dmarc.brevo.com` (TTL 3600) |
| Preuve Brevo | `brevo-code` est deja publie a la racine |
| `mail.sbcenter.fr` | A `176.171.14.46` (TTL 300) |

Le PTR de `176.171.14.46` est actuellement
`boo67-h01-176-171-14-46.dsl.sta.abo.bbox.fr.`, et non `mail.sbcenter.fr`.
Il doit etre aligne par le fournisseur de cette IP pour la reputation du serveur
mail, mais ce defaut ne bloque pas les remises externes tant que Brevo assure
l'envoi sortant.

## Configuration privee de l'application

Creer ou modifier uniquement `/srv/filfinance/.env.server.prod` (mode `0600`,
proprietaire `gitlab-ci`). Les valeurs suivantes sont privees et ne sont jamais
des variables `EXPO_PUBLIC_*`, `VITE_*`, `NEXT_PUBLIC_*`, `PUBLIC_*` ou
`REACT_APP_*` :

```text
SMTP_HOST
SMTP_PORT
SMTP_SECURE
SMTP_REQUIRE_TLS
SMTP_USER
SMTP_PASSWORD
SMTP_FROM_EMAIL
SMTP_FROM_NAME
BETTER_AUTH_URL
```

Pour la production FilFinance, le preflight et le runtime imposent :

```text
SMTP_HOST=smtp-relay.brevo.com
SMTP_PORT=587
SMTP_SECURE=false
SMTP_REQUIRE_TLS=true
SMTP_FROM_EMAIL=noreply@filfinance.fr
BETTER_AUTH_URL=https://api.filfinance.fr
```

`SMTP_FROM_NAME` est le nom visible du produit (actuellement `FilFinance`).
`SMTP_USER` et `SMTP_PASSWORD` sont la premiere cle SMTP Brevo, dediee a
l'application. Ne pas reutiliser cette cle dans Stalwart. La configuration est
validee au demarrage sans afficher de valeur de secret et les echecs de
transport sont journalises de maniere generique.

Better Auth `1.6.23` utilise ses callbacks officiels
`emailVerification.sendVerificationEmail` et
`emailAndPassword.sendResetPassword`. Les callbacks de lien sont toujours
reecrits vers les deux pages HTTPS explicites de la PWA ; les origines
autorisees restent bornees a `filfinance://` et
`https://app.filfinance.fr` (plus la landing seulement si elle utilise Better
Auth).

## Brevo

1. Dans Brevo, verifier le domaine emetteur `filfinance.fr` et l'adresse
   `noreply@filfinance.fr` avant toute mise en production.
2. Creer une premiere cle SMTP dediee a FilFinance et la placer seulement dans
   `SMTP_USER` et `SMTP_PASSWORD` du fichier prive du serveur.
3. Creer une seconde cle SMTP dediee a Stalwart. Ne jamais la copier dans
   l'environnement FilFinance.
4. Copier dans DNS les enregistrements DKIM fournis par Brevo pour ce domaine.
   Les selecteurs et cibles sont propres au compte Brevo : ne pas les deviner
   ni ecraser un enregistrement existant.
5. Conserver un unique enregistrement SPF. Apres inventaire des emetteurs qui
   restent autorises, le mettre a jour avec l'autorisation Brevo fournie dans
   son ecran de verification ; ne pas publier plusieurs enregistrements SPF.
6. Conserver DMARC en observation pendant les premiers controles, puis faire
   evoluer progressivement sa politique apres validation DKIM/SPF des messages
   Brevo.

## Stalwart sur `mail.sbcenter.fr`

Configurer le domaine `filfinance.fr` dans Stalwart avec la boite
`support@filfinance.fr`, puis creer les alias suivants vers cette boite :

```text
noreply@filfinance.fr
postmaster@filfinance.fr
dmarc@filfinance.fr
```

Configurer les services exposes : SMTP entrant sur 25 avec TLS/STARTTLS,
IMAPS sur 993 et Submission sur 587 avec STARTTLS. Le port 587 doit exiger une
authentification ; le port 25 ne doit relayer que les domaines locaux ou les
flux explicitement autorises. Ne pas ouvrir de relais anonyme.

Dans la configuration de routage sortant Stalwart, definir une route de relais
SMTP vers `smtp-relay.brevo.com`, port 587, STARTTLS obligatoire, avec la
seconde cle Brevo. Selectionner cette route pour les destinations externes afin
que l'IP de `mail.sbcenter.fr` ne fasse jamais de remise directe. Stocker cette
seconde cle dans le magasin de secrets propre a Stalwart, pas dans le depot ni
dans l'environnement FilFinance.

## Bascule DNS reception

Apres test local de Stalwart et accord de bascule, remplacer les trois MX OVH
de `filfinance.fr` par un seul MX :

```dns
filfinance.fr. 300 IN MX 10 mail.sbcenter.fr.
```

Verifier d'abord que `mail.sbcenter.fr` pointe toujours vers l'IP Stalwart et
que le serveur accepte le domaine `filfinance.fr`. Garder les anciens MX dans
le ticket de changement pour le retour arriere ; ne les supprimer de la zone
qu'au moment de la bascule approuvee.

## Validation apres activation

1. Lancer `bash scripts/check_better_auth_env.sh /srv/filfinance/.env.server.prod prod` puis reconstruire uniquement le service auth avec la stack de production.
2. Depuis une boite externe de test, creer un compte et demander le lien de verification. Controler l'expediteur `FilFinance <noreply@filfinance.fr>`, l'origine `https://api.filfinance.fr` du lien et sa reception.
3. Demander une reinitialisation du mot de passe vers cette meme boite et verifier le lien de reset.
4. Envoyer un message externe vers `support@filfinance.fr`, puis confirmer sa reception dans Stalwart.
5. Repondre depuis `support@filfinance.fr` et confirmer dans Brevo que la remise utilise la cle Stalwart, distincte de celle de FilFinance.
6. Dans les en-tetes du message externe recu, verifier `DKIM=pass` et `DMARC=pass` (et SPF conforme). Archiver uniquement le resultat, jamais les identifiants SMTP ni les jetons de liens.

Les tests automatises du depot utilisent un serveur SMTP de capture local : ils
verifient les deux flux Better Auth, l'expediteur configure et les URLs de
verification/reinitialisation sans remettre de message a l'exterieur.

## Retour arriere cible

- Incident d'envoi Better Auth : revenir au precedent commit/deploiement de
  `filfinance-auth` et a son fichier d'environnement prive connu, sans modifier
  les MX ni Stalwart. Ne pas desactiver la verification d'adresse en production
  pour contourner l'incident ; les nouveaux enregistrements et resets peuvent
  etre temporairement indisponibles, les sessions verifiees existantes restent
  actives.
- Incident de reception publique : restaurer immediatement les trois MX OVH
  inventories plus haut, attendre le TTL de 300 secondes, puis verifier la
  reception sur OVH. Cette operation ne change pas l'envoi Better Auth par
  Brevo.
