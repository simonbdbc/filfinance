# Mise en œuvre technique réutilisable

Les exemples de ce fichier sont des **modèles proposés**, à intégrer dans le projet cible. Ils ne sont pas des modifications du code ACREA. Le [guide principal](./README.md) décrit séparément le comportement ACREA vérifié.

## 1. Deux façons de reprendre le dispositif

Pour un autre projet Astro + Expo, reprendre les responsabilités de la carte des fichiers du guide. Copier les quatre pages, le layout et le helper ne suffit pas : ils dépendent aussi de la configuration, du thème, du footer, du lecteur de variables et des alias du projet.

Pour un autre framework, conserver le contrat d'URL et traduire les composants. Une route publique Next, Nuxt ou autre doit produire le document sans authentification ; une app React Web peut utiliser de simples liens HTML. Aucune bibliothèque partagée entre les deux frontends n'est nécessaire. Les quatre chemins peuvent être répétés dans les configurations locales et contrôlés par la recette.

Structure cible indicative :

```text
landing/
  src/config/legal.config.ts
  src/layouts/LegalLayout.astro
  src/components/LegalNavigation.astro
  src/pages/mentions-legales.astro
  src/pages/politique-de-confidentialite.astro
  src/pages/conditions-generales-utilisation.astro
  src/pages/cookies.astro
  src/styles/legal.css
  scripts/validate-legal-build.mjs
app/
  configuration publique
  helper des URL légales
  intégration inscription + paramètres
```

## 2. Configuration publique

Conserver les noms des cinq variables ACREA facilite la reprise. Les variables de nom, date et version ci-dessous sont des ajouts génériques pour éviter des constantes de marque dispersées.

| Variable | Consommateur | Rôle |
| --- | --- | --- |
| `PUBLIC_SITE_ORIGIN` | Landing, build et validation | Origine publique de la landing |
| `PUBLIC_APP_ORIGIN` | Landing | Destination des liens vers l'app |
| `PUBLIC_SITE_NAME` | Landing | Nom affiché ; ajout du modèle |
| `PUBLIC_LEGAL_ENTITY_NAME` | Landing | Éditeur juridique |
| `PUBLIC_LEGAL_REGISTERED_OFFICE` | Landing | Adresse professionnelle |
| `PUBLIC_LEGAL_PUBLICATION_DIRECTOR` | Landing | Responsable de publication |
| `PUBLIC_LEGAL_PRIVACY_CONTACT` | Landing | Contact données personnelles |
| `PUBLIC_LEGAL_DATA_RETENTION` | Landing | Texte de conservation validé |
| `PUBLIC_LEGAL_UPDATED_AT` | Landing | Date éditoriale ; ajout du modèle |
| `PUBLIC_LEGAL_VERSION` | Landing | Version éditoriale ; ajout du modèle |
| `PUBLIC_LEGAL_PUBLICATION_ENFORCED` | Validateur ACREA uniquement | Active le refus des données manquantes ; le modèle ci-dessous est toujours strict |
| `EXPO_PUBLIC_LANDING_ORIGIN` | App Expo | Origine des documents publics |
| `EXPO_PUBLIC_PWA_ANALYTICS_ENDPOINT` | App ACREA | Doit rester vide dans son pipeline tant que le dispositif prévu n'est pas livré |

Les informations `PUBLIC_*` et `EXPO_PUBLIC_*` sont publiques. Ne jamais y mettre de secret, clé SMTP, token d'administration ou clé privée.

Exemple de fichier de préparation à compléter ; les `{{...}}` doivent être refusés en production :

```dotenv
PUBLIC_SITE_ORIGIN=https://www.example.com
PUBLIC_APP_ORIGIN=https://app.example.com
PUBLIC_SITE_NAME="{{NOM_DU_PROJET}}"
PUBLIC_LEGAL_ENTITY_NAME="{{EDITEUR_JURIDIQUE}}"
PUBLIC_LEGAL_REGISTERED_OFFICE="{{ADRESSE_PROFESSIONNELLE}}"
PUBLIC_LEGAL_PUBLICATION_DIRECTOR="{{RESPONSABLE_PUBLICATION}}"
PUBLIC_LEGAL_PRIVACY_CONTACT="{{CONTACT_CONFIDENTIALITE}}"
PUBLIC_LEGAL_DATA_RETENTION="{{REGLES_DE_CONSERVATION_CONFIRMEES}}"
PUBLIC_LEGAL_UPDATED_AT="{{DATE_DE_MISE_A_JOUR}}"
PUBLIC_LEGAL_VERSION="{{VERSION}}"
```

