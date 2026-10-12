# NEWS-146 - Stabilisation theme clair sombre actuel

## Objectif

Fiabiliser le theme actuel sans changer la palette claire/sombre.

## Scope

- Extraire la source de verite des themes dans des modules dedies.
- Conserver les couleurs actuelles `light` et `dark`.
- Appliquer le theme local immediatement.
- Laisser la preference profil gagner quand elle est disponible.
- Ajouter un minimum de CSS web root pour eviter les surfaces non themees.

## Criteres d'acceptation

- Aucun nouveau theme n'est introduit.
- Les surfaces principales lisent les tokens.
- Le theme ne reste pas bloque dans un etat different de la preference effective.
- Un chargement local tardif ne peut pas ecraser une preference profil deja appliquee.
- Les tests de type et build web restent verts.
