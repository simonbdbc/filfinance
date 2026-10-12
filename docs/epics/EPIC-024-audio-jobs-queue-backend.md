# EPIC-024 - Queue backend audio_jobs

## Objectif

Remplacer l'admission audio live basee uniquement sur le `broadcast` des traductions par une vraie queue backend `audio_jobs`, afin de garantir la generation audio quand plusieurs news arrivent a la suite ou quand plusieurs sources publient simultanement.

Le pipeline E2E de test reste volontairement dans le meme flux qu'une vraie news: une news synthetique peut produire une traduction, entrer dans la queue audio, generer un clip HLS et avancer la chronologie audio selon les memes regles qu'une news live.

## Statut

A finir. `EPIC-021` valide le modele live audio par clips HLS; `EPIC-024` doit maintenant durcir le pipeline producteur pour qu'aucune news eligible ne soit perdue si la traduction, la TTS ou la generation HLS ralentit.

## Probleme

Aujourd'hui le worker TTS consomme directement `translated_broadcast_tx`.

Ce modele est simple, mais il a des limites:

- le `broadcast` Tokio n'est pas durable;
- un consommateur lent peut lagger et perdre des messages;
- il n'y a pas de statut audio persiste par traduction;
- il n'y a pas de retry borne;
- il n'y a pas de backpressure explicite;
- l'observabilite ne permet pas de distinguer proprement `pending`, `processing`, `ready`, `failed` et `skipped`;
- l'arrivee simultanee de plusieurs sources peut saturer le worker audio sans trace exploitable par job.

## Decisions

- `audio_jobs` devient la source de verite de la file audio backend.
- Un job audio est cree apres persistence d'une traduction `ready` eligible.
- Les runs de test E2E entrent dans `audio_jobs` comme les vraies news.
- Les runs de test E2E ne sont pas affiches dans le fil d'actus principal de la home page; ce filtrage reste une responsabilite d'affichage/frontend ou d'API de feed, pas du pipeline audio.
- La queue est bornee par la meme politique data que news/traductions: conserver J et J-1, supprimer J-2+.
- Le worker audio consomme les jobs dans un ordre deterministic base sur la chronologie metier.
- La watermark monotone reste une regle de diffusion audio live, mais elle doit etre appliquee de facon observable dans la queue, avec statut `skipped` si un job est trop ancien.
- Le premier palier reste dans le crate Rust actuel, sans Cargo workspace.

## Architecture cible

```text
translation persisted ready
  |
  v
audio_jobs insert pending
  |
  v
audio worker loop
  |
  +--> claim next pending job
  |
  +--> load translation + source news
  |
  +--> check admission / watermark
  |
  +--> synthesize intro
  |
  +--> publish audio_ready
  |
  +--> synthesize body
  |
  +--> publish HLS clip + replay
  |
  +--> mark ready / failed / skipped
```

## Modele de donnees cible

Table SQLite proposee:

```sql
CREATE TABLE audio_jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    translation_id INTEGER NOT NULL UNIQUE,
    news_event_id INTEGER NOT NULL,
    source TEXT NOT NULL,
    event_kind TEXT NOT NULL,
    status TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 0,
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 3,
    scheduled_at TEXT NOT NULL,
    claimed_at TEXT,
    completed_at TEXT,
    skipped_at TEXT,
    failed_at TEXT,
    last_error TEXT,
    stream_id TEXT,
    playlist_url TEXT,
    published_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

Index minimum:

```sql
CREATE INDEX idx_audio_jobs_pending
ON audio_jobs (status, priority DESC, published_at, translation_id);

