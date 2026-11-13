//! User-facing text in English and French.
//!
//! Static labels live in [`Texts`]; messages built from engine events go through
//! the [`Localize`] trait. Code, logs for developers and error `Display` impls
//! stay in English.

use crate::engine::{Detail, JobError, Notice, Status};
use crate::extract::{ArchiveProblem, ExtractError};
use crate::par2::Par2Error;
use crate::settings::LanguagePref;
use crate::sfv::{Problem, SfvError};

/// Interface language actually used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    /// English.
    En,
    /// French.
    Fr,
}

impl Lang {
    /// Resolves a preference; `System` uses the Windows display language.
    pub fn resolve(pref: LanguagePref) -> Self {
        match pref {
            LanguagePref::English => Self::En,
            LanguagePref::French => Self::Fr,
            LanguagePref::System => Self::from_locale(sys_locale::get_locale().as_deref()),
        }
    }

    /// Maps a BCP-47 locale (`fr-FR`, `en-US`…) to a supported language. Default: English.
    pub fn from_locale(locale: Option<&str>) -> Self {
        match locale {
            Some(l) if l.to_ascii_lowercase().starts_with("fr") => Self::Fr,
            _ => Self::En,
        }
    }

    /// Static labels for this language.
    pub fn texts(self) -> &'static Texts {
        match self {
            Self::En => &EN,
            Self::Fr => &FR,
        }
    }

    /// "scanned N s ago".
    pub fn scanned_ago(self, secs: u64) -> String {
        match self {
            Self::En => format!("scanned {secs} s ago"),
            Self::Fr => format!("analysé il y a {secs} s"),
        }
    }

    /// `"Using: <path>"` for the resolved par2 executable.
    pub fn par2_used(self, path: &str) -> String {
        match self {
            Self::En => format!("Using: {path}"),
            Self::Fr => format!("Utilisé : {path}"),
        }
    }

    /// Settings save failure.
    pub fn save_failed(self, error: &str) -> String {
        match self {
            Self::En => format!("Could not save the settings: {error}"),
            Self::Fr => format!("Impossible d'enregistrer les réglages : {error}"),
        }
    }

    /// Tooltip of a set name: folder and number of files.
    pub fn set_tooltip(self, dir: &str, files: usize) -> String {
        match self {
            Self::En => format!("{dir}\n{files} file(s)"),
            Self::Fr => format!("{dir}\n{files} fichier(s)"),
        }
    }
}

/// Static interface labels.
#[allow(missing_docs)] // one field per label; names are self-explanatory
#[derive(Debug)]
pub struct Texts {
    pub start: &'static str,
    pub start_hint: &'static str,
    pub pause: &'static str,
    pub pause_hint: &'static str,
    pub scan_now: &'static str,
    pub scan_now_hint: &'static str,
    pub settings: &'static str,
    pub no_folder: &'static str,
    pub watching: &'static str,
    pub paused: &'static str,
    pub open_in_explorer: &'static str,
    pub log: &'static str,
    pub errors_only: &'static str,
    pub onboarding_title: &'static str,
    pub onboarding_body: &'static str,
    pub choose_folder: &'static str,
    pub empty_running: &'static str,
    pub empty_paused: &'static str,
    pub col_name: &'static str,
    pub col_type: &'static str,
    pub col_status: &'static str,
    pub col_progress: &'static str,
    pub col_detail: &'static str,
    pub password_hint: &'static str,
    pub ok: &'static str,
    pub retry: &'static str,
    pub skip: &'static str,
    pub skip_hint: &'static str,
    pub open: &'static str,
    pub open_folder: &'static str,
    pub remove: &'static str,
    pub settings_title: &'static str,
    pub watch_dir: &'static str,
    pub destination: &'static str,
    pub destination_hint: &'static str,
    pub par2_exe: &'static str,
    pub par2_auto: &'static str,
    pub par2_missing: &'static str,
    pub browse: &'static str,
    pub scan_every: &'static str,
    pub wait_stable: &'static str,
    pub seconds: &'static str,
    pub seconds_unchanged: &'static str,
    pub recursive: &'static str,
    pub subfolder: &'static str,
    pub nested: &'static str,
    pub overwrite: &'static str,
    pub trash_after: &'static str,
    pub autostart: &'static str,
    pub passwords: &'static str,
    pub passwords_hint: &'static str,
    pub language: &'static str,
    pub theme: &'static str,
    pub follow_system: &'static str,
    pub english: &'static str,
    pub french: &'static str,
    pub light: &'static str,
    pub dark: &'static str,
    pub save: &'static str,
    pub cancel: &'static str,
    pub pick_watch_dir: &'static str,
    pub program_filter: &'static str,
}

