# Manifeste à remplir pour le nouveau projet

Copier ce fichier dans le dépôt cible. Remplacer chaque `{{...}}` par une donnée confirmée ou par une mention interne « non applicable » motivée. Les placeholders servent à la préparation ; ils ne doivent pas atteindre les pages publiques. Ne pas copier les coordonnées d'ACREA.

## Identité, périmètre et destinations

```yaml
project:
  name: "{{NOM_COMMERCIAL}}"
  description: "{{SERVICE_REELLEMENT_FOURNI}}"
  locale: "fr-FR"
  countries: ["{{PAYS_CIBLES}}"]
  audience: "{{PUBLIC_AGE_MINIMUM_MINEURS_OU_PROFESSIONNELS}}"
  business_model: "{{GRATUIT_ABONNEMENT_ACHAT_AUTRE}}"
  landing_framework: "{{FRAMEWORK}}"
  app_framework: "{{FRAMEWORK}}"
  app_platforms: ["{{WEB_PWA_IOS_ANDROID}}"]

publisher:
  legal_name: "{{IDENTITE_JURIDIQUE}}"
  legal_form: "{{STATUT_FORME_JURIDIQUE}}"
  registered_office: "{{ADRESSE_PROFESSIONNELLE}}"
  registration: "{{IMMATRICULATION_APPLICABLE}}"
  share_capital: "{{CAPITAL_SI_APPLICABLE}}"
  vat_number: "{{TVA_SI_APPLICABLE}}"
  publication_director: "{{RESPONSABLE_PUBLICATION}}"
  contact_email: "{{CONTACT_GENERAL}}"
  contact_phone: "{{CONTACT_TELEPHONIQUE_APPLICABLE}}"
  privacy_contact: "{{CONTACT_DONNEES_PERSONNELLES}}"
  dpo_contact: "{{DPO_SI_DESIGNE}}"
  regulated_activity_details: "{{INFORMATIONS_SECTORIELLES_APPLICABLES}}"
  information_source_and_owner: "{{SOURCE_RESPONSABLE_DATE}}"

hosting:
  landing:
    name: "{{RAISON_SOCIALE_HEBERGEUR}}"
    address: "{{ADRESSE}}"
    phone: "{{TELEPHONE}}"
    website: "{{URL}}"
  app_and_backend:
    providers_and_roles: "{{FOURNISSEURS_ROLES}}"
    data_locations_and_transfers: "{{LOCALISATIONS_TRANSFERTS_GARANTIES}}"

environments:
  local:
    landing_origin: "http://localhost:4321"
    app_origin: "{{ORIGINE_LOCALE_APP}}"
  staging:
    landing_origin: "{{ORIGINE_LANDING_RECETTE}}"
    app_origin: "{{ORIGINE_APP_RECETTE}}"
  production:
    landing_origin: "{{ORIGINE_LANDING_PRODUCTION}}"
    app_origin: "{{ORIGINE_APP_PRODUCTION}}"

legal:
  paths:
    legal_notice: "/mentions-legales/"
    privacy: "/politique-de-confidentialite/"
    terms: "/conditions-generales-utilisation/"
    cookies: "/cookies/"
  scope: "{{DOMAINES_ET_APPLICATIONS_COUVERTS}}"
  updated_at: "{{DATE_LISIBLE}}"
  version: "{{VERSION_EDITORIALE}}"
  terms_acceptance: "{{INFORMATION_SEULE_OU_ACCEPTATION_VERSIONNEE}}"
  permanent_app_entry: "{{PARAMETRES_A_PROPOS_OU_AUTRE}}"
  offline_access: "{{LIEN_EN_LIGNE_OU_COPIE_VERSIONNEE}}"
  paid_offer_documents: "{{NON_APPLICABLE_OU_CGV_ET_PARCOURS_A_PREVOIR}}"

tracking:
  mode: "{{AUCUN_NON_ESSENTIEL_OU_CONSENTEMENT_REQUIS}}"
  audit_date: "{{DATE}}"
  inventory_evidence: "{{REFERENCE}}"
  consent_manager_if_needed: "{{SOLUTION_ET_PERIMETRE}}"
  consent_withdrawal_entry: "{{EMPLACEMENT_SI_APPLICABLE}}"
  cross_origin_behavior: "{{CHOIX_SEPARES_OU_SYNCHRONISATION_DEFINIE}}"

publication:
  content_owner: "{{RESPONSABLE}}"
  content_review: "{{STATUT_DATE_REFERENCE}}"
  ci_provider: "{{GITLAB_GITHUB_AUTRE}}"
  html_validation_command: "{{COMMANDE}}"
  deploy_depends_on_validation: true
```

## Inventaire des traitements

Dupliquer les lignes selon les fonctions présentes. Les catégories ci-dessous sont des pistes d'inventaire et ne signifient pas que le projet les utilise.

| Traitement réel | Données / caractère obligatoire | Finalité | Base légale à confirmer | Destinataires / prestataires | Lieux / transferts / garanties | Durée ou critères | Suppression et preuve |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Compte / authentification | | | | | | | |
| Données métier / historique | | | | | | | |
| Support / messagerie | | | | | | | |
| Sécurité / journaux | | | | | | | |
| Paiement, si présent | | | | | | | |
| IA / profilage, si présent | | | | | | | |
| Statistiques / marketing, si présents | | | | | | | |

Pour chaque prestataire, préciser le rôle, les données reçues et les garanties réellement applicables. Pour chaque durée, distinguer base active, archives, sauvegardes, logs et copies locales ; indiquer qui exécute la suppression ou l'anonymisation. Référer au [guide principal](./README.md) pour les points d'information à couvrir.

## Inventaire des stockages et traceurs

| Origine / plateforme | Nom ou mécanisme | Type | Déclenchement | Finalité | Durée | Destinataire | Exemption justifiée ou consentement | Preuve |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Landing | | Cookie / stockage / script / autre | | | | | | |
| App | | Session / IndexedDB / service worker / SDK / autre | | | | | | |
| Infrastructure | | CDN / anti-abus / injection / autre | | | | | | |

Ne pas enregistrer de tokens ni de valeurs de session dans ce manifeste.

## Textes à préparer

- **Mentions légales** : identité complète selon le statut, publication, hébergement, contact et informations sectorielles.
- **Confidentialité** : périmètre landing/app, responsable, traitements de l'inventaire, conservation, droits et modalités de contact ; compléter les cas particuliers applicables.
- **CGU** : service réellement fourni, compte, règles d'usage, propriété, suspension, disponibilité, responsabilité, évolution, contact et droit applicable.
- **Cookies** : inventaire compréhensible, mécanismes nécessaires, mécanismes soumis à consentement et gestion des choix si présents.
- **Notice près du formulaire** : identité du responsable, usages principaux des données, accès aux droits et liens vers le détail ; séparer une éventuelle acceptation des CGU.

## Décisions et données encore manquantes

| Point à résoudre | Responsable | Source attendue | Bloque la publication ? | Décision / date |
| --- | --- | --- | --- | --- |
| | | | | |

Le manifeste est une spécification documentaire. Aucun script ACREA ne lit ce YAML automatiquement. Le développeur doit reporter les informations approuvées dans la configuration et les contenus du projet cible.
