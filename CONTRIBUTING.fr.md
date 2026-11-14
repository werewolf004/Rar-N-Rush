# Contribuer

[English](CONTRIBUTING.md) · **Français**

Merci de ton intérêt pour Rar N Rush. Ce guide explique comment compiler, tester et proposer des modifications.

## Règles de base

- Le code, les identifiants, les commentaires et les messages de commit sont en **anglais**. Chaque fichier de documentation existe en anglais et en français (`FICHIER.md` et `FICHIER.fr.md`) : mets les deux à jour.
- Chaque texte visible par l'utilisateur passe par [`src/i18n.rs`](src/i18n.rs), dans les deux langues. Aucun libellé en dur dans l'interface.
- Pas d'`unsafe` (interdit par le compilateur). Pas d'`unwrap`, `expect` ou `panic` hors des tests : renvoie plutôt une erreur typée (`thiserror`).
- Chaque modification s'accompagne de tests : tests unitaires à côté du code, tests d'intégration dans `tests/`, tests d'interface dans `tests/ui.rs`.
- Une modification visible par l'utilisateur ajoute une ligne sous `## [Non publié]` dans `CHANGELOG.fr.md` **et** sous `## [Unreleased]` dans `CHANGELOG.md`.
- Un réglage nouveau ou modifié est documenté dans `docs/OPTIONS.md` et `docs/OPTIONS.fr.md`.
- Une nouvelle dépendance est justifiée dans `docs/DEPENDENCIES.md` / `.fr.md`, doit passer `cargo deny`, et `docs/THIRD-PARTY-LICENSES.md` doit être régénéré.

## Installation