Dans l'environnement de l'app, définir séparément `EXPO_PUBLIC_LANDING_ORIGIN` avec la même origine de landing. En développement local ACREA, son exemple utilise `http://localhost:4321`.

Les données légales sont intégrées au HTML pendant le build statique. Modifier une variable chez l'hébergeur sans reconstruire la landing ne met pas à jour les pages déjà générées. Les variables Expo de build nécessitent aussi une nouvelle compilation, sauf mécanisme explicite de surcharge runtime comme celui d'ACREA.

Le validateur Node lit `process.env`, alors qu'Astro peut lire des fichiers `.env` : garantir que les deux outils reçoivent les mêmes valeurs. En CI, les exporter dans l'environnement du job. Ne pas supposer que `pnpm validate` charge automatiquement `.env`.

### Exemple `src/config/legal.config.ts`

```ts
const text = (value: string | undefined) => value?.trim() || "__A_COMPLETER__";

export const legalPaths = {
  legalNotice: "/mentions-legales/",
  privacy: "/politique-de-confidentialite/",
  terms: "/conditions-generales-utilisation/",
  cookies: "/cookies/",
} as const;

export const legalNavigation = [
  { href: legalPaths.legalNotice, label: "Mentions légales" },
  { href: legalPaths.privacy, label: "Confidentialité" },
  { href: legalPaths.terms, label: "CGU" },
  { href: legalPaths.cookies, label: "Cookies" },
];

export const legalConfig = {
  siteName: text(import.meta.env.PUBLIC_SITE_NAME),
  siteOrigin: import.meta.env.PUBLIC_SITE_ORIGIN || "http://localhost:4321",
  updatedAt: text(import.meta.env.PUBLIC_LEGAL_UPDATED_AT),
  version: text(import.meta.env.PUBLIC_LEGAL_VERSION),
  publisher: {
    name: text(import.meta.env.PUBLIC_LEGAL_ENTITY_NAME),
    registeredOffice: text(import.meta.env.PUBLIC_LEGAL_REGISTERED_OFFICE),
    publicationDirector: text(import.meta.env.PUBLIC_LEGAL_PUBLICATION_DIRECTOR),
    privacyContact: text(import.meta.env.PUBLIC_LEGAL_PRIVACY_CONTACT),
    dataRetention: text(import.meta.env.PUBLIC_LEGAL_DATA_RETENTION),
  },
};
```

Ce schéma reprend les cinq champs ACREA, sans couvrir toutes les informations d'une entreprise. Ajouter les champs du manifeste applicables à l'éditeur et l'hébergeur, puis les rendre dans les pages. Le validateur doit évoluer avec eux.

Les lectures `import.meta.env.PUBLIC_...` sont explicites dans cet exemple. Pour une reprise du helper ACREA utilisant `import.meta.env[name]`, vérifier le HTML généré avec la version du bundler du projet cible ; ne pas considérer un objet de configuration non vide comme une preuve de substitution correcte.

## 3. Layout et navigation publics

### `src/components/LegalNavigation.astro`

```astro
---
import { legalNavigation } from "../config/legal.config";
---
<nav aria-label="Informations légales">
  {legalNavigation.map(({ href, label }) => <a href={href}>{label}</a>)}
</nav>
```

Intégrer ce composant dans le footer de l'accueil **et** dans celui de chaque document. Les liens marketing présents sur les pages légales doivent utiliser `/#section` au lieu de `#section`. Ne pas ajouter au nouveau projet les ancres ACREA qui n'y existent pas.

### `src/layouts/LegalLayout.astro`

Ce layout minimal fonctionne sans le layout ACREA. Dans un projet existant, intégrer ses responsabilités au layout commun pour conserver le thème, les métadonnées et la navigation du produit.

