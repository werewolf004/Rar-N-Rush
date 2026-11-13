# Politique de sécurité

[English](SECURITY.md) · **Français**

## Versions prises en charge

Seule la dernière release reçoit les correctifs de sécurité.

## Signaler une faille

Merci de **ne pas** ouvrir d'issue publique. Utilise le signalement privé de GitHub : **Security → Report a vulnerability** sur la page du dépôt. Tu recevras une réponse sous 7 jours. Indique la version, les étapes pour reproduire et, si possible, une archive d'exemple.

## Modèle de sécurité

Rar N Rush traite des fichiers téléchargés depuis Internet, qu'il faut considérer comme hostiles. Les mesures suivantes sont en place.

### Code

- `#![forbid(unsafe_code)]` : le code propre au projet ne contient aucun `unsafe`.
- `unwrap`, `expect` et `panic` sont interdits hors des tests (lints `clippy`, imposés en CI). Chaque échec est une erreur typée (`thiserror`) affichée à l'utilisateur.
- Un verrou empoisonné ne fait pas planter l'application : l'état partagé ne contient que des données simples et il est récupéré.

### Archives et fichiers

- Les entrées d'archive avec un chemin absolu, une lettre de lecteur ou un composant `..` sont **ignorées** : rien ne peut être écrit hors du dossier de destination.
- Un fichier existant n'est **jamais écrasé**, sauf si l'utilisateur l'active ; un nom libre (`nom (1).ext`) est utilisé à la place.
- Les fichiers écrits avec un mauvais mot de passe sont supprimés avant d'essayer le mot de passe suivant.
- L'analyse ne suit pas les liens symboliques : un lien ne peut pas faire entrer des fichiers extérieurs au dossier surveillé.
- Les noms de dossier tirés des noms d'archives sont nettoyés (caractères interdits par Windows, noms réservés comme `CON` ou `NUL`).
- Les fichiers sources sont envoyés à la **corbeille**, jamais supprimés définitivement, et seulement après un traitement entièrement réussi.
- `par2.exe` est lancé sans shell, avec des arguments passés séparément (pas d'injection de commande), sans fenêtre console, et uniquement depuis le chemin choisi par l'utilisateur, le dossier du programme ou le `PATH`.

### Données persistantes

- Les réglages et la liste des jeux traités sont écrits de façon atomique (fichier temporaire puis renommage) : un plantage ne peut pas laisser un fichier tronqué.
- Un jeu est enregistré comme traité **avant** que son statut ne s'affiche : un plantage ne peut pas provoquer un double traitement.
- **Limite connue** : les mots de passe d'archives sont stockés en clair dans `rar-n-rush.toml`, comme indiqué sous le champ dans la fenêtre des réglages. Leur passage dans le Gestionnaire d'identifiants Windows est prévu en v0.3.0.

### Code tiers

- Le seul code `unsafe` vient des dépendances. La plus sensible est l'extraction RAR : le crate `unrar` appelle la bibliothèque C++ officielle UnRAR via FFI.
- `unrar` 0.5.8 contenait une lecture hors limites au changement de volume, qui faisait planter le processus au hasard. La copie dans `vendor/unrar` la corrige ; voir [vendor/unrar/PATCHES.md](vendor/unrar/PATCHES.md).
- `cargo deny` vérifie la base de failles RustSec, les licences et les sources à chaque exécution de la CI. Dependabot propose les mises à jour chaque semaine.
- Les GitHub Actions sont épinglées sur des SHA de commit complets, et les workflows tournent par défaut avec des permissions en lecture seule.

### Releases

- Chaque archive de release est publiée avec sa somme de contrôle SHA-256. Vérifie-la avant utilisation (voir le README).
