# Référence des réglages

[English](OPTIONS.md) · **Français**

Chaque réglage est accessible dans la fenêtre **Réglages** et enregistré dans `rar-n-rush.toml`. Ce fichier se trouve à côté de `rar-n-rush.exe` (mode portable), ou dans `%APPDATA%\RarNRush` quand le dossier du programme est protégé en écriture.

Une clé absente prend sa valeur par défaut : un fichier ancien ou partiel reste donc valide. Un nombre hors plage est ramené dans la plage autorisée au chargement.

![Fenêtre des réglages](screenshots/settings-fr-light.png)

## Dossiers

| Clé | Libellé dans la fenêtre | Type | Défaut | Effet |
|---|---|---|---|---|
| `watch_dir` | Dossier surveillé | chemin | *(vide)* | Dossier analysé pour trouver de nouveaux jeux. Vide : la surveillance ne peut pas démarrer. |
| `extract_dir` | Destination | chemin | *(vide)* | Dossier d'extraction. Vide : à côté des fichiers sources. S'il se trouve dans le dossier surveillé, il est exclu des analyses. |
| `par2_path` | par2.exe | chemin | *(vide)* | Chemin de `par2.exe`. Vide : recherche automatique, d'abord à côté de `rar-n-rush.exe`, puis dans le `PATH`. La fenêtre affiche le chemin trouvé, ou un avertissement. |

## Délais

| Clé | Libellé dans la fenêtre | Type | Défaut | Plage | Effet |
|---|---|---|---|---|---|
| `scan_interval_secs` | Analyser toutes les | secondes | `15` | 2–3600 | Délai entre deux analyses du dossier surveillé. Le bouton **Analyser** force une analyse immédiate. |
| `stable_secs` | Attendre la fin des fichiers | secondes | `10` | 0–3600 | Un jeu n'est traité qu'une fois ses fichiers (nombre, taille, date de modification) inchangés pendant cette durée. |

## Comportement

| Clé | Libellé dans la fenêtre | Type | Défaut | Effet |
|---|---|---|---|---|
| `recursive` | Analyser aussi les sous-dossiers | booléen | `true` | Analyse les sous-dossiers, jusqu'à 8 niveaux. Les liens symboliques ne sont pas suivis. |
| `create_subfolder` | Extraire dans un sous-dossier au nom du jeu | booléen | `true` | Extrait `Vacances.part1.rar` dans `<destination>\Vacances\`. Les caractères interdits par Windows sont remplacés par `_`. |
| `extract_nested` | Extraire les archives RAR contenues dans une archive | booléen | `true` | Extrait les archives trouvées dans une archive extraite (jusqu'à 3 niveaux), puis supprime ces archives internes. |
| `overwrite_existing` | Écraser les fichiers existants | booléen | `false` | À `false`, un fichier existant est conservé et la copie extraite est nommée `nom (1).ext`, `nom (2).ext`… |
| `delete_after_success` | Envoyer les fichiers PAR2/SFV/RAR à la corbeille après succès | booléen | `false` | Après un traitement réussi, envoie à la corbeille les fichiers PAR2, le SFV, les volumes RAR et les sauvegardes de par2 (`fichier.1`). Les fichiers vérifiés qui ne sont pas des RAR sont toujours conservés. |
| `autostart` | Démarrer la surveillance au lancement | booléen | `true` | Démarre la surveillance au lancement si un dossier surveillé est défini. |

## Mots de passe

| Clé | Libellé dans la fenêtre | Type | Défaut | Effet |
|---|---|---|---|---|
| `passwords` | Mots de passe à essayer (un par ligne) | liste de textes | `[]` | Essayés dans l'ordre sur les archives protégées. Un mot de passe saisi dans une ligne de la fenêtre principale est essayé en premier. |

> **Attention** : dans cette version, les mots de passe sont stockés en clair dans `rar-n-rush.toml`. Leur passage dans le Gestionnaire d'identifiants Windows est prévu en v0.3.0 (voir [ROADMAP.fr.md](../ROADMAP.fr.md)).

## Apparence

| Clé | Libellé dans la fenêtre | Valeurs | Défaut | Effet |
|---|---|---|---|---|
| `language` | Langue | `system`, `english`, `french` | `system` | Langue de l'interface. `system` suit la langue d'affichage de Windows (français si elle commence par `fr`, anglais sinon). |
| `theme` | Thème | `system`, `light`, `dark` | `system` | `system` suit le réglage clair/foncé de Windows, en direct. |

## Exemple de fichier

```toml
watch_dir = 'D:\Telechargements'
extract_dir = 'D:\Extraits'
create_subfolder = true
recursive = true
par2_path = ''
delete_after_success = false
extract_nested = true
overwrite_existing = false
scan_interval_secs = 15
stable_secs = 10
passwords = []
autostart = true
language = "system"
theme = "system"
```

## Autres fichiers

| Fichier | Contenu |
|---|---|
| `rar-n-rush-processed.txt` | Une ligne par jeu déjà traité ou ignoré, pour ne pas le retraiter après un redémarrage. Supprime une ligne (ou le fichier) pour retraiter un jeu. |