```astro
---
import { legalConfig } from "../config/legal.config";
import LegalNavigation from "../components/LegalNavigation.astro";
import "../styles/legal.css";

interface Props { title: string; description: string }
const { title, description } = Astro.props;
const canonical = new URL(Astro.url.pathname, legalConfig.siteOrigin).toString();
---
<!doctype html>
<html lang="fr">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>{title} | {legalConfig.siteName}</title>
    <meta name="description" content={description} />
    <meta name="robots" content="index, follow" />
    <link rel="canonical" href={canonical} />
  </head>
  <body>
    <a class="skip-link" href="#contenu">Aller au contenu</a>
    <header><a href="/">{legalConfig.siteName} — Retour à l'accueil</a></header>
    <main id="contenu">
      <article class="legal-content">
        <h1>{title}</h1>
        <slot />
        <p>Dernière mise à jour : {legalConfig.updatedAt}</p>
        <p>Version : {legalConfig.version}</p>
      </article>
    </main>
    <footer><LegalNavigation /></footer>
  </body>
</html>
```

Ici, le `h1` appartient au layout : **ne pas en ajouter dans les pages**. Dans le code ACREA, il appartient à chaque page. Choisir une convention pour éviter les doubles titres principaux.

### `src/styles/legal.css`

Exemple autonome, à adapter aux couleurs et typographies du produit :

```css
* { box-sizing: border-box; }
body { margin: 0; color: #17212b; background: #fff; font: 1rem/1.65 system-ui, sans-serif; }
header, main, footer { padding: 1.5rem max(1rem, calc((100% - 52rem) / 2)); }
a { color: #0758a5; text-decoration: underline; text-underline-offset: .2em; }
a:focus-visible { outline: 3px solid #0758a5; outline-offset: 4px; }
nav { display: flex; flex-wrap: wrap; gap: .5rem 1.25rem; }
nav a, header a { display: inline-flex; align-items: center; min-height: 44px; }
h1 { font-size: clamp(2rem, 6vw, 3.5rem); line-height: 1.12; }
h2 { font-size: 1.5rem; line-height: 1.3; }
.legal-content { overflow-wrap: anywhere; }
.legal-content section { margin-block: 2rem; }
.legal-content dl > div { margin-block: 1rem; }
.legal-content dt { font-weight: 700; }
.legal-content dd { margin: 0; }
.skip-link { position: absolute; left: 1rem; top: -10rem; background: #fff; padding: .75rem; }
.skip-link:focus { top: 1rem; }
@media print { header, footer, .skip-link { display: none; } }
```

Pour reproduire le style ACREA, reprendre plutôt son bloc `.legal-*`, ses media queries et ses dépendances `.page-shell`, `.site-header`, `.header-inner`, `.brand`, `.skip-link`, footer, polices et variables de couleur. `.acronym-graphic`, visible dans la feuille de style de l'accueil, n'appartient pas au dispositif légal.

## 4. Pages : structure et contenus

Créer les quatre fichiers dont les noms correspondent à `legalPaths`. Exemple de départ pour `mentions-legales.astro` :

```astro
---
import LegalLayout from "../layouts/LegalLayout.astro";
import { legalConfig } from "../config/legal.config";
---
<LegalLayout title="Mentions légales" description="Identité de l'éditeur et informations de publication.">
  <section>
    <h2>Éditeur</h2>
    <dl>
      <div><dt>Identité</dt><dd>{legalConfig.publisher.name}</dd></div>
      <div><dt>Adresse professionnelle</dt><dd>{legalConfig.publisher.registeredOffice}</dd></div>
      <div><dt>Responsable de publication</dt><dd>{legalConfig.publisher.publicationDirector}</dd></div>
    </dl>
  </section>
  <section><h2>Informations propres au statut de l'éditeur</h2><p>__A_COMPLETER__</p></section>
  <section><h2>Hébergement</h2><p>__A_COMPLETER__</p></section>
  <section><h2>Contact</h2><p>{legalConfig.publisher.privacyContact}</p></section>
  <section><h2>Propriété intellectuelle</h2><p>__A_COMPLETER__</p></section>
</LegalLayout>
```

Ce squelette est volontairement non publiable tant que les sections ne sont pas renseignées. Pour les autres documents, utiliser le même layout et les textes approuvés du manifeste :

| Fichier | `title` attendu | Organisation |
| --- | --- | --- |
| `politique-de-confidentialite.astro` | `Politique de confidentialité` | Périmètre, responsable, traitements, destinataires/transferts, conservation, droits/contact, particularités applicables |
| `conditions-generales-utilisation.astro` | `Conditions générales d'utilisation` | Objet, utilisateurs, compte, usage, propriété, suspension, responsabilité, évolution, contact/droit |
| `cookies.astro` | `Cookies et traceurs` | Inventaire par surface, finalités/durées, mécanismes exemptés, choix/retrait si applicable |