static EN: Texts = Texts {
    start: "▶  Start",
    start_hint: "Watch the folder and process new files",
    pause: "⏸  Pause",
    pause_hint: "Suspend monitoring",
    scan_now: "🔄  Scan",
    scan_now_hint: "Scan the folder now",
    settings: "⚙  Settings",
    no_folder: "No folder is being watched.",
    watching: "Watching:",
    paused: "Paused:",
    open_in_explorer: "Open in File Explorer",
    log: "Log",
    errors_only: "Errors only",
    onboarding_title: "Choose the folder where your downloads arrive",
    onboarding_body: "Rar N Rush checks PAR2/SFV files, repairs when needed and extracts RAR archives automatically.",
    choose_folder: "Choose a folder…",
    empty_running: "Nothing to process yet.",
    empty_paused: "Monitoring is paused.",
    col_name: "Name",
    col_type: "Type",
    col_status: "Status",
    col_progress: "Progress",
    col_detail: "Detail",
    password_hint: "Password",
    ok: "OK",
    retry: "Retry",
    skip: "Skip",
    skip_hint: "Do not process this set (cancels it if running)",
    open: "Open",
    open_folder: "Open the folder in File Explorer",
    remove: "Remove from list",
    settings_title: "Settings",
    watch_dir: "Watched folder",
    destination: "Destination",
    destination_hint: "same folder as the files",
    par2_exe: "par2.exe",
    par2_auto: "automatic lookup",
    par2_missing: "par2.exe not found: PAR2 repair is unavailable.",
    browse: "Browse…",
    scan_every: "Scan every",
    wait_stable: "Wait for files to settle",
    seconds: " s",
    seconds_unchanged: " s without change",
    recursive: "Also scan sub-folders",
    subfolder: "Extract into a sub-folder named after the set",
    nested: "Extract RAR archives found inside an archive",
    overwrite: "Overwrite existing files (otherwise use “name (1).ext”)",
    trash_after: "Move PAR2/SFV/RAR files to the Recycle Bin after success",
    autostart: "Start monitoring when the app starts",
    passwords: "Passwords to try (one per line)",
    passwords_hint: "Stored in plain text in the settings file.",
    language: "Language",
    theme: "Theme",
    follow_system: "Follow Windows",
    english: "English",
    french: "Français",
    light: "Light",
    dark: "Dark",
    save: "Save",
    cancel: "Cancel",
    pick_watch_dir: "Folder to watch",
    program_filter: "Program",
};

static FR: Texts = Texts {
    start: "▶  Démarrer",
    start_hint: "Surveille le dossier et traite les nouveaux fichiers",
    pause: "⏸  Pause",
    pause_hint: "Suspend la surveillance",
    scan_now: "🔄  Analyser",
    scan_now_hint: "Analyse le dossier maintenant",
    settings: "⚙  Réglages",
    no_folder: "Aucun dossier surveillé.",
    watching: "Surveille :",
    paused: "En pause :",
    open_in_explorer: "Ouvrir dans l'Explorateur",
    log: "Journal",
    errors_only: "Erreurs seulement",
    onboarding_title: "Choisis le dossier où arrivent tes téléchargements",
    onboarding_body: "Rar N Rush vérifie les PAR2/SFV, répare si besoin et extrait les RAR automatiquement.",
    choose_folder: "Choisir un dossier…",
    empty_running: "Rien à traiter pour l'instant.",
    empty_paused: "Surveillance en pause.",
    col_name: "Nom",
    col_type: "Type",
    col_status: "Statut",
    col_progress: "Progression",
    col_detail: "Détail",
    password_hint: "Mot de passe",
    ok: "OK",
    retry: "Réessayer",
    skip: "Ignorer",
    skip_hint: "Ne pas traiter ce jeu (l'annule s'il est en cours)",
    open: "Ouvrir",
    open_folder: "Ouvrir le dossier dans l'Explorateur",
    remove: "Retirer de la liste",
    settings_title: "Réglages",
    watch_dir: "Dossier surveillé",
    destination: "Destination",
    destination_hint: "même dossier que les fichiers",
    par2_exe: "par2.exe",
    par2_auto: "recherche automatique",
    par2_missing: "par2.exe introuvable : la réparation PAR2 est indisponible.",
    browse: "Parcourir…",
    scan_every: "Analyser toutes les",
    wait_stable: "Attendre la fin des fichiers",
    seconds: " s",
    seconds_unchanged: " s sans changement",
    recursive: "Analyser aussi les sous-dossiers",
    subfolder: "Extraire dans un sous-dossier au nom du jeu",
    nested: "Extraire les archives RAR contenues dans une archive",
    overwrite: "Écraser les fichiers existants (sinon « nom (1).ext »)",
    trash_after: "Envoyer les fichiers PAR2/SFV/RAR à la corbeille après succès",
    autostart: "Démarrer la surveillance au lancement",
    passwords: "Mots de passe à essayer (un par ligne)",
    passwords_hint: "Stockés en clair dans le fichier de réglages.",
    language: "Langue",
    theme: "Thème",
    follow_system: "Suivre Windows",
    english: "English",
    french: "Français",
    light: "Clair",
    dark: "Foncé",
    save: "Enregistrer",
    cancel: "Annuler",
    pick_watch_dir: "Dossier à surveiller",
    program_filter: "Programme",
};

