# Préparation de sécurité et contenu public

La préparation locale ne qualifie pas la production. Les fixtures calendrier et
benchmark, modèles TTS, captures et conditions du code doivent être revus avant
export ; un fichier sans secret n'a pas automatiquement un droit de redistribution.
Profils Chromium, cookies, données utilisateurs, bases, logs et configurations
privées sont exclus. Un futur historique public doit être neuf et contrôlé sur
tous ses objets ; les refs privées ne sont pas un candidat public.

## Signature des tickets HLS : vérification puis renouvellement autorisé

Une valeur présumée secrète de NEWS_HLS_ACCESS_TICKET_SECRET a été trouvée dans
l'ancien .env.server.example, introduite le 12 mai2026 et retirée le 19 juin2026.
Elle est absente de l'arbre actuel. Pour le dépôt de présentation public,
le champ est laissé vide dans les trois modèles d’environnement. Le contrôle
porte sur l’absence de valeur secrète dans tout le nouvel historique public.
Son éventuel usage sur une instance et sa rotation relèvent d’un suivi
opérationnel séparé ; vider les modèles ne révoque pas une clé déployée. Ne pas recopier cette
valeur ni l'utiliser pour essayer un endpoint.

Consommateurs : configuration config/hls.rs au démarrage du backend Rust,
signature/vérification features/audio/hls/access.rs, endpoint de ticket et
playlists/segments audio. Les clients reçoivent un ticket HS256, jamais la clé.
Un bucket de durée T produit une expiration jusqu'à deuxT après son début,
avec la tolérance de validation de la bibliothèque ; ne pas supposer un TTL
strictement égal au paramètre. Caches origin/CDN et lecteurs déjà ouverts doivent
être pris en compte.

1. Identifier les instances réellement actives, leur ref déployée et le canal
   administratif privé. Vérifier par ce canal si l'ancienne valeur était fictive
   ou utilisée, sans la tester comme credential ni transmettre des secrets au chat.
2. Relever flags HLS/contrôle d'accès, TTL et caches ; sauvegarder configuration
   et ref dans le stockage privé, tester restauration isolée. Recenser toutes les
   instances Rust qui signent ou vérifient les mêmes flux.
3. Préparer une nouvelle clé aléatoire forte dans le gestionnaire privé, et les
   modifications des seuls consommateurs concernés. Prévoir une courte coupure
   audio et le renouvellement des tickets ; ne pas désactiver le contrôle d'accès.
4. Après autorisation explicite, appliquer la clé de manière coordonnée puis
   recréer/redémarrer les processus concernés pour recharger la configuration.
   Traiter les caches de playlists/tickets selon la topologie relevée ; aucune
   commande générique de purge ou de déploiement n'est lancée par ce document.
5. Vérifier santé, acquisition d'un nouveau ticket par compte autorisé et lecture
   des playlists/segments. Vérifier les refus manquant/malformé/expiré avec des
   cas synthétiques. Comparaison de configuration par administration privée,
   sans essai de l'ancienne clé exposée. Consigner uniquement références/SHA et
   résultats expurgés.
6. En cas d'échec, garder les routes audio concernées arrêtées/restrictives,
   restaurer une configuration saine avec une clé nouvelle puis diagnostiquer.
   Ne jamais réintroduire une clé compromise ni ouvrir l'accès sans contrôle.

Recette isolée : cargo test features::audio::hls::access vérifie aussi qu'un ticket
signé par une première clé inventée est refusé après changement et qu'un nouveau
ticket légitime est accepté. Ce test ne prouve aucun renouvellement de production.

## Collector et fichiers temporaires

Le [runbook canonique](../financialjuice-browser-collector/README.md#deblocage-challenge-via-nomachine)
crée un dossier temporaire700 et fichiers600 avant extraction de l'environnement
Docker ; un trap retire les copies privées. Une Xauthority privée authentifie la
session X11 au lieu d'ouvrir l'accès à tous les utilisateurs locaux. Droits/UID,
image, chemins et autorisation d'arrêt/reprise doivent être vérifiés avant usage.
Cette documentation n'autorise aucune ingestion ni redistribution amont.
