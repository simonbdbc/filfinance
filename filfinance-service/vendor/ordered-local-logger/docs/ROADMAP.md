# Roadmap de développement de RLOGGER

Version de cadrage : 8 septembre 2026.

## Objet et statut

État au 8 septembre 2026 : **M0 à M5 validés**, RLOG-001 à RLOG-050 terminés.
Les tickets cochés renvoient aux preuves ; RLOG-051 à RLOG-055 restent différés.
Socle historique 0.1.0 vérifié localement. L’extension horaire 0.2.0 est suivie
par RLOG-057/058 et ses [preuves actuelles](../../docs/VALIDATION-HORAIRE.md).
Aucune publication externe.

Documents de référence :

- [Vision et périmètre](../../docs/01-vision-et-perimetre.md)
- [Fonctionnement et chronologie](../../docs/02-fonctionnement-et-chronologie.md)
- [Instances, contexte et fichiers](../../docs/03-instances-contexte-et-fichiers.md)
- [Configuration et intégration](../../docs/04-configuration-et-integration.md)
- [Décisions D01–D15, propositions P01–P12 et questions Q01–Q10](../../docs/05-decisions.md)
- [Orchestration globale et jalons d’intégration](../../ROADMAP-GLOBAL.md)
- [Roadmap du lecteur local Expo/gluestack](../../front-react-logger/docs/ROADMAP.md)

La roadmap couvre le socle et la première version utilisable, puis les extensions
différées. Aucun calendrier n’est inventé : l’estimation intervient après la
conception de l’admission et les premiers prototypes de performance.

Le lecteur et son compagnon local sont développés dans `front-react-logger` selon
D17–D21. Ils consomment les fichiers de la bibliothèque sans ajouter de serveur,
de WebSocket ou d’attente réseau au cœur du logger. Les tickets RLOG restent
inchangés ; leurs points d’intégration sont coordonnés dans la roadmap globale.

## Contrats à préserver

1. L’ordre est fixé à l’admission sérialisée, avec capture et publication coordonnées.
2. Le worker traite les événements acceptés dans cet ordre. Aucun timeout ne
   permet d’écrire ces événements hors ordre.
3. La projection dans chaque destination conserve l’ordre ; les groupes sont des
   résumés et les flushs inter-fichiers n’ont pas d’ordre global garanti.
4. Aucune I/O de fichiers ni attente de capacité dans l’émission. La courte attente
   de synchronisation entre producteurs reste possible et doit être mesurée.
5. Les ressources sont bornées au-delà de la seule file ; les refus sont visibles.
6. La capture utilise un instant monotone et une date civile ; rotation en heure locale.
7. LATENCY est présente sur chaque ligne, maximale par groupe, et ne mesure pas le flush.
8. La rétention volontaire vise nettement moins d’une seconde ; aucune garantie
   matérielle de visibilité n’est annoncée sans conditions.

## Jalons

| Jalon | Résultat vérifiable | Epics |
| --- | --- | --- |
| M0 — Contrats et squelette | Organisation, API cible et invariants documentés | E00–E01 |
| M1 — Moteur ordonné | Admission bornée, worker et signalement testables en mémoire | E02–E04 |
| M2 — Fichiers lisibles | Rotations locales, répétitions et délais de flush | E05–E06 |
| M3 — Intégration native | Cycle de vie complet, exemples et bilan d’arrêt | E07–E08 |
| M4 — Écosystème Rust | Adaptateur tracing avec contexte async | E09 |
| M5 — Première version prête à distribuer | Tests de bout en bout, mesures et package vérifié | E10–E11 |

Les tests unitaires pertinents accompagnent chaque ticket ; E10 apporte les
scénarios transversaux, pas une phase de test repoussée à la fin.

## Convention des tickets

- Identifiant stable `RLOG-NNN` ; référence des décisions dans chaque epic.
- Priorités : **P0** contrat/correction, **P1** livraison initiale, **P2** extension.
- État initial représenté par `[ ]`. Cocher seulement après réalisation et validation.
- Chaque ticket précise objectif, dépendances et critères d’acceptation.
- Une difficulté nouvelle entraîne une mise à jour du contrat ou du ticket,
  jamais une réduction silencieuse des garanties.

## E00 — Cadrer les contrats et l’organisation

**But :** lever les ambiguïtés avant d’engager les mécanismes concurrents.
**Références :** D01–D15 ; Q01–Q10.
**Sortie d’epic :** décisions techniques initiales consignées et périmètre M5 figé.

