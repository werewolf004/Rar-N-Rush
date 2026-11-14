# Journal des modifications

[English](CHANGELOG.md) · **Français**

Toutes les modifications notables du projet sont documentées ici. Le format suit [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/) et le projet respecte le [versionnage sémantique](https://semver.org/lang/fr/spec/v2.0.0.html).

## [Non publié]

### Ajouté

- L'espace libre du disque de destination est vérifié avant l'extraction : si le contenu de l'archive plus une réserve de 64 Mo ne tient pas, le jeu échoue avec un message clair (tailles nécessaire et disponible) au lieu d'une extraction à moitié écrite.
- **Les mots de passe d'archives sont conservés dans le Gestionnaire d'identifiants Windows**, et non plus en clair dans `rar-n-rush.toml`. Ceux d'un ancien fichier de réglages y sont déplacés au lancement. Si le Gestionnaire d'identifiants est indisponible, ils restent dans le fichier pour qu'aucun ne soit perdu, et la fenêtre l'indique.
- Les archives et exécutables des releases portent une provenance de build signée (attestation d'artefact GitHub) ; `gh attestation verify` la vérifie.

### Corrigé

- Une extraction en échec ou annulée ne laisse plus de fichiers derrière elle : tout ce qu'elle a créé est supprimé (les fichiers déjà présents sont conservés), donc **Réessayer** repart de zéro au lieu de produire des doublons `nom (1).ext`.

## [0.2.0] - 2026-11-14

Expérience utilisateur : le dossier surveillé dans la fenêtre principale, les options de Par-N-Rar (rangement, zone de notification), les raccourcis clavier et des réglages plus sûrs.

### Ajouté

- Barre **Dossier surveillé** dans la fenêtre principale : le dossier surveillé se change à tout moment, même pendant un traitement, sans ouvrir les réglages. Saisis un chemin et valide avec Entrée (Échap annule), choisis l'un des 10 derniers dossiers dans **Dossiers récents**, ou utilise **Parcourir…**. Un dossier inexistant est refusé avec un message, et les réglages en cours sont conservés.
- Nouveau réglage `recent_watch_dirs` (liste des dossiers surveillés récemment).
- **Déplacement des fichiers annexes** (`move_extra_files`) : après l'extraction, les autres fichiers du jeu (`.nfo`, extraits…) vont dans la destination. Uniquement pour un jeu seul dans son propre sous-dossier : les autres téléchargements ne sont jamais touchés.
- **Suppression des dossiers vides** (`delete_empty_dirs`) : un dossier de jeu resté vide après un traitement réussi part à la corbeille, puis ses parents vides ; jamais le dossier surveillé lui-même.
- **Garder visible le jeu en cours** (`follow_active`, activé par défaut) : la liste défile jusqu'à chaque nouveau jeu traité.
- **Zone de notification** : réduire (`minimize_to_tray`) et/ou fermer (`close_to_tray`) dans une icône ; la surveillance continue, un clic fait revenir la fenêtre, **Quitter** dans son menu ferme l'application.
- Icône de l'application pour la fenêtre, la barre des tâches et la zone de notification.
- Raccourcis clavier : **F5** lance une analyse, **Ctrl+,** ouvre les réglages, **Échap** les ferme.
- La fenêtre des réglages vérifie les valeurs avant d'enregistrer : **Enregistrer** est désactivé, avec la raison affichée, tant que le dossier surveillé n'existe pas, que la destination est un fichier ou en lecture seule, ou que `par2.exe` n'est pas au chemin indiqué.
- Fermer la fenêtre des réglages avec des modifications non enregistrées demande d'abord **Abandonner tes modifications ?**.
- La taille et la position de la fenêtre sont mémorisées (table `window` du fichier de réglages).
- Test automatique : chaque couleur de statut atteint le contraste WCAG 2.2 AA (4,5:1) sur chaque fond, dans les deux thèmes.

### Modifié

- L'état de la surveillance (active / en pause, heure de la dernière analyse) est passé dans la barre d'outils.
- La fenêtre des réglages est découpée en trois pages (Général, Traitement, Fenêtre) : Enregistrer et Annuler restent toujours visibles.

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

[Non publié]: https://github.com/werewolf004/Rar-N-Rush/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/werewolf004/Rar-N-Rush/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/werewolf004/Rar-N-Rush/releases/tag/v0.1.0
