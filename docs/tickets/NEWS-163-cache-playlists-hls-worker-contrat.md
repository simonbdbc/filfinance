# NEWS-163 - Contrat cache playlists HLS Worker

## Objectif

Definir le contrat fonctionnel et de securite du cache tres court des playlists HLS dans
le Worker Edge.

## Scope

- Ajouter une variable Worker pour piloter le TTL playlist.
- Garder `hls_ticket` dans la cache key.
- Definir les ressources playlist cacheables.
- Definir les statuts de diagnostic.
- Definir les limites de securite et de fraicheur.

## Contrat cible

Variable Worker:

```env
FILFINANCE_HLS_PLAYLIST_CACHE_TTL_SECONDS=2
```

Regles:

- `0` ou valeur absente: cache playlist desactive;
- valeurs ciblees: `1` ou `2`;
- valeur maximale acceptee cote code: `5`, pour eviter un TTL accidentellement long;
- seules les requetes `GET` sont cacheables;
- la cache key conserve l'URL complete du Worker, query string comprise;
- une playlist sans `hls_ticket` ne peut pas reutiliser une entree valide;
- une playlist avec ticket invalide ne doit jamais etre stockee;
- seules les reponses origin `200` sont stockees.

Ressources concernees:

```text
/hls/audio/replay/today/playlist.m3u8?hls_ticket=...
/hls/audio/clips/{stream_id}/playlist.m3u8?hls_ticket=...
```

## Diagnostics

Le Worker doit enrichir `X-FilFinance-HLS-Proxy-Cache`:

- `PLAYLIST_HIT`: playlist servie depuis cache Worker;
- `PLAYLIST_MISS`: playlist chargee depuis origin puis stockee;
- `BYPASS_PLAYLIST_DISABLED`: TTL absent ou `0`;
- `BYPASS_ORIGIN_ERROR`: reponse origin non `200`, non stockee;
- `BYPASS_METHOD`: methode non cacheable;
- `BYPASS_RANGE`: requete range;
- `HIT` / `MISS`: conserve pour les segments `.ts`.

## Risques

- Playlist legerement stale: bornee par TTL `1s` a `2s`.
- Autorisation stale: bornee par le TTL et par la cache key qui conserve `hls_ticket`.
- Cache fragmentation: limitee par le ticket partage par bucket temporel, pas par utilisateur.
- Mauvaise config TTL: bornee par le max code `5s`.

## Criteres d'acceptation

- Le cache est inactif si la variable est absente ou `0`.
- Un TTL `2` produit des `PLAYLIST_HIT` apres warmup.
- Les `401`, `403`, `404`, `5xx` restent non stockes.
- Les tests Worker couvrent la lecture TTL, la cacheability playlist et les statuts.

## Dependances

- `EPIC-027`
- `NEWS-159`
- `NEWS-162`