Utiliser `legalConfig.publisher.dataRetention` dans la politique, en le complétant par les distinctions du manifeste si un paragraphe unique est insuffisant. Rendre les variables dans des nœuds de texte, sans injection HTML brute de contenu non maîtrisé.

## 5. Routage, SEO et hébergement

Pour Astro, reprendre `output: "static"` et `trailingSlash: "always"` dans la configuration existante. Attendre les sorties `dist/mentions-legales/index.html`, etc. La page légale doit rester publique même si toute l'app est protégée.

Ajouter au sitemap l'accueil et `Object.values(legalPaths)`, en produisant chaque URL avec l'origine canonique. Faire référencer ce sitemap par `robots.txt`. Une politique `noindex` peut être choisie pour la recette ; adapter les contrôles et ne pas la propager accidentellement à la production. L'indexation est un choix SEO distinct de l'accessibilité des documents.

L'en-tête CSP et les autres en-têtes du projet doivent aussi couvrir les pages légales. Sur une plateforme ne lisant pas `_headers`, traduire les règles dans sa configuration. Ne pas copier une CSP qui casse des fonctionnalités légitimes du site. Vérifier les réponses HTTP après déploiement.

## 6. Intégration dans l'app

### Helper Expo simplifié pour le nouveau projet

Cet exemple utilise une variable de build explicite, sans valeur de secours ACREA. Le garde HTTP permet seulement le développement local. `__DEV__` est fourni par Expo/React Native ; pour un autre framework, utiliser son indicateur de développement.

```ts
export function getLegalLinks() {
  const value = process.env.EXPO_PUBLIC_LANDING_ORIGIN?.trim();
  if (!value) throw new Error("EXPO_PUBLIC_LANDING_ORIGIN manquante");
  const base = new URL(value);
  const local = __DEV__ && ["localhost", "127.0.0.1", "[::1]"].includes(base.hostname);
  if (base.protocol !== "https:" && !(local && base.protocol === "http:")) {
    throw new Error("L'origine légale doit utiliser HTTPS");
  }
  if (base.pathname !== "/" || base.search || base.hash || base.username || base.password) {
    throw new Error("Une origine sans chemin, paramètres ou identifiants est attendue");
  }
  return {
    legalNotice: new URL("/mentions-legales/", base).href,
    privacy: new URL("/politique-de-confidentialite/", base).href,
    terms: new URL("/conditions-generales-utilisation/", base).href,
    cookies: new URL("/cookies/", base).href,
  };
}
```

Valider aussi cette variable avant le build pour ne pas découvrir son absence uniquement dans l'écran d'inscription. Si l'origine doit être configurable à l'exécution, intégrer le helper au lecteur déjà existant et contrôler la configuration effective lors de la recette.

### Ouverture native et PWA

ACREA utilise `Text`, `accessibilityRole="link"` et `Linking.openURL(url)`. Pour la nouvelle app, gérer aussi l'échec d'ouverture. Exemple à intégrer dans les conventions UI du projet :

```tsx
import { useState } from "react";
import { Linking, Pressable, Text, View } from "react-native";

export function LegalLink({ url, label }: { url: string; label: string }) {
  const [error, setError] = useState("");
  async function open() {
    setError("");
    try { await Linking.openURL(url); }
    catch { setError("Impossible d'ouvrir ce document. Réessayez avec une connexion."); }
  }
  return (
    <View>
      <Pressable accessibilityRole="link" onPress={() => { void open(); }} style={{ minHeight: 44, justifyContent: "center" }}>
        <Text style={{ textDecorationLine: "underline" }}>{label}</Text>
      </Pressable>
      {error ? <Text accessibilityLiveRegion="polite">{error}</Text> : null}
    </View>
  );
}
```

Pour une app uniquement Web, préférer les vrais liens HTML. Exemple :

```tsx
<a href={legalLinks.privacy} target="_blank" rel="noopener noreferrer">
  Confidentialité (nouvel onglet)
</a>
```

Placer la notice courte et les liens près de la création de compte, avant l'envoi. Pour éviter un résultat seulement décoratif, tester focus clavier, activation et retour sans perte des données saisies. Ajouter une rubrique « Informations légales » dans les paramètres ou « À propos », avec les quatre liens.

Ne pas ajouter un simple booléen `accepted: true` à la requête d'inscription en prétendant obtenir une preuve contractuelle. Si le manifeste requiert cette preuve, appliquer le mécanisme versionné décrit dans le guide, avec stockage et contrôle serveur.