### RLOG-001 — Fixer le package et l’organisation du dépôt

- [x] **P0** — Dépendances : aucune.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Choisir le nom de crate, sa version Rust minimale et la licence. Le dossier source
  est désormais fixé à `lib-rust-logger` ; la documentation y a été déplacée selon D16.
- Acceptation : une seule racine Cargo retenue, distinction package/crate expliquée,
  liens de documentation mis à jour si nécessaire ; le frontend Expo/gluestack
  possède un cadrage distinct et n’est pas inclus dans cet epic.

### RLOG-002 — Spécifier l’admission et la portée de l’ordre

- [x] **P0** — Dépendances : aucune.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Définir le point d’admission, la capture, la séquence, les refus, les événements
  de contrôle et la projection dans les fichiers.
- Acceptation : diagrammes de concurrence pour deux producteurs, filtrage,
  suspension et échec ; aucun trou de publication ne bloque le worker ; limitation
  explicite de la reconstruction après regroupement.

### RLOG-003 — Arrêter le modèle de ressources et de temps

- [x] **P0** — Dépendances : RLOG-002.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Examiner P01–P05 : budgets, réservations, groupes, métadonnées, contrôle et délais.
- Acceptation : tableau des propriétaires de mémoire et moments de libération ;
  limites des contextes, champs et spans prévues ; latence d’émission, LATENCY et
  visibilité distinguées ; valeurs retenues identifiées comme mesurées ou provisoires.

### RLOG-004 — Définir erreurs, cycle de vie et modèle de configuration

- [x] **P0** — Dépendances : RLOG-002, RLOG-003.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Définir états du moteur, résultats d’émission, portée de flush, shutdown, Drop,
  erreurs disque, identité d’instance et changements de fuseau.
- Acceptation : transitions et erreurs publiques documentées, politique sur panne
  de destination décidée, configuration invalide rejetée avant émission ; propositions
  restantes marquées explicitement dans le registre.

## E01 — Initialiser la crate et le modèle d’événement

**But :** rendre les contrats exécutables et indépendants des fichiers.
**Références :** D01–D05, D11 ; P07 ; Q01, Q06.
**Sortie d’epic :** crate compilable et objets internes bornables.

### RLOG-005 — Créer le squelette Cargo et les contrôles de base

- [x] **P1** — Dépendances : RLOG-001.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Initialiser bibliothèque, modules, exemples et contrôles format/lint/tests.
- Acceptation : build sur la version minimale choisie ; cœur sans Tokio obligatoire ;
  matrice de features prévue ; choix des dépendances documenté et limité au besoin.

### RLOG-006 — Implémenter les types d’événements et de champs

- [x] **P0** — Dépendances : RLOG-002, RLOG-003, RLOG-005.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Définir niveaux, séquences, source, message, champs typés et métadonnées réservées.
- Acceptation : données envoyables au worker et possédées ; égalité déterministe
  indépendante de l’ordre d’insertion des champs ; taille comptable ; comportement
  des nombres flottants, clés dupliquées et valeurs non finies défini.

### RLOG-007 — Introduire horloges et sorties de test contrôlables

- [x] **P0** — Dépendances : RLOG-005, RLOG-006.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Séparer capture monotone/civile, conversion locale et interface de sortie interne.
- Acceptation : tests capables d’avancer le temps, faire reculer l’heure civile,
  suspendre une sortie et injecter une erreur sans longues attentes réelles ; ces
  interfaces internes n’imposent pas une API de plugins publique.

### RLOG-008 — Valider la configuration et les erreurs typées

- [x] **P1** — Dépendances : RLOG-003, RLOG-004, RLOG-006.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Implémenter builder de configuration et erreurs de création/émission/arrêt.
- Acceptation : capacités nulles ou incohérentes refusées ; limites globales et
  options par instance distinguées ; valeurs par défaut centralisées et documentées.

## E02 — Construire l’admission sérialisée et bornée

**But :** assurer la propriété centrale sans I/O côté producteur.
**Références :** D02–D04, D14 ; P01 ; Q02–Q03.
**Sortie d’epic :** ordre et crédits validés sous concurrence, avec un sink mémoire.

### RLOG-009 — Prototyper et choisir la primitive d’admission

- [x] **P0** — Dépendances : RLOG-002, RLOG-005, RLOG-007.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Comparer un petit nombre de solutions de synchronisation et de file compatibles
  avec l’ordre requis ; mesurer la contention de leur section critique.
