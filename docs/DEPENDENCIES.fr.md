# Dépendances

[English](DEPENDENCIES.md) · **Français**

Chaque dépendance directe est listée ci-dessous avec la raison de sa présence. La liste complète des 229 crates compilés dans le binaire, avec leurs licences, est générée dans [THIRD-PARTY-LICENSES.md](THIRD-PARTY-LICENSES.md) (`python scripts/third-party.py`).

Règles appliquées aux dépendances :

- chaque licence doit figurer dans la liste autorisée de [`deny.toml`](../deny.toml) (licences permissives uniquement) ;
- aucune vulnérabilité connue (base RustSec), vérifié par `cargo deny` en CI ;
- crates.io uniquement : pas de dépôt git ni de registre inconnu ;
- Dependabot propose les mises à jour chaque semaine.

## Dépendances d'exécution

| Crate | Version | Licence | Rôle |
|---|---|---|---|
| [`eframe`](https://crates.io/crates/eframe) | 0.37 | MIT OR Apache-2.0 | Fenêtre native, rendu et boucle d'événements pour egui. |
| [`egui`](https://crates.io/crates/egui) | 0.37 | MIT OR Apache-2.0 | Interface graphique en mode immédiat. Fonctionnalité activée : `monochrome_emoji_fonts`, pour que les icônes s'affichent à l'identique sur toutes les machines. |
| [`egui_extras`](https://crates.io/crates/egui_extras) | 0.37 | MIT OR Apache-2.0 | Tableau redimensionnable des jeux de fichiers. |
| [`rfd`](https://crates.io/crates/rfd) | 0.18 | MIT | Sélecteurs natifs Windows de dossier et de fichier. |
| [`unrar`](https://crates.io/crates/unrar) | 0.5.8 (corrigé) | MIT OR Apache-2.0 ; embarque UnRAR (licence UnRAR) | Extraction RAR avec la bibliothèque officielle UnRAR. Copie embarquée dans `vendor/unrar` avec un correctif de sécurité mémoire, voir [vendor/unrar/PATCHES.md](../vendor/unrar/PATCHES.md). |
| [`crc32fast`](https://crates.io/crates/crc32fast) | 1 | MIT OR Apache-2.0 | CRC32 pour la vérification SFV (accéléré SIMD). |
| [`trash`](https://crates.io/crates/trash) | 5 | MIT | Envoie les fichiers sources à la corbeille au lieu de les supprimer. |
| [`serde`](https://crates.io/crates/serde) | 1 | MIT OR Apache-2.0 | (Dé)sérialisation des réglages. |
| [`toml`](https://crates.io/crates/toml) | 1 | MIT OR Apache-2.0 | Fichier de réglages lisible. |
| [`thiserror`](https://crates.io/crates/thiserror) | 2 | MIT OR Apache-2.0 | Énumérations d'erreurs typées. |
| [`chrono`](https://crates.io/crates/chrono) | 0.4 | MIT OR Apache-2.0 | Heure locale des lignes du journal. Seule la fonctionnalité `clock` est activée. |
| [`sys-locale`](https://crates.io/crates/sys-locale) | 0.3 | MIT OR Apache-2.0 | Lit la langue d'affichage de Windows pour choisir français ou anglais. |
| [`tray-icon`](https://crates.io/crates/tray-icon) | 0.26 | MIT OR Apache-2.0 | Icône et menu dans la zone de notification (créés seulement si une option de zone de notification est activée). Fonctionnalités par défaut désactivées : sous Windows, n'ajoute que `muda` (menu), `windows-sys` et `crossbeam-channel`. |
| [`fs4`](https://crates.io/crates/fs4) | 0.13 | MIT OR Apache-2.0 | Espace libre du disque de destination, vérifié avant chaque extraction. Seule la fonctionnalité `sync` est activée. |
| [`keyring-core`](https://crates.io/crates/keyring-core) et [`windows-native-keyring-store`](https://crates.io/crates/windows-native-keyring-store) | 1 | MIT OR Apache-2.0 | Mots de passe d'archives dans le Gestionnaire d'identifiants Windows. Fonctionnalités par défaut du magasin désactivées (pas de `regex`). |
| [`windows-registry`](https://crates.io/crates/windows-registry) | 0.6 | MIT OR Apache-2.0 | Accès sûr au registre de l'utilisateur courant : démarrage avec Windows (clé `Run`) et identité des notifications Windows. Partage la famille `windows-*` 0.62 déjà tirée par d'autres dépendances. |
| [`tauri-winrt-notification`](https://crates.io/crates/tauri-winrt-notification) | 0.8 | MIT OR Apache-2.0 | Notifications Windows (toasts WinRT). Utilise le crate `windows` 0.62 déjà présent ; n'ajoute que `windows-version`. |

## Dépendances de développement

Utilisées par les tests uniquement ; absentes du binaire.

| Crate | Version | Licence | Rôle |
|---|---|---|---|
| [`tempfile`](https://crates.io/crates/tempfile) | 3 | MIT OR Apache-2.0 | Dossiers temporaires pour les tests, supprimés automatiquement. |
| [`egui_kittest`](https://crates.io/crates/egui_kittest) | 0.37 | MIT OR Apache-2.0 | Tests d'interface sans fenêtre via l'arbre d'accessibilité, et rendu hors écran des captures de la documentation. |

## Dépendances de compilation

Utilisées par `build.rs` pendant la compilation ; non livrées dans le binaire.

| Crate | Version | Licence | Rôle |
|---|---|---|---|
| [`embed-resource`](https://crates.io/crates/embed-resource) | 3 | MIT | Compile le script de ressources qui intègre l'icône et les informations de version dans `rar-n-rush.exe` (Explorateur, épingle de la barre des tâches, Gestionnaire des tâches), avec le compilateur de ressources du SDK Windows `rc.exe`. L'icône elle-même est dessinée par le code du programme. |

## Programmes externes

| Programme | Licence | Rôle |
|---|---|---|
| `par2.exe` ([par2cmdline-turbo](https://github.com/animetosho/par2cmdline-turbo) ou par2cmdline) | GPL-2.0 | Vérification et réparation PAR2. par2cmdline-turbo 1.5.0 est fourni sans modification dans le zip de release (`par2.exe`, avec sa licence dans `par2-COPYING.txt`) ; son archive source exacte est jointe à chaque release. Il est lancé comme processus séparé, jamais lié : trouvé au chemin indiqué dans les réglages, à côté de l'exécutable ou dans le `PATH`. Version et SHA-256 sont figés dans `scripts/fetch-par2.ps1`. |

## Outils de développement

| Outil | Rôle |
|---|---|
| `rustfmt`, `clippy` | Formatage et lints (`clippy::pedantic` ; `unwrap_used`, `expect_used`, `panic` interdits). |
| [`cargo-deny`](https://github.com/EmbarkStudios/cargo-deny) | Licences, failles connues, sources interdites. |
| WinRAR `Rar.exe` | Uniquement pour régénérer les archives de test (`tests/fixtures/make-fixtures.ps1`). |
| [WiX Toolset](https://github.com/wixtoolset/wix) 5.0.2 (MS-RL) | Construit l'installateur (`.msi`) ; non livré. Version et SHA-256 figés dans `scripts/fetch-wix.ps1` ; il tourne sur un runtime .NET installé. WiX 5 est gardé exprès : WiX 6 et suivants demandent une contribution aux projets qui génèrent des revenus. |
