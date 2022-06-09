#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use eframe::{CreationContext, run_native};
use egui::{Button, Color32, Vec2, Visuals};

const GIT_LINK:&str   = "https://github.com/git-for-windows/git/releases/download/v2.36.1.windows.1/Git-2.36.1-64-bit.exe";
const JRE_LINK:&str   = "https://download.bell-sw.com/java/18.0.1.1+2/bellsoft-jre18.0.1.1+2-windows-amd64-full.msi";
const REPOSITORY:&str = "https://github.com/sirkostya009/java-sources";
const DIRECTORY:&str  = "java-sources";

struct LauncherApp {
    status: String,
    runnable: bool,
    restart_required: bool,
    log: File,
}

impl LauncherApp {
    fn new(cc: &CreationContext) -> LauncherApp {
        cc.egui_ctx.set_visuals(Visuals::dark());

        let mut app = LauncherApp {
            status: "Standby".to_string(),
            runnable: false,
            restart_required: false,
            log: File::create("log.txt").unwrap(),
        };
        app.runnable = app.check_for_git() && app.check_for_java() && app.check_directory();

        app
    }

    fn check_for(&mut self, what: &str) -> bool {
        self.log(format!("checking if {what} exists"));

        !matches!(Command::new(what).output(), Err(_e))
    }

    pub fn check_for_git(&mut self) -> bool {
        self.check_for("git")
    }

    pub fn check_for_java(&mut self) -> bool {
        self.check_for("java")
    }

    pub fn check_directory(&mut self) -> bool {
        self.log("checking if gaem directory exists...".to_string());

        Path::new(DIRECTORY).exists()
    }

    pub fn clone_repository(&mut self) {
        self.log("cloning gaem...\n".to_string());
        std::thread::spawn(|| {
            Command::new("git")
                .args(["clone", REPOSITORY])
                .output()
                .expect("failed to clone from remote repository")
        });
    }

    fn download_and_install(&mut self, what: &'static &str, link: &'static &str) {
        self.log(format!("downloading and installing {what}"));

        std::thread::spawn(move || {
            Command::new("curl")
                .args(["-O", link])
                .output()
                .unwrap_or_else(|_| panic!("failed to download {what}"));

            let program_name = link.split('/').last().unwrap();

            Command::new(program_name)
                .arg("/quiet")
                .output()
                .unwrap_or_else(|_| panic!("failed to install {what}"));
        });
    }

    pub fn download_and_install_jre(&mut self) {
        self.download_and_install(&"jre",&JRE_LINK);
        self.restart_required = true;
    }

    pub fn download_and_install_git(&mut self) {
        self.download_and_install(&"git",&GIT_LINK);
    }

    pub fn log(&mut self, mut info: String) {
        info.push('\n');
        self.log.write_all(info.as_bytes())
            .unwrap_or_else(|_| panic!("failed to log {:?}", info));
    }
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx,|ui| {
            ui.add_space(100f32);

            ui.vertical_centered(|ui| {
                if ui.add_sized(Vec2::new(160f32,50f32),Button::new("Run")).clicked() {
                    if self.runnable {
                        self.log.write_all(b"java Main\n").unwrap();
                        Command::new("java")
                            .current_dir(format!("./{DIRECTORY}/"))
                            .arg("Main")
                            .output()
                            .expect("java Main failed");
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

                ui.add_space(10f32);

                if self.restart_required {
                    ui.label(egui::RichText::new("system restart required").color(Color32::RED));
                } else if self.runnable {
                    ui.label("Ready");
                } else {
                    ui.label(self.status.to_string());
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