- Windows 10/11, Rust ≥ 1.95 (`rustup` installe la bonne version grâce à `rust-toolchain.toml`).
- Outils de build C++ de Visual Studio (MSVC), nécessaires pour compiler UnRAR.
- Optionnel : [`cargo-deny`](https://github.com/EmbarkStudios/cargo-deny) (`cargo install cargo-deny`), Python 3 pour `scripts/third-party.py`, WinRAR pour régénérer les archives de test.

## Avant chaque commit

Ce sont les contrôles de la CI ; lance-les d'abord en local :

```bash
cargo fmt --all
```

```bash
cargo clippy --all-targets --locked -- -D warnings
```

```bash
cargo test --all-targets --locked
```

```bash
cargo deny check
```

## Organisation du projet

| Chemin | Contenu |
|---|---|
| `src/scan.rs` | Regroupe les fichiers en jeux (PAR2, SFV, volumes RAR) |
| `src/par2.rs` | Lance `par2.exe` et lit sa progression |
| `src/sfv.rs` | Vérification CRC32 |
| `src/extract.rs` | Extraction RAR, mots de passe, politique d'écrasement, sécurité des chemins |
| `src/engine.rs` | Tâche de fond : boucle d'analyse, file d'attente, traitement |
| `src/settings.rs` | Réglages et état persistant (écritures atomiques) |
| `src/i18n.rs` | Tous les textes visibles, en anglais et en français |
| `src/ui.rs` | Interface egui |
| `src/secrets.rs` | Mots de passe d'archives dans le Gestionnaire d'identifiants Windows |
| `src/tidy.rs` | Déplacement des fichiers annexes, suppression des dossiers vides |
| `src/tray.rs`, `src/icon.rs` | Icône de la zone de notification, icône de l'application |
| `src/instance.rs`, `src/autostart.rs`, `src/notify.rs` | Une seule instance, démarrage avec Windows, notifications Windows |
| `src/uninstall.rs` | Nettoyage du registre lancé par l'installateur à la désinstallation |
| `installer/rar-n-rush.wxs`, `scripts/*-msi.ps1`, `scripts/fetch-wix.ps1` | Paquet Windows Installer (WiX 5) : source, construction, test de bout en bout |
| `fuzz/` | Cibles de fuzzing (`cargo fuzz`), hors du build principal |
| `tests/` | Tests d'intégration et d'interface ; `tests/fixtures/` contient de vraies archives RAR |
| `vendor/unrar/` | Crate `unrar` corrigé, voir `PATCHES.md` |

## Tests

- `tests/fixtures/*.rar` sont de vraies archives créées avec WinRAR par `tests/fixtures/make-fixtures.ps1`. Elles sont versionnées : la CI n'a pas besoin de WinRAR.
- Le comportement PAR2 est testé avec de petits scripts batch qui imitent la sortie et les codes de retour de `par2` (`tests/common/mod.rs`), et de bout en bout avec le vrai par2cmdline-turbo (`tests/par2_real.rs`) : volume abîmé, volume manquant, blocs insuffisants, et un jeu qui attend ses volumes de récupération. La CI les lance toujours ; en local :

  ```powershell
  pwsh scripts/fetch-par2.ps1   # version figée, SHA-256 vérifié -> target/par2/par2.exe
  $env:RAR_N_RUSH_PAR2 = (Resolve-Path target/par2/par2.exe)
  cargo test --test par2_real
  ```

  Sans `RAR_N_RUSH_PAR2`, ces tests sont ignorés en local et échouent en CI.
- L'installateur est testé de bout en bout par `scripts/test-msi.ps1` (la CI le lance à chaque push) : installation silencieuse, mise à jour, désinstallation, avec vérification des fichiers, du raccourci du menu Démarrer et du registre après chaque étape. Il faut un runtime .NET (6 ou plus récent) pour WiX, que `scripts/fetch-wix.ps1` télécharge (version figée, SHA-256 vérifié). Il installe pour l'utilisateur courant : il refuse donc de tourner si Rar N Rush est déjà installé ou enregistré sur ce compte, et supprime tout ce qu'il a créé à la fin :

  ```powershell
  pwsh scripts/fetch-par2.ps1
  cargo build --release --locked
  pwsh scripts/test-msi.ps1     # ou : pwsh scripts/build-msi.ps1 -> target/msi/*.msi
  ```
- Les tests d'interface pilotent la vraie application via son arbre d'accessibilité avec `egui_kittest`. Aucune fenêtre n'est ouverte.
- Les tests n'affichent jamais de notification Windows et ne touchent jamais la vraie entrée de démarrage : les notifications passent par un faux `Notifier`, et les tests du registre utilisent des clés jetables sous `HKEY_CURRENT_USER\Software\RarNRush-test-<pid>-…`, supprimées à la fin. `tests/single_instance.rs` lance le vrai programme pendant que le verrou est tenu et vérifie qu'il se ferme aussitôt, sans fenêtre.
- Les tests de capture pixel par pixel (`snapshots` dans `tests/ui.rs`) comparent les écrans principaux à `tests/snapshots/*.png` : fenêtre principale (anglais clair et foncé, français), les trois pages des réglages, l'écran d'accueil. Ils sont rendus par le rasteriseur logiciel de Windows (WARP) : pas besoin de GPU, et ils tournent en CI. Les heures sont figées pour que chaque exécution donne la même image ; la démo est placée dans `C:\Users\Public\RarNRush-snapshots`. Ils vérifient aussi que chaque widget a un nom accessible. Après un changement visuel voulu, regarde les fichiers `*.diff.png` (la CI les joint dans l'artefact `snapshot-diffs`), puis mets les images à jour et commite-les :

  ```powershell
  $env:UPDATE_SNAPSHOTS = 1; cargo test --test ui snapshots; Remove-Item Env:UPDATE_SNAPSHOTS
  ```

- `tests/properties.rs` vérifie des propriétés des parseurs de données non fiables (PAR2, SFV, noms RAR, chemins d'entrées, noms de dossiers) sur des milliers d'entrées générées avec `proptest`.
- Un test écrit dans le vrai Gestionnaire d'identifiants Windows, sous un préfixe unique `test-<pid>-`, puis supprime ce qu'il a écrit.

### Fuzzing

Les mêmes parseurs sont soumis au fuzzing avec `cargo fuzz` (nightly). Sous Windows, libFuzzer a besoin d'AddressSanitizer, dont la DLL est fournie avec les outils de build MSVC :

```powershell
rustup toolchain install nightly --profile minimal
cargo install cargo-fuzz --locked
$env:PATH = "<VS>\VC\Tools\MSVC\<version>\bin\Hostx64\x64;$env:PATH"   # clang_rt.asan_dynamic-x86_64.dll
cargo +nightly fuzz run par2_names -- -max_total_time=60
```

Cibles : `par2_names`, `sfv_text`, `names`. Le workflow **Fuzz** les lance chacune toutes les semaines et à la demande.

### Captures d'écran

Les captures de la documentation sont rendues hors écran (aucune fenêtre n'apparaît) :

```bash
UPDATE_SCREENSHOTS=1 cargo test --test ui screenshots -- --test-threads=1
```

Elles sont écrites dans `docs/screenshots/`. Les données de démo sont placées dans `C:\Users\Public\RarNRush-demo` pour qu'aucun chemin personnel n'apparaisse dans une image.

## Commits et pull requests

- Un sujet par pull request ; décris ce qui change pour l'utilisateur et comment c'est testé.
- Messages de commit en anglais, à l'impératif, première ligne courte (`Add free-space check before extraction`).
- La CI doit être verte avant la fusion.

## Releases

1. Déplace les entrées `[Unreleased]` / `[Non publié]` dans une nouvelle section `## [X.Y.Z] - AAAA-MM-JJ` des deux journaux et mets à jour les liens en bas de page.
2. Mets `version = "X.Y.Z"` dans `Cargo.toml`, lance `cargo build` pour mettre à jour `Cargo.lock`.
3. Commite, puis tague : `git tag -a vX.Y.Z -m "vX.Y.Z"` et pousse le tag.
4. Le workflow **Release** vérifie que le tag correspond à `Cargo.toml`, lance les tests, compile le binaire et publie la release avec la section du journal comme notes.

## Code de conduite

Ce projet suit le [code de conduite](CODE_OF_CONDUCT.fr.md).