- Acceptation : choix justifié par invariants et mesures, sans revendiquer lock-free
  ou temps réel sans preuve ; aucune opération utilisateur coûteuse sous verrou.

### RLOG-010 — Implémenter les réservations et crédits mémoire

- [x] **P0** — Dépendances : RLOG-003, RLOG-006, RLOG-008, RLOG-009.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Borner événements et octets depuis la préparation réservée jusqu’à leur libération.
- Acceptation : crédits restitués après erreur, refus ou abandon ; transfert vers
  un groupe n’élargit pas le budget ; plusieurs producteurs ne dépassent pas les
  limites par allocations simultanées incontrôlées du logger.

### RLOG-011 — Publier atomiquement au sens du contrat d’ordre

- [x] **P0** — Dépendances : RLOG-009, RLOG-010.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Coordonner test d’ouverture, séquence, capture et insertion sans publication différée.
- Acceptation : un événement accepté apparaît exactement une fois dans le flux ;
  séquences strictement croissantes ; un producteur suspendu avant admission n’a
  pas de place réservée dans le flux ; panic/poisoning traité selon le contrat.

### RLOG-012 — Construire la boucle du worker

- [x] **P0** — Dépendances : RLOG-007, RLOG-011.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Consommer par lots bornés tout en servant les contrôles et échéances.
- Acceptation : repos sans boucle active inutile ; arrivée continue n’affame pas
  les échéances ; worker sans mutex partagé avec les producteurs pendant les I/O ;
  panne du thread détectable ; arrêt minimal disponible pour les tests.

### RLOG-013 — Vérifier les entrelacements d’admission

- [x] **P0** — Dépendances : RLOG-011, RLOG-012.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Tester plusieurs producteurs avec points de suspension pilotés ; envisager un
  outil de vérification de concurrence si des atomiques personnalisés le justifient.
- Acceptation : aucune inversion, duplication ou fuite de crédits après les
  scénarios capture/publication/refus ; résultat de l’admission réconcilié au sink.

## E03 — Gérer les instances et le contexte

**But :** isoler les usages sans multiplier les workers.
**Références :** D05–D07 ; P02, P07, P10 ; Q03, Q10.
**Sortie d’epic :** handles partageables et contexte prévisible sous concurrence.

### RLOG-014 — Créer le registre borné des instances

- [x] **P1** — Dépendances : RLOG-008, RLOG-011.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Implémenter identifiants, noms, configuration et handles clonables.
- Acceptation : clone = même instance ; nouvelle instance = identité distincte ;
  doublon de nom, limite et tentative après fermeture ont des résultats explicites ;
  aucune création d’un thread par instance.

### RLOG-015 — Implémenter l’héritage de contexte

- [x] **P1** — Dépendances : RLOG-006, RLOG-010, RLOG-014.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Ajouter contexte d’instance, enrichissement d’opération et champs d’événement.
- Acceptation : priorité des clés stable ; métadonnées internes protégées ; deux
  opérations concurrentes ne se modifient pas mutuellement ; instantané conservé
  après mutation des données métier ; contextes partagés bornés et comptabilisés.

### RLOG-016 — Capturer les métadonnées d’origine et d’exécution

- [x] **P1** — Dépendances : RLOG-015.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Prévoir module/crate, fichier/ligne et thread optionnel ; champs explicites pour
  worker, méthode, API et bibliothèque appelée.
- Acceptation : source réelle distincte de la cible ; capture par événement du
  thread courant ; aucune prétention à déduire la méthode ou la feature Cargo ;
  coût de capture optionnelle isolable dans les benchmarks.

### RLOG-017 — Définir et implémenter le routage borné

- [x] **P1** — Dépendances : RLOG-014, RLOG-015, RLOG-016.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Résoudre cible explicite, préfixe de module et repli par crate.
- Acceptation : préfixes comparés par segments ; priorité déterministe ; destination
  inconnue traitée explicitement ; champs libres sans création illimitée de fichiers ;
  limites globales et séparation des instances testées.

## E04 — Rendre les refus et la santé observables

**But :** rendre la capacité bornée exploitable sans perte silencieuse.
**Références :** D13 ; P04–P05 ; Q04–Q05.
**Sortie d’epic :** surcharge observable même lorsque la file applicative est pleine.

### RLOG-018 — Implémenter les résultats d’émission et compteurs

