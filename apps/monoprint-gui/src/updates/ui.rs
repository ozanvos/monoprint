use crate::{studio::*, updates::Status};
#[cfg(target_os = "linux")]
use gpui_kit::component::menu::DropdownMenu;
use gpui_kit::*;
#[cfg(not(target_os = "macos"))]
use gpui_kit::{
    component::{button::*, *},
    prelude::*,
};

impl Studio {
    pub fn check_for_updates(
        &mut self,
        _: &CheckForUpdates,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Status::Disabled(reason) = &self.updates.read(cx).status {
            self.notice = Some(reason.clone());
        } else {
            self.show_update_status = true;
            self.dismissed_update_status = None;
            self.updates.update(cx, |updates, cx| updates.check(cx));
        }
        cx.notify();
    }

    pub fn toggle_automatic_updates(
        &mut self,
        _: &ToggleAutomaticUpdates,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.updates.read(cx).available() {
            return;
        }
        match self
            .updates
            .update(cx, |updates, cx| updates.toggle_automatic(cx))
        {
            Ok(()) => {
                self.notice = Some(if self.updates.read(cx).automatic {
                    "Automatic update checks are on.".into()
                } else {
                    "Automatic update checks are off. You can still check manually.".into()
                })
            }
            Err(error) => {
                self.notice = Some(format!("Could not save update preferences: {error:#}"))
            }
        }
        cx.notify();
    }

    pub fn restart_to_update(
        &mut self,
        _: &RestartToUpdate,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        #[cfg(target_os = "linux")]
        if self.installing {
            return;
        }
        let result = self
            .session
            .save(self.revision, &self.document)
            .and_then(|()| self.updates.read(cx).restart());
        match result {
            Ok(()) => cx.quit(),
            Err(error) => {
                self.notice = Some(format!("Could not restart to update: {error:#}"));
                cx.notify();
            }
        }
    }

