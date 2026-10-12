# NEWS-126 - Runtime Docker OPUS-MT traduction

## Objectif

Retrouver en deploiement Docker la vraie traduction EN -> FR via `OPUS-MT tc-big en-fr`, au lieu du fallback `heuristic_fr`.

Le symptome actuel n'est pas une regression du pipeline: le serveur Docker est encore configure en `NEWS_TRANSLATION_PROVIDER=heuristic_fr`, ce qui produit une traduction partielle et mot-a-mot.

## Priorite

Priorite 1 immediate.

Ce ticket doit passer avant les optimisations de qualite, de pool ou de HLS, car il remet le texte FR canonique au niveau attendu.

## Statut

Livre cote repo le 3 mai 2026.

## Scope

- Installer le runtime Python de traduction dans l'image Docker ou dans un venv Docker stable.
- Installer `ctranslate2` et `sentencepiece`.
- Copier `scripts/opus_mt_local_command.py` dans l'image.
- Definir un chemin modele stable dans le conteneur: `/opt/filfinance/models/opus-mt-tc-big-en-fr-ct2-int8`.
- Installer le modele pendant le build Docker.
- Basculer `.env.server.example` vers le provider Docker cible sans secrets.
- Ajouter un smoke post-deploy traduction si `NEWS_TRANSLATION_PROVIDER=opus_mt_local_session`.

## Changements livres

- L'image Docker cree le venv `/opt/filfinance/opus-mt-venv`.
- L'image installe `ctranslate2` et `sentencepiece` depuis `scripts/requirements-opus-mt.txt`.
- L'image installe `huggingface_hub` pour telecharger le modele preconverti pendant le build.
- L'image telecharge et verifie `keisuke-miyako/opus-mt-tc-big-en-fr-ct2-int8` dans `/opt/filfinance/models/opus-mt-tc-big-en-fr-ct2-int8`.
- Le helper `scripts/opus_mt_local_command.py` est deja copie dans `/app/scripts`.
- `.env.server.example` pointe maintenant vers les chemins Docker OPUS-MT.
- `scripts/smoke_backend_runtime.sh` teste la traduction si le provider configure est local/OPUS-MT.

## Reference CTranslate2

La doc officielle CTranslate2 recommande l'installation Python par wheel `pip install ctranslate2` pour Linux x86-64/AArch64 avec Python `>=3.9`. Le guide OPUS-MT CTranslate2 confirme que les modeles OPUS-MT/Marian sont compatibles avec CTranslate2 et peuvent etre servis dans un format CTranslate2.

References:

- [Installation CTranslate2](https://opennmt.net/CTranslate2/installation.html)
- [Guide OPUS-MT CTranslate2](https://opennmt.net/CTranslate2/guides/opus_mt.html)

## Configuration cible

```env
NEWS_TRANSLATION_ENABLED=true
NEWS_TRANSLATION_SOURCE_LANGUAGE=en
NEWS_TRANSLATION_TARGET_LANGUAGE=fr
NEWS_TRANSLATION_PROVIDER=opus_mt_local_session
NEWS_TRANSLATION_COMMAND=/opt/filfinance/opus-mt-venv/bin/python
NEWS_TRANSLATION_COMMAND_ARGS=/app/scripts/opus_mt_local_command.py,--ct2-model-dir,/opt/filfinance/models/opus-mt-tc-big-en-fr-ct2-int8,--sentencepiece-model,/opt/filfinance/models/opus-mt-tc-big-en-fr-ct2-int8/source.tokenizer.model,--target-sentencepiece-model,/opt/filfinance/models/opus-mt-tc-big-en-fr-ct2-int8/target.tokenizer.model,--device,cpu,--compute-type,int8,--translation-mode,opus_mt_tc_big_local_session
NEWS_TRANSLATION_COMMAND_TIMEOUT_MS=5000
NEWS_TRANSLATION_WARMUP_ENABLED=true
```

## Installation modele Docker

Les assets modele restent hors Git, mais ils sont maintenant installes dans l'image Docker pendant le build.

Le Dockerfile execute:

```bash
/opt/filfinance/opus-mt-venv/bin/python /app/scripts/install_opus_mt_model.py \
  --output-dir /opt/filfinance/models/opus-mt-tc-big-en-fr-ct2-int8
```

Le deploy verifie ensuite le modele via `scripts/smoke_backend_runtime.sh`.

Verification manuelle possible dans le conteneur:

```bash
cd /home/traefik/filfinance-docker
docker compose --env-file .env.server -f compose.yaml -f compose.prod.yaml exec filfinance-service \
  /opt/filfinance/opus-mt-venv/bin/python /app/scripts/opus_mt_local_command.py \
  --ct2-model-dir /opt/filfinance/models/opus-mt-tc-big-en-fr-ct2-int8 \
  --sentencepiece-model /opt/filfinance/models/opus-mt-tc-big-en-fr-ct2-int8/source.tokenizer.model \
  --target-sentencepiece-model /opt/filfinance/models/opus-mt-tc-big-en-fr-ct2-int8/target.tokenizer.model \
  --device cpu \
  --compute-type int8 \
  --beam-size 4 \
  --translation-mode opus_mt_tc_big_local_session <<'JSON'
{"source_language":"en","target_language":"fr","title":"Federal Reserve signals rates stay higher for longer","body":null}
JSON
```

Si le build Docker ne peut pas telecharger le modele, le deploy doit echouer. En mode incident uniquement, revenir temporairement a:

```env
NEWS_TRANSLATION_PROVIDER=heuristic_fr
NEWS_TRANSLATION_COMMAND=
NEWS_TRANSLATION_COMMAND_ARGS=
```

## Critere d'acceptation

- Le serveur Docker demarre avec `NEWS_TRANSLATION_PROVIDER=opus_mt_local_session` sans copie manuelle d'assets modele.
- Le warmup OPUS-MT passe au demarrage.
- Le run de test E2E produit une vraie phrase FR, pas un mix EN/FR heuristique.
- En cas de panne OPUS-MT runtime, le smoke post-deploy echoue avant promotion.
- En cas d'erreur ponctuelle de traduction runtime, le fallback heuristique reste non bloquant pour l'ingestion.

## Tests

- `docker compose config`.
- Smoke post-deploy traduction.
- Run de test E2E depuis le client.
- Verification de `/api/runtime/translation`.
- Verification d'une news FinancialJuice reelle.