- [x] **P0** — Dépendances : RLOG-008, RLOG-011, RLOG-014.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Exposer accepté, filtré et refus par motif, avec agrégats globaux/par instance.
- Acceptation : surcharge, taille excessive, fermeture et échec distingués ;
  statistiques lisibles sans émettre de log ; définition claire de pending et des
  instantanés ; bilan cohérent une fois le moteur quiescent.

### RLOG-019 — Enregistrer les ruptures de continuité

- [x] **P0** — Dépendances : RLOG-002, RLOG-010, RLOG-018.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Concevoir un état borné permettant d’associer les refus à des frontières d’admission.
- Acceptation : fonctionne file pleine ; pas de récursion ; refus entre deux A
  identiques empêche leur regroupement trompeur ; plusieurs frontières ne créent
  pas de liste non bornée ; sur-découpage conservateur explicitement documenté si retenu.

### RLOG-020 — Émettre les résumés de surcharge

- [x] **P1** — Dépendances : RLOG-012, RLOG-019.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Produire des messages de contrôle séquencés, limiter leur fréquence et vider le solde.
- Acceptation : chaque refus compté une fois dans les résumés réussis ; instant du
  résumé distingué de la période des refus ; LATENCY présente ; aucun dépassement
  rétroactif de l’ordre ; retour à la normale sans nouvel événement métier traité.

### RLOG-021 — Exposer état du worker et erreurs de sortie

- [x] **P0** — Dépendances : RLOG-004, RLOG-012, RLOG-018.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Implémenter transitions de santé et bilan d’échec selon la politique décidée.
- Acceptation : une panne du worker ne ressemble pas à une file éternellement pleine ;
  événements acceptés non écrits comptabilisés ; erreurs partielles distinctes des
  succès ; une écriture possiblement partielle n’est pas rejouée aveuglément.

## E05 — Écrire les fichiers et gérer les rotations locales

**But :** obtenir les fichiers demandés sans collision ni ressources illimitées.
**Références :** D06–D09 ; P02, P06, P11 ; Q07.
**Sortie d’epic :** writer testé aux frontières civiles et aux limites du cache.

### RLOG-022 — Construire les chemins d’exécution et de destination

- [x] **P1** — Dépendances : RLOG-014, RLOG-017.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Créer run unique, instance, date et nom journalier/horaire à la demande.
- Acceptation : suffixes `00` à `23` ; noms invalides et traversées de chemin
  rejetés ; aucun écrasement d’un run existant ; collisions de noms normalisés
  empêchées, y compris sur système de fichiers insensible à la casse.

### RLOG-023 — Implémenter conversion locale et clés de rotation

- [x] **P0** — Dépendances : RLOG-004, RLOG-007, RLOG-022.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Déterminer date/heure locales de la capture et décalage applicable côté worker.
- Acceptation : minuit différé, heure sautée, heure répétée et recul d’horloge
  testés avec règles contrôlées ; aucune dépendance au fuseau de la machine de CI ;
  comportement d’un changement de fuseau en cours de run documenté.

### RLOG-024 — Implémenter le cache borné de writers

- [x] **P1** — Dépendances : RLOG-021, RLOG-022, RLOG-023.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Ouvrir en ajout, bufferiser, vider avant éviction et rouvrir les fichiers nécessaires.
- Acceptation : nombre de descripteurs borné ; anciens jours/heures accessibles ;
  erreur d’ouverture ou d’éviction remontée ; données non perdues lors d’un changement
  de fichier ; budgets respectés pendant remplacement d’un writer.

### RLOG-025 — Stabiliser et implémenter le format texte

- [x] **P1** — Dépendances : RLOG-006, RLOG-016, RLOG-023.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Définir date/heure, niveau, LATENCY, séquence, contexte et message ; ordre stable des champs.
- Acceptation : offset local lisible, précision et arrondi documentés ; sauts de
  ligne et caractères de contrôle échappés pour préserver une occurrence par ligne ;
  format des groupes et séquences non contiguës explicitement prévu.

### RLOG-026 — Relier le worker au writer avec injection de pannes

- [x] **P0** — Dépendances : RLOG-012, RLOG-021, RLOG-024, RLOG-025.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Brancher les fichiers au pipeline ordonné et au suivi des erreurs.
- Acceptation : plusieurs instances/destinations sans mélange ; erreurs write,
  flush et création testées ; progression ou arrêt conforme au contrat ; aucune
  opération disque déplacée dans la section d’admission.

## E06 — Regrouper les répétitions et borner la rétention

