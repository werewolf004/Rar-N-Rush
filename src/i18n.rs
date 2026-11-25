//! User-facing text in English and French.
//!
//! Static labels live in [`Texts`]; messages built from engine events go through
//! the [`Localize`] trait. Code, logs for developers and error `Display` impls
//! stay in English.

use crate::engine::{Detail, JobError, Notice, Status};
use crate::extract::{ArchiveProblem, ExtractError};
use crate::par2::Par2Error;
use crate::settings::{LanguagePref, LoadIssue};
use crate::sfv::{Problem, SfvError};
use crate::tidy::{NotMoved, TidyError};

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

    /// Warning shown when the passwords had to stay in the settings file.
    /// A Windows notification could not be shown or set up.
    pub fn notify_failed(self, error: &str) -> String {
        match self {
            Self::En => format!("Windows notification failed ({error})."),
            Self::Fr => format!("Échec de la notification Windows ({error})."),
        }
    }

    /// The start-with-Windows entry could not be written or removed.
    pub fn autostart_failed(self, error: &str) -> String {
        match self {
            Self::En => format!("Could not update the start with Windows ({error})."),
            Self::Fr => format!("Impossible de mettre à jour le démarrage avec Windows ({error})."),
        }
    }

    /// `"Settings saved, but…"` when the passwords stayed in the settings file.
    pub fn passwords_in_file(self, error: &str) -> String {
        match self {
            Self::En => format!(
                "Settings saved, but the archive passwords could not be stored in the Windows Credential Manager ({error}). They were kept in the settings file, in plain text."
            ),
            Self::Fr => format!(
                "Réglages enregistrés, mais les mots de passe d'archives n'ont pas pu être placés dans le Gestionnaire d'identifiants Windows ({error}). Ils ont été conservés en clair dans le fichier de réglages."
            ),
        }
    }

    /// Log export failure.
    pub fn export_failed(self, error: &str) -> String {
        match self {
            Self::En => format!("Could not export the log: {error}"),
            Self::Fr => format!("Impossible d'exporter le journal : {error}"),
        }
    }

    /// Settings save failure.
    pub fn save_failed(self, error: &str) -> String {
        match self {
            Self::En => format!("Could not save the settings: {error}"),
            Self::Fr => format!("Impossible d'enregistrer les réglages : {error}"),
        }
    }

    /// A newer release exists (banner and Windows notification).
    pub fn update_available(self, latest: &str, current: &str) -> String {
        match self {
            Self::En => format!("Rar N Rush {latest} is available (you have {current})."),
            Self::Fr => format!("Rar N Rush {latest} est disponible (tu as la {current})."),
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
    pub about: &'static str,
    pub about_hint: &'static str,
    pub about_title: &'static str,
    pub about_description: &'static str,
    pub version: &'static str,
    pub about_source: &'static str,
    pub about_guide: &'static str,
    pub about_releases: &'static str,
    pub about_licence: &'static str,
    pub about_credits: &'static str,
    pub about_offline: &'static str,
    pub about_close: &'static str,
    pub no_folder: &'static str,
    pub watching: &'static str,
    pub paused: &'static str,
    pub open_in_explorer: &'static str,
    pub watch_hint: &'static str,
    pub watch_apply_hint: &'static str,
    pub recent_folders: &'static str,
    pub folder_missing: &'static str,
    pub log: &'static str,
    pub errors_only: &'static str,
    pub export_log: &'static str,
    pub export_log_hint: &'static str,
    pub text_filter: &'static str,
    pub log_to_file: &'static str,
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
    pub tab_general: &'static str,
    pub settings_hint: &'static str,
    pub problem_watch_dir: &'static str,
    pub problem_dest_not_dir: &'static str,
    pub problem_dest_read_only: &'static str,
    pub problem_par2: &'static str,
    pub watch_dir_unavailable: &'static str,
    pub discard_question: &'static str,
    pub overwrite_question: &'static str,
    pub enable: &'static str,
    pub show_toasts: &'static str,
    pub discard: &'static str,
    pub keep_editing: &'static str,
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
    pub group_processing: &'static str,
    pub group_cleanup: &'static str,
    pub group_window: &'static str,
    pub delete_empty_dirs: &'static str,
    pub move_extra_files: &'static str,
    pub move_extra_files_hint: &'static str,
    pub follow_active: &'static str,
    pub minimize_to_tray: &'static str,
    pub close_to_tray: &'static str,
    pub close_to_tray_hint: &'static str,
    pub start_with_windows: &'static str,
    pub start_with_windows_hint: &'static str,
    pub system_notifications: &'static str,
    pub system_notifications_hint: &'static str,
    pub tray_show: &'static str,
    pub tray_quit: &'static str,
    pub tray_failed: &'static str,
    pub passwords: &'static str,
    pub passwords_hint: &'static str,
    pub passwords_hint_vault: &'static str,
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
    pub pick_extract_dir: &'static str,
    pub extract_hint: &'static str,
    pub extract_apply_hint: &'static str,
    pub program_filter: &'static str,
    pub updates: &'static str,
    pub check_updates: &'static str,
    pub check_updates_hint: &'static str,
    pub daily: &'static str,
    pub weekly: &'static str,
    pub monthly: &'static str,
    pub update_release: &'static str,
    pub update_later: &'static str,
}

static EN: Texts = Texts {
    start: "▶  Start",
    start_hint: "Watch the folder and process new files",
    pause: "⏸  Pause",
    pause_hint: "Suspend monitoring",
    scan_now: "🔄  Scan",
    scan_now_hint: "Scan the folder now (F5)",
    settings: "⚙  Settings",
    about: "ℹ  About",
    about_hint: "Version, licence and links (F1)",
    about_title: "About Rar N Rush",
    about_description: "Watches a download folder, verifies PAR2/SFV sets, repairs them and extracts RAR archives automatically.",
    version: "Version",
    about_source: "Source code on GitHub",
    about_guide: "User guide",
    about_releases: "Releases and changelog",
    about_licence: "Licence: MIT OR Apache-2.0, free and open source.",
    about_credits: "PAR2 repair: par2cmdline-turbo (GPL-2.0), shipped unmodified. RAR extraction: the UnRAR library by Alexander Roshal.",
    about_offline: "Rar N Rush only goes online to check for updates, and only if you turn it on in the settings. These links open in your browser.",
    about_close: "Close",
    no_folder: "No folder is being watched.",
    watching: "Watching",
    paused: "Paused",
    open_in_explorer: "Open in File Explorer",
    watch_hint: "Type a folder path or browse…",
    watch_apply_hint: "Press Enter to watch this folder, Esc to cancel",
    recent_folders: "Recent folders",
    folder_missing: "This folder does not exist.",
    log: "Log",
    errors_only: "Errors only",
    export_log: "Export…",
    export_log_hint: "Save the lines shown to a text file",
    text_filter: "Text file",
    log_to_file: "Also write the log to rar-n-rush.log (next to the settings)",
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
    remove: "Remove from list (Del)",
    settings_title: "Settings",
    tab_general: "General",
    settings_hint: "Open the settings (Ctrl+,)",
    problem_watch_dir: "Watched folder: this folder does not exist.",
    problem_dest_not_dir: "Destination: this path is a file, not a folder.",
    problem_dest_read_only: "Destination: this folder cannot be written to.",
    problem_par2: "par2.exe: no file at this path.",
    watch_dir_unavailable: "Watched folder not found right now (drive unplugged?). The settings can still be saved; monitoring stays paused until the folder is back.",
    discard_question: "Discard your changes?",
    overwrite_question: "Overwriting replaces existing files for good, with no copy in the Recycle Bin. Enable it?",
    enable: "Enable",
    show_toasts: "Show a short notice when a set finishes, fails or needs a password",
    discard: "Discard",
    keep_editing: "Keep editing",
    watch_dir: "Watched folder",
    destination: "Extraction folder",
    destination_hint: "same as the watched folder",
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
    overwrite: "Overwrite existing files (otherwise they are kept as they are)",
    trash_after: "Move PAR2/SFV/RAR files to the Recycle Bin after success",
    autostart: "Start monitoring when the app starts",
    group_processing: "Processing",
    group_cleanup: "After a successful run",
    group_window: "Window",
    delete_empty_dirs: "Move folders left empty to the Recycle Bin",
    move_extra_files: "Move the other files of the set (.nfo, samples…) to the destination",
    move_extra_files_hint: "Only when the set is alone in its own sub-folder.",
    follow_active: "Keep the set being processed in view",
    minimize_to_tray: "Minimise to the notification area",
    close_to_tray: "Close to the notification area",
    close_to_tray_hint: "Quit from the icon menu in the notification area.",
    start_with_windows: "Start with Windows (minimised)",
    start_with_windows_hint: "Starts when you sign in to Windows. With \u{201c}Minimise to the notification area\u{201d}, only the icon appears.",
    system_notifications: "Windows notifications when the window is not in front",
    system_notifications_hint: "When a set finishes, fails or needs a password. They follow the Windows settings (Do not disturb, Focus).",
    tray_show: "Show Rar N Rush",
    tray_quit: "Quit",
    tray_failed: "The notification area icon could not be created:",
    passwords: "Passwords to try (one per line)",
    passwords_hint: "Stored in plain text in the settings file (the Windows Credential Manager is unavailable).",
    passwords_hint_vault: "Stored in the Windows Credential Manager, not in the settings file.",
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
    pick_extract_dir: "Folder to extract into",
    extract_hint: "Same as the watched folder",
    extract_apply_hint: "Press Enter to extract into this folder (empty: the watched folder), Esc to cancel",
    program_filter: "Program",
    updates: "Updates",
    check_updates: "Check for updates",
    check_updates_hint: "About a minute after start-up, asks GitHub for the number of the latest release. Nothing else is sent, and nothing is downloaded or installed: you choose when to update.",
    daily: "Every day",
    weekly: "Once a week",
    monthly: "Once a month",
    update_release: "See the release",
    update_later: "Later",
};

static FR: Texts = Texts {
    start: "▶  Démarrer",
    start_hint: "Surveille le dossier et traite les nouveaux fichiers",
    pause: "⏸  Pause",
    pause_hint: "Suspend la surveillance",
    scan_now: "🔄  Analyser",
    scan_now_hint: "Analyse le dossier maintenant (F5)",
    settings: "⚙  Réglages",
    about: "ℹ  À propos",
    about_hint: "Version, licence et liens (F1)",
    about_title: "À propos de Rar N Rush",
    about_description: "Surveille un dossier de téléchargements, vérifie les fichiers PAR2/SFV, les répare et extrait les archives RAR automatiquement.",
    version: "Version",
    about_source: "Code source sur GitHub",
    about_guide: "Guide utilisateur",
    about_releases: "Versions et journal des modifications",
    about_licence: "Licence : MIT OR Apache-2.0, libre et gratuit.",
    about_credits: "Réparation PAR2 : par2cmdline-turbo (GPL-2.0), livré sans modification. Extraction RAR : la bibliothèque UnRAR d'Alexander Roshal.",
    about_offline: "Rar N Rush ne se connecte à Internet que pour vérifier les mises à jour, et seulement si tu l'actives dans les réglages. Ces liens s'ouvrent dans ton navigateur.",
    about_close: "Fermer",
    no_folder: "Aucun dossier surveillé.",
    watching: "Surveillance active",
    paused: "En pause",
    open_in_explorer: "Ouvrir dans l'Explorateur",
    watch_hint: "Saisis le chemin d'un dossier ou parcours…",
    watch_apply_hint: "Entrée pour surveiller ce dossier, Échap pour annuler",
    recent_folders: "Dossiers récents",
    folder_missing: "Ce dossier n'existe pas.",
    log: "Journal",
    errors_only: "Erreurs seulement",
    export_log: "Exporter…",
    export_log_hint: "Enregistrer les lignes affichées dans un fichier texte",
    text_filter: "Fichier texte",
    log_to_file: "Écrire aussi le journal dans rar-n-rush.log (à côté des réglages)",
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
    skip_hint: "Ne pas faire cette extraction (l'annule si elle est en cours)",
    open: "Ouvrir",
    open_folder: "Ouvrir le dossier dans l'Explorateur",
    remove: "Retirer de la liste (Suppr)",
    settings_title: "Réglages",
    tab_general: "Général",
    settings_hint: "Ouvrir les réglages (Ctrl+,)",
    problem_watch_dir: "Dossier surveillé : ce dossier n'existe pas.",
    problem_dest_not_dir: "Destination : ce chemin est un fichier, pas un dossier.",
    problem_dest_read_only: "Destination : impossible d'écrire dans ce dossier.",
    problem_par2: "par2.exe : aucun fichier à ce chemin.",
    watch_dir_unavailable: "Dossier surveillé introuvable pour l'instant (disque débranché ?). Les réglages peuvent quand même être enregistrés ; la surveillance reste en pause jusqu'à son retour.",
    discard_question: "Abandonner tes modifications ?",
    overwrite_question: "L'écrasement remplace les fichiers existants définitivement, sans copie dans la corbeille. L'activer ?",
    enable: "Activer",
    show_toasts: "Afficher un court message quand une extraction se termine, échoue ou demande un mot de passe",
    discard: "Abandonner",
    keep_editing: "Continuer",
    watch_dir: "Dossier surveillé",
    destination: "Dossier d'extraction",
    destination_hint: "identique au dossier surveillé",
    par2_exe: "par2.exe",
    par2_auto: "recherche automatique",
    par2_missing: "par2.exe introuvable : la réparation PAR2 est indisponible.",
    browse: "Parcourir…",
    scan_every: "Analyser toutes les",
    wait_stable: "Attendre la fin des fichiers",
    seconds: " s",
    seconds_unchanged: " s sans changement",
    recursive: "Analyser aussi les sous-dossiers",
    subfolder: "Extraire dans un sous-dossier au nom de l'archive",
    nested: "Extraire les archives RAR contenues dans une archive",
    overwrite: "Écraser les fichiers existants (sinon ils sont gardés tels quels)",
    trash_after: "Envoyer les fichiers PAR2/SFV/RAR à la corbeille après succès",
    autostart: "Démarrer la surveillance au lancement",
    group_processing: "Traitement",
    group_cleanup: "Après un traitement réussi",
    group_window: "Fenêtre",
    delete_empty_dirs: "Envoyer les dossiers restés vides à la corbeille",
    move_extra_files: "Déplacer les autres fichiers (.nfo, échantillons…) vers le dossier d'extraction",
    move_extra_files_hint: "Uniquement si l'archive est seule dans son propre sous-dossier.",
    follow_active: "Garder visible l'extraction en cours",
    minimize_to_tray: "Réduire dans la zone de notification",
    close_to_tray: "Fermer dans la zone de notification",
    close_to_tray_hint: "Pour quitter, utilise le menu de l'icône dans la zone de notification.",
    start_with_windows: "Démarrer avec Windows (réduit)",
    start_with_windows_hint: "Démarre à l'ouverture de ta session Windows. Avec « Réduire dans la zone de notification », seule l'icône apparaît.",
    system_notifications: "Notifications Windows quand la fenêtre n'est pas au premier plan",
    system_notifications_hint: "Quand une extraction se termine, échoue ou demande un mot de passe. Elles suivent les réglages de Windows (Ne pas déranger, Concentration).",
    tray_show: "Afficher Rar N Rush",
    tray_quit: "Quitter",
    tray_failed: "Impossible de créer l'icône de la zone de notification :",
    passwords: "Mots de passe à essayer (un par ligne)",
    passwords_hint: "Stockés en clair dans le fichier de réglages (le Gestionnaire d'identifiants Windows est indisponible).",
    passwords_hint_vault: "Stockés dans le Gestionnaire d'identifiants Windows, pas dans le fichier de réglages.",
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
    pick_extract_dir: "Dossier où extraire",
    extract_hint: "Identique au dossier surveillé",
    extract_apply_hint: "Entrée pour extraire dans ce dossier (vide : le dossier surveillé), Échap pour annuler",
    program_filter: "Programme",
    updates: "Mises à jour",
    check_updates: "Vérifier les mises à jour",
    check_updates_hint: "Environ une minute après le lancement, demande à GitHub le numéro de la dernière version. Rien d'autre n'est envoyé, et rien n'est téléchargé ni installé : tu choisis quand mettre à jour.",
    daily: "Tous les jours",
    weekly: "Une fois par semaine",
    monthly: "Une fois par mois",
    update_release: "Voir la release",
    update_later: "Plus tard",
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
                ExtractError::NotEnoughSpace {
                    dest,
                    needed,
                    available,
                } => match lang {
                    Lang::En => format!(
                        "not enough free space in {}: {} needed, {} available",
                        dest.display(),
                        human_size(*needed, lang),
                        human_size(*available, lang)
                    ),
                    Lang::Fr => format!(
                        "espace libre insuffisant dans {} : {} nécessaires, {} disponibles",
                        dest.display(),
                        human_size(*needed, lang),
                        human_size(*available, lang)
                    ),
                },
            },
        }
    }
}