    /// App management stays in a menu; the workspace only shows actionable or
    /// explicitly requested update feedback, which can always be dismissed.
    pub fn update_notification(&self, cx: &Context<Self>) -> Option<AnyElement> {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = cx;
            None
        }
        #[cfg(target_os = "linux")]
        {
            let updates = self.updates.read(cx);
            if self.dismissed_update_status.as_ref() == Some(&updates.status) {
                return None;
            }
            let text = updates.status.notification(self.show_update_status)?;
            let ready = matches!(updates.status, Status::Ready(_));
            let failed = matches!(updates.status, Status::Failed(_));
            let installing = self.installing;
            let p = crate::appearance::palette(cx);
            Some(
                div()
                    .id("update-notification")
                    .flex()
                    .items_center()
                    .gap_2()
                    .mx_5()
                    .mb_3()
                    .p_3()
                    .rounded_md()
                    .bg(rgb(p.accent).opacity(0.12))
                    .text_color(rgb(p.accent))
                    .child(div().flex_1().min_w_0().text_size(px(12.)).child(text))
                    .when(ready, |view| {
                        view.child(
                            Button::new("restart-to-update")
                                .small()
                                .primary()
                                .label("Restart to update")
                                .disabled(installing)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.restart_to_update(&RestartToUpdate, window, cx)
                                })),
                        )
                    })
                    .when(failed, |view| {
                        view.child(Button::new("retry-update").small().label("Retry").on_click(
                            cx.listener(|this, _, window, cx| {
                                this.check_for_updates(&CheckForUpdates, window, cx)
                            }),
                        ))
                    })
                    .child(
                        Button::new("dismiss-update")
                            .small()
                            .ghost()
                            .label("Dismiss")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.show_update_status = false;
                                this.dismissed_update_status =
                                    Some(this.updates.read(cx).status.clone());
                                cx.notify();
                            })),
                    )
                    .into_any_element(),
            )
        }
    }

    pub fn linux_title_bar(&self, cx: &Context<Self>) -> AnyElement {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = cx;
            div().into_any_element()
        }
        #[cfg(target_os = "linux")]
        {
            let updates = self.updates.clone();
            let focus = self.focus.clone();
            let installed = self.installed_for_user;
            let installing = self.installing;
            let p = crate::appearance::palette(cx);
            TitleBar::new()
                .bg(rgb(p.surface))
                .border_color(rgb(p.border))
                .on_close_window(|_, _, cx| cx.quit())
                .child(
                    Button::new("application-menu")
                        .small()
                        .ghost()
                        .label("Monoprint")
                        .icon(gpui_kit::assets::IconName::ChevronDown)
                        .accessibility_label("Monoprint menu")
                        .dropdown_menu(move |menu, _, cx| {
                            let installable = std::env::var_os("APPIMAGE").is_some() && !installed;
                            let updates = updates.read(cx);
                            let busy =
                                matches!(updates.status, Status::Checking | Status::Downloading);
                            let ready = matches!(updates.status, Status::Ready(_));
                            menu.action_context(focus.clone())
                                .menu("Open Source…", Box::new(OpenSource))
                                .menu("Export Image…", Box::new(ExportImage))
                                .separator()
                                .menu_with_disabled(
                                    "Check for Updates…",
                                    Box::new(CheckForUpdates),
                                    busy || ready,
                                )
                                .menu_with_check_and_disabled(
                                    "Automatic Update Checks",
                                    updates.automatic,
                                    Box::new(ToggleAutomaticUpdates),
                                    !updates.available(),
                                )
                                .when(ready, |menu| {
                                    menu.menu_with_disabled(
                                        "Restart to Update",
                                        Box::new(RestartToUpdate),
                                        installing,
                                    )
                                })
                                .when(installable, |menu| {
                                    menu.separator().menu_with_disabled(
                                        "Install for This User…",
                                        Box::new(InstallForUser),
                                        installing,
                                    )
                                })
                                .when(installed, |menu| {
                                    menu.separator().menu_with_disabled(
                                        "Uninstall Monoprint…",
                                        Box::new(UninstallApp),
                                        installing,
                                    )
                                })
                                .separator()
                                .menu("Monoprint Support", Box::new(OpenSupport))
                                .menu("Quit Monoprint", Box::new(Quit))
                        }),
                )
                .into_any_element()
        }
    }

    pub fn update_bar(&self, cx: &Context<Self>) -> Option<AnyElement> {
        #[cfg(not(target_os = "windows"))]
        {
            let _ = cx;
            None
        }
        #[cfg(target_os = "windows")]
        {
            let updates = self.updates.read(cx);
            if !updates.available() {
                return None;
            }
            let text = match &updates.status {
                Status::Idle => "Monoprint is ready".into(),
                Status::Disabled(reason) => reason.clone(),
                Status::Checking => "Checking for updates…".into(),
                Status::Downloading => "Downloading update…".into(),
                Status::Current => "Monoprint is up to date".into(),
                Status::Ready(version) => format!("Monoprint {version} is ready to install"),
                Status::Failed(error) => format!("Could not update: {error}"),
            };
            let ready = matches!(updates.status, Status::Ready(_));
            let busy = matches!(updates.status, Status::Checking | Status::Downloading);
            let automatic = updates.automatic;
            let available = updates.available();
            let p = crate::appearance::palette(cx);
            Some(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .border_t_1()
                    .border_color(rgb(p.border))
                    .child(div().flex_1().min_w_0().text_size(px(12.)).child(text))
                    .when(available, |bar| {
                        bar.child(
                            Button::new("automatic-updates")
                                .small()
                                .label(if automatic {
                                    "Automatic checks: On"
                                } else {
                                    "Automatic checks: Off"
                                })
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.toggle_automatic_updates(
                                        &ToggleAutomaticUpdates,
                                        window,
                                        cx,
                                    )
                                })),
                        )
                        .child(
                            Button::new("check-for-updates")
                                .small()
                                .label("Check for updates")
                                .disabled(busy || ready)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.check_for_updates(&CheckForUpdates, window, cx)
                                })),
                        )
                    })
                    .when(ready, |bar| {
                        bar.child(
                            Button::new("restart-to-update")
                                .small()
                                .primary()
                                .label("Restart to update")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.restart_to_update(&RestartToUpdate, window, cx)
                                })),
                        )
                    })
                    .into_any_element(),
            )
        }
    }

    pub fn install_for_user(&mut self, _: &InstallForUser, _: &mut Window, cx: &mut Context<Self>) {
        #[cfg(target_os = "linux")]
        {
            if self.installing {
                return;
            }
            let Some(source) = std::env::var_os("APPIMAGE").map(std::path::PathBuf::from) else {
                self.notice =
                    Some("Download the AppImage to install Monoprint for this user.".into());
                cx.notify();
                return;
            };
            let install = match super::linux_install::Installation::for_user() {
                Ok(install) => install,
                Err(error) => {
                    self.notice = Some(error.to_string());
                    cx.notify();
                    return;
                }
            };
            if let Err(error) = self.session.save(self.revision, &self.document) {
                self.notice = Some(format!(
                    "Save the current draft before installing: {error:#}"
                ));
                cx.notify();
                return;
            }
            self.installing = true;
            self.notice = Some("Installing Monoprint for this user…".into());
            let install = cx
                .background_executor()
                .spawn(async move { install.install(&source) });
            cx.spawn(async move |this, cx| {
                let result = install.await;
                let _ = this.update(cx, |this, cx| {
                    this.installing = false;
                    match result.and_then(|path| {
                        this.installed_for_user = true;
                        // Edits may have continued during the AppImage copy.
                        this.session.save(this.revision, &this.document)?;
                        std::process::Command::new(path)
                            .env_remove("APPIMAGE")
                            .env_remove("APPDIR")
                            .spawn()?;
                        Ok(())
                    }) {
                        Ok(()) => cx.quit(),
                        Err(error) => {
                            this.notice =
                                Some(format!("Could not install or launch Monoprint: {error:#}"));
                            cx.notify();
                        }
                    }
                });
            })
            .detach();
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = cx;
        }
    }

    pub fn uninstall_app(&mut self, _: &UninstallApp, window: &mut Window, cx: &mut Context<Self>) {
        #[cfg(target_os = "linux")]
        {
            if self.installing {
                return;
            }
            let install = match super::linux_install::Installation::for_user() {
                Ok(install) if install.installed() => install,
                _ => {
                    self.notice = Some("This copy was not installed by Monoprint. Remove it using its package manager or delete the downloaded AppImage.".into());
                    cx.notify();
                    return;
                }
            };
            let prompt = window.prompt(PromptLevel::Warning, "Uninstall Monoprint?", Some("Remove the installed AppImage and its application menu entry. Choose whether to keep your saved draft."), &["Cancel", "Uninstall and Keep Draft", "Uninstall and Delete Draft"], cx);
            cx.spawn(async move |this, cx| {
                let Ok(answer) = prompt.await else {
                    return;
                };
                if answer == 0 {
                    return;
                }
                let _ = this.update(cx, |this, cx| {
                    // Disable all queued preview/quit saves before opt-in data
                    // removal, otherwise shutdown would recreate the draft.
                    let remove_data = answer == 2;
                    let result = this
                        .session
                        .save(this.revision, &this.document)
                        .and_then(|()| {
                            if remove_data {
                                this.session.set_saving_enabled(false);
                            }
                            install.uninstall(remove_data)
                        });
                    match result {
                        Ok(()) => cx.quit(),
                        Err(error) => {
                            this.installed_for_user = install.installed();
                            this.session.set_saving_enabled(true);
                            this.notice = Some(format!("Could not uninstall: {error:#}"));
                            cx.notify();
                        }
                    }
                });
            })
            .detach();
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (window, cx);
        }
    }
}
