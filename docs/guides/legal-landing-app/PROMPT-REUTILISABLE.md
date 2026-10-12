# Prompt de reproduction

Copier le dossier entier dans le nouveau projet. Le fichier `MANIFESTE.md` contient l’adaptation Fil Finance et `MANIFESTE-ACREA.md` conserve l’exemple original. Pour un autre service, remplir une copie de `MANIFESTE-MODELE.md` avec ses propres données. Donner ensuite le prompt suivant à l'agent de code. Les informations manquantes ne doivent pas empêcher la préparation locale des fichiers ; elles doivent empêcher une publication contenant des valeurs provisoires.

```text
Mets en place les pages légales du couple landing + app de ce dépôt en utilisant
le dossier documentaire joint : README.md, MANIFESTE.md et IMPLEMENTATION.md.
Le manifeste rempli décrit le nouveau service ; les références à ACREA expliquent
l'origine du modèle et ne doivent pas être copiées comme faits du nouveau projet.

Commence par lire les instructions du dépôt et examiner les frameworks, les routes,
les layouts, le footer, la configuration publique, l'inscription et la CI existants.
Préserve les modifications locales présentes. Adapte l'implémentation à cette
architecture sans migration de framework, d'authentification ou de backend.

Livrables :
1. Quatre routes publiques sur la landing : mentions légales, confidentialité,
   conditions générales d'utilisation et cookies, selon les chemins du manifeste.
2. Un layout commun lisible et responsive : langue, titre et description propres,
   canonical, H1 unique, sections structurées, date/version, accès clavier,
   retour accueil et liens vers tous les documents.
3. Une configuration centralisée de l'éditeur, de l'hébergeur et des URL, avec
   des champs supplémentaires selon le statut et les traitements décrits.
4. Des contenus adaptés au service réel. Utilise les informations approuvées.
   Marque explicitement les éléments inconnus en développement ; ne les invente pas.
5. Les liens absolus vers la landing avant envoi du formulaire d'inscription,
   avec une notice courte, et un accès permanent aux quatre documents dans l'app.
   Prévois le comportement invité, connecté, mobile et hors connexion du manifeste.
6. Le sitemap et les règles de routage nécessaires pour servir de véritables pages
   publiques. Vérifie qu'aucune route ne renvoie silencieusement l'accueil de la SPA.
7. Une validation du HTML généré et un blocage de publication pour les informations
   manquantes, placeholders, destinations incorrectes ou pages absentes.
8. Les contrôles du mécanisme réel d'analytics/traceurs selon le manifeste.
   N'active pas de collecte facultative sans le dispositif prévu et vérifié.
9. Une documentation locale des fichiers modifiés, variables par environnement,
   commandes, procédure de mise à jour et résultats de recette.

Contraintes :
- Aucun nom, email, domaine, hébergeur, délai ou texte métier ACREA résiduel.
- Aucun secret dans les variables publiques ou les URL.
- Aucun partage de session ni CORS supplémentaire pour de simples liens légaux.
- Aucun import obligatoire du code de l'app depuis la landing ou inversement.
- Distingue information sur les données, acceptation contractuelle et consentement
  aux traitements facultatifs. Une mention de lecture ne prouve pas l'acceptation.
- Si une acceptation versionnée est demandée, vérifie le parcours et la preuve
  côté serveur ; ne simule pas cette preuve avec un booléen du frontend.
- Si cette extension requiert un changement architectural absent du périmètre
  approuvé, documente les fichiers et changements requis avant de le proposer.
- Adapte les documents supplémentaires, notamment CGV, seulement au besoin décrit.

Exécute les contrôles adaptés aux fichiers changés : build, validation HTML,
vérification des liens, contrôle négatif des données manquantes et placeholders,
puis recette navigateur si disponible. Distingue résultats locaux et distants.
N'annonce pas une conformité juridique complète sur la seule base de tests.

La demande porte sur la préparation et l'intégration dans le dépôt. Ne déploie pas
et ne modifie pas les réglages de services externes sans instruction à cet effet.
Termine avec les fichiers livrés, les commandes réellement exécutées et les points
de contenu ou de validation restant ouverts. N'attribue pas à ACREA des fonctions
que le guide identifie comme de simples extensions proposées.
```
