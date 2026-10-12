# NEWS-221 - Retrait du legacy n8n calendrier

## Objectif

Supprimer n8n du chemin calendrier des que le parcours Rust natif complet a
reussi son test de recuperation local. n8n fournit le processus historique a
caler, mais ses resultats ne sont pas une reference de validation.

**Statut au 30 aout 2026 : retrait du depot et de la CI realise sur `main`.**
La gate locale Rust-only a reussi : `cargo fmt --check`, le test de recuperation
cible et `cargo test` (341 passants) ont ete executes depuis
`filfinance-service`. Aucun push, tag de release ni deploiement PROD n'a ete
effectue dans le cadre de cette validation locale.

## Gate de retrait

- Un unique test de recuperation complet local du chemin Rust seul est la gate
  de validation. Il couvre dans un meme scenario les fixtures, la collecte,
  la normalisation, OpenRouter, SQLite, resume, Piper/HLS, alertes, interruption
  et reprise idempotente, ainsi que les surfaces calendrier et l'independance du
  live news.
- Aucun resultat n8n, campagne multi-jours ou rapport shadow n'est une condition
  de retrait.
- Le tag de release et le rollback vers la version precedente sont documentes
  pour le deploiement ; ils ne constituent pas une seconde validation de la
  migration.

## Retrait

1. **Fait dans le depot :** workflows, `compose.n8n.yaml`, endpoints internes,
   variables, secrets, scripts CI/runbooks et modules Rust devenus legacy sont
   retires. Les checks DEV/n8n sont remplaces par le test Rust de recuperation,
   et `filfinance_news_error_alert` disparait avec son workflow.
2. **A faire avant livraison :** taguer la release actuellement deployee, qui
   devient la cible de rollback, puis inventorier et exporter hors Git les
   workflows calendrier sans credential ni secret.
3. **A faire lors du deploiement :** pousser `main` et deployer Rust-only ; le
   script retire les conteneurs n8n sans supprimer leur base ni leurs volumes.
4. **Conserve :** migrations SQLite appliquees et champs historiques `n8n_*`
   nullables afin de ne pas modifier les contrats admin. Toute purge de volume
   ou de base exige une autorisation distincte.

Le rollback redeploie le tag de release precedent. Il ne depend pas d'un n8n
actif ; une restauration de workflow depuis l'export protege reste une decision
operateur explicite.

## Dependances

- [`NEWS-220`](./NEWS-220-bascule-calendrier-rust-sqlite-audio-alertes.md)
