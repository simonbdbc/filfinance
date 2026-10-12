# RLOGGER

RLOGGER est le nom public de la bibliothèque Rust 1.95+, édition 2024,
version 0.2.0, licence MIT.

> Le package et la crate de la version présente s’appellent encore
> `ordered-local-logger` et `ordered_local_logger`. Ils seront renommés en
> `rlogger` par RLOG-056 avant publication. Les exemples ci-dessous emploient
> donc les noms réellement compilables aujourd’hui.
Un worker écrit des logs locaux ordonnés pour plusieurs instances. Aucun serveur,
runtime async ou singleton n’est démarré par l’application sans demande explicite.

```rust
use ordered_local_logger::{Config, Runtime, InstanceConfig, Fields, Value, info};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let runtime = Runtime::new(Config::new(std::env::temp_dir().join("logger-example")))?;
let logger = runtime.instance(InstanceConfig::new("backend"))?;
let operation = logger.with_context(&Fields::from([
    ("call_id".into(), Value::from("c1")),
]))?;
info!(operation, "Début action {}", 1);
runtime.flush()?;
let report = runtime.shutdown()?;
assert_eq!(report.totals.accepted, report.totals.written);
# Ok(())
# }
```

Installation locale dans une application voisine :

```toml
[dependencies]
ordered-local-logger = { path = "../lib-rust-logger" }
```

Le nom du package contient des tirets, le nom importé utilise des underscores.
Le package est vérifié localement ; aucune publication sur crates.io n’est requise.
La publication du code source passe d’abord par le dépôt GitHub public RLOGGER.

## API et résultats

`Config::new(directory)` expose les limites globales. `Runtime::instance(config)`
crée une identité ; `Logger::clone()` la partage. `with_context(&Fields)` construit
un instantané immuable, même dans des tâches concurrentes. Priorités : application,
instance, opération, événement. `Fields` est un BTreeMap trié ; dernière insertion
gagnante ; métadonnées internes réservées. Flottants finis, égalité par bits.

`emit(level, source, destination, fields, message)` et `emit_args` retournent
`Accepted(sequence)`, `Filtered` ou `Refused(reason)`. Motifs : capacité pleine,
taille excessive, fermé, panne, champs invalides ou destination inconnue.
Les macros `trace!`, `debug!`, `info!`, `warn!`, `error!` capturent module/fichier/ligne
et ne calculent pas les arguments filtrés. Les champs typés passent par `emit` ou
le handle contextualisé. Formatage arbitraire : coût et allocations utilisateur
possibles ; la chaîne retenue par le logger est bornée, sans troncature silencieuse.

L’admission sérialisée fixe capture, séquence et insertion. Une réservation mémoire
avant formatage ne réserve aucun ordre. Aucun accès disque ni attente de capacité
dans l’émission ; la contention du mutex est possible. Le worker conserve la
projection de cet ordre dans chaque destination. Des fichiers distincts n’ont pas
d’ordre global de flush. `A A B A A` donne `A x2 / B / A x2` si les métadonnées
significatives sont identiques. Les bornes d’un groupe ne décrivent pas nécessairement
une plage contiguë de séquences globales et ne reconstruisent pas ses occurrences.

## Destinations, fichiers et temps

Déclarer `InstanceConfig.destinations` et `routes` avant l’instance. Priorité :
destination explicite reconnue, plus long préfixe de module par segments `::`,
destination portant le nom de crate si déclarée, sinon première destination.
Les noms sont minuscules ASCII (`a-z`, chiffres, `_`, `-`, max 64 octets).
Ils ne permettent pas un chemin libre ; les champs ne créent aucune destination.

`Config::directory` est le parent applicatif. Le writer crée `rlogger/<jour>/<instance>/`
`<destination>-HH-<run-id>-h<début-heure-UTC>-s<segment>.active.log`.
Rotation horaire obligatoire ; `Rotation` et `InstanceConfig.rotation` sont retirés.
`log_root()` remplace `run_path()` pour la racine commune ; `run_id()` identifie
les fichiers du runtime. Création exclusive, cache LRU borné, segments uniques.
Date de capture locale, offset dans chaque ligne, intervalle UTC dans chaque nom :
deux heures locales répétées restent distinctes. Un fuseau stable est requis pendant le run.

