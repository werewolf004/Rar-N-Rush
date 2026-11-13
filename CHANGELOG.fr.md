# Journal des modifications

[English](CHANGELOG.md) · **Français**

Toutes les modifications notables du projet sont documentées ici. Le format suit [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/) et le projet respecte le [versionnage sémantique](https://semver.org/lang/fr/spec/v2.0.0.html).

## [Non publié]

## [0.1.0] - 2026-11-13

Première version publique : toute la chaîne de traitement, des fondations qualité et la documentation.

### Ajouté

- Surveillance d'un dossier de téléchargements (sous-dossiers en option) à intervalle réglable. Un jeu n'est traité qu'une fois ses fichiers stables.
- Regroupement des fichiers en jeux : index et volumes PAR2 (noms protégés lus dans les paquets `FileDesc`), fichiers SFV et volumes RAR (`.partN.rar`, `.rar` + `.rNN`/`.sNN`).
- Vérification et réparation PAR2 via un `par2.exe` externe, avec progression en direct et annulation. Si la réparation est impossible, le jeu attend et repart à l'arrivée de nouveaux fichiers.
- Vérification SFV (CRC32) native.
- Extraction RAR avec la bibliothèque officielle UnRAR : archives multi-volumes, liste de mots de passe, mot de passe saisi dans la fenêtre, en-têtes chiffrés, archives imbriquées (jusqu'à 3 niveaux).
- Protection des données : un fichier existant est conservé (`nom (1).ext`) sauf si l'écrasement est activé ; les entrées à chemin absolu ou contenant `..` sont refusées ; les fichiers sources vont à la corbeille, jamais supprimés définitivement ; un jeu traité est enregistré avant que son statut ne s'affiche.
- Interface egui : liste des jeux avec statut, progression et détail, actions Ignorer, Réessayer, Ouvrir et Retirer, journal filtrable, fenêtre des réglages.
- Interface en français et en anglais, qui suit la langue de Windows par défaut.
- Thème clair, foncé ou automatique (suit Windows en direct).
- Réglages portables à côté de l'exécutable avec écriture atomique, repli sur `%APPDATA%\RarNRush`. Un fichier de réglages existant est toujours retrouvé (d'abord à côté de l'exécutable, puis dans `%APPDATA%`), même après un déplacement de l'exécutable.
- Les réglages ne sont jamais perdus en silence : chaque clé est chargée séparément, une valeur invalide est remise par défaut individuellement après une copie de sauvegarde (`rar-n-rush.toml.<date-heure>.bak`), et un bandeau avertit l'utilisateur. Le BOM UTF-8 du Bloc-notes est accepté.
- Aucun accès réseau : ni vérification de mise à jour, ni télémétrie. Les crates réseau sont interdits par `cargo deny`.
- Documentation en français et en anglais : README, référence des réglages, dépendances, guide de contribution, politique de sécurité, feuille de route, captures générées hors écran.
- CI (GitHub Actions) : formatage, `clippy::pedantic` avec avertissements bloquants, tests unitaires, d'intégration et d'interface sans fenêtre, vérification de la MSRV, `cargo deny`. Workflow de release qui publie le binaire, son SHA-256 et la section correspondante de ce journal.

### Corrigé

- Plantages aléatoires `STATUS_ACCESS_VIOLATION` sur les archives multi-volumes, causés par une lecture hors limites dans le crate `unrar` 0.5.8 (copie corrigée dans `vendor/unrar`).

### Sécurité

- `#![forbid(unsafe_code)]` sur tout le crate ; `unwrap`, `expect` et `panic` sont interdits hors des tests.

[Non publié]: https://github.com/werewolf004/Rar-N-Rush/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/werewolf004/Rar-N-Rush/releases/tag/v0.1.0
