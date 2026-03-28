use gtk4 as gtk;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;

use crate::services::config::AppConfig;

pub struct FnKeyModel {
    gesperrt: bool,
    check_gesperrt: gtk::CheckButton,
    check_normal: gtk::CheckButton,
}

#[derive(Debug)]
pub enum FnKeyMsg {
    GesperrtUmschalten(bool),
}

#[derive(Debug)]
pub enum FnKeyCommandOutput {
    InitWert(bool),
    Gesetzt(bool),
    Fehler(String),
}

const MODPROBE_PFAD: &str = "/etc/modprobe.d/asus_wmi.conf";

#[relm4::component(pub)]
impl Component for FnKeyModel {
    type Init = ();
    type Input = FnKeyMsg;
    type Output = ();
    type CommandOutput = FnKeyCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: "Funktionstaste",

            add = &adw::ActionRow {
                set_title: "Hinweis",
                set_subtitle: "Änderungen an dieser Einstellung werden erst nach einem Systemneustart wirksam.",
                set_selectable: false,
            },

            add = &adw::ActionRow {
                set_title: "Gesperrte Fn-Taste",
                set_subtitle: "Drücken Sie F1–F12, um die angegebene Schnelltasten-Funktion zu aktivieren.",
                add_prefix = &model.check_gesperrt.clone(),
                set_activatable_widget: Some(&model.check_gesperrt),
            },

            add = &adw::ActionRow {
                set_title: "Normale Fn-Taste",
                set_subtitle: "Drücken Sie F1–F12, um die F1–F12-Funktionen zu verwenden.",
                add_prefix = &model.check_normal.clone(),
                set_activatable_widget: Some(&model.check_normal),
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let check_gesperrt = gtk::CheckButton::new();
        let check_normal = gtk::CheckButton::new();

        check_normal.set_group(Some(&check_gesperrt));
        check_normal.set_active(true);

        {
            let sender = sender.clone();
            check_gesperrt.connect_toggled(move |b| {
                if b.is_active() {
                    sender.input(FnKeyMsg::GesperrtUmschalten(true));
                }
            });
        }
        {
            let sender = sender.clone();
            check_normal.connect_toggled(move |b| {
                if b.is_active() {
                    sender.input(FnKeyMsg::GesperrtUmschalten(false));
                }
            });
        }

        let model = FnKeyModel {
            gesperrt: false,
            check_gesperrt,
            check_normal,
        };

        let widgets = view_output!();

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    match tokio::fs::read_to_string(MODPROBE_PFAD).await {
                        Ok(inhalt) => {
                            let gesperrt = inhalt.contains("fnlock_default=1");
                            out.emit(FnKeyCommandOutput::InitWert(gesperrt));
                        }
                        Err(_) => {
                            // Datei existiert nicht → Normal-Modus (Standard)
                            out.emit(FnKeyCommandOutput::InitWert(false));
                        }
                    }
                })
                .drop_on_shutdown()
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: FnKeyMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            FnKeyMsg::GesperrtUmschalten(gesperrt) => {
                if gesperrt == self.gesperrt {
                    return;
                }
                self.gesperrt = gesperrt;

                AppConfig::update(|c| c.fn_key_gesperrt = gesperrt);

                let wert = if gesperrt { 1 } else { 0 };
                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            let result = tokio::task::spawn_blocking(move || {
                                std::process::Command::new("pkexec")
                                    .args([
                                        "sh",
                                        "-c",
                                        &format!(
                                            "echo 'options asus_wmi fnlock_default={wert}' > {MODPROBE_PFAD}"
                                        ),
                                    ])
                                    .status()
                            })
                            .await;

                            match result {
                                Ok(Ok(status)) if status.success() => {
                                    out.emit(FnKeyCommandOutput::Gesetzt(gesperrt));
                                }
                                Ok(Ok(status)) => {
                                    out.emit(FnKeyCommandOutput::Fehler(format!(
                                        "pkexec fehlgeschlagen mit Exit-Code: {}",
                                        status.code().unwrap_or(-1)
                                    )));
                                }
                                Ok(Err(e)) => {
                                    out.emit(FnKeyCommandOutput::Fehler(format!(
                                        "pkexec starten fehlgeschlagen: {e}"
                                    )));
                                }
                                Err(e) => {
                                    out.emit(FnKeyCommandOutput::Fehler(format!(
                                        "spawn_blocking fehlgeschlagen: {e}"
                                    )));
                                }
                            }
                        })
                        .drop_on_shutdown()
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: FnKeyCommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            FnKeyCommandOutput::InitWert(gesperrt) => {
                self.gesperrt = gesperrt;
                if gesperrt {
                    self.check_gesperrt.set_active(true);
                } else {
                    self.check_normal.set_active(true);
                }
            }
            FnKeyCommandOutput::Gesetzt(gesperrt) => {
                eprintln!(
                    "asus_wmi fnlock_default={} geschrieben (wirksam nach Neustart)",
                    if gesperrt { 1 } else { 0 }
                );
            }
            FnKeyCommandOutput::Fehler(e) => {
                eprintln!("Fehler (FnKey): {e}");
            }
        }
    }
}
