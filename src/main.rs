#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::process::Command;
use eframe::run_native;
use egui::{Vec2, Visuals};
use eframe::CreationContext;
use egui::Button;
use std::os::windows::process::CommandExt;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::JoinHandle;
use std::time::Duration;
use winapi::um::winbase::CREATE_NO_WINDOW;

const JRE_LINK:&str = "https://download.bell-sw.com/java/18.0.1.1+2/bellsoft-jre18.0.1.1+2-windows-amd64-full.msi";
const SETUP:&str    = "bellsoft-jre18.0.1.1+2-windows-amd64-full.msi";
const GAEM_LINK:&str= "https://github.com/sirkostya009/term-paper/releases/download/second-release/term-paper.jar";
const JAR_NAME:&str = "term-paper.jar";

fn path() -> String {
    format!("C:/Users/{}/AppData/Roaming/GaemApp", whoami::username())
}

fn curl(sender: Sender<String>, stall_sender: Sender<bool>, what: &'static str, link: &'static str) -> JoinHandle<()> {
    std::thread::spawn(move || {
        stall_sender.send(true);
        sender.send(format!("curling {what}..."));

        Command::new("curl")
            .current_dir(path())
            .creation_flags(CREATE_NO_WINDOW)
            .args(["-LO", link])
            .output();

        sender.send(format!("{what} has been succesffully curl'd"));
        stall_sender.send(false);
    })
}

struct LauncherApp {
    status: String,
    sender: Sender<String>,
    receiver: Receiver<String>,
    sb: Sender<bool>,
    rb: Receiver<bool>,
}

impl LauncherApp {
    pub fn new(cc: &CreationContext) -> Self {
        cc.egui_ctx.set_visuals(Visuals::dark());

        let (s, r) = std::sync::mpsc::channel();
        let (sb, rb) = std::sync::mpsc::channel();

        LauncherApp {
            status: "Standby".to_string(),
            sender: s,
            receiver: r,
            sb,
            rb,
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

    fn jre_is_present(&self) -> bool {
        self.sender.send("checking if jre is present".to_string());
        let output = Command::new("java")
            .arg("-version")
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .is_ok();

        if !output {
            output
        } else {
            let (s, r) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                let status =
                    Command::new("java")
                        .creation_flags(CREATE_NO_WINDOW)
                        .args(["-jar", format!("{}/{JAR_NAME}", path()).as_str()])
                        .output();

                status.unwrap();
                s.send(());
            });

            std::thread::sleep(Duration::from_millis(500));

            !matches!(r.try_recv(), Ok(_))
        }
    }

    fn gaem_is_present(&self) -> bool {
        self.sender.send("checking if gaem is present".to_string());

        if !std::path::Path::new(&path()).exists() {
            std::fs::create_dir(path());
        }

        std::path::Path::new(&format!("{}/{JAR_NAME}", path())).exists()
    }

    fn install_jre(&self) {
        if self.rb.try_recv().is_ok() && self.rb.try_recv().unwrap() {
            return;
        }

        let sender = self.sender();
        let boolean = self.sb.clone();
        std::thread::spawn(move || {
            if curl(sender.clone(), boolean.clone(), "jre", JRE_LINK).join().is_err() {
                sender.send("failed to curl jre".to_string());
                return;
            }

            boolean.send(true);
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
            boolean.send(false);
        });
    }

    fn curl_gaem(&self) {
        curl(self.sender.clone(), self.sb.clone(),"gaem", GAEM_LINK);
    }
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx,|ui| {
            ui.add_space(100f32);

            ui.vertical_centered(|ui| {
                if ui.add_sized(Vec2::new(160f32,50f32),Button::new("Run")).clicked() {
                    let yes = if let Ok(b) = self.rb.try_recv() { b } else { false };

                    if !yes {
                        if !self.gaem_is_present() {
                            self.curl_gaem();
                        } else if !self.jre_is_present() {
                            self.install_jre();
                        } else {
                            self.sender.send("Ready".to_string());
                        }
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
