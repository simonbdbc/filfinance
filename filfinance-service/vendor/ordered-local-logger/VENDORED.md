# Source embarquée

Ce répertoire contient `ordered-local-logger` 0.2.0, copié le 13 septembre 2026
depuis l’instantané local non publié `dev-rust-logger/lib-rust-logger`. Il reste une dépendance Cargo par
chemin afin que les builds Docker et CI de Filfinance soient autonomes.

La copie est sous licence MIT : voir [LICENSE](LICENSE). Pour la mettre à jour,
copier la version complète de la bibliothèque, conserver sa licence et mettre à
jour cette liste d'empreintes SHA-256 avant de lancer les tests de Filfinance.

| Fichier | SHA-256 |
| --- | --- |
| `Cargo.toml` | `29b3829dc8e3cce5ceed06b3d778437a781c6516a04d3ad3036d500962ef6c5f` |
| `src/engine.rs` | `788233d8fcfdca5796c23ab48f0e1c44fa34bfda3015bdbf8e1e1d00262e9993` |
| `src/lib.rs` | `cb2b750afc2340ede4bff3edf400d717da3bbba2cc3db9a11a6ff19f2b05540d` |
| `src/model.rs` | `7e7f7f544daaf259138439e8e71633ce6816d3cff67aba1de6d58cb6cbe9bf03` |
| `src/output.rs` | `b6e5acbb0c98c6745ac8701567fb75c5a767dd8689023cda7c21a7a0bb411f0f` |
| `src/storage.rs` | `b52c7832c46440dc014232743247511d0f61b3e97444567c3b0a9ab49b1afe15` |
| `src/tests.rs` | `a630fc4cc540e93a59c061d52ccc17d8a6d9b5803459554b77295f457ded5e0f` |
| `src/tracing_layer.rs` | `50d7d5f0adbb6c726bd27b41d3f7798a9c91c50c611b327f64b1c4a8c4a3f11e` |
| `CHANGELOG.md` | `1381f3cb91f881c60315dae1dcc2d04ce7ce20ca1f66c605a47c248d7b15b87c` |
| `Cargo.lock` | `c9db17e6f750a8214c912e6317b9415d3edd302fb9e09912e57c5831f16ed96b` |
| `docs/ROADMAP.md` | `d04fa7cf14dbfeb501f5069b1292c04be44f3fc741ed31e9e311cae0be221097` |
| `fixtures/v1/rust.log` | `eb208bf77af1b2e2dfdf02a83f81960c98f32f7e52b969795b320eaef2a6da15` |
| `examples/admission_bench.rs` | `f302fb99e083f29bbe9c746b4bfc741ad60b0cd22f3099537031381114ef0a0e` |
| `examples/measure.rs` | `e251f2dba292d2e8ff98863cfa6271b3c66c27963ef9b00e30a8f7276b4de589` |
| `examples/native.rs` | `2ac6797a473a626a00f057981e7c98a82ffb71b7764b90e565ab6d1011a74e51` |
| `examples/noisy.rs` | `5f7b9883ebe041da35acd222a3700c086168a92e75aaa46283b83c223c2c53e9` |
| `examples/tracing_workflow.rs` | `73b76e3d39cecfef09ad09be667a0f5fb0bfb2789820a585ae0d8c29b4ea8dd5` |
| `examples/workflow.rs` | `be36dca8f83a1727e909a7885f45f8bf385edc8cdd78539682041d259b083af5` |
