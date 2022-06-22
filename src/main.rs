#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::process::Command;
use eframe::run_native;
use egui::{Vec2, Visuals};
use eframe::CreationContext;
use egui::Button;
use std::os::windows::process::CommandExt;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::JoinHandle;
use winapi::um::winbase::CREATE_NO_WINDOW;

const JRE_LINK:&str = "https://download.bell-sw.com/java/18.0.1.1+2/bellsoft-jre18.0.1.1+2-windows-amd64-full.msi";
const SETUP:&str    = "bellsoft-jre18.0.1.1+2-windows-amd64-full.msi";
const VERSION:&str  = "18.0.1.1+2";
const GAEM_LINK:&str= "https://github.com/sirkostya009/term-paper/releases/download/second-release/term-paper.jar";
const JAR_NAME:&str = "term-paper.jar";

fn path() -> String {
    format!("C:/Users/{}/AppData/Roaming/GaemApp", whoami::username())
}

fn jre_is_present(sender: Sender<String>) -> bool {
    sender.send("checking if jre is present".to_string());
    let result = Command::new("java")
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .is_ok();

    if !result {
        result
    } else {
        String::from_utf8(
            Command::new("java")
                .creation_flags(CREATE_NO_WINDOW)
                .arg("--version")
                .output()
                .unwrap().stdout)
            .unwrap()
            .contains(VERSION)
    }
}

fn gaem_is_present(sender: Sender<String>) -> bool {
    sender.send("checking if gaem is present".to_string());

    if !std::path::Path::new(&format!("{}", path())).exists() {
        std::fs::create_dir(path());
    }

    std::path::Path::new(&format!("{}/{JAR_NAME}", path())).exists()
}

fn curl(sender: Sender<String>, what: &'static str, link: &'static str) -> JoinHandle<()> {
    std::thread::spawn(move || {
        sender.send(format!("curling {what}..."));

        Command::new("curl")
            .current_dir(path())
            .creation_flags(CREATE_NO_WINDOW)
            .args(["-LO", link])
            .output();

        sender.send(format!("{what} has been succesffully curl'd"));
    })
}

fn install_jre(sender: Sender<String>) {
    std::thread::spawn(move || {
        if curl(sender.clone(), "jre", JRE_LINK).join().is_err() {
            sender.send("failed to curl jre".to_string());
            return;
        }

        sender.send("starting installer...".to_string());
        let ran = Command::new("msiexec")
            .current_dir(path())
            .creation_flags(CREATE_NO_WINDOW)
            .args(["/i", SETUP])
            .output()
            .is_ok();

        sender.send(
            if ran {
                "jre has been installed"
            } else {
                "failed to install jre"
            }.to_string()
        );
    });
}

fn curl_gaem(sender: Sender<String>) {
    curl(sender, "gaem", GAEM_LINK);
}

struct LauncherApp {
    status: String,
    sender: Sender<String>,
    receiver: Receiver<String>,
}

impl LauncherApp {
    pub fn new(cc: &CreationContext) -> Self {
        cc.egui_ctx.set_visuals(Visuals::dark());

        let (s, r) = std::sync::mpsc::channel();

        LauncherApp {
            status: "Standby".to_string(),
            sender: s,
            receiver: r,
        }
    }

    pub fn poll_status(&mut self) {
        if let Ok(s) = self.receiver.try_recv() {
            self.status = s;
        }
    }

    pub fn sender(&self) -> Sender<String> {
        self.sender.clone()
    }
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx,|ui| {
            ui.add_space(100f32);

            ui.vertical_centered(|ui| {
                if ui.add_sized(Vec2::new(160f32,50f32),Button::new("Run")).clicked() {
                    if !jre_is_present(self.sender()) {
                        install_jre(self.sender());
                    } else if !gaem_is_present(self.sender()) {
                        curl_gaem(self.sender());
                    } else {
                        let sender = self.sender();
                        sender.send("starting gaem...".to_string());
                        std::thread::spawn(move ||
                            Command::new("java")
                                .current_dir(path())
                                .args(["-jar", JAR_NAME])
                                .output()
                                .unwrap_or_else(|err| {
                                    let msg = format!("failed to launch gaem, {err}");
                                    sender.send(msg.clone());
                                    panic!("{msg}")
                                })
                        );
                    }
                }

                self.poll_status();

                ui.add_space(10f32);

                ui.label(self.status.as_str());
            })
        });
    }
}

fn main() {
    run_native(
        "Gaem launcher",
        eframe::NativeOptions {
            always_on_top: false,
            maximized: false,
            decorated: true,
            drag_and_drop_support: false,
            icon_data: None,
            initial_window_pos: None,
            initial_window_size: Some(Vec2::new(320., 280.)),
            min_window_size: None,
            max_window_size: None,
            resizable: false,
            transparent: false,
            vsync: false,
            multisampling: 0,
            depth_buffer: 0,
            stencil_buffer: 0,
        },
        Box::new(|cc|Box::new(LauncherApp::new(cc)))
    )
}
