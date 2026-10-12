# NEWS-170 - Fix resume audio du jour qui demarre au bord live

## Objectif

Corriger le bouton frontend `Lancer le resume audio du jour`.

Le comportement attendu est une lecture du replay HLS du jour depuis le debut de la
journee courante. Le lecteur ne doit pas rejoindre automatiquement le bord live de la
playlist.

## Priorite

Priorite 1.

Le replay HLS est une fonction utilisateur visible sur la page Feed. S'il demarre au bord
courant ou reste en attente de futurs segments, le resume du jour parait ne pas
fonctionner alors que les segments existent.

## Audit

Le bouton Feed appelle `replayAudioForHistory`, puis `startReplayTodayHlsPlayback`.

Chemin execute:

1. `GET /api/replay/today` recupere la metadata protegee du replay.
2. Le client charge la playlist `/audio/replay/today/playlist.m3u8` ou son proxy
   `/hls/audio/replay/today/playlist.m3u8`.
3. Le backend rend une playlist HLS `EVENT`, sans `#EXT-X-ENDLIST`, car la journee
   continue.

Le point fragile etait le demarrage du player:

- une playlist `EVENT` sans `ENDLIST` est interpretable comme un flux encore vivant;
- HLS.js demarre par defaut au live edge (`startPosition = -1`);
- le replay du jour n'indiquait pas de `#EXT-X-START`;
- le resume pouvait donc ignorer le debut de journee ou attendre au bord courant au lieu
  de rejouer le contenu depuis minuit.

## Correctif

- Le backend ajoute `#EXT-X-START:TIME-OFFSET=0,PRECISE=YES` aux playlists replay du
  jour non vides.
- Le client HLS.js force `startPosition: 0` pour le mode `replay_today`.
- Les clips HLS gardent leur comportement existant.

Cette double protection couvre les lecteurs qui honorent le manifeste et le chemin HLS.js
nominal du client web.

## Critere d'acceptation

- `Lancer le resume audio du jour` lit le replay depuis le premier segment disponible du
  jour courant.
- La playlist replay reste une playlist `EVENT` sans `#EXT-X-ENDLIST` pendant la journee.
- La lecture par clips `audio_ready` n'est pas modifiee.
- Les segments de J-1 restent exclus du replay du jour.
- `cargo test replay_today_playlist_exposes_all_current_day_segments` passe.
- `npm run typecheck` passe cote client.

## Verification

```bash
cd filfinance-service
cargo test replay_today_playlist_exposes_all_current_day_segments

cd ../filfinance-client
npm run typecheck
```

Verification manuelle recommandee:

1. attendre au moins deux clips HLS publies dans le replay du jour;
2. ouvrir la page Feed;
3. cliquer sur `Lancer le resume audio du jour`;
4. verifier que la lecture commence par le premier segment du jour, pas par le dernier
   segment publie.

## Hors scope

- Transformer le replay du jour en VOD finalisee.
- Ajouter un replay multi-jours.
- Modifier la generation TTS ou les clips `audio_ready`.
