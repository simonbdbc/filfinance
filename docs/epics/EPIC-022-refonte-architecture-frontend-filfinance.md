# EPIC-022 - Refonte architecture frontend FilFinance

## Objectif

Refondre `filfinance-client` pour reprendre l'architecture de reference de `maroutine`, sans refonte visuelle.

Le style, les couleurs et les themes clair/sombre actuels de FilFinance restent la source de verite. Le but de cet EPIC est de rendre le code frontend plus maintenable, plus modulaire et plus fiable.

## Decisions

- `maroutine` sert de reference d'architecture, pas de reference visuelle.
- Le prototype est isole avant integration dans le chemin de production.
- `expo-router` devient la cible de navigation frontend.
- Les routes restent fines et deleguent aux ecrans situes dans `src/features`.
- Les features exposent des API publiques via `index.ts`.
- Le theme local est applique immediatement, puis la preference profil gagne quand elle est chargee.
- Le modele audio evenementiel `EPIC-021` reste l'invariant a preserver: pas de `/ws/audio`, pas de radio HLS permanente, audio global persistant.

## Architecture cible

```text
app/
  _layout.tsx
  (auth)/
  (app)/
src/
  application/
  features/
    audio/
      components/
      hooks/
      services/
      types/
      screens/
      index.ts
  shared/
    theme/
    ui/
    api/
```

## Tickets

- [NEWS-144 - Audit architecture et theme actuel](../tickets/NEWS-144-audit-architecture-theme-frontend.md)
- [NEWS-145 - Prototype expo-router sans changement visuel](../tickets/NEWS-145-prototype-expo-router-sans-changement-visuel.md)
- [NEWS-146 - Stabilisation theme clair sombre actuel](../tickets/NEWS-146-stabilisation-theme-clair-sombre-actuel.md)
- [NEWS-147 - Architecture feature-first inspiree maroutine](../tickets/NEWS-147-architecture-feature-first-maroutine.md)
- [NEWS-148 - Migration live audio global historique](../tickets/NEWS-148-migration-live-audio-global-historique.md)
- [NEWS-149 - Migration auth tests profil admin](../tickets/NEWS-149-migration-auth-tests-profil-admin.md)
- [NEWS-150 - QA architecture theme non-regression frontend](../tickets/NEWS-150-qa-architecture-theme-non-regression-frontend.md)

## Definition of Done

- Le client utilise `expo-router` avec routes directes web.
- Les domaines frontend critiques sont separes en `components`, `hooks`, `services`, `types`, `screens` et `index.ts`.
- Le theme clair/sombre actuel reste visuellement coherent et applique partout.
- L'audio live reste global et ne se coupe pas au changement de page.
- Les contrats audio legacy `/ws/audio` et `/audio/live` ne reviennent pas.
- `npm run typecheck` et `npm run build:web` passent.
