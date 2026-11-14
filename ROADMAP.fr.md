# Feuille de route

[English](ROADMAP.md) · **Français**

Chaque bloc se termine par une release taguée avec son entrée dans le journal. L'ordre et le contenu peuvent évoluer ; ce qui est fait passe dans [CHANGELOG.fr.md](CHANGELOG.fr.md).

## ✅ v0.1.0 — Fondations

Chaîne de traitement complète (analyse, PAR2, SFV, RAR), erreurs typées, ni `unsafe` ni `unwrap`, tests unitaires, d'intégration et d'interface sans fenêtre, CI/CD, documentation bilingue, licence.

## ✅ v0.2.0 — UI/UX et accessibilité

Dossier surveillé modifiable dans la fenêtre principale avec les dossiers récents ; options de Par-N-Rar (déplacement des fichiers annexes, suppression des dossiers vides, suivi du jeu en cours, zone de notification) ; réglages en trois pages avec validation et alerte « modifications non enregistrées » ; raccourcis F5, Ctrl+, et Échap ; taille et position de la fenêtre mémorisées ; test de contraste WCAG 2.2 AA ; libellés AccessKit vérifiés par les tests d'interface.

## v0.3.0 — Données et sécurité

- Reporté de la v0.2.0 : navigation complète de la liste au clavier et Suppr pour retirer ; confirmation avant les actions destructrices et notifications non bloquantes ; mise à l'échelle DPI testée de 100 à 200 % ; tests de capture des écrans principaux.
- ✅ Mots de passe d'archives stockés dans le Gestionnaire d'identifiants Windows au lieu du fichier de réglages.
- ✅ Vérification de l'espace libre avant extraction ; nettoyage des fichiers partiels en cas d'échec.
- Fuzzing des parseurs PAR2 et SFV (`cargo fuzz`), tests de propriétés pour le regroupement des noms.
- ✅ Export et rotation du journal ; aucun secret jamais écrit dans le journal.
- ✅ Provenance de build signée pour les releases (Sigstore / attestations d'artefacts GitHub).

## v0.4.0 — PAR2 complet

- Tests de bout en bout avec un vrai `par2` (version et somme de contrôle figées) en CI : fichiers abîmés ou manquants, réparation, blocs insuffisants.
- Téléchargement optionnel de par2cmdline-turbo depuis les réglages, avec vérification de la somme de contrôle et confirmation de l'utilisateur.

## v0.5.0 — Intégration Windows

- ~~Icône dans la zone de notification, réduction et fermeture vers cette zone.~~ Avancé en v0.2.0.
- Notifications Windows quand un jeu est terminé ou demande un mot de passe.
- Démarrage avec Windows (optionnel), une seule instance à la fois.

## v1.0.0 — Distribution

- Installateur (MSI) et paquet winget.
- Signature du code.
- Guide utilisateur avec une capture de chaque écran.
