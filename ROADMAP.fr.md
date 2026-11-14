# Feuille de route

[English](ROADMAP.md) · **Français**

Chaque bloc se termine par une release taguée avec son entrée dans le journal. L'ordre et le contenu peuvent évoluer ; ce qui est fait passe dans [CHANGELOG.fr.md](CHANGELOG.fr.md).

## ✅ v0.1.0 — Fondations

Chaîne de traitement complète (analyse, PAR2, SFV, RAR), erreurs typées, ni `unsafe` ni `unwrap`, tests unitaires, d'intégration et d'interface sans fenêtre, CI/CD, documentation bilingue, licence.

## ✅ v0.2.0 — UI/UX et accessibilité

Dossier surveillé modifiable dans la fenêtre principale avec les dossiers récents ; options de Par-N-Rar (déplacement des fichiers annexes, suppression des dossiers vides, suivi du jeu en cours, zone de notification) ; réglages en trois pages avec validation et alerte « modifications non enregistrées » ; raccourcis F5, Ctrl+, et Échap ; taille et position de la fenêtre mémorisées ; test de contraste WCAG 2.2 AA ; libellés AccessKit vérifiés par les tests d'interface.

## ✅ v0.3.0 — Données et sécurité

Mots de passe d'archives dans le Gestionnaire d'identifiants Windows (migrés depuis le fichier de réglages, jamais perdus) ; vérification de l'espace libre avant extraction et nettoyage des extractions en échec ; fichier journal avec rotation et export, sans aucun secret écrit ; provenance de build signée pour les releases ; tests de propriétés et cibles `cargo fuzz` pour chaque parseur de données non fiables ; navigation de la liste au clavier, messages non bloquants et confirmation avant l'écrasement.

## ✅ v0.4.0 — PAR2 complet

par2cmdline-turbo fourni dans le zip de release (version et SHA-256 figés, licence et source incluses ; l'application reste hors ligne) ; tests de bout en bout avec le vrai par2 en CI (volumes abîmés ou manquants, blocs insuffisants, attente des volumes de récupération) ; tests de capture pixel par pixel des écrans principaux sans GPU ; noms accessibles pour chaque champ des réglages.

## ✅ v0.5.0 — Intégration Windows

Icône dans la zone de notification (avancée en v0.2.0) ; une seule instance par dossier de réglages, un deuxième lancement ramène la fenêtre ; démarrage avec Windows en option, réduit ou dans la zone de notification ; notifications Windows quand un jeu se termine, échoue ou demande un mot de passe alors que la fenêtre n'est pas au premier plan. Tout reste dans le registre de l'utilisateur courant et est annulé quand l'option est désactivée.

## v1.0.0 — Distribution

- Installateur (MSI) et paquet winget.
- Signature du code.
- ✅ Guide utilisateur avec une capture de chaque écran ([docs/USER-GUIDE.fr.md](docs/USER-GUIDE.fr.md)).
