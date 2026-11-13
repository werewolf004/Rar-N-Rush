# Feuille de route

[English](ROADMAP.md) · **Français**

Chaque bloc se termine par une release taguée avec son entrée dans le journal. L'ordre et le contenu peuvent évoluer ; ce qui est fait passe dans [CHANGELOG.fr.md](CHANGELOG.fr.md).

## ✅ v0.1.0 — Fondations

Chaîne de traitement complète (analyse, PAR2, SFV, RAR), erreurs typées, ni `unsafe` ni `unwrap`, tests unitaires, d'intégration et d'interface sans fenêtre, CI/CD, documentation bilingue, licence.

## v0.2.0 — UI/UX et accessibilité

- Réglages dans une fenêtre modale avec validation en direct (dossier existant, destination accessible en écriture, par2 trouvé) et alerte « modifications non enregistrées ».
- Navigation complète au clavier et focus visible ; raccourcis (F5 analyser, Ctrl+, réglages, Suppr retirer).
- Libellés lecteur d'écran sur chaque contrôle (AccessKit), vérifiés par des tests.
- Contraste vérifié selon WCAG 2.2 AA dans les deux thèmes par un test automatique.
- Confirmation avant les actions destructrices ; notifications non bloquantes pour les résultats.
- Taille et position de la fenêtre mémorisées ; mise à l'échelle DPI testée de 100 à 200 %.
- Tests de capture des écrans principaux pour détecter les régressions visuelles.

## v0.3.0 — Données et sécurité

- Mots de passe d'archives stockés dans le Gestionnaire d'identifiants Windows au lieu du fichier de réglages.
- Vérification de l'espace libre avant extraction ; nettoyage des fichiers partiels en cas d'échec.
- Fuzzing des parseurs PAR2 et SFV (`cargo fuzz`), tests de propriétés pour le regroupement des noms.
- Export et rotation du journal ; aucun secret jamais écrit dans le journal.
- Sommes de contrôle des releases signées (Sigstore / attestations d'artefacts GitHub).

## v0.4.0 — PAR2 complet

- Tests de bout en bout avec un vrai `par2` (version et somme de contrôle figées) en CI : fichiers abîmés ou manquants, réparation, blocs insuffisants.
- Téléchargement optionnel de par2cmdline-turbo depuis les réglages, avec vérification de la somme de contrôle et confirmation de l'utilisateur.

## v0.5.0 — Intégration Windows

- Icône dans la zone de notification, réduction et fermeture vers cette zone.
- Notifications Windows quand un jeu est terminé ou demande un mot de passe.
- Démarrage avec Windows (optionnel), une seule instance à la fois.

## v1.0.0 — Distribution

- Installateur (MSI) et paquet winget.
- Signature du code.
- Guide utilisateur avec une capture de chaque écran.