Le worker clôt à la fin de l’heure, même au repos, à l’éviction LRU et à l’arrêt sain :
groupes terminés, flush, fermeture, renommage sans écrasement en `.log`.
Un fichier finalisé n’est jamais rouvert ; une capture retardée crée un nouveau segment.
Le compagnon peut récupérer les anciens actifs abandonnés en `.recovered.log`,
seulement après acquisition du verrou système du run. Il gère tailles, téléchargement,
suppression explicite et dossiers anciens vides. Aucun serveur n’est ajouté à la lib.
Le guide de migration et coordination appartient au dépôt amont RLOGGER ; il n’est pas inclus dans cette copie du composant.

Format texte versionné `RLOG/1` ; contrôles échappés, Unicode conservé.
`LATENCY` mesure capture monotone → préparation de ligne ; le groupe utilise
la première capture, donc son maximum. Écriture, flush et affichage sont exclus.

## Ressources et fermeture

Défauts : 8 192 slots et 32 Mio de crédits, réservation pessimiste de 8 Kio par
événement (donc 4 096 réservations simultanées avec ces deux limites), 32 instances,
128 destinations, 32 writers de 64 Kio, 1 Mio de contextes d’opérations. Les groupes
conservent le crédit de leur premier événement ; les occurrences compactées sont
des compteurs. Les copies transitoires, métadonnées et l’allocateur s’ajoutent au budget.

Groupes de 200 ms maximum, flush des buffers toutes les 100 ms, contrôles servis sous
flux continu. Ce budget volontaire ne garantit pas une visibilité sous une seconde
sur disque bloqué ou sous forte contention. Les refus de capacité sont comptés et
résumés au plus une fois/seconde/instance, avec solde à flush/arrêt. Toute erreur
d’émission coupe conservativement la continuité des groupes de l’instance.

`stats()` donne l’instantané global et `Logger::counters()` celui de l’instance.
`written` compte les occurrences confirmées après flush ; `lines` inclut les
résumés internes ; `pending = accepted - written`. En panne, certains octets peuvent
être visibles sans être confirmés : aucune écriture partielle n’est rejouée.
Les compteurs après fermeture des handles peuvent dépasser le bilan déjà retourné.

`flush()` et `shutdown()` sont des opérations bloquantes globales. Shutdown ferme
l’admission, draine, termine groupes/soldes, flush et join ; appel idempotent.
`ShutdownError.report` conserve un bilan partiel en cas de panne fail-stop.
Drop du runtime draine aussi et peut attendre le disque, mais ignore les erreurs.
Drop d’un clone ne ferme rien. Pas de timeout arbitraire ni fsync promis.

## tracing et exemples

Ajouter `features = ["tracing"]`, puis composer
`tracing_subscriber::registry().with(LoggerLayer::new(logger))`. Aucun subscriber
global n’est installé implicitement. `log_instance = "name"` choisit une instance
préalablement ajoutée avec `with_instance`; une inconnue est refusée.
Spans mis à jour, parents explicites et contexte des futures instrumentées sont
pris en charge. Ne pas conserver un guard `enter()` à travers un await.
Adaptateur borné : 1 024 spans, 64 collectes simultanées, profondeur 32 ; jeux de
champs de 8 Kio, jusqu’à plusieurs copies temporaires par collecte. Le subscriber
de l’application possède ses propres ressources.

Depuis ce dossier :

```sh
cargo run --example native -- /tmp/local-logger-native
cargo run --example workflow -- /tmp/local-logger-workflow
cargo run --features tracing --example tracing_workflow
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
cargo package --allow-dirty
```

Les exemples n’utilisent ni réseau ni modèle IA. Le workflow simule début, token,
fin, erreur et annulation, en déportant shutdown avec `spawn_blocking`.
Les métriques reproductibles et le guide du lecteur sont dans les documents du
dépôt parent : `docs/measurements`, `docs/08-format-v1.md`, `front-react-logger/README.md`.
La seule plateforme officiellement validée dans cette livraison est macOS Apple Silicon.
