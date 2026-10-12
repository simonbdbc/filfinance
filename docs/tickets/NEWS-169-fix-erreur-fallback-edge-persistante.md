# NEWS-169 - Fix erreur fallback edge persistante

## Objectif

Corriger l'affichage persistant de l'erreur frontend:

```text
Flux: Flux edge indisponible, fallback backend actif.
```

Le fallback backend est un chemin de recuperation valide. Il ne doit pas rester affiche
dans `Dernieres erreurs` une fois le WebSocket backend reconnecte.

## Priorite

Priorite 1.

Le bug n'interrompt pas le flux quand le fallback fonctionne, mais il degrade la lecture
operationnelle: l'interface signale une erreur alors que le transport est revenu dans un
etat exploitable.

## Audit

Le message est pose par `useNewsSocket` dans deux cas:

- echec de generation du ticket edge avant ouverture du WebSocket edge;
- fermeture ou erreur du WebSocket edge avant bascule vers le backend.

Le hook reconnecte ensuite le WebSocket backend via `fallbackWsUrl`, mais `onopen`
ne nettoyait que `Connexion WebSocket instable`. Le message `Flux edge indisponible,
fallback backend actif.` restait donc stocke dans `lastError` meme apres la recuperation.

Le symptome observe confirme ce diagnostic: un refresh complet de la page remonte le hook
React et remet `lastError` a `null`, ce qui masque immediatement l'alerte.

## Correctif

`useNewsSocket` distingue maintenant les erreurs transport recuperables:

- `Connexion WebSocket instable`;
- `Flux edge indisponible, fallback backend actif.`;
- les fermetures WebSocket formatees.

Quand une connexion WebSocket s'ouvre correctement, y compris la connexion backend de
fallback, ces messages sont nettoyes. Les erreurs metier et les notices serveur restent
conservees pour ne pas masquer un vrai probleme applicatif.

## Critere d'acceptation

- Si le ticket ou le WebSocket edge echoue, le client peut afficher temporairement la
  bascule fallback.
- Quand le fallback backend s'ouvre, `Dernieres erreurs` ne conserve plus le message edge.
- Une erreur metier recue via `system_notice`, un payload invalide ou un rejet auth reste
  affiche apres reconnexion.
- Un refresh manuel n'est plus necessaire pour faire disparaitre l'alerte de fallback.
- `npm run typecheck` passe cote client.

## Verification

```bash
cd filfinance-client
npm run typecheck
```

Verification manuelle recommandee:

1. configurer un `EXPO_PUBLIC_FILFINANCE_EDGE_WS_URL` invalide ou indisponible;
2. ouvrir la page Flux;
3. verifier que la bascule fallback peut apparaitre pendant la reconnexion;
4. verifier que l'encart `Dernieres erreurs` disparait quand le backend fallback est ouvert.

## Hors scope

- Corriger la disponibilite reelle du worker edge.
- Modifier les secrets ou variables de production.
- Supprimer le fallback backend.
