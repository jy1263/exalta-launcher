use std::fmt::Error;

use exalta_core::{
    auth::AuthController,
    ExaltaClient,
};
use registries::UpdateError;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

mod login;
mod play;

mod args;
mod launchargs;
mod registries;

use eframe::egui;

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Exalta Launcher",
        options,
        Box::new(|_cc| Box::new(ExaltaLauncher::default())),
    );
}

#[derive(Serialize, Deserialize, Clone)]
struct LauncherAuth {
    username: String,
    password: String,
}
struct ResultTimeWrapper {
    result: Result<(), Box<dyn std::error::Error>>,
    time: std::time::Instant,
}
struct ExaltaLauncher {
    auth: LauncherAuth,
    auth_save: bool,
    auth_con: Option<AuthController>,

    entry: keyring::Entry,
    runtime: Runtime,

    run_res: ResultTimeWrapper,
}

impl Default for ExaltaLauncher {
    fn default() -> Self {
        let entry = keyring::Entry::new(&"exalt", &"jsondata");        
        let password_res = entry.get_password();

        let mut run_res = ResultTimeWrapper {
            result: Ok(()),
            time: std::time::Instant::now(),
        };

        let runtime = Runtime::new().unwrap();

        if cfg!(windows) {
            let updatechecker = || -> Result<(), Box<dyn std::error::Error>> {
                let buildid = crate::registries::get_build_id()?;
                let client = ExaltaClient::new()?;
                let buildhash = runtime.block_on(client.init("Unity", None))?.build_hash;
                if buildid != buildhash {
                    return Err(Box::new(UpdateError(String::from(
                        "An update for the game is available, please run the official launcher to update the game first."
                    ))));
                }
                Ok(())
            };
            run_res = ResultTimeWrapper {
                result: updatechecker().map_err(|x| {
                    if x.is::<UpdateError>() {
                        x
                    }
                    else {
                        Box::new(UpdateError(String::from("Failed to check for updates.")))
                    }
                }),
                time: std::time::Instant::now(),
            };
            let credchecker = || -> Result<(), Box<dyn std::error::Error>> {
                if password_res.is_err() {
                    
                }
                Ok(())
            };
        }

        let mut self_inst = Self {
            auth: LauncherAuth {
                username: String::new(),
                password: String::new(),
            },
            auth_save: true,
            auth_con: None,
            entry,
            runtime,
            run_res
        };

        if let Ok(val) = password_res {
            if let Ok(foundauth) = serde_json::from_str::<LauncherAuth>(&val) {
                self_inst.auth = foundauth;
                self_inst.login().ok();
            };
        };

        self_inst
    }
}

impl eframe::App for ExaltaLauncher {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_pixels_per_point(2.0);
        if let Err(err) = egui::CentralPanel::default()
            .show(ctx, |ui| -> Result<(), Box<dyn std::error::Error>> {
                ui.heading("Exalta Launcher");

                // play
                if self.auth_con.is_some() {
                    self.render_play(ui)
                }
                // login
                else {
                    self.render_login(ui)
                }
            })
            .inner
        {
            self.run_res = ResultTimeWrapper {
                result: Err(err),
                time: std::time::Instant::now(),
            };
        };

        if let Err(e) = &self.run_res.result {
            if &self.run_res.time.elapsed().as_secs() < &5 {
                egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
                    ui.vertical_centered_justified(|ui| {
                        ui.label(e.to_string());
                    });
                });
            }
        }
    }
}
impl ExaltaLauncher {
    fn login(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let auth_con = self.runtime.block_on(
            ExaltaClient::new()
                .unwrap()
                .login(&self.auth.username.as_str(), &self.auth.password.as_str()),
        )?;

        self.run_res.result = Ok(());
        self.auth_con = Some(auth_con);

        if self.auth_save {
            if let Some(json) = serde_json::to_string(&self.auth).ok() {
                self.entry.set_password(json.as_str())?;
            }
        }
        
        Ok(())
    }
}
