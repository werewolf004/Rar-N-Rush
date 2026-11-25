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
- Le fichier journal (`rar-n-rush.log`) ne contient jamais de mot de passe : les entrées du journal sont des événements typés sans secret, et un test le vérifie.
- L'espace libre du disque de destination est vérifié avant l'extraction (taille de l'archive plus une réserve de 64 Mo).
- Une extraction en échec ou annulée supprime les fichiers qu'elle a créés ; les fichiers déjà présents ne sont jamais supprimés.
- Les fichiers sources sont envoyés à la **corbeille**, jamais supprimés définitivement, et seulement après un traitement entièrement réussi.
- `par2.exe` est lancé sans shell, avec des arguments passés séparément (pas d'injection de commande), sans fenêtre console, et uniquement depuis le chemin choisi par l'utilisateur, le dossier du programme ou le `PATH`. Le `par2.exe` fourni dans le zip de release est le build officiel de par2cmdline-turbo, vérifié contre un SHA-256 figé avant d'être testé et empaqueté, et couvert par l'attestation de provenance de la release.

### Accès réseau : uniquement la vérification des mises à jour, facultative

- Aucune télémétrie, aucun rapport de plantage. Par défaut, Rar N Rush ne fait **aucune requête réseau**.
- Seule exception : la vérification des mises à jour, **désactivée par défaut** (Réglages → Général → Mises à jour). Quand elle est activée, environ une minute après le lancement puis au plus une fois par jour, semaine ou mois, le programme demande à `https://api.github.com/repos/werewolf004/Rar-N-Rush/releases/latest` le numéro de la dernière version. La requête ne contient qu'un agent utilisateur `rar-n-rush/<version>` : aucun identifiant, aucune statistique. Rien n'est téléchargé ni installé ; la fenêtre affiche un lien vers la page de la release et tu décides.
- La requête passe par `%SystemRoot%\System32\curl.exe`, fourni avec Windows 10 et 11 (jamais une copie trouvée via le `PATH`), en HTTPS uniquement, redirections comprises, avec une limite de 30 secondes et de 1 Mo. Seul un numéro de version strict `vX.Y.Z` est lu dans la réponse ; le lien affiché est construit à partir de ce numéro, jamais repris de la réponse.
- Le programme lui-même ne contient toujours aucun code réseau, ce qu'impose `cargo deny` : les crates HTTP, TLS et réseau asynchrone (`reqwest`, `hyper`, `rustls`, `tokio`, `self_update`…) sont interdits, la CI échoue si l'un d'eux entre dans les dépendances. Les tests ne vont jamais sur Internet (une fausse réponse les remplace).

### Données persistantes

- Les réglages ne sont jamais perdus en silence : un fichier de réglages illisible n'est jamais remplacé, un fichier enregistré en « ANSI » est lu en Windows-1252, un Gestionnaire d'identifiants illisible au démarrage n'est jamais écrit pendant cette session ; chaque clé est lue séparément, une valeur invalide est remise par défaut individuellement, et le fichier d'origine est d'abord sauvegardé (`rar-n-rush.toml.<date-heure>.bak`). L'utilisateur est averti par un bandeau.
- Un fichier de réglages existant est toujours retrouvé, à côté de l'exécutable ou dans `%APPDATA%\RarNRush`, même après un déplacement de l'exécutable.
- Les réglages et la liste des jeux traités sont écrits de façon atomique (fichier temporaire puis renommage) : un plantage ne peut pas laisser un fichier tronqué.
- Un jeu est enregistré comme traité **avant** que son statut ne s'affiche : un plantage ne peut pas provoquer un double traitement.
- Les écritures dans le registre se limitent à l'utilisateur courant (`HKEY_CURRENT_USER`), ne demandent pas de droits administrateur et sont annulées quand l'option est désactivée : la valeur `Rar N Rush` sous `...\CurrentVersion\Run` (démarrage avec Windows) et la clé `RarNRush.App` sous `Software\Classes\AppUserModelId` (nom et icône des notifications Windows).
- L'installateur est par utilisateur : il n'écrit que dans le profil de l'utilisateur et sous `HKEY_CURRENT_USER` (pas de droits administrateur, pas d'invite UAC, pas de service, rien pour les autres comptes). À la désinstallation (pas lors d'une mise à jour), il lance `rar-n-rush.exe --uninstall`, qui supprime l'entrée de démarrage seulement si elle lance cette copie installée, ainsi que l'identité de notification. Les réglages et les mots de passe enregistrés sont conservés.
- Une seule instance par dossier de réglages : un fichier verrou tenu par le programme en cours (libéré par Windows quand il se termine, même après un plantage) empêche deux copies d'écrire en même temps les mêmes réglages et listes.
- Les mots de passe d'archives sont conservés dans le Gestionnaire d'identifiants Windows (protégé par ton compte Windows), et non dans `rar-n-rush.toml`. L'écriture résiste aux plantages : une nouvelle copie est écrite, puis activée, puis l'ancienne est supprimée. Si le Gestionnaire d'identifiants est indisponible, les mots de passe restent en clair dans le fichier de réglages pour qu'aucun ne soit perdu, et la fenêtre l'indique.
- **Limite connue** : les copies de sauvegarde du fichier de réglages faites par les versions précédentes (`rar-n-rush.toml.<date-heure>.bak`) peuvent encore contenir des mots de passe en clair ; supprime-les si besoin.

### Code tiers

- Le seul code `unsafe` vient des dépendances. La plus sensible est l'extraction RAR : le crate `unrar` appelle la bibliothèque C++ officielle UnRAR via FFI.
- `unrar` 0.5.8 contenait une lecture hors limites au changement de volume, qui faisait planter le processus au hasard. La copie dans `vendor/unrar` la corrige ; voir [vendor/unrar/PATCHES.md](vendor/unrar/PATCHES.md).
- `cargo deny` vérifie la base de failles RustSec, les licences et les sources à chaque exécution de la CI. Dependabot propose les mises à jour chaque semaine.
- Les GitHub Actions sont épinglées sur des SHA de commit complets, et les workflows tournent par défaut avec des permissions en lecture seule.

### Releases

- Le programme et l'installateur ne portent pas de signature Authenticode (les certificats de signature ne sont pas gratuits) : Windows SmartScreen peut donc avertir au premier lancement. Leur origine se prouve à la place par la somme SHA-256 et la provenance de build signée ci-dessous.
- Chaque archive et chaque installateur de release sont publiés avec leur somme de contrôle SHA-256. Vérifie-la avant utilisation (voir le README).
- L'installateur est construit avec WiX Toolset 5.0.2, figé par SHA-256 (`scripts/fetch-wix.ps1`) ; à chaque push, la CI l'installe, le met à jour et le désinstalle en silence, et vérifie ce que chaque étape laisse.
- À partir de la v0.3.0, l'archive et l'exécutable portent une provenance de build SLSA signée (attestation d'artefact GitHub, Sigstore) : À partir de la v1.0.0, l'installateur l'est aussi. `gh attestation verify` prouve qu'ils ont été produits par le workflow de release de ce dépôt à partir du commit tagué.
