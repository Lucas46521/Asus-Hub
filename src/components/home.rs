// Asus Hub - Unofficial Control Center for Asus Laptops
// Copyright (C) 2026 Guido Philipp
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see https://www.gnu.org/licenses/.

use gtk4 as gtk;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use crate::services::commands::pkexec_read;

pub struct HomeModel {
    status_page: adw::StatusPage,
    board_row: adw::ActionRow,
    bios_row: adw::ActionRow,
    kernel_row: adw::ActionRow,
    serial_row: adw::ActionRow,
    reveal_button: gtk::Button,
}

#[derive(Debug)]
pub enum HomeMsg {
    RevealSerial,
}

#[derive(Debug)]
pub enum HomeCommandOutput {
    DataLoaded {
        product_name: String,
        board_name: String,
        bios_version: String,
        bios_date: String,
        kernel: String,
    },
    SerialRevealed(Result<String, String>),
}

#[relm4::component(pub)]
impl Component for HomeModel {
    type Init = ();
    type Input = HomeMsg;
    type Output = String;
    type CommandOutput = HomeCommandOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_vexpand: true,

            append = &model.status_page.clone(),

            append = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_start: 12,
                set_margin_end: 12,
                set_margin_bottom: 24,

                append = &adw::PreferencesGroup {
                    set_title: &t!("home_info_group_title"),

                    add = &model.board_row.clone(),
                    add = &model.bios_row.clone(),
                    add = &model.kernel_row.clone(),
                    add = &model.serial_row.clone(),
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let status_page = adw::StatusPage::new();
        status_page.set_icon_name(Some("computer-symbolic"));
        status_page.set_title(&t!("home_loading"));

        let board_row = adw::ActionRow::new();
        board_row.set_title(&t!("home_board_title"));
        board_row.set_selectable(false);

        let bios_row = adw::ActionRow::new();
        bios_row.set_title(&t!("home_bios_title"));
        bios_row.set_selectable(false);

        let kernel_row = adw::ActionRow::new();
        kernel_row.set_title(&t!("home_kernel_title"));
        kernel_row.set_selectable(false);

        let serial_row = adw::ActionRow::new();
        serial_row.set_title(&t!("home_serial_title"));
        serial_row.set_subtitle(&t!("home_serial_hidden"));
        serial_row.set_selectable(false);

        let reveal_button = gtk::Button::with_label(&t!("home_serial_reveal"));
        reveal_button.set_valign(gtk::Align::Center);
        reveal_button.add_css_class("flat");
        {
            let sender = sender.clone();
            reveal_button.connect_clicked(move |_| {
                sender.input(HomeMsg::RevealSerial);
            });
        }
        serial_row.add_suffix(&reveal_button);

        let model = HomeModel {
            status_page,
            board_row,
            bios_row,
            kernel_row,
            serial_row,
            reveal_button,
        };

        let widgets = view_output!();

        sender.command(move |out, shutdown| {
            shutdown
                .register(async move {
                    let product_name = tokio::fs::read_to_string("/sys/class/dmi/id/product_name")
                        .await
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default();

                    let board_name = tokio::fs::read_to_string("/sys/class/dmi/id/board_name")
                        .await
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default();

                    let bios_version = tokio::fs::read_to_string("/sys/class/dmi/id/bios_version")
                        .await
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default();

                    let bios_date = tokio::fs::read_to_string("/sys/class/dmi/id/bios_date")
                        .await
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default();

                    let kernel = tokio::process::Command::new("uname")
                        .arg("-r")
                        .output()
                        .await
                        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                        .unwrap_or_default();

                    out.emit(HomeCommandOutput::DataLoaded {
                        product_name,
                        board_name,
                        bios_version,
                        bios_date,
                        kernel,
                    });
                })
                .drop_on_shutdown()
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: HomeMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            HomeMsg::RevealSerial => {
                self.reveal_button.set_sensitive(false);
                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            let result =
                                pkexec_read("cat /sys/class/dmi/id/product_serial").await;
                            out.emit(HomeCommandOutput::SerialRevealed(result));
                        })
                        .drop_on_shutdown()
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: HomeCommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            HomeCommandOutput::DataLoaded {
                product_name,
                board_name,
                bios_version,
                bios_date,
                kernel,
            } => {
                self.status_page.set_title(&product_name);
                self.board_row.set_subtitle(&board_name);
                self.bios_row
                    .set_subtitle(&format!("{bios_version} / {bios_date}"));
                self.kernel_row.set_subtitle(&kernel);
            }
            HomeCommandOutput::SerialRevealed(Ok(serial)) => {
                self.serial_row.set_subtitle(&serial);
            }
            HomeCommandOutput::SerialRevealed(Err(e)) => {
                self.reveal_button.set_sensitive(true);
                let _ = sender.output(e);
            }
        }
    }
}