**But :** produire des logs compacts sans masquer les transitions ou bloquer la visibilité.
**Références :** D10–D12, D14 ; P03.
**Sortie d’epic :** groupes, annotations et flushs corrects sous flux continu.

### RLOG-027 — Implémenter les groupes consécutifs par destination

- [x] **P0** — Dépendances : RLOG-006, RLOG-010, RLOG-017, RLOG-019, RLOG-025.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Grouper après routage selon l’égalité du message et de son contexte effectif.
- Acceptation : `A A B A A`, niveaux, sources, call_id et workers différents testés ;
  comparaison réelle en complément d’un éventuel hash ; une seule ligne par groupe ;
  budget des groupes borné même après vidage de la file.

### RLOG-028 — Gérer les échéances et coupures de groupes

- [x] **P0** — Dépendances : RLOG-023, RLOG-027.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Couper sur durée maximale, rotation, décalage local et rupture de continuité.
- Acceptation : série continue divisée sans recompter les occurrences ; groupe
  isolé émis sans attendre un nouveau message ; un groupe n’englobe pas plusieurs
  fichiers ; les groupes restent liés à la destination même si son writer est évincé.

### RLOG-029 — Calculer LATENCY et les bornes des groupes

- [x] **P0** — Dépendances : RLOG-007, RLOG-025, RLOG-027, RLOG-028.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Calculer la durée monotone au moment de préparer chaque ligne.
- Acceptation : annotation systématique pour événements simples et contrôles ;
  maximum de groupe exact, incluant sa rétention ; recul de l’heure civile sans
  durée négative ; bornes temporelles et séquences sans prétendre à une plage contiguë.

### RLOG-030 — Planifier les flushs sans affamer le worker

- [x] **P0** — Dépendances : RLOG-012, RLOG-024, RLOG-028, RLOG-029.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Servir l’échéance de flush des fichiers modifiés en complément des lots d’événements.
- Acceptation : intervalles mesurés avec horloge contrôlée, repos sans flushs inutiles,
  échéances servies sous débit continu ; test du budget nominal groupe + flush ;
  mesures séparées du délai réel quand la sortie ralentit.

## E07 — Finaliser le cycle de vie et l’arrêt propre

**But :** donner à l’application un arrêt déterministe sur les données acceptées.
**Références :** D03, D13 ; P09, P12 ; Q08.
**Sortie d’epic :** barrières et bilan vérifiés, y compris en concurrence et en erreur.

### RLOG-031 — Implémenter la barrière flush

- [x] **P0** — Dépendances : RLOG-004, RLOG-011, RLOG-020, RLOG-030.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Ordonnancer une barrière de contrôle et accuser réception après traitement/flush.
- Acceptation : tous les événements admis avant la barrière couverts ; groupes
  terminés ; producteurs ultérieurs autorisés ; file pleine ne bloque pas l’accès
  au contrôle indéfiniment par défaut de place réservée ; erreurs transmises.

### RLOG-032 — Fermer l’admission et drainer le moteur

- [x] **P0** — Dépendances : RLOG-031.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Implémenter shutdown avec fermeture coordonnée, drainage, soldes et join.
- Acceptation : émission concurrente soit acceptée et drainée soit refusée fermée ;
  aucune attente sur un producteur n’ayant pas encore été admis ; handles survivants
  sans panic ; dernier groupe et dernier résumé traités avant fin.

### RLOG-033 — Produire un bilan d’arrêt vérifiable

- [x] **P1** — Dépendances : RLOG-018, RLOG-021, RLOG-032.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Retourner compteurs, erreurs et données non confirmées écrites selon le contrat.
- Acceptation : distinguer occurrences et lignes/groupes ; réconciliation des
  acceptés après arrêt sain ; bilan partiel disponible après erreur ; limite des
  refus survenant après retour de shutdown documentée.

### RLOG-034 — Définir Drop et la fermeture des instances

- [x] **P1** — Dépendances : RLOG-004, RLOG-014, RLOG-032, RLOG-033.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Implémenter les politiques choisies pour abandon du runtime, derniers handles,
  double fermeture et éventuelle fermeture individuelle d’instance.
- Acceptation : destruction d’un clone n’arrête pas le moteur ; pas de join sur
  soi-même ; durée/blocage de Drop documentés ; pas de promesse de récupérer une
  erreur de flush via Drop ; comptes de ressources libérés au moment prévu.

## E08 — Livrer l’API native et les exemples d’intégration

