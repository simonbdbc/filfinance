# Manifeste Fil Finance

Version du 12 septembre 2026. Périmètre approuvé : quatre documents publics sur la landing, liens dans l’app et contrôle avant publication. L’[exemple ACREA original](./MANIFESTE-ACREA.md) est conservé séparément, à l’identique.

## Identité et offre

| Information | Valeur retenue |
| --- | --- |
| Éditeur | SB ALGO |
| Adresse | 60 rue François 1er, 75008 Paris |
| Responsable de publication | sbdev |
| Contact général et données personnelles | support@filfinance.fr |
| Conservation du compte et des préférences | Tant que le compte existe ; demandes de suppression au support, sous réserve des obligations légales et de la défense de droits. Aucun délai automatique de 30 jours. |
| Public | Majeurs francophones |
| Offre | Gratuite actuellement ; CGV à traiter lors de l’introduction d’une offre payante |

Ces valeurs proviennent des décisions de l’éditeur pour cette livraison. Les cinq variables `PUBLIC_LEGAL_*` sont centralisées dans [legal.config.ts](../../../apps/landing/src/config/legal.config.ts) et fournies en CI. Aucun champ obligatoire supplémentaire ni mécanisme d’acceptation n’est introduit.

## Service et documents

Fil Finance propose un flux audio de news en français, un fil d’actualités financières, un résumé audio du jour en français et un calendrier économique hebdomadaire. Les CGU décrivent un service d’information : aucun conseil personnalisé, exécution d’ordres ou résultat garanti.

Les textes de référence sont les quatre fichiers Astro dans [apps/landing/src/pages](../../../apps/landing/src/pages). Ils sont publiés aux chemins `/mentions-legales/`, `/politique-de-confidentialite/`, `/conditions-generales-utilisation/` et `/cookies/`. La date éditoriale commune est le **12 septembre 2026**.

L’app ouvre ces documents sans transférer de session ou de données de formulaire. La notice de confidentialité précède le bouton d’inscription ; les CGU restent un lien distinct. Les quatre documents sont accessibles dans le cadre commun d’authentification et dans le profil connecté. Aucun suivi serveur d’acceptation ni copie hors ligne n’est ajouté.

## Origines configurées

| Environnement | Landing / `EXPO_PUBLIC_LANDING_ORIGIN` | PWA |
| --- | --- | --- |
| Local habituel | `http://localhost:4321` | `http://localhost:8052` en Docker ; port Expo selon lancement |
| DEV | `https://dev.filfinance.fr` | `https://app.dev.filfinance.fr` |
| PROD | `https://filfinance.fr` | `https://app.filfinance.fr` |

La recette isolée utilise `127.0.0.1:4321` et `127.0.0.1:8053`, avec des réponses de compte/API simulées. Les destinations DEV et PROD sont des configurations, pas une preuve de publication.

## Hébergement et données effectivement concernées

- **Diffusion publique :** Bunny Storage et Pull Zones selon l’[architecture Fil Finance](../../architecture.md). Coordonnées officielles de [BunnyWay](https://bunny.net/privacy/) consultées le 12 septembre 2026 : BunnyWay d.o.o., Dunajska cesta 165, 1000 Ljubljana, Slovénie ; support@bunny.net. Cloudflare Pages d’ACREA n’est pas repris comme hébergeur.
- **Compte et préférences :** Better Auth, service Rust et PostgreSQL sur l’infrastructure de l’éditeur ; email, nom facultatif, authentification, sessions, rôle, voix et thème. Le mot de passe est géré par l’authentification, jamais ajouté aux liens légaux.
- **Emails et support :** vérification d’email, réinitialisation du mot de passe, demandes au support ; infrastructure de messagerie et relais Brevo. Aucun marketing ajouté.
- **Stockages du terminal :** cookies Better Auth sur Web, stockage sécurisé natif ; `filfinance.theme` pour l’app, `filfinance.landing.theme` pour la landing et filtres locaux du calendrier. Les préférences des deux origines restent indépendantes.
- **Journaux :** fonctionnement, diagnostic et sécurité des services. Les pages ne promettent ni localisation intégralement française ni délai de purge non établi.

Les sources techniques sont [l’authentification](../../../apps/pwa/src/features/auth), [la configuration publique](../../../apps/pwa/src/shared/config/runtimeEnv.ts), [le profil](../../../apps/pwa/src/features/profile), [le thème](../../../apps/pwa/src/shared/theme), [le calendrier](../../../apps/pwa/src/features/calendar) et la documentation d’architecture. Aucun analytics, bandeau de consentement, API, table ou contrat d’authentification n’est ajouté dans ce lot.

## Validation et maintenance

La [recette Fil Finance](./RECETTE-FILFINANCE.md) décrit la configuration, les commandes, les preuves et leurs limites. Le contrôle strict refuse une des cinq informations absente ou provisoire et compare la configuration au HTML réellement généré. Les publications landing et PWA attendent ce contrôle ; leurs déclencheurs manuels restent inchangés.

Mettre à jour les fichiers Astro et la date commune lors d’un changement de texte ; modifier les cinq valeurs publiques dans la configuration, l’exemple et la CI lorsqu’elles changent. Reconstruire puis recetter avant publication. L’éditeur reste responsable de la revue des textes. Cette livraison technique locale n’est ni un déploiement ni une certification juridique.