/// Size with one decimal and a binary unit: `1.5 GB` in English, `1,5 Go` in French.
pub fn human_size(bytes: u64, lang: Lang) -> String {
    let units: [&str; 5] = match lang {
        Lang::En => ["B", "KB", "MB", "GB", "TB"],
        Lang::Fr => ["o", "Ko", "Mo", "Go", "To"],
    };
    let mut unit = 0;
    let mut whole = bytes;
    while whole >= 1024 * 1024 && unit < units.len() - 2 {
        whole /= 1024;
        unit += 1;
    }
    // `whole` is below 1024² here, so tenths are computed exactly in integers.
    if whole < 1024 {
        return format!("{whole} {}", units.get(unit).unwrap_or(&"B"));
    }
    let tenths = whole * 10 / 1024;
    let sep = if lang == Lang::Fr { "," } else { "." };
    let unit = units.get(unit + 1).unwrap_or(&"TB");
    format!("{}{sep}{} {unit}", tenths / 10, tenths % 10)
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
            Self::Extracted { count, kept: 0, dest } => match lang {
                Lang::En => format!("{count} file(s) extracted to {}", dest.display()),
                Lang::Fr => format!("{count} fichier(s) extrait(s) dans {}", dest.display()),
            },
            Self::Extracted { count, kept, dest } => match lang {
                Lang::En => format!(
                    "{count} file(s) extracted to {}, {kept} already there and kept",
                    dest.display()
                ),
                Lang::Fr => format!(
                    "{count} fichier(s) extrait(s) dans {}, {kept} déjà présent(s) et gardé(s)",
                    dest.display()
                ),
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

impl Localize for LoadIssue {
    fn localize(&self, lang: Lang) -> String {
        let backup_note = |backup: &Option<std::path::PathBuf>| match (backup, lang) {
            (Some(b), Lang::En) => format!(" A copy of the original file was saved as {}.", b.display()),
            (Some(b), Lang::Fr) => format!(
                " Une copie du fichier d'origine a été enregistrée sous {}.",
                b.display()
            ),
            (None, Lang::En) => " The original file could not be copied.".to_owned(),
            (None, Lang::Fr) => " Le fichier d'origine n'a pas pu être copié.".to_owned(),
        };
        match (self, lang) {
            (Self::Unreadable { path, error }, Lang::En) => format!(
                "The settings file {} could not be read ({error}). Default settings are used for now; your file is kept and will not be replaced while it cannot be read.",
                path.display()
            ),
            (Self::Unreadable { path, error }, Lang::Fr) => format!(
                "Le fichier de réglages {} est illisible ({error}). Les réglages par défaut sont utilisés pour l'instant ; ton fichier est conservé et ne sera pas remplacé tant qu'il est illisible.",
                path.display()
            ),
            (Self::Corrupt { backup }, Lang::En) => {
                format!(
                    "The settings file was damaged; default settings are used.{}",
                    backup_note(backup)
                )
            }
            (Self::Corrupt { backup }, Lang::Fr) => {
                format!(
                    "Le fichier de réglages était endommagé ; les réglages par défaut sont utilisés.{}",
                    backup_note(backup)
                )
            }
            (Self::InvalidValues { keys, backup }, Lang::En) => format!(
                "Invalid values were reset to their default: {}. Your other settings were kept.{}",
                keys.join(", "),
                backup_note(backup)
            ),
            (Self::InvalidValues { keys, backup }, Lang::Fr) => format!(
                "Des valeurs invalides ont été remises par défaut : {}. Tes autres réglages ont été conservés.{}",
                keys.join(", "),
                backup_note(backup)
            ),
            (Self::NotUtf8 { invalid_keys, backup }, Lang::En) => {
                let reset = if invalid_keys.is_empty() {
                    String::new()
                } else {
                    format!(
                        " Invalid values were reset to their default: {}.",
                        invalid_keys.join(", ")
                    )
                };
                format!(
                    "The settings file was not saved in UTF-8. It was read as Windows-1252 and all your settings were kept; it will be saved in UTF-8.{reset}{}",
                    backup_note(backup)
                )
            }
            (Self::NotUtf8 { invalid_keys, backup }, Lang::Fr) => {
                let reset = if invalid_keys.is_empty() {
                    String::new()
                } else {
                    format!(
                        " Des valeurs invalides ont été remises par défaut : {}.",
                        invalid_keys.join(", ")
                    )
                };
                format!(
                    "Le fichier de réglages n'était pas enregistré en UTF-8. Il a été lu en Windows-1252 et tous tes réglages ont été conservés ; il sera enregistré en UTF-8.{reset}{}",
                    backup_note(backup)
                )
            }
            (Self::Vault { error }, Lang::En) => format!(
                "The Windows Credential Manager could not be read ({error}). Your saved archive passwords are not available in this session; they were not deleted."
            ),
            (Self::Vault { error }, Lang::Fr) => format!(
                "Le Gestionnaire d'identifiants Windows est illisible ({error}). Tes mots de passe d'archives enregistrés ne sont pas disponibles pour cette session ; ils n'ont pas été supprimés."
            ),
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
            (Self::ExtraFilesMoved { count, dest }, En) => {
                format!("{count} extra file(s) moved to {}.", dest.display())
            }
            (Self::ExtraFilesMoved { count, dest }, Fr) => {
                format!("{count} fichier(s) annexe(s) déplacé(s) vers {}.", dest.display())
            }
            (Self::ExtraFilesKept { name, reason }, _) => extra_files_kept(*reason, name, lang),
            (Self::EmptyFoldersTrashed { count }, En) => {
                format!("{count} empty folder(s) moved to the Recycle Bin.")
            }
            (Self::EmptyFoldersTrashed { count }, Fr) => {
                format!("{count} dossier(s) vide(s) envoyé(s) à la corbeille.")
            }
            (Self::TidyFailed(e), _) => e.localize(lang),
            (Self::ProcessingAgain { name }, En) => {
                format!("“{name}” was processed before: processing it again.")
            }
            (Self::ProcessingAgain { name }, Fr) => {
                format!("« {name} » a déjà été extrait : nouvelle extraction.")
            }
            (Self::ExistingKept { name, count }, En) => format!(
                "“{name}”: {count} file(s) already in the extraction folder were kept (overwriting is off)."
            ),
            (Self::ExistingKept { name, count }, Fr) => format!(
                "« {name} » : {count} fichier(s) déjà présent(s) dans le dossier d'extraction gardé(s) (écrasement désactivé)."
            ),
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

impl Localize for TidyError {
    fn localize(&self, lang: Lang) -> String {
        use Lang::{En, Fr};
        match (self, lang) {
            (TidyError::ReadDir { path, source }, En) => format!("Cannot read {}: {source}", path.display()),
            (TidyError::ReadDir { path, source }, Fr) => {
                format!("Lecture de {} impossible : {source}", path.display())
            }
            (TidyError::Move { from, to, source }, En) => {
                format!("Cannot move {} to {}: {source}", from.display(), to.display())
            }
            (TidyError::Move { from, to, source }, Fr) => {
                format!(
                    "Impossible de déplacer {} vers {} : {source}",
                    from.display(),
                    to.display()
                )
            }
            (TidyError::Trash { path, message }, En) => {
                format!("Cannot send {} to the Recycle Bin: {message}", path.display())
            }
            (TidyError::Trash { path, message }, Fr) => {
                format!(
                    "Impossible d'envoyer {} à la corbeille : {message}",
                    path.display()
                )
            }
        }
    }
}

/// Message explaining why the extra files of set `name` were kept in place.
fn extra_files_kept(reason: NotMoved, name: &str, lang: Lang) -> String {
    use Lang::{En, Fr};
    match (reason, lang) {
        (NotMoved::InWatchedFolder, En) => {
            format!("“{name}”: extra files kept in place (set not in a folder of its own).")
        }
        (NotMoved::InWatchedFolder, Fr) => {
            format!("« {name} » : fichiers annexes laissés en place (archive sans dossier à elle).")
        }
        (NotMoved::SharedFolder, En) => {
            format!("“{name}”: extra files kept in place (folder shared with other sets).")
        }
        (NotMoved::SharedFolder, Fr) => {
            format!(
                "« {name} » : fichiers annexes laissés en place (dossier partagé avec d'autres archives)."
            )
        }
        (NotMoved::SameFolder, En) => {
            format!("“{name}”: extra files kept in place (extracted into the same folder).")
        }
        (NotMoved::SameFolder, Fr) => {
            format!("« {name} » : fichiers annexes laissés en place (extraction dans le même dossier).")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_are_readable_in_both_languages() {
        assert_eq!(human_size(0, Lang::En), "0 B");
        assert_eq!(human_size(1023, Lang::En), "1023 B");
        assert_eq!(human_size(1536, Lang::En), "1.5 KB");
        assert_eq!(human_size(64 * 1024 * 1024, Lang::Fr), "64,0 Mo");
        assert_eq!(human_size(3 * 1024 * 1024 * 1024 / 2, Lang::En), "1.5 GB");
        assert_eq!(human_size(u64::MAX, Lang::En), "16777215.9 TB"); // rounded down
    }

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