**But :** intégrer la bibliothèque sans connaissance de son worker.
**Références :** D01, D05–D06 ; P07.
**Sortie d’epic :** exemples exécutables illustrant l’usage réel.

### RLOG-035 — Exposer Runtime, Logger et le builder public

- [x] **P1** — Dépendances : RLOG-008, RLOG-014, RLOG-015, RLOG-033, RLOG-034.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Stabiliser signatures, erreurs, configuration globale et options par instance.
- Acceptation : aucun singleton imposé ; types internes non exposés inutilement ;
  résultats explicites d’émission ; documentation des garanties Send/Sync et des
  opérations bloquantes ; API conceptuelle de la documentation remplacée par l’API réelle.

### RLOG-036 — Ajouter macros, niveaux et filtrage précoce

- [x] **P1** — Dépendances : RLOG-016, RLOG-035.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Fournir macros de niveaux, messages statiques, champs structurés et texte formaté.
- Acceptation : argument coûteux non évalué si filtré ; usage depuis une crate
  externe validé ; source correcte ; taille des formats bornée ; coût du formatage
  côté producteur documenté ; retour des macros conforme au choix public.

### RLOG-037 — Écrire l’exemple multithread multi-instance

- [x] **P1** — Dépendances : RLOG-035, RLOG-036.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Démontrer clones, contextes, début/fin d’actions, rotations différentes et shutdown.
- Acceptation : exemple exécutable sans réseau ni modèle installé ; fichiers
  illustratifs reproductibles ; explication du numéro de séquence et des refus.

### RLOG-038 — Écrire l’exemple async de workflow agentique

- [x] **P1** — Dépendances : RLOG-035, RLOG-036.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Simuler agents et appels IA concurrents avec workflow_id, agent_id, call_id et parent.
- Acceptation : début/premier token/fin/erreur/annulation illustrés ; contexte conservé
  entre tâches ; shutdown déporté hors des workers async ; runtime async uniquement
  dépendance d’exemple/dev ; distinction durée d’appel et LATENCY expliquée.

## E09 — Ajouter l’adaptateur tracing optionnel

**But :** intégrer le moteur dans un backend déjà instrumenté.
**Références :** D05–D06 ; P08, P10 ; Q03.
**Sortie d’epic :** Layer composable sans dépendances tracing imposées au cœur.

### RLOG-039 — Créer la feature et la Layer composable

- [x] **P1** — Dépendances : RLOG-035, RLOG-036.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Ajouter feature optionnelle et construction d’une Layer adossée au runtime.
- Acceptation : build sans feature et avec feature ; aucun subscriber global installé
  implicitement ; coexistence avec une autre Layer ; capture située à l’admission commune.

### RLOG-040 — Capturer les événements et router les instances

- [x] **P1** — Dépendances : RLOG-017, RLOG-018, RLOG-039.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Visiter champs typés et messages, conserver source réelle, mapper cibles/modules.
- Acceptation : instance par défaut et sélection explicite testées ; instance inconnue
  gérée sans création incontrôlée ; champs Debug/Display bornés ; refus du collecteur
  visibles dans les compteurs ; aucun log récursif du moteur via sa propre Layer.

### RLOG-041 — Propager les spans avec des ressources bornées

- [x] **P0** — Dépendances : RLOG-003, RLOG-015, RLOG-039, RLOG-040.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Capturer création, champs enregistrés, parents et portée courante des spans.
- Acceptation : mise à jour des champs, spans explicites et profondeur limite testés ;
  stockage libéré à fermeture ; absence de fuite entre tâches async ; exemples utilisant
  l’instrumentation des futures, sans guard d’entrée gardé à travers un await.

### RLOG-042 — Valider la parité native/tracing

- [x] **P1** — Dépendances : RLOG-038, RLOG-040, RLOG-041.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Produire un scénario mêlant événements natifs et tracing sur le même moteur.
- Acceptation : ordre commun, routage et contexte corrects, shutdown partagé ;
  différence de coût et de point de collecte documentée ; exemple composable compilé.

## E10 — Valider la correction et mesurer les performances

**But :** prouver les garanties et caractériser les coûts sur une version identifiable.
**Références :** tous les contrats, P01–P03 ; Q09–Q10.
**Sortie d’epic :** rapport reproductible de correction, performance et limites.

### RLOG-043 — Tester les scénarios de bout en bout

