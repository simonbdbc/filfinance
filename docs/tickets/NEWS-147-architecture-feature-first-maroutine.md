# NEWS-147 - Architecture feature-first inspiree maroutine

## Objectif

Reorganiser le frontend en modules feature-first proches de `maroutine`.

## Structure cible

Chaque feature importante doit tendre vers:

- `components/`
- `hooks/`
- `services/`
- `types/`
- `screens/`
- `index.ts`

## Scope

- Ajouter les aliases `@application`, `@features/*` et `@shared/*`.
- Ajouter des exports publics par feature.
- Decouper progressivement les responsabilites historiquement concentrees dans `FilFinanceApp`.
- Garder les composants et styles existants quand ils sont corrects.

## Criteres d'acceptation

- Les route files n'orchestrent pas les domaines metier.
- Les imports passent par les exports publics quand c'est possible.
- Aucun fichier frontend ne redevient un orchestrateur global monolithique.

## Avancement

- `expo-router` et les aliases `@application`, `@features`, `@shared` sont actifs.
- Les ecrans de route deleguent deja a `src/features/*/screens`.
- `src/application` porte les layouts applicatifs, la navigation, le provider runtime racine et les hooks applicatifs.
- Le domaine profil expose maintenant son runtime via `@features/profile`.
- Les appels profil utilisateur sont places dans `profile/services`.
- Le domaine historique expose maintenant `history/hooks/useHistoryRuntime` avec ses refs de replay et son historique journalier.
- Le canal metadata live expose maintenant `live-feed/hooks/useLiveMetadataRuntime`, incluant le choix edge/backend et le fallback.
- Le domaine test-run expose maintenant `test-runs/hooks/useTestRunRuntime`, incluant la trace, les panneaux et l'item livre.
- Le domaine audio expose maintenant `audio/hooks/useAudioRuntime`, incluant la queue live, l'item courant et les callbacks `audio_ready`.
- Les fichiers de features sont ranges sous `components/`, `hooks/`, `services/`, `providers/` ou `screens/`; la racine de chaque feature ne garde que `index.ts`.
- Les imports externes aux features passent par les exports publics `@features/<feature>`.
- Les imports vers le socle commun passent par `@shared/*` au lieu de chemins relatifs fragiles.
- `npm run qa:architecture` verrouille ces conventions pour eviter les regressions.
