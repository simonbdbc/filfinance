# NEWS-175 - Fix session Supabase refusee transitoire cote flux live

## Constat

Le frontend peut afficher dans `Dernieres erreurs`:

```text
Flux: Session Supabase refusee par le backend. Deconnecte-toi puis reconnecte-toi.
```

Un rafraichissement complet de la page fait disparaitre le message. Cela indique que la
session utilisateur reste exploitable, mais que le flux live a tente de s'authentifier
avec un jeton local devenu stale ou refuse pendant une transition de session.

## Cause

Le backend ferme correctement les WebSockets non authentifies avec le code policy `1008`
et une raison `invalid auth token`.

Cote client, `useNewsSocket` traitait ce close comme une erreur terminale:

- aucun refresh Supabase n'etait tente;
- aucune reconnexion n'etait programmee avec un jeton frais;
- le message etait conserve dans `lastError` jusqu'au remount complet de l'app.

Le refresh manuel de page reparait le symptome parce que Supabase restaurait alors une
session a jour avant le remontage du WebSocket.

## Correctif

- `AuthProvider` expose maintenant `refreshSession()`, qui force un refresh Supabase et
  retourne le nouvel access token.
- Le runtime live transmet ce refresh au hook WebSocket du flux metadata.
- Quand le WebSocket live est ferme avec `1008` pour une raison `auth` ou `token`,
  `useNewsSocket` tente d'abord un refresh local puis reconnecte le flux avec le jeton
  rafraichi.
- Le message `Session Supabase refusee par le backend` reste affiche uniquement si le
  refresh echoue, ne retourne aucune session ou si le refus n'est pas un cas recuperable
  par refresh.

## Critere d'acceptation

- Une expiration ou transition de jeton Supabase ne demande plus un refresh manuel de la
  page pour recuperer le flux.
- Le flux live retente la connexion apres refresh de session.
- Un vrai refus persistant de session continue d'afficher le message demandant une
  reconnexion utilisateur.
- Les erreurs transport recuperables et les erreurs metier gardent leur comportement
  existant.

## Verification

```bash
cd filfinance-client
npm run typecheck
```

Verification manuelle recommandee:

1. ouvrir l'app avec une session Supabase valide;
2. laisser la session atteindre une rotation de token ou forcer un rejet temporaire du
   jeton WebSocket;
3. verifier que le message transitoire disparait apres reconnexion du flux;
4. verifier qu'un vrai token invalide conserve l'erreur.