- [x] **P0** — Dépendances : RLOG-013, RLOG-026, RLOG-030, RLOG-032, RLOG-042.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Combiner threads/tâches, instances, séries A A B A A, rotations et arrêt.
- Acceptation : chaque événement accepté réconcilié par occurrence ; ordre par
  destination préservé ; séquences des groupes interprétées correctement ; aucune
  dépendance fragile à des sleeps pour provoquer une course.

### RLOG-044 — Tester saturation prolongée et pannes

- [x] **P0** — Dépendances : RLOG-019, RLOG-020, RLOG-021, RLOG-033, RLOG-043.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Injecter sortie lente/bloquée, erreurs de création/write/flush, grosse charge et refus.
- Acceptation : mémoire logger stable dans les bornes prévues ; pas d’interblocage
  au rétablissement ; ruptures et résumés corrects ; panne distincte de saturation ;
  limites d’un shutdown face à une I/O bloquée explicitement observées.

### RLOG-045 — Mesurer le coût de l’émission

- [x] **P1** — Dépendances : RLOG-036, RLOG-042.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Mesurer appels filtrés, messages statiques, champs, formats complexes, contexte et
  tracing avec 1, 2, 4, 8 producteurs puis selon le matériel disponible.
- Acceptation : p50/p99/p99,9, allocations et taux de refus publiés séparément pour
  acceptés/refusés ; contention d’admission incluse ; matériel, OS, build, charge
  et limites statistiques documentés ; aucune promesse issue du seul débit moyen.

### RLOG-046 — Mesurer débit, mémoire et visibilité

- [x] **P1** — Dépendances : RLOG-030, RLOG-044, RLOG-045.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Mesurer débit soutenable, pending, RSS et mémoire comptabilisée, LATENCY et délai
  capture → visibilité par un lecteur externe ; comparer les réglages proposés.
- Acceptation : charge faible, nominale et saturée distinguées ; tailles de messages
  et nombre de destinations variés ; pas de confusion flush/persistance ; impact
  d’une instance bruyante mesuré ; valeurs P01–P03 confirmées ou révisées avec preuves.

### RLOG-047 — Vérifier la portabilité et la matrice de features

- [x] **P1** — Dépendances : RLOG-023, RLOG-042, RLOG-043.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Tester les OS officiellement ciblés, la MSRV et les configurations de features.
- Acceptation : rotation locale, noms de fichiers et shutdown couverts sur les
  plateformes annoncées ; exemples et doctests compilés ; versions/dépendances
  compatibles ; restrictions de plateforme documentées plutôt que supposées absentes.

## E11 — Préparer la livraison et la documentation utilisateur

**But :** livrer une première version compréhensible et intégrable.
**Références :** D01, D15 ; ensemble du registre des décisions.
**Sortie d’epic :** package local vérifié, documentation alignée et limitations explicites.

### RLOG-048 — Rédiger le guide utilisateur final

- [x] **P1** — Dépendances : RLOG-037, RLOG-038, RLOG-042, RLOG-046.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Documenter installation, démarrage natif/tracing, contexte, routage, rotations,
  surcharge, erreurs, flush et shutdown avec l’API réelle.
- Acceptation : exemples compilables ; pas de noms provisoires présentés comme
  disponibles ; contrat chronologique et limites des groupes expliqués ; valeurs
  par défaut conformes au code ; lien vers le rapport de mesures.

### RLOG-049 — Préparer le package et la vérification de distribution

- [x] **P1** — Dépendances : RLOG-047, RLOG-048.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Compléter métadonnées Cargo, licence, README, changelog, exclusions et contenu du package.
- Acceptation : package construit et vérifié localement avec features appropriées ;
  aucun log généré/personnel ni résultat de test embarqué (fixture contractuelle synthétique et sources de test conservées) ; instructions d’intégration
  depuis une application externe vérifiées.

### RLOG-050 — Réaliser la revue de sortie

