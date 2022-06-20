#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::path::Path;
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::sync::mpsc::{Receiver, Sender};
use eframe::{CreationContext, run_native};
use egui::{Button, Color32, Vec2, Visuals};

const GIT_LINK:&str   = "https://github.com/git-for-windows/git/releases/download/v2.36.1.windows.1/Git-2.36.1-64-bit.exe";
const JRE_LINK:&str   = "https://download.bell-sw.com/java/18.0.1.1+2/bellsoft-jre18.0.1.1+2-windows-amd64-full.msi";
const VERSION:&str    = "18.0.1.1+2";
const REPOSITORY:&str = "https://github.com/sirkostya009/java-sources";
const DIRECTORY:&str  = "java-sources";

struct LauncherApp {
    status: String,
    runnable: bool,
    restart_required: bool,
    receiver: Receiver<String>,
    sender: Sender<String>,
}

impl LauncherApp {
    fn new(cc: &CreationContext) -> LauncherApp {
        cc.egui_ctx.set_visuals(Visuals::dark());

        let (s, r) = std::sync::mpsc::channel();

        let mut app = LauncherApp {
            status: "Standby".to_string(),
            runnable: false,
            restart_required: false,
            receiver: r,
            sender: s
        };
        app.runnable = app.check_for_git() && app.check_for_java() && app.check_directory();

        app
    }

    fn check_for(&mut self, what: &str) -> bool {
        self.sender.send(format!("checking if {what} exists... ")).unwrap();

        let result = !matches!(Command::new(what).creation_flags(0x08000000).output(), Err(_e));
        self.sender.send(format!("{what} is {}available", if result { "" } else { "not " })).unwrap();

        result
    }

    pub fn check_for_git(&mut self) -> bool {
        self.check_for("git")
    }

    pub fn check_for_java(&mut self) -> bool {
        let mut result = self.check_for("java");

        if result {
            let output = String::from_utf8(Command::new("java").arg("--version")
                .creation_flags(0x08000000)
                .output()
                .unwrap()
                .stdout)
                .unwrap();

            result &= output.contains(VERSION); // checks if exactly the same version of jre is installed,
        }                                           // otherwise you'll need to install the one required

        result
    }

    pub fn check_directory(&mut self) -> bool {
        self.sender.send(String::from("checking if gaem directory exists...")).unwrap();

        let mut result = true;

        if !Path::new(DIRECTORY).exists() {
            result = false;
        }

        if !self.check_for_git() {
            result = false;
        }

        self.sender.send((if result { "directory exists!" } else { "directory does not exist" }).to_string()).unwrap();
        if !result { return result }

        result = String::from_utf8(Command::new("git")
            .creation_flags(0x08000000)
            .current_dir(DIRECTORY)
            .arg("status")
            .output()
            .unwrap().stdout)
            .unwrap()
            .contains(&"up to date");

        self.sender.send(format!("directory is {}up to date", if result { "" } else { "not " })).unwrap();
        if !result && Command::new("git")
            .creation_flags(0x08000000)
            .current_dir(DIRECTORY)
            .arg("pull")
            .output().is_ok()
        {
            result = true
        }

        result
    }

    pub fn clone_repository(&mut self) {
        self.sender.send(String::from("cloning gaem...")).unwrap();
        std::thread::spawn(|| Command::new("git")
            .creation_flags(0x08000000)
            .args(["clone", REPOSITORY])
            .output()
            .expect("failed to clone from remote repository")
        );
    }

    fn download_and_install(&mut self, what: &'static &str, link: &'static &str) {
        let sender = self.sender.clone();

        std::thread::spawn(move || {
            sender.send(format!("downloading {what}")).unwrap();
            Command::new("curl")
                .creation_flags(0x08000000)
                .args(["-O", link])
                .output()
                .unwrap_or_else(|err| {
                    let msg = format!("failed to download {what}, {err}");
                    sender.send(msg.clone()).unwrap();
                    panic!("{msg}")
                });

            let setup = link.split('/').last().unwrap();
            let extension = Path::new(setup).extension().unwrap();

            sender.send(format!("installing {what}")).unwrap();
            let handle = |err| {
                let msg = format!("failed to install {what}, {err}");
                sender.send(msg.clone()).unwrap();
                panic!("{msg}")
            };

            if extension == "msi" {
                Command::new("msiexec")
                    .creation_flags(0x08000000)
                    .args(["/i", setup])
                    .output()
                    .unwrap_or_else(handle);
            } else if extension == "exe" {
                Command::new(setup)
                    .creation_flags(0x08000000)
                    .arg("/VERYSILENT")
                    .output()
                    .unwrap_or_else(handle);
            }
        });

        self.restart_required = true;
    }

    pub fn download_and_install_jre(&mut self) {
        self.download_and_install(&"jre",&JRE_LINK);
    }

    pub fn download_and_install_git(&mut self) {
        self.download_and_install(&"git",&GIT_LINK);
    }

    fn poll_status(&mut self) {
        if let Ok(s) = self.receiver.try_recv() {
            self.status = s;
        }
    }
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx,|ui| {
            ui.add_space(100f32);

            ui.vertical_centered(|ui| {
                if ui.add_sized(Vec2::new(160f32,50f32),Button::new("Run")).clicked() {
                    if self.runnable {
                        std::thread::spawn(||Command::new("java")
                            .current_dir(format!("./{DIRECTORY}/"))
                            .creation_flags(0x08000000)
                            .arg("Main")
                            .output()
                            .expect("java Main failed"));
                    } else {
                        if !self.check_for_git() {
                            self.download_and_install_git();
                        } else if !self.check_for_java() {
                            self.download_and_install_jre();
                        } else if !self.check_directory() {
                            self.clone_repository();
                        }

                        self.runnable = self.check_for_git() && self.check_for_java() && self.check_directory();
                    }
                }

                self.poll_status();

                ui.add_space(10f32);

                ui.label(self.status.as_str());

                if self.restart_required {
                    ui.label(egui::RichText::new("program restart required").color(Color32::RED));
                } else if self.runnable {
                    self.sender.send("Ready".to_string()).unwrap();
                }
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