/// Values that can be shown to the user in several languages.
pub trait Localize {
    /// Text in `lang`.
    fn localize(&self, lang: Lang) -> String;
}

impl Localize for Status {
    fn localize(&self, lang: Lang) -> String {
        let (en, fr) = match self {
            Status::Waiting => ("Waiting", "En attente"),
            Status::Queued => ("Ready", "Prêt"),
            Status::Verifying => ("Verifying", "Vérification"),
            Status::Repairing => ("Repairing", "Réparation"),
            Status::Extracting => ("Extracting", "Extraction"),
            Status::NeedPassword => ("Password required", "Mot de passe requis"),
            Status::RepairImpossible => ("Repair impossible", "Réparation impossible"),
            Status::Done => ("Done", "Terminé"),
            Status::Failed => ("Error", "Erreur"),
            Status::Skipped => ("Skipped", "Ignoré"),
        };
        pick(lang, en, fr).to_owned()
    }
}

fn pick(lang: Lang, en: &'static str, fr: &'static str) -> &'static str {
    match lang {
        Lang::En => en,
        Lang::Fr => fr,
    }
}

impl Localize for ArchiveProblem {
    fn localize(&self, lang: Lang) -> String {
        let (en, fr) = match self {
            Self::Corrupt => ("corrupt data (CRC error)", "données corrompues (CRC incorrect)"),
            Self::Invalid => ("invalid archive", "archive invalide"),
            Self::UnknownFormat => ("unknown archive format", "format d'archive inconnu"),
            Self::MissingVolume => ("missing or unreadable volume", "volume manquant ou illisible"),
            Self::CannotCreate => (
                "cannot create a destination file",
                "impossible de créer un fichier de destination",
            ),
            Self::WriteFailed => ("write error (disk full?)", "erreur d'écriture (disque plein ?)"),
            Self::OutOfMemory => ("out of memory", "mémoire insuffisante"),
            Self::Other => ("UnRAR error", "erreur UnRAR"),
        };
        pick(lang, en, fr).to_owned()
    }
}

impl Localize for Problem {
    fn localize(&self, lang: Lang) -> String {
        match (self, lang) {
            (Self::Missing(n), Lang::En) => format!("{n} (missing)"),
            (Self::Missing(n), Lang::Fr) => format!("{n} (manquant)"),
            (Self::Mismatch(n), Lang::En) => format!("{n} (bad CRC)"),
            (Self::Mismatch(n), Lang::Fr) => format!("{n} (CRC incorrect)"),
            (Self::Unreadable(n), Lang::En) => format!("{n} (unreadable)"),
            (Self::Unreadable(n), Lang::Fr) => format!("{n} (illisible)"),
        }
    }
}