- [x] **P0** — Dépendances : RLOG-044, RLOG-046, RLOG-049.
- Preuve : [réalisation et validation du ticket](../../docs/VALIDATION.md#rust).
- Relire contrats, API, résultats, risques et propositions encore ouvertes.
- Acceptation : aucune question P0 bloquante ; écarts explicitement arbitrés ;
  roadmap et registre synchronisés ; liste des plateformes supportées et limites
  connue ; résultat « prêt à distribuer » distinct d’une publication externe.

La publication effective vers un registre n’est pas réalisée par la création de
cette roadmap. Elle fera l’objet de l’action de livraison correspondante lorsque
le package sera prêt et que sa destination sera définie.

## E12 — Backlog différé

Ces tickets ne conditionnent pas M5. Ils nécessitent un cadrage avant estimation.

### RLOG-051 — Sortie brute sans regroupement et export JSON

- [ ] **P2** — Dépendances : RLOG-050.
- Étudier une sortie globale brute pour reconstruction exacte et un schéma JSON versionné.
- Acceptation : coût disque/mémoire mesuré ; toutes les occurrences et séquences
  disponibles en brut ; aucune prétention de reconstruction exacte depuis les seuls groupes.

### RLOG-052 — Rétention et compression des fichiers

- [ ] **P2** — Dépendances : RLOG-050.
- Définir politique par âge/taille et compression des sorties closes.
- Acceptation : fichiers actifs et heures revisitées protégés ; action de suppression
  explicite et testée ; aucune I/O supplémentaire côté producteur.

### RLOG-053 — Fuseau configurable et durabilité optionnelle

- [ ] **P2** — Dépendances : RLOG-050.
- Cadrer séparément fuseaux arbitraires et synchronisation vers le stockage.
- Acceptation : frontières de rotation déterministes ; coût et garanties de sync
  documentés ; compatibilité des chemins et du format existants examinée.

### RLOG-054 — Arrêt avec timeout

- [ ] **P2** — Dépendances : RLOG-050.
- Définir la propriété du worker et des données si le délai d’attente expire.
- Acceptation : aucun arrêt forcé dangereux de thread ; timeout distinct du succès ;
  données non confirmées signalées ; poursuite/abandon et ressources résiduelles documentés.

### RLOG-055 — Équité et évolution du nombre de workers

- [ ] **P2** — Dépendances : RLOG-046, RLOG-050.
- Selon les mesures, étudier quotas par instance puis intérêt de workers supplémentaires.
- Acceptation : besoin démontré ; contrat d’ordre préservé ou redéfini explicitement ;
  gains mesurés face au worker commun ; aucun changement silencieux de causalité promise.

## E13 — Préparer le nom de package public

**But :** aligner le nom public RLOGGER, le package Cargo et les guides avant toute
publication vers un registre.
**Références :** D23 et D24 dans `docs/05-decisions.md`.

### RLOG-056 — Renommer le package et la crate en `rlogger`

- [ ] **P0** — Dépendances : RLOG-050.
- Remplacer `ordered-local-logger` et `ordered_local_logger` par `rlogger` dans le
  manifeste, les imports, les exemples, les consommateurs externes, les locks et
  la documentation de distribution.
- Acceptation : build, tests, clippy, package et consommateurs externes passent
  avec `rlogger`; aucun guide ne recommande l’ancien nom sans le marquer comme
  historique ; changelog et compatibilité expliquent la rupture éventuelle pour
  les intégrateurs locaux ; le package généré porte le nom `rlogger`.

## Conditions de clôture d’un ticket

### Extension stockage horaire — logger 0.2.0

Le [prompt approuvé](../../docs/10-prompt-gestion-fichiers-horaires.md) remplace
les choix historiques quotidiens/réouverture des tickets RLOG-022–024.
RLOG-056 (renommage Cargo) demeure hors périmètre de cette extension.

- [x] **RLOG-057** — Arbre `rlogger/jour/instance`, API `log_root()`/`run_id()`,
  rotation horaire obligatoire, captures tardives et segments exclusifs.
  Preuve : [tests rotation/concurrence](../../docs/VALIDATION-HORAIRE.md).
- [x] **RLOG-058** — Finalisation au repos/LRU/arrêt, marqueur/bail de run,
  coordination courte, absence de rejeu après panne et compatibilité consommateur.
  Preuve : [tests cycle de vie/package](../../docs/VALIDATION-HORAIRE.md).

### Règles communes

1. Critères d’acceptation satisfaits et validation attachée au ticket.
2. Tests pertinents ajoutés ou adaptés pour les invariants modifiés.
3. Documentation et registre actualisés si le comportement public change.
4. Ressources, erreur et fermeture examinées pour les nouveaux états conservés.
5. Aucune garantie de performance non mesurée ajoutée à la documentation.

## Conditions de livraison M5

- RLOG-001 à RLOG-050 terminés ; RLOG-051 à RLOG-055 peuvent rester au backlog.
- Scénarios concurrence/saturation/rotation/arrêt validés et résultats reproductibles.
- API native et feature tracing intégrables depuis une application externe.
- Paramètres par défaut justifiés par mesures et limitations publiées.
- Documentation métier, documentation API et comportement effectif cohérents.
