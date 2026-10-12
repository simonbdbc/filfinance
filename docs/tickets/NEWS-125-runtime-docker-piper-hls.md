# NEWS-125 - Runtime Docker Piper pour TTS/HLS

## Objectif

Rendre l'image Docker backend capable de lancer le provider `piper_local_session` quand `NEWS_TTS_ENABLED=true`.

Le probleme constate en live etait:

```text
failed to spawn piper helper for piper_local_session: No such file or directory
```

L'origin HLS etait actif et servait bien une playlist vide valide, mais la generation audio echouait car le runtime Piper n'etait pas disponible dans le conteneur.

## Statut

Livre cote repo le 3 mai 2026.

## Changements

- L'image Docker installe Python, `python3-venv`, `piper-tts` et les libs runtime utiles.
- Le helper `scripts/piper_local_session.py` est copie dans `/app/scripts`.
- Un venv stable est cree dans `/opt/filfinance/piper-venv`.
- `.env.server.example` pointe `NEWS_TTS_COMMAND` vers ce venv Docker.
- Le script `bootstrap_piper_local.sh` accepte des overrides pour bootstraper les voix dans le volume Docker.
- Le backend Docker lance aussi ce bootstrap automatiquement au demarrage quand Piper est active et que la voix manque dans `/app/data/models/piper`.

## Configuration serveur attendue

```env
NEWS_TTS_ENABLED=true
NEWS_TTS_PROVIDER=piper_local_session
NEWS_TTS_VOICE=fr_FR-upmc-medium-jessica
NEWS_TTS_COMMAND=/opt/filfinance/piper-venv/bin/python
NEWS_TTS_COMMAND_ARGS=/app/scripts/piper_local_session.py
NEWS_TTS_MODELS_DIR=/app/data/models/piper

NEWS_HLS_ENABLED=true
NEWS_HLS_SEGMENTS_DIR=/app/data/hls
NEWS_HLS_FFMPEG_COMMAND=ffmpeg
```

## Bootstrap voix serveur

Les assets voix ne sont pas packages dans l'image. Ils doivent rester dans le volume persistant `/app/data`.

Depuis `/home/traefik/filfinance-docker` sur le serveur:

```bash
docker compose --env-file .env.server -f compose.yaml -f compose.prod.yaml exec filfinance-service \
  env PIPER_VENV_DIR=/opt/filfinance/piper-venv \
      PIPER_DATA_DIR=/app/data/models/piper \
      PIPER_INSTALL_DEPS=false \
  /app/scripts/bootstrap_piper_local.sh fr_FR-upmc-medium
```

Puis valider la voix cible:

```bash
docker compose --env-file .env.server -f compose.yaml -f compose.prod.yaml exec filfinance-service \
  /opt/filfinance/piper-venv/bin/python /app/scripts/piper_local_session.py \
  --data-dir /app/data/models/piper \
  --validate-voice fr_FR-upmc-medium-jessica
```

## Validation attendue

Apres rebuild/deploy:

```bash
bash scripts/smoke_backend_runtime.sh
curl -i https://filfinance-api.sbcenter.fr/audio/live/playlist.m3u8
```

Le smoke doit echouer si `NEWS_TTS_ENABLED=true` mais que Piper ne peut pas generer un WAV court.

## Points de vigilance

- Le volume `./volumes/service/data:/app/data` masque tout contenu `/app/data` de l'image: les voix doivent donc etre installees dans le volume serveur.
- `NEWS_TTS_COMMAND` ne doit pas pointer vers un chemin hote du type `/home/traefik/...`; il est execute dans le conteneur.
- `NEWS_HLS_ENABLED=true` exige `NEWS_TTS_ENABLED=true`.