## 7. Validation du HTML généré

Dans ACREA, utiliser le validateur existant :

```sh
pnpm --dir apps/landing build
PUBLIC_LEGAL_PUBLICATION_ENFORCED=true pnpm --dir apps/landing validate
```

Ces commandes supposent les dépendances installées et les variables approuvées déjà fournies. `build` seul n'exécute pas `validate`. Ne pas copier le validateur ACREA intégralement dans un autre produit : il attend sa marque, ses CTA, ses assets et ses anciennes URL.

### Modèle autonome `scripts/validate-legal-build.mjs`

Ce script est à placer dans le frontend landing cible et à exécuter depuis sa racine, après build. Il est toujours strict. Il contrôle la structure et les omissions évidentes ; il ne juge pas la validité juridique du texte ni la véracité des informations.

```js
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";

const pending = /__A_COMPLETER__|\{\{[^}]+\}\}|À renseigner avant publication en production/iu;
const required = [
  "PUBLIC_SITE_ORIGIN", "PUBLIC_SITE_NAME",
  "PUBLIC_LEGAL_ENTITY_NAME", "PUBLIC_LEGAL_REGISTERED_OFFICE",
  "PUBLIC_LEGAL_PUBLICATION_DIRECTOR", "PUBLIC_LEGAL_PRIVACY_CONTACT",
  "PUBLIC_LEGAL_DATA_RETENTION", "PUBLIC_LEGAL_UPDATED_AT", "PUBLIC_LEGAL_VERSION",
];
for (const key of required) {
  const value = process.env[key]?.trim();
  assert(value && !pending.test(value), `${key} manquante ou provisoire`);
}
const base = new URL(process.env.PUBLIC_SITE_ORIGIN);
assert.equal(base.protocol, "https:", "Origine publique HTTPS attendue");
assert.equal(base.pathname, "/", "Origine sans préfixe de chemin attendue");
assert(!base.search && !base.hash && !base.username && !base.password);
assert(!/(^|\.)example\.(com|org|net)$/i.test(base.hostname), "Remplacer le domaine d'exemple");
const routes = [
  ["mentions-legales", "Mentions légales"],
  ["politique-de-confidentialite", "Politique de confidentialité"],
  ["conditions-generales-utilisation", "Conditions générales d'utilisation"],
  ["cookies", "Cookies et traceurs"],
];
// Décodage des entités usuelles produites par le rendu statique.
const decode = (s) => s.replace(/&#(x[0-9a-f]+|[0-9]+);|&(amp|apos|quot|lt|gt);/gi, (_, n, name) =>
  n ? String.fromCodePoint(n[0].toLowerCase() === "x" ? parseInt(n.slice(1), 16) : Number(n))
    : ({ amp: "&", apos: "'", quot: '"', lt: "<", gt: ">" }[name.toLowerCase()]));
const read = (p) => readFile(path.join("dist", p), "utf8");
const [home, sitemap, robots] = await Promise.all([
  read("index.html"), read("sitemap.xml"), read("robots.txt"),
]);
assert(robots.includes(`Sitemap: ${new URL("/sitemap.xml", base).href}`), "Sitemap absent de robots");
for (const [slug, title] of routes) {
  const html = await read(`${slug}/index.html`);
  assert(!pending.test(decode(html)), `${slug}: contenu provisoire`);
  assert.equal((html.match(/<h1\b/gi) || []).length, 1, `${slug}: H1 unique`);
  const pageTitle = decode(html.match(/<title[^>]*>([\s\S]*?)<\/title>/i)?.[1] || "").trim();
  assert.equal(pageTitle, `${title} | ${process.env.PUBLIC_SITE_NAME.trim()}`, `${slug}: titre`);
  const canonical = [...html.matchAll(/<link\b[^>]*>/gi)].map(m => m[0])
    .find(tag => /\brel="canonical"/i.test(tag));
  const href = canonical?.match(/\bhref="([^"]+)"/i)?.[1];
  const url = new URL(`/${slug}/`, base).href;
  assert.equal(decode(href || ""), url, `${slug}: canonical`);
  assert(sitemap.includes(`<loc>${url}</loc>`), `${slug}: sitemap`);
  for (const [other] of routes) {
    const link = `href="/${other}/"`;
    assert(home.includes(link), `Accueil: lien ${other}`);
    assert(html.includes(link), `${slug}: lien ${other}`);
  }
  assert(html.includes('href="/"'), `${slug}: retour accueil`);
}
console.log("Pages légales : validation du HTML réussie.");
```

