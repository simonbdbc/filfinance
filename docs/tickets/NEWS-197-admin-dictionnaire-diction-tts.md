# NEWS-197 - Admin Dictionnaire & diction TTS

## Objectif

Faire evoluer la page admin glossaire en outil unique `Dictionnaire & diction TTS`.

L'interface doit rester dense et operationnelle: formulaire, aperçu, registre compact et regles systeme non editables.

## Scope frontend

- Renommer la surface admin en `Dictionnaire & diction TTS`.
- Ajouter le champ `Portee`.
- Ajouter l'aperçu textuel:
  - texte de test;
  - depart `FR affiche` ou `EN source`;
  - resultat `FR affiche`;
  - resultat `FR vocal`;
  - texte final envoye a Piper.
- Afficher les regles systeme TTS en lecture seule.
- Rendre le registre plus compact:
  - option `density="compact"` sur `AdminDataGrid`;
  - colonne `Regle` fusionnant source, FR affiche, vocal, notes et date;
  - colonne `Portee` pour mode, priorite et casse;
  - boutons locaux courts `Modifier` et `Off`;
  - suppression de la colonne `Notes` dediee.

## UX attendue

- Le tableau reste scannable sur desktop.
- Les actions ne debordent pas sur mobile.
- La difference `Pipeline complet` / `Vocal seulement` est visible dans le formulaire et le registre.
- Les regles systeme sont presentees comme reference, pas comme elements editables.
- L'aperçu audio reel est explicitement absent de cette v1.

## Critere d'acceptation

- Un admin peut creer une entree `full_pipeline`.
- Un admin peut creer une entree `spoken_only`.
- Un admin peut lancer un aperçu textuel sans sauvegarder.
- Un admin voit les familles de regles systeme TTS.
- Le registre n'a plus de colonne `Notes` separee.
- `npm run typecheck` et `npm run build:web` passent.

## Hors scope

- Import/export CSV.
- Bulk edit.
- Lecteur audio de preview.
- Refonte globale des autres pages admin.