CREATE INDEX idx_audio_jobs_retention
ON audio_jobs (published_at, created_at);
```

## Statuts

- `pending`: job cree, pas encore pris par le worker.
- `processing`: job reclame par un worker.
- `ready`: audio publie et finalise.
- `failed`: erreur definitive apres retry borne.
- `skipped`: job volontairement ignore par une regle d'admission, par exemple watermark.

## Regles d'entree

Un job peut etre cree si:

- la traduction est en langue cible;
- `status = ready`;
- `narration_text` n'est pas vide;
- la traduction n'a pas deja un job audio.

Un job ne doit pas etre cree pour:

- une traduction en erreur;
- une langue non cible;
- une narration vide;
- une traduction deja representee dans `audio_jobs`.

## Ordre de consommation

Ordre nominal:

1. `priority DESC`
2. timestamp metier `published_at`
3. `translation_id`

Le timestamp metier doit rester le meme que celui expose au live:

1. `translation.published_at`
2. `news_event.published_at`
3. `news_event.event_time`
4. `news_event.ingested_at`
5. `translation.created_at`

## Concurrence

Palier 1:

- un seul worker audio;
- traitement strictement sequentiel;
- claim atomique du prochain job `pending`;
- reset au demarrage des jobs restes `processing` depuis un ancien process.

Palier 2 eventuel:

- plusieurs workers;
- claim atomique avec lease;
- limitation de concurrence TTS selon provider;
- maintien d'un ordre de diffusion live stable.

## Retry

Retry borne:

- incrementer `attempts` a chaque prise en charge;
- repasser en `pending` si erreur transitoire et `attempts < max_attempts`;
- passer en `failed` si `attempts >= max_attempts`;
- enregistrer `last_error`;
- distinguer erreurs TTS, HLS intro, HLS body, replay et publication control plane.

## Observabilite

Ajouter au snapshot runtime audio:

- `audio_jobs_pending_total`
- `audio_jobs_processing_total`
- `audio_jobs_ready_total`
- `audio_jobs_failed_total`
- `audio_jobs_skipped_total`
- `audio_jobs_oldest_pending_age_ms`
- `audio_jobs_last_error`
- `audio_jobs_last_ready_stream_id`

Logs structures attendus:

- `audio_job_created`
- `audio_job_claimed`
- `audio_job_skipped`
- `audio_job_ready`
- `audio_job_failed`
- `audio_job_retry_scheduled`

## Endpoints admin futurs

Hors MVP strict, mais a prevoir:

- `GET /api/admin/audio/jobs?status=pending|processing|ready|failed|skipped`
- `POST /api/admin/audio/jobs/{id}/retry`
- `POST /api/admin/audio/jobs/{id}/skip`

Ces endpoints doivent etre reserves aux roles `moderator`/`admin`.

## Retention

La retention doit supprimer J-2+ pour:

- `audio_jobs`;
- artefacts HLS associes si le fichier est dans le stockage local de replay ou clips;
- erreurs anciennes liees aux jobs.

Les jobs J et J-1 restent consultables pour debug et reprise.

## Migration

1. Ajouter table et repository `audio_jobs`.
2. Creer un job apres persistence d'une traduction `ready`.
3. Faire consommer le worker TTS depuis `audio_jobs` au lieu de consommer directement `translated_broadcast_tx`.
4. Garder `translated_broadcast_tx` pour WebSocket, Telegram et edge metadata.
5. Reporter les etats `ready`, `failed`, `skipped` dans `audio_jobs`.
6. Ajouter retention J/J-1.
7. Ajouter les compteurs runtime.
8. Ajouter les tests de concurrence et d'ordre.

## Tests

Tests backend attendus:

- creation d'un job pour une traduction `ready`;
- pas de job pour une langue non cible;
- pas de job pour une traduction en erreur;
- idempotence par `translation_id`;
- claim du prochain job dans l'ordre attendu;
- job `processing` ancien remis en `pending` au demarrage;
- retry borne puis `failed`;
- watermark produit `skipped` observable;
- run de test E2E entre bien dans la queue;
- retention supprime J-2+;
- plusieurs news simultanees restent toutes traitees ou tracees.

## Definition of Done

- Le worker audio ne depend plus du `broadcast` comme source de garantie.
- Une rafale de traductions `ready` produit des jobs persistants.
- Les jobs sont traites un par un dans un ordre deterministic.
- Aucun job eligible n'est perdu si le worker TTS lagge temporairement.
- Les erreurs TTS/HLS sont visibles par job et retryees de facon bornee.
- Les runs de test E2E valident le meme pipeline audio que les vraies news.
- Le fil d'actus principal peut exclure les runs de test sans modifier le pipeline audio.
- `cargo fmt --check`, `cargo test` et `cargo clippy -- -D warnings` passent.
