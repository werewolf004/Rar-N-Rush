# Rar N Rush

[English](README.md) · **Français**

[![CI](https://github.com/werewolf004/Rar-N-Rush/actions/workflows/ci.yml/badge.svg)](https://github.com/werewolf004/Rar-N-Rush/actions/workflows/ci.yml)
[![Licence : MIT OR Apache-2.0](https://img.shields.io/badge/licence-MIT%20OR%20Apache--2.0-blue.svg)](#licence)
![Plateforme : Windows](https://img.shields.io/badge/plateforme-Windows%2010%2F11-lightgrey.svg)
![MSRV 1.95](https://img.shields.io/badge/MSRV-1.95-orange.svg)

Rar N Rush surveille ton dossier de téléchargements. Pour chaque jeu de fichiers, il vérifie les données PAR2 ou SFV, répare les fichiers abîmés si besoin, puis extrait automatiquement les archives RAR.

Écrit en Rust avec une interface [egui](https://github.com/emilk/egui). Le concept s'inspire de l'application Par-N-Rar, mais le code est une réécriture indépendante.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/main-fr-dark.png">
  <img alt="Fenêtre principale : quatre jeux de fichiers avec leur statut, leur progression et leur détail, et le journal en dessous" src="docs/screenshots/main-fr-light.png">
</picture>

## Fonctionnalités

- **Surveillance d'un dossier** : le dossier surveillé (et, au choix, ses sous-dossiers) est analysé à intervalle régulier. Un jeu n'est traité qu'une fois ses fichiers stables : un téléchargement en cours n'est jamais touché.
- **PAR2** : vérification et réparation via `par2.exe`. Le zip de release contient [par2cmdline-turbo](https://github.com/animetosho/par2cmdline-turbo) 1.5.0 à côté de `rar-n-rush.exe` : ça marche sans rien installer. S'il manque des blocs de récupération, le jeu attend et repart dès que de nouveaux fichiers arrivent.
- **SFV** : vérification CRC32 native.
- **RAR** : extraction avec la bibliothèque officielle UnRAR. Gère les archives multi-volumes (`.partN.rar`, `.rar` + `.r00`), les mots de passe (liste essayée automatiquement, ou saisie dans la fenêtre), les en-têtes chiffrés et les archives contenues dans une archive.
- **Sécurité des données** : par défaut, un fichier existant n'est jamais écrasé (on utilise `nom (1).ext`). Les chemins qui sortiraient du dossier de destination sont refusés. Les fichiers sources peuvent être envoyés à la corbeille, jamais supprimés définitivement.
- **Interface** : français ou anglais (suit Windows par défaut) ; thème clair, foncé ou automatique, qui suit Windows en direct ; icône facultative dans la zone de notification (réduire ou fermer vers cette zone) ; notifications Windows quand la fenêtre n'est pas au premier plan ; démarrage avec Windows en option ; une seule instance à la fois (la relancer ramène la fenêtre ouverte) ; la liste suit le jeu en cours de traitement.
- **Rangement** (facultatif) : les autres fichiers d'un jeu (`.nfo`, extraits…) le suivent dans la destination, et les dossiers restés vides partent à la corbeille.
- **Réglages fiables** : fichier portable à côté de l'exécutable, retrouvé même si tu le déplaces ; une valeur invalide est remise par défaut individuellement (jamais tout le fichier) après une copie de sauvegarde, et tu en es averti.
- **Hors ligne** : aucune vérification de mise à jour, aucune télémétrie, aucun accès réseau. La mise à jour reste ton choix : télécharge une nouvelle release quand tu le veux.

## Installation

1. Télécharge `rar-n-rush-vX.Y.Z-windows-x64.zip` depuis la page [Releases](https://github.com/werewolf004/Rar-N-Rush/releases).
2. Vérifie l'archive avec le fichier `.sha256` publié à côté :
   ```powershell
   Get-FileHash .\rar-n-rush-vX.Y.Z-windows-x64.zip -Algorithm SHA256
   ```
   À partir de la v0.3.0, tu peux aussi vérifier que l'archive a bien été produite par le workflow de release de ce dépôt (provenance de build signée, nécessite la [GitHub CLI](https://cli.github.com/)) :
   ```powershell
   gh attestation verify .\rar-n-rush-vX.Y.Z-windows-x64.zip --repo werewolf004/Rar-N-Rush
   ```
3. Décompresse-la où tu veux et lance `rar-n-rush.exe`.
4. La réparation PAR2 utilise le `par2.exe` fourni dans le zip. Pour en utiliser un autre, indique son chemin dans les réglages (ou retire celui fourni et mets le tien dans le `PATH`). Rar N Rush ne télécharge jamais rien.

Configuration requise : Windows 10 ou 11 (64 bits).

## Prise en main

Au premier lancement, choisis le dossier où arrivent tes téléchargements :

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/onboarding-fr.png">
  <img alt="Premier lancement : un bouton invite à choisir le dossier à surveiller" src="docs/screenshots/onboarding-fr.png">
</picture>

La surveillance démarre tout de suite. Tu peux changer de dossier à tout moment, même pendant un traitement, depuis la barre **Dossier surveillé** en haut de la fenêtre : saisis un chemin et valide avec Entrée (Échap annule), choisis-en un dans **Dossiers récents**, ou clique sur **Parcourir…**. **Ouvrir** affiche le dossier dans l'Explorateur.

Chaque jeu de fichiers apparaît dans la liste avec son statut :

| Statut | Signification |
|---|---|
| En attente | Des fichiers arrivent encore. |
| Prêt | Complet, attend son tour. |
| Vérification / Réparation / Extraction | En cours de traitement. |
| Mot de passe requis | Saisis le mot de passe dans la ligne et valide avec Entrée. |
| Réparation impossible | Pas assez de blocs PAR2 : le jeu attend d'autres fichiers. |
| Terminé | Extrait avec succès. |
| Erreur | Voir la colonne Détail et le journal ; **Réessayer** relance le jeu. |
| Ignoré | Ignoré par toi ; **Réessayer** le traite quand même. |

Ouvre **Réglages** pour changer les dossiers, les délais, la langue, le thème et le comportement. Chaque option est décrite dans [docs/OPTIONS.fr.md](docs/OPTIONS.fr.md).

| Raccourci | Action |
|---|---|
| **F5** | Analyser le dossier surveillé maintenant |
| **Ctrl+,** | Ouvrir les réglages |
| **Échap** | Fermer les réglages (demande d'abord si quelque chose a changé) ; dans le champ du dossier surveillé, annuler la saisie |
| **Entrée** | Dans le champ du dossier surveillé, surveiller le dossier saisi ; dans un champ de mot de passe, essayer le mot de passe |
| **↑ / ↓** | Sélectionner un jeu dans la liste (ou cliquer dessus) |
| **Suppr** | Retirer le jeu sélectionné de la liste (ses fichiers ne sont pas touchés) |
| **Entrée** (jeu sélectionné) | Ouvrir le dossier du jeu sélectionné |

La fenêtre se rouvre à la taille et à la position qu'elle avait à la fermeture. Quand un jeu se termine, échoue ou demande un mot de passe, un court message apparaît quelques secondes en bas à droite, sans t'interrompre.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/settings-fr-dark.png">
  <img alt="Fenêtre des réglages" src="docs/screenshots/settings-fr-light.png">
</picture>

## Documentation

| Document | Contenu |
|---|---|
| [docs/USER-GUIDE.fr.md](docs/USER-GUIDE.fr.md) | Guide utilisateur : chaque écran, pas à pas, avec captures |
| [docs/OPTIONS.fr.md](docs/OPTIONS.fr.md) | Chaque réglage : nom, valeur par défaut, plage, effet |
| [docs/DEPENDENCIES.fr.md](docs/DEPENDENCIES.fr.md) | Chaque dépendance et son rôle |
| [docs/THIRD-PARTY-LICENSES.md](docs/THIRD-PARTY-LICENSES.md) | Les 229 crates du binaire, avec leurs licences |
| [CHANGELOG.fr.md](CHANGELOG.fr.md) | Les changements de chaque version |
| [ROADMAP.fr.md](ROADMAP.fr.md) | La suite prévue |
| [CONTRIBUTING.fr.md](CONTRIBUTING.fr.md) | Compiler, tester et contribuer |
| [SECURITY.fr.md](SECURITY.fr.md) | Modèle de sécurité et signalement d'une faille |

## Compiler depuis les sources

Il faut Rust 1.95 ou plus récent et les outils de build C++ de Visual Studio (MSVC), qui compilent UnRAR.

```bash
cargo build --release
```

L'exécutable est produit dans `target/release/rar-n-rush.exe`. Voir [CONTRIBUTING.fr.md](CONTRIBUTING.fr.md) pour les tests et contrôles.

## Licence

Rar N Rush est distribué au choix sous la [licence MIT](LICENSE-MIT) ou la [licence Apache 2.0](LICENSE-APACHE).

La bibliothèque UnRAR embarquée est sous licence UnRAR : elle autorise l'extraction d'archives mais interdit d'utiliser son code pour recréer l'algorithme de compression RAR. Voir [docs/THIRD-PARTY-LICENSES.md](docs/THIRD-PARTY-LICENSES.md).