impl Localize for JobError {
    fn localize(&self, lang: Lang) -> String {
        match self {
            Self::Par2NotFound => pick(
                lang,
                "par2.exe not found: set its path in the settings",
                "par2.exe introuvable : indique son chemin dans les réglages",
            )
            .to_owned(),
            Self::Par2(e) => match (e.as_ref(), lang) {
                (Par2Error::Spawn { source, .. }, Lang::En) => format!("cannot start par2: {source}"),
                (Par2Error::Spawn { source, .. }, Lang::Fr) => {
                    format!("impossible de lancer par2 : {source}")
                }
                (Par2Error::Wait(source), _) => source.to_string(),
                (Par2Error::Failed { code, message }, Lang::En) => {
                    format!("par2 failed (code {}) {message}", code.unwrap_or(-1))
                }
                (Par2Error::Failed { code, message }, Lang::Fr) => {
                    format!("par2 a échoué (code {}) {message}", code.unwrap_or(-1))
                }
            },
            Self::Sfv(e) => match (e.as_ref(), lang) {
                (SfvError::Read { source, .. }, Lang::En) => format!("cannot read the SFV file: {source}"),
                (SfvError::Read { source, .. }, Lang::Fr) => format!("SFV illisible : {source}"),
            },
            Self::SfvMismatch(problems) => {
                let list = problems
                    .iter()
                    .map(|p| p.localize(lang))
                    .collect::<Vec<_>>()
                    .join(", ");
                match lang {
                    Lang::En => format!("SFV check failed: {list}"),
                    Lang::Fr => format!("Contrôle SFV en échec : {list}"),
                }
            }
            Self::FirstVolumeMissing => pick(
                lang,
                "first RAR volume not found",
                "premier volume RAR introuvable",
            )
            .to_owned(),
            Self::Extract(e) => match e.as_ref() {
                ExtractError::CreateDir { path, source } => match lang {
                    Lang::En => format!("cannot create {}: {source}", path.display()),
                    Lang::Fr => format!("impossible de créer {} : {source}", path.display()),
                },
                ExtractError::Archive {
                    archive,
                    entry,
                    problem,
                } => {
                    let file = entry.as_deref().unwrap_or(archive.as_path());
                    let name = file.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
                    format!("{name}: {}", problem.localize(lang))
                }
            },
        }
    }
}

impl Localize for Detail {
    fn localize(&self, lang: Lang) -> String {
        match self {
            Self::None => String::new(),
            Self::NewFiles => pick(
                lang,
                "New files arrived, trying again",
                "Nouveaux fichiers, nouvel essai",
            )
            .into(),
            Self::VerifyingPar2 => pick(lang, "PAR2 verification", "Vérification PAR2").into(),
            Self::VerifyingSfv => pick(lang, "SFV verification", "Vérification SFV").into(),
            Self::ExtractingTo(p) => match lang {
                Lang::En => format!("Extracting to {}", p.display()),
                Lang::Fr => format!("Extraction vers {}", p.display()),
            },
            Self::WaitingForPar2(details) => {
                let base = pick(
                    lang,
                    "waiting for more PAR2 files",
                    "en attente d'autres fichiers PAR2",
                );
                if details.is_empty() {
                    base.into()
                } else {
                    format!("{details} — {base}")
                }
            }
            Self::EnterPassword => pick(lang, "Type the password", "Saisis le mot de passe").into(),
            Self::WrongPassword => pick(lang, "Wrong password", "Mot de passe incorrect").into(),
            Self::Extracted { count, dest } => match lang {
                Lang::En => format!("{count} file(s) extracted to {}", dest.display()),
                Lang::Fr => format!("{count} fichier(s) extrait(s) dans {}", dest.display()),
            },
            Self::NothingToExtract => {
                pick(lang, "Verified, nothing to extract", "Vérifié, rien à extraire").into()
            }
            Self::SkippedByUser => pick(lang, "Skipped by the user", "Ignoré par l'utilisateur").into(),
            Self::Cancelled => pick(lang, "Cancelled", "Annulé").into(),
            Self::Error(e) => e.localize(lang),
        }
    }
}