Ajouter au `package.json` de la landing :

```json
{ "scripts": { "validate:legal": "node scripts/validate-legal-build.mjs" } }
```

Fusionner ce fragment avec les scripts existants. Le validateur suppose des attributs HTML entre guillemets doubles et les titres du tableau ; adapter si le moteur cible sérialise autrement. Le compléter avec les champs supplémentaires du manifeste, le contrôle des métadonnées, des domaines résiduels, des ancres et des scripts suivant le projet. Il ne lance pas le build.

### CI générique

Étapes à insérer dans le job de la landing, **après installation des dépendances** et avec les variables publiques approuvées dans l'environnement du job :

```yaml
script:
  - pnpm build
  - pnpm validate:legal
artifacts:
  paths:
    - dist/
```

Adapter le répertoire de travail ou employer `pnpm --dir <landing> ...` et le chemin d'artefact correspondant. Le job de publication doit dépendre de ce job réussi et publier précisément cet artefact. Ne pas reconstruire avec un autre jeu de variables après validation. Reproduire les conditions sur les branches et environnements qui peuvent réellement publier.

Dans l'app, vérifier l'origine légale avant build et contrôler les liens dans l'artefact ou par test UI. Si le projet reprend le mécanisme d'analytics ACREA et sa position sans activation, conserver le contrôle `test -z "$EXPO_PUBLIC_PWA_ANALYTICS_ENDPOINT"`. Avec un autre outil de statistiques, contrôler son mécanisme réel : recopier un nom de variable inutilisé ne protège rien.

Ne pas retirer le contrôle d'analytics uniquement pour faire passer le pipeline. Si des traceurs doivent être activés, livrer d'abord les choix, le blocage et les vérifications correspondant au manifeste.

## 8. Dépannage

| Symptôme | Cause à vérifier | Action |
| --- | --- | --- |
| La page légale affiche l'accueil | Fallback SPA ou mauvais artefact | Vérifier le fichier statique et les règles de routage |
| L'app ouvre le domaine du produit précédent | Valeur de secours ou ancienne compilation | Supprimer le fallback spécifique, définir l'origine et reconstruire |
| L'app ouvre son propre `/cookies/` | Lien relatif | Construire une URL absolue vers la landing |
| `#section` ne fonctionne pas depuis une page légale | Ancre locale au document | Utiliser `/#section` et vérifier que la section existe |
| La CI passe, mais les informations restent anciennes | Artefact reconstruit, cache CDN ou variable non appliquée | Comparer commit, variables de build et HTML servi |
| Le validateur échoue alors que `.env` est rempli | Node n'a pas chargé ce fichier | Fournir les variables au processus de validation comme au build |
| Le titre est présent mais le contrôle échoue | Encodage d'apostrophe ou sérialisation HTML différente | Décoder les entités ou utiliser le parseur HTML du projet |
| Des événements partent malgré un endpoint CI vide | Surcharge runtime, SDK ou injection d'hébergement | Examiner la configuration effective et le réseau sur les deux origines |
| L'ouverture fonctionne sur Web mais pas en natif | Comportement du navigateur ou gestion d'erreur absente | Tester la plateforme cible et rendre l'échec explicite |

Achever avec la [recette et la fiche de preuve](./README.md#6-recette-reproductible). Un build valide et une recette distante réussie sont deux preuves différentes.

## 9. Vérifications effectuées sur ce dossier

Le 12 septembre 2026, les liens Markdown locaux et la fermeture des blocs de code
ont été contrôlés. Le script `validate-legal-build.mjs` présenté ci-dessus a été
extrait du document et exécuté avec Node sur des fichiers HTML temporaires :

- cas valide accepté, y compris une apostrophe encodée dans le titre ;
- variable obligatoire absente refusée ;
- variable contenant un placeholder refusée ;
- placeholder dans le HTML refusé ;
- canonical incorrect refusé ;
- fichier d'une page absent refusé.

Ces vérifications portent sur le modèle de validation documentaire. Les exemples
Astro et Expo n'ont pas été compilés ni testés visuellement dans un nouveau projet.
Aucun build de l'app ACREA, audit navigateur de production ou déploiement n'a été
effectué dans le cadre de la rédaction de ce dossier.
