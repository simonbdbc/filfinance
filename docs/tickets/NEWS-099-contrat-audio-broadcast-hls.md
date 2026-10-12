# NEWS-099 - Contrat audio broadcast HLS

## Objectif

Figer le contrat audio live et replay avant implementation Rust.

Le but est de remplacer le transport audio binaire `/ws/audio` par un flux broadcast HLS global, cacheable par Cloudflare CDN, sans creer de clips replay par news.

## Decision produit

Le live audio devient une radio Fil Finance globale:

- une seule narration TTS est produite cote backend;
- tous les utilisateurs ecoutent le meme flux live;
- la preference de voix utilisateur ne personnalise plus le live global;
- les anciennes news hors chronologie restent visibles dans l'historique, mais ne doivent pas etre inserees dans le live audio apres une news plus recente deja emise;
- le replay est un seul flux HLS de la journee courante, de minuit a maintenant;
- le replay est reset a minuit;
- aucun historique de replay n'est conserve;
- aucun clip replay par news n'est cree.

## Format audio cible

Le provider TTS actuel produit du WAV en interne. Ce WAV ne doit pas devenir le format public HLS.

Format public cible:

```text
codec: AAC-LC
container segment: MPEG-TS
extension segment: .ts
playlist: HLS classique .m3u8
```

Parametres cibles initiaux:

```text
canaux: mono
bitrate: 48 a 64 kbps
sample rate: 44100 Hz si l'encodeur resample proprement, sinon 22050 Hz conserve au depart
segment target duration: 4 secondes
segment duration maximum: 6 secondes
latence live acceptee au depart: 8 a 20 secondes
```

LL-HLS n'est pas retenu pour le premier palier. Il reste une optimisation future si la latence HLS classique devient un probleme produit mesure.

## Endpoints HLS

### Live

Playlist live:

```http
GET /audio/live/playlist.m3u8
```

Segments live:

```http
GET /audio/live/segments/{sequence}.ts
```

Regles live:

- la playlist live expose une fenetre glissante;
- la fenetre initiale cible 6 segments;
- les segments restent disponibles quelques minutes apres leur sortie de la playlist pour absorber les reconnects et le cache CDN;
- les noms de segments sont immuables une fois publies;
- le numero de sequence est monotone pendant la vie du service.

### Replay du jour

Playlist replay:

```http
GET /audio/replay/today/playlist.m3u8
```

Segments replay:

```http
GET /audio/replay/today/segments/{local_day}/{sequence}.ts
```

Regles replay:

- `local_day` est au format `YYYY-MM-DD`;
- le jour produit est calcule en timezone `Europe/Paris`;
- le replay reutilise les segments emis par le live depuis minuit;
- il ne regenere pas de clips par news;
- il ne remonte pas un historique multi-jours;
- a minuit `Europe/Paris`, la playlist repart a zero et les anciens segments sortent du contrat applicatif.

## Headers HTTP

Playlists:

```http
Content-Type: application/vnd.apple.mpegurl
Cache-Control: public, max-age=1, must-revalidate
```

Segments:

```http
Content-Type: video/MP2T
Cache-Control: public, max-age=86400, immutable
```

Justification:

- les playlists doivent rester fraiches car elles changent souvent;
- les segments peuvent etre caches longtemps parce que leur URL ne doit jamais changer de contenu;
- le CDN doit absorber les lectures concurrentes sans renvoyer chaque client vers Rust origin.

## Metadata edge associees

Le control plane reste porte par `EPIC-018` et le contrat `EdgeAudioState`.

Etat audio annonce:

```json
{
  "status": "idle | buffering | playing | delayed | error",
  "live_playlist_url": "/audio/live/playlist.m3u8",
  "replay_today_playlist_url": "/audio/replay/today/playlist.m3u8",
  "detail": null,
  "updated_at": "2026-05-03T08:00:10Z"
}
```

Regles:

- `idle`: aucun flux audio disponible;
- `buffering`: segments en preparation ou pas encore assez nombreux;
- `playing`: live HLS lisible;
- `delayed`: flux lisible mais en retard par rapport aux metadata live;
- `error`: flux audio indisponible.

`EdgeNowPlaying` reste la source pour relier le flux audio au contenu courant: `stream_id`, `translation_id`, `news_event_id`, `title`, `news_level`, `started_at` et `timestamp`.

## Auth et cache

Le premier palier ne met pas de JWT par segment HLS.

Raison:

- verifier chaque segment avec une session utilisateur casse l'interet du CDN;
- 1000 utilisateurs doivent pouvoir lire les memes fichiers cacheables;
- les URLs audio sont decouvertes via l'app authentifiee et les metadata live.

Limite acceptee:

- une URL HLS connue peut etre partagee tant que le palier initial reste en place.

Evolution possible sans changer le contrat fonctionnel:

- signer les URLs de playlists avec TTL court;
- garder les segments cacheables;
- laisser Cloudflare valider l'acces avant de servir la playlist.

## Compatibilite transitoire

`/ws/audio` reste disponible pendant `NEWS-100`, `NEWS-101` et `NEWS-102`.

Il cesse d'etre le chemin nominal seulement avec `NEWS-103`, quand le client web sait lire le live HLS et le replay du jour.

## Criteres d'acceptation NEWS-099

- le contrat distingue live et replay;
- le replay est unique pour la journee courante;
- le reset est explicitement cale sur minuit `Europe/Paris`;
- aucun endpoint de clip par news n'est defini;
- le format public HLS cible est separe du WAV interne TTS;
- les headers CDN sont explicites;
- les metadata edge necessaires au client sont identifiees;
- la compatibilite transitoire avec `/ws/audio` est documentee.
