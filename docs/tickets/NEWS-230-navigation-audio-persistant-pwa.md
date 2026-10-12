# NEWS-230 — Cadre connecté, navigation responsive et audio persistant

## Statut

**Terminé localement — 17 septembre 2026.** gluestack v5 / UniWind / Tailwind CSS 4.

EPIC : [EPIC-039](../epics/EPIC-039-refonte-pwa-gluestack.md).
Référence : [charte PWA et inventaire des routes](../design/pwa/README.md).

## Objectif métier

Passer d’une page à l’autre pendant une session de trading sans perdre la lecture audio ni les repères de navigation.

## Périmètre

- Refondre le cadre connecté, la navigation et la présentation du lecteur commun.
- Sidebar dès 1024 px ; navigation inférieure et menu complémentaire en dessous, avec destinations autorisées.
- Liste/détail dès 1280 px et panneau accessible sous ce seuil ; conserver la redirection / vers /feed et les routes existantes.

## Hors périmètre

Nouveau moteur audio, second abonnement, autoplay, nouveau contrat HLS/WebSocket, changement d’authentification, nouveau tableau de bord /session.

## Approche technique

- Conserver l’orchestration application/ et le montage du runtime dans le layout protégé ; fournir états et callbacks existants à AudioPlayerView.
- Ne pas placer le moteur sous une clé de route ou de thème. Ne pas dupliquer un lecteur actif pour les variantes desktop/mobile.
- Centraliser les destinations ; filtrer la navigation admin par rôle sans remplacer les protections d’accès direct.
- Associer nom, route active et focus aux commandes. Prévoir le menu admin complet et les zones sûres de navigation inférieure.
- Préserver les comportements du direct et du résumé existants, afficher leurs états réels et laisser le lecteur absent de toutes les vues auth.

## Critères d’acceptation

- Navigation /feed, /calendar, /profile et routes admin autorisées sans redémarrage ni double abonnement audio.
- Seuils 1024/1280 conformes, retour des panneaux et focus cohérents, aucun contenu masqué par la barre inférieure.
- Compte non admin sans accès admin dans le menu ou par URL ; session absente prise en charge par la garde existante.
- Le thème change sans interrompre le direct ou le résumé ; aucun démarrage audio automatique nouveau.

## Tests attendus

- Instrumenter le nombre de montages/abonnements et l’état de lecture pendant navigation et bascules répétées, direct et résumé, puis déconnexion/reconnexion.
- Vérifier 320, 390, 768, 1023/1024, 1279/1280 et 1440 px, navigation clavier et liens directs.
- Tester sans session, utilisateur et admin ; exécuter check:pwa dont shell, audio-legacy et audio-idle-polling, puis build:pwa.

## Dépendances

- [NEWS-229](./NEWS-229-composants-reutilisables-pwa.md).

Les commentaires du code livré restent en français. Les preuves de recette locale restent distinctes de tout déploiement.

## Livraison et preuves

Sidebar et navigation mobile livrées ; lecteur unique persistant, direct et résumé HLS testés pendant navigation et bascule.

[Recette locale, commandes, résultats et captures](../design/pwa/recette/2026-09-16/README.md).
Aucun commit, push, déploiement ou livraison native.