impl Localize for Notice {
    fn localize(&self, lang: Lang) -> String {
        use Lang::{En, Fr};
        match (self, lang) {
            (Self::Started, En) => "Rar N Rush started.".into(),
            (Self::Started, Fr) => "Rar N Rush démarré.".into(),
            (Self::MonitoringStarted, En) => "Monitoring started.".into(),
            (Self::MonitoringStarted, Fr) => "Surveillance démarrée.".into(),
            (Self::MonitoringPaused, En) => "Monitoring paused.".into(),
            (Self::MonitoringPaused, Fr) => "Surveillance en pause.".into(),
            (Self::WatchDirMissing(p), En) => format!("Watched folder not found: {}", p.display()),
            (Self::WatchDirMissing(p), Fr) => format!("Dossier surveillé introuvable : {}", p.display()),
            (Self::ScanFailed(e), En) => format!("Scan failed: {e}"),
            (Self::ScanFailed(e), Fr) => format!("Analyse impossible : {e}"),
            (Self::Processing { name, kind }, En) => format!("Processing “{name}” ({})", kind.label()),
            (Self::Processing { name, kind }, Fr) => format!("Traitement de « {name} » ({})", kind.label()),
            (Self::Par2Output(line), _) => format!("par2: {line}"),
            (Self::AllCorrect { name }, En) => format!("“{name}”: all files are correct."),
            (Self::AllCorrect { name }, Fr) => format!("« {name} » : tous les fichiers sont corrects."),
            (Self::Repaired { name }, En) => format!("“{name}”: repaired."),
            (Self::Repaired { name }, Fr) => format!("« {name} » : réparation réussie."),
            (Self::RepairImpossible { name }, En) => format!("“{name}”: repair impossible for now."),
            (Self::RepairImpossible { name }, Fr) => {
                format!("« {name} » : réparation impossible pour l'instant.")
            }
            (Self::SfvOk { name }, En) => format!("“{name}”: SFV is correct."),
            (Self::SfvOk { name }, Fr) => format!("« {name} » : SFV correct."),
            (Self::PasswordRequired { name }, En) => format!("“{name}”: password required."),
            (Self::PasswordRequired { name }, Fr) => format!("« {name} » : mot de passe requis."),
            (Self::NestedArchive { file }, En) => format!("Nested archive: {file}"),
            (Self::NestedArchive { file }, Fr) => format!("Archive imbriquée : {file}"),
            (Self::SourcesTrashed { count }, En) => {
                format!("{count} source file(s) moved to the Recycle Bin.")
            }
            (Self::SourcesTrashed { count }, Fr) => {
                format!("{count} fichier(s) source envoyé(s) à la corbeille.")
            }
            (Self::TrashFailed(e), En) => format!("Could not move source files to the Recycle Bin: {e}"),
            (Self::TrashFailed(e), Fr) => format!("Impossible d'envoyer les sources à la corbeille : {e}"),
            (Self::Finished { name }, En) => format!("“{name}” done."),
            (Self::Finished { name }, Fr) => format!("« {name} » terminé."),
            (Self::Failed { name, error }, En) => format!("“{name}” failed: {}", error.localize(lang)),
            (Self::Failed { name, error }, Fr) => format!("« {name} » en échec : {}", error.localize(lang)),
            (Self::Cancelled { name }, En) => format!("“{name}” cancelled."),
            (Self::Cancelled { name }, Fr) => format!("« {name} » annulé."),
            (Self::StateError(e), En) => format!("Could not save state: {e}"),
            (Self::StateError(e), Fr) => format!("Impossible d'enregistrer l'état : {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_detection() {
        assert_eq!(Lang::from_locale(Some("fr-FR")), Lang::Fr);
        assert_eq!(Lang::from_locale(Some("FR")), Lang::Fr);
        assert_eq!(Lang::from_locale(Some("fr-CA")), Lang::Fr);
        assert_eq!(Lang::from_locale(Some("en-US")), Lang::En);
        assert_eq!(Lang::from_locale(Some("de-DE")), Lang::En);
        assert_eq!(Lang::from_locale(None), Lang::En);
        assert_eq!(Lang::resolve(LanguagePref::French), Lang::Fr);
        assert_eq!(Lang::resolve(LanguagePref::English), Lang::En);
    }

    #[test]
    fn every_status_has_distinct_labels() {
        let all = [
            Status::Waiting,
            Status::Queued,
            Status::Verifying,
            Status::Repairing,
            Status::Extracting,
            Status::NeedPassword,
            Status::RepairImpossible,
            Status::Done,
            Status::Failed,
            Status::Skipped,
        ];
        for lang in [Lang::En, Lang::Fr] {
            let labels: std::collections::HashSet<String> = all.iter().map(|s| s.localize(lang)).collect();
            assert_eq!(labels.len(), all.len(), "duplicate status label in {lang:?}");
        }
    }

    #[test]
    fn job_errors_are_localized() {
        let e = JobError::SfvMismatch(vec![Problem::Mismatch("a.rar".into())]);
        assert_eq!(e.localize(Lang::En), "SFV check failed: a.rar (bad CRC)");
        assert_eq!(
            e.localize(Lang::Fr),
            "Contrôle SFV en échec : a.rar (CRC incorrect)"
        );
    }
}
