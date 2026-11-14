//! Notification-area (system tray) icon and the window behaviour around it.
//!
//! The icon exists only while "minimise to tray" or "close to tray" is enabled.
//! A click on it (or **Show**) brings the window back; **Quit** really exits.
//! The decision of what to do with the window is a pure function,
//! [`window_action`], so it is unit-tested without a desktop.

use crate::i18n::Texts;
use std::sync::mpsc::{self, Receiver};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

/// Size of the icon bitmap given to Windows (scaled down as needed).
const ICON_SIZE: u32 = 32;

/// Errors raised while creating the icon.
#[derive(Debug, thiserror::Error)]
pub enum TrayError {
    /// The icon bitmap was rejected.
    #[error("invalid tray icon: {0}")]
    Icon(#[from] tray_icon::BadIcon),
    /// The menu could not be built.
    #[error("cannot build the tray menu: {0}")]
    Menu(#[from] tray_icon::menu::Error),
    /// Windows refused the icon.
    #[error("cannot create the tray icon: {0}")]
    Create(#[from] tray_icon::Error),
}

/// Request coming from the tray icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    /// Show the window again.
    Show,
    /// Exit the application.
    Quit,
}

/// The tray icon. Dropping it removes the icon.
pub struct Tray {
    _icon: TrayIcon,
    requests: Receiver<Request>,
}

impl Tray {
    /// Creates the icon. `wake` is called after each request so a hidden window
    /// still gets a chance to handle it.
    pub fn new(texts: &Texts, wake: impl Fn() + Send + Sync + 'static) -> Result<Self, TrayError> {
        let show = MenuItem::new(texts.tray_show, true, None);
        let quit = MenuItem::new(texts.tray_quit, true, None);
        let menu = Menu::new();
        menu.append(&show)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&quit)?;

        let (tx, requests) = mpsc::channel();
        let wake = std::sync::Arc::new(wake);
        {
            let (tx, wake) = (tx.clone(), wake.clone());
            let (show_id, quit_id) = (show.id().clone(), quit.id().clone());
            MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
                let request = if event.id == quit_id {
                    Request::Quit
                } else if event.id == show_id {
                    Request::Show
                } else {
                    return;
                };
                let _ = tx.send(request);
                wake();
            }));
        }
        TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
            let left_click = matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } | TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                }
            );
            if left_click {
                let _ = tx.send(Request::Show);
                wake();
            }
        }));

        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_tooltip("Rar N Rush")
            .with_icon(Icon::from_rgba(
                crate::icon::rgba(ICON_SIZE),
                ICON_SIZE,
                ICON_SIZE,
            )?)
            .build()?;
        Ok(Self {
            _icon: icon,
            requests,
        })
    }

    /// Next pending request, if any.
    pub fn poll(&self) -> Option<Request> {
        self.requests.try_recv().ok()
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        // The handlers hold a sender and the repaint callback: release them.
        MenuEvent::set_event_handler(None::<fn(MenuEvent)>);
        TrayIconEvent::set_event_handler(None::<fn(TrayIconEvent)>);
    }
}

/// What the window currently looks like and what the user just did.
#[derive(Debug, Clone, Copy, Default)]
#[allow(clippy::struct_excessive_bools)] // independent observations of one frame
pub struct WindowInput {
    /// A tray icon is available.
    pub tray: bool,
    /// The setting "minimise to tray".
    pub minimize_to_tray: bool,
    /// The setting "close to tray".
    pub close_to_tray: bool,
    /// The user asked to close the window (title bar ✕, Alt+F4).
    pub close_requested: bool,
    /// The window is minimised.
    pub minimized: bool,
    /// The window is currently hidden in the tray.
    pub hidden: bool,
    /// The user chose **Quit** in the tray menu.
    pub quitting: bool,
}

/// What to do with the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowAction {
    /// Nothing to do.
    None,
    /// Cancel the close and hide the window in the tray.
    HideInsteadOfClosing,
    /// Hide the minimised window in the tray.
    HideMinimized,
}

/// Decides how the window reacts to a close or a minimise.
/// Without a tray icon the window always behaves normally, so it can never be
/// hidden with no way to bring it back.
pub fn window_action(input: WindowInput) -> WindowAction {
    if !input.tray || input.quitting {
        return WindowAction::None;
    }
    if input.close_requested && input.close_to_tray {
        return WindowAction::HideInsteadOfClosing;
    }
    if input.minimized && input.minimize_to_tray && !input.hidden {
        return WindowAction::HideMinimized;
    }
    WindowAction::None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> WindowInput {
        WindowInput {
            tray: true,
            minimize_to_tray: true,
            close_to_tray: true,
            ..WindowInput::default()
        }
    }

    #[test]
    fn close_hides_only_when_enabled() {
        let close = WindowInput {
            close_requested: true,
            ..input()
        };
        assert_eq!(window_action(close), WindowAction::HideInsteadOfClosing);
        let disabled = WindowInput {
            close_to_tray: false,
            ..close
        };
        assert_eq!(window_action(disabled), WindowAction::None);
    }

    #[test]
    fn quit_from_the_menu_really_closes() {
        let quit = WindowInput {
            close_requested: true,
            quitting: true,
            ..input()
        };
        assert_eq!(window_action(quit), WindowAction::None);
    }

    #[test]
    fn minimise_hides_once() {
        let minimized = WindowInput {
            minimized: true,
            ..input()
        };
        assert_eq!(window_action(minimized), WindowAction::HideMinimized);
        let already = WindowInput {
            hidden: true,
            ..minimized
        };
        assert_eq!(window_action(already), WindowAction::None);
        let disabled = WindowInput {
            minimize_to_tray: false,
            ..minimized
        };
        assert_eq!(window_action(disabled), WindowAction::None);
    }

    #[test]
    fn without_tray_icon_the_window_is_never_hidden() {
        let everything = WindowInput {
            tray: false,
            close_requested: true,
            minimized: true,
            ..input()
        };
        assert_eq!(window_action(everything), WindowAction::None);
    }
}

/// Creates the real icon for an instant (it appears briefly in the notification
/// area), so it is not run by default: `cargo test -- --ignored tray`.
#[cfg(test)]
mod desktop {
    #[test]
    #[ignore = "touches the real notification area"]
    fn tray_icon_can_be_created_and_removed() {
        let tray = super::Tray::new(crate::i18n::Lang::En.texts(), || {}).unwrap();
        assert_eq!(tray.poll(), None);
        drop(tray);
    }
}
