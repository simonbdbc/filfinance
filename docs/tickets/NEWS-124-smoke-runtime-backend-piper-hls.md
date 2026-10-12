# NEWS-124 - Smoke runtime backend Piper/HLS post-deploy

## Objectif

Bloquer le deploiement backend si la configuration serveur annonce TTS/Piper ou HLS actifs alors que le runtime conteneur n'est pas operationnel.

Ce smoke protege le passage progressif vers l'audio serveur: tant que `NEWS_TTS_ENABLED=false` et `NEWS_HLS_ENABLED=false`, le deploy reste compatible avec le bootstrap backend actuel. Des qu'un flag est active, le job GitLab doit verifier que la chaine correspondante fonctionne reellement.

Depuis `NEWS-126`, ce smoke verifie aussi OPUS-MT quand `NEWS_TRANSLATION_PROVIDER` cible un provider local/OPUS-MT.

## Statut

Livre cote repo le 3 mai 2026.

## Scope

- Ajouter `scripts/smoke_backend_runtime.sh`.
- Executer le smoke apres `docker compose up -d --build --force-recreate` dans `deploy:home-srv`.
- Verifier le endpoint `/health` dans le conteneur.
- Si `NEWS_TRANSLATION_PROVIDER=opus_mt_local_session`, verifier le runtime OPUS-MT.
- Si `NEWS_TTS_ENABLED=true`, verifier le runtime Piper.
- Si `NEWS_HLS_ENABLED=true`, verifier le runtime HLS.
- Echouer le job GitLab si un check obligatoire echoue.
- Afficher `docker compose ps`, l'etat `docker inspect` et les derniers logs du service si `/health` ne devient pas joignable.

## Checks OPUS-MT

Le smoke OPUS-MT s'execute si `NEWS_TRANSLATION_PROVIDER` vaut:

- `opus_mt_local_session`;
- `local_command_session`;
- `opus_mt_local_command`;
- `local_command`.

Il verifie:

- `NEWS_TRANSLATION_COMMAND` disponible et executable dans le conteneur;
- `NEWS_TRANSLATION_COMMAND_ARGS` present;
- valeurs obligatoires presentes dans `NEWS_TRANSLATION_COMMAND_ARGS`, notamment `--translation-mode`;
- helper OPUS-MT present;
- traduction courte qui retourne un payload `translated_title`;
- absence de champ `error` dans la reponse du helper.

Le parsing interne ajoute un retour ligne final avant de convertir la liste CSV en arguments shell. Sans ce retour ligne, `read` peut ignorer le dernier champ, ce qui ferait passer `--translation-mode` sans sa valeur au helper Python.

Si `NEWS_TRANSLATION_PROVIDER=heuristic_fr`, le check OPUS-MT est ignore.

## Checks Piper

Le smoke Piper s'execute uniquement si `NEWS_TTS_ENABLED=true`.

Il verifie:

- `NEWS_TTS_PROVIDER=piper_local_session`;
- `NEWS_TTS_COMMAND` disponible et executable dans le conteneur;
- helper Piper present, via le premier argument de `NEWS_TTS_COMMAND_ARGS` ou `/app/scripts/piper_local_session.py`;
- `NEWS_TTS_MODELS_DIR` present;
- voix `NEWS_TTS_VOICE` valide;
- synthese courte JSONL qui retourne un payload `audio_base64`.

Ce check detecte donc les cas courants:

- Python absent;
- paquet `piper-tts` absent;
- script Piper non copie dans l'image;
- modele/voix manquant;
- erreur runtime Piper a la premiere synthese.

## Checks HLS

Le smoke HLS s'execute uniquement si `NEWS_HLS_ENABLED=true`.

Il verifie:

- `NEWS_TTS_ENABLED=true`, car HLS depend de la TTS serveur;
- `NEWS_HLS_FFMPEG_COMMAND` disponible dans le conteneur;
- `NEWS_HLS_SEGMENTS_DIR` creeable et writable;
- `/audio/replay/today/playlist.m3u8` retourne une playlist HLS valide.

Si `NEWS_HLS_ACCESS_CONTROL_ENABLED=true`, le smoke verifie a la place que cette playlist refuse une URL nue en `401`. Le chemin avec ticket est couvert par les tests backend `NEWS-159` et par les smokes/load tests authentifies.

Depuis `EPIC-021`, le smoke ne teste plus `/audio/live/playlist.m3u8`: la radio HLS permanente est supprimee du runtime. Le chemin nominal live passe par des evenements `audio_ready` et des clips finalises sous `/audio/clips/{stream_id}/playlist.m3u8`. La generation automatique d'un clip complet reste a traiter dans un topic CI dedie.

## Execution manuelle serveur

Depuis le dossier de deploiement:

```bash
cd /home/traefik/filfinance-docker
bash scripts/smoke_backend_runtime.sh
```

Variables optionnelles:

```bash
FILFINANCE_ENV_FILE=.env.server \
FILFINANCE_COMPOSE_FILES=compose.yaml:compose.prod.yaml \
FILFINANCE_SERVICE=filfinance-service \
FILFINANCE_HEALTH_RETRY_ATTEMPTS=30 \
FILFINANCE_HEALTH_RETRY_DELAY_SECS=2 \
bash scripts/smoke_backend_runtime.sh
```

## Integration CI

Le job `deploy:home-srv` execute le smoke apres:

```bash
docker compose --env-file .env.server -f compose.yaml -f compose.prod.yaml up -d --build --force-recreate
docker compose --env-file .env.server -f compose.yaml -f compose.prod.yaml ps
```

`--force-recreate` est volontaire: une modification de `.env.server` doit etre injectee dans un nouveau conteneur meme si l'image Docker est deja a jour.

Si `NEWS_TTS_ENABLED=true` ou `NEWS_HLS_ENABLED=true` et que le runtime correspondant n'est pas pret, le job GitLab echoue.

Si le conteneur redemarre avant `/health`, le smoke imprime l'etat Compose, l'etat Docker detaille (`exit_code`, `oom_killed`, `restart_count`) et les 200 dernieres lignes de logs du service avant de sortir en erreur. C'est volontaire: le log GitLab doit contenir la cause concrete du crash, pas seulement une boucle de retries.

## Suite runtime

Ce ticket ajoute le garde-fou de deploiement. Le packaging du runtime Piper dans l'image Docker est traite par `NEWS-125`.

Pour activer TTS/HLS en production serveur, il faut:

- une image contenant Python, `piper-tts` et `scripts/piper_local_session.py`;
- les assets voix presents dans `/app/data/models/piper`;
- `ffmpeg`.
