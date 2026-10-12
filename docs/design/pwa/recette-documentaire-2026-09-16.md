# NEWS-227 — Vérification documentaire du 16 septembre 2026

Périmètre : [EPIC-039](../../epics/EPIC-039-refonte-pwa-gluestack.md),
[référentiel PWA](./README.md), onze tickets, galerie et liens README/roadmap.
**Documentation locale uniquement.** NEWS-227 est terminé ; l'EPIC et les dix
tickets d'implémentation restent planifiés. Aucun test d'application, build,
commit, push, déploiement ou test iOS/Android n'est revendiqué par cette recette.

## Contrôles documentaires

- Un seul fichier EPIC-039, onze fichiers NEWS-227 à NEWS-237 avec identifiants
  uniques, sections du contrat et dépendances attendues.
- Liens relatifs des nouveaux documents et liens ajoutés au README/roadmap résolus.
- Cinq images incluses dans la galerie ; cinq références actives et deux originaux
  historiques. Aucune alternative non retenue ni ancienne référence gamma.
- Inventaire comparé à `apps/pwa/app/**/*.tsx` : seize routes, trois layouts,
  chacun rattaché à un ticket. Les modes auth ne sont pas inventés comme routes.
- Palettes identiques dans l'EPIC et le référentiel, sept rôles par thème ; familles,
  dimensions, états et règles de contraste documentés.
- `git diff --check` sans erreur. Périmètre de travail limité aux documents et PNG.

## Provenance et intégrité des PNG

Le [manifeste](./maquettes/manifest.json) contient les noms sources, répertoire
local ImageGen, dates, dimensions, octets et empreintes complètes. Les sept copies
ont été comparées octet par octet avec leurs sources respectives, puis vérifiées
par SHA-256 et en-tête PNG. Elles mesurent toutes **1644 × 957 px**.

| Fichier | Octets | SHA-256 |
| --- | ---: | --- |
| `historique/pwa-table-de-session-v1.png` | 1402870 | `61854b9f9f10286b7f9d774d264b3b861d6066bb8f774bec6e8f63fb5f3738bb` |
| `historique/pwa-calendrier-v1.png` | 1320571 | `000b477c97c0202bd3c6b74023be794b39ad126d66f0ba8ab1e348345fdf3b91` |
| `pwa-actualites-v1.png` | 1361689 | `5c76d81de9a113fbe58a4e0620cd5353ae3ad62bbde4575fb1743f42dc8a0d9e` |
| `pwa-profil-v1.png` | 1287244 | `0fe977515ce3735256c925b63e22566dbba8f8d76ed195f09a7964138f714a87` |
| `pwa-administration-v1.png` | 1364303 | `6f3fc14d02782078397c796947f9a4451718ea95a6d4d62e34f38b77999993d8` |
| `pwa-table-de-session-v2.png` | 1468604 | `a768e0108fc0f552d7047585f8efd1abf06953ba24afab8df4b3ebde52941dfa` |
| `pwa-calendrier-v2.png` | 1461919 | `655c41b7c867036ed8f5eefb2ad419da6500e6265ddc34dfeba5efb887a0693a` |

Les cinq fichiers v1 sont les originaux du 15 septembre, copiés à l'identique.
Les deux fichiers v2 sont des éditions ImageGen du 16 septembre, elles aussi
copiées à l'identique de leurs nouveaux fichiers générés. Aucun réencodage local.

Confirmation utilisateur de la référence Actualités : le fichier fourni
`/Users/simon/Downloads/Image Codex 16 sept. 2026, 11_01_10.png` est strictement
identique à `pwa-actualites-v1.png` (1 361 689 octets, 1644 × 957 px, SHA-256
`5c76d81de9a113fbe58a4e0620cd5353ae3ad62bbde4575fb1743f42dc8a0d9e`).
Aucune modification du PNG : cette page est spécifiée sans calendrier ; seule
l'entrée de navigation mène au calendrier séparé. L'EPIC, le référentiel et
NEWS-231 portent cette précision.

Contrôle visuel des cinq planches actives : vues ordinateur claire et mobile
sombre présentes, composition conservée. Sur les corrections, trois sélecteurs
vérifiés : direction ordinateur, Calendrier ordinateur et Calendrier mobile.
Chacun montre **Lun 14, Mar 15, Mer 16, Jeu 17, Ven 18**, avec mardi sélectionné.
Les plages hebdomadaires visibles indiquent le **14–18 septembre 2026** ; les
jours et dates sont également vérifiés par calcul calendaire. La direction mobile
ne contient pas de sélecteur hebdomadaire ; aucun ajout n'a été effectué.

La régénération des deux planches n'est pas une retouche garantissant l'identité
de chaque pixel hors calendrier. Les différences de rendu et approximations
connues sont encadrées dans le référentiel ; les originaux historiques restent
disponibles pour comparer la composition. Les données de ces images sont fictives.

Pour revérifier l'intégrité depuis la racine du dépôt :

```sh
python3 - <<'PY'
from pathlib import Path
import hashlib, json, struct
root = Path('docs/design/pwa/maquettes')
for item in json.loads((root / 'manifest.json').read_text())['images']:
    data = (root / item['file']).read_bytes()
    assert data[:8] == b'\x89PNG\r\n\x1a\n'
    assert struct.unpack('>II', data[16:24]) == (item['width'], item['height']) == (1644, 957)
    assert len(data) == item['size_bytes']
    assert hashlib.sha256(data).hexdigest() == item['sha256']
    source = Path(item['source_directory']) / item['source_name']
    if source.exists():
        assert source.read_bytes() == data
    print('OK', item['file'])
PY
git diff --check
```

Le test contre les sources locales n'est reproductible que sur une machine qui
les possède encore ; les empreintes archivées restent vérifiables dans le dépôt.

## Travail local préexistant préservé

Les empreintes suivantes ont été comparées au début et à la fin de cette livraison.
Ces fichiers ne font pas partie du lot documentaire ; leurs modifications ne
sont ni annulées, ni remplacées, ni attribuées à NEWS-227.

| Fichier | SHA-256 avant/après |
| --- | --- |
| `apps/pwa/src/features/admin/lib/rustLogger/api.ts` | `9610379aec762f1bfb925a9b02b2f95fd035235c245056c554e8c2823461f785` |
| `apps/pwa/src/features/admin/lib/rustLogger/headers.ts` | `0207021fe8e3e94be2e11cace4fbfc0e085084d7cb4183f687636422f19f4a46` |
| `apps/pwa/tests/rustLoggerHeaders.test.mjs` | `9aaea1d15fa4465caa7b3c077f5a96af510ce8d2b53b6acf427f98f130f50d3c` |

## Validation à réaliser ensuite

Cette recette ne démontre ni le fonctionnement gluestack, ni les contrastes du
rendu effectif, ni la continuité audio. Ces contrôles sont dans NEWS-228 à NEWS-237,
avec les deux thèmes et les cinq largeurs, les parcours et toutes les fonctions
admin. Les captures d'application et éventuelles preuves de déploiement seront
archivées séparément des références ImageGen.
