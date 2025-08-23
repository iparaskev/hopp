use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use serde::{Deserialize, Serialize};

const APP_STATE_FILE: &str = "app_state.json";
const OLD_TRAY_NOTIFICATION_FILE: &str = "tray_notification.json";

/// Current version of the application state structure.
///
/// This struct represents the complete application state that gets
/// persisted to disk. It includes all user preferences and settings
/// that should survive between application restarts.
#[derive(Debug, Serialize, Deserialize)]
struct AppStateInternal {
    /// Whether the notifications which shows that hopp is in the menu bar will be shown
    pub tray_notification: bool,

    /// The device ID of the last used microphone.
    pub last_used_mic: Option<String>,

    /// Flag indicating if this is the user's first time running the application.
    pub first_run: bool,
}

/// Legacy version of the application state structure.
#[derive(Debug, Serialize, Deserialize)]
struct OldAppStateInternal {
    pub tray_notification: bool,
    pub last_used_mic: Option<String>,
}

impl Default for AppStateInternal {
    /// Creates a new application state with default values.
    ///
    /// Default settings:
    /// - Tray notification: enabled
    /// - Last used microphone: none
    /// - First run: true
    fn default() -> Self {
        AppStateInternal {
            tray_notification: true,
            last_used_mic: None,
            first_run: true,
        }
    }
}

/// Thread-safe application state manager.
///
/// This struct provides thread-safe access to application settings and handles
/// persistence to disk. It includes migration logic for backward compatibility
/// and uses a mutex to ensure safe concurrent access.
#[derive(Debug, Serialize, Deserialize)]
pub struct AppState {
    /// The internal state data.
    state: AppStateInternal,

    /// Root folder path where state files are stored.
    root_folder: PathBuf,

    /// Mutex for thread-safe access to state modifications.
    lock: Mutex<()>,
}

/// Retrieves and migrates legacy tray notification setting.
fn retrieve_old_tray(root_folder: &Path) -> bool {
    let mut path = root_folder.to_path_buf();
    path.push(OLD_TRAY_NOTIFICATION_FILE);
    if path.exists() {
        let contents = fs::read_to_string(&path).unwrap_or_else(|e| {
            log::error!("Failed to read tray_notification.json: {e}");
            r#"{"show": true}"#.to_string()
        });

        let json: serde_json::Value = serde_json::from_str(&contents).unwrap_or_else(|e| {
            log::error!("Failed to parse tray_notification.json: {e}");
            serde_json::json!({ "show": true })
        });

        fs::remove_file(path).unwrap_or_else(|e| {
            log::error!("Failed to remove tray_notification.json: {e}");
        });

        return json["show"].as_bool().unwrap_or(true);
    }

    true
}

impl AppState {
    /// Creates a new AppState instance, loading from disk or using defaults.
    ///
    /// This constructor handles the complete initialization process including:
    /// - Loading existing state from disk
    /// - Migrating from legacy formats
    /// - Creating default state if no existing state is found
    /// - Setting up thread-safe access
    ///
    /// # Arguments
    ///
    /// * `root_folder` - Directory where state files should be stored
    ///
    /// # Returns
    ///
    /// A new `AppState` instance ready for use
    pub fn new(root_folder: &Path) -> Self {
        let app_state_path = root_folder.join(APP_STATE_FILE);
        if !app_state_path.exists() {
            let state = AppStateInternal {
                tray_notification: retrieve_old_tray(root_folder),
                ..Default::default()
            };

            if let Ok(serialized) = serde_json::to_string_pretty(&state) {
                let _ = fs::write(app_state_path, serialized);
            }

            return AppState {
                state,
                root_folder: root_folder.to_path_buf(),
                lock: Mutex::new(()),
            };
        }

        match fs::read_to_string(app_state_path) {
            Ok(contents) => match serde_json::from_str::<AppStateInternal>(&contents) {
                Ok(state) => {
                    return AppState {
                        state,
                        root_folder: root_folder.to_path_buf(),
                        lock: Mutex::new(()),
                    };
                }
                Err(_) => {
                    log::error!("Failed to parse app state from file, using default state.");
                    /* Fallback for migration from old app state. */
                    match serde_json::from_str::<OldAppStateInternal>(&contents) {
                        Ok(state) => {
                            let new_state = AppStateInternal {
                                tray_notification: state.tray_notification,
                                last_used_mic: state.last_used_mic,
                                first_run: false,
                            };

                            let app_state_path = root_folder.join(APP_STATE_FILE);
                            if !Self::write_file(&app_state_path, &new_state) {
                                log::error!("Failed to write new app state to file.");
                            }

                            return AppState {
                                state: new_state,
                                root_folder: root_folder.to_path_buf(),
                                lock: Mutex::new(()),
                            };
                        }
                        Err(_) => {
                            log::error!(
                                "Failed to parse old app state from file, using default state."
                            );
                        }
                    }
                }
            },
            Err(_) => {
                log::error!("Failed to read app state file, using default state.");
            }
        }
        AppState {
            state: AppStateInternal::default(),
            root_folder: root_folder.to_path_buf(),
            lock: Mutex::new(()),
        }
    }

    /// Gets the current tray notification setting.
    pub fn tray_notification(&self) -> bool {
        let _lock = self.lock.lock().unwrap();
        self.state.tray_notification
    }

    /// Updates the tray notification setting and saves to disk.
    pub fn set_tray_notification(&mut self, value: bool) {
        log::info!("set_tray_notification: {value}");
        let _lock = self.lock.lock().unwrap();
        self.state.tray_notification = value;
        if !self.save() {
            log::error!("set_tray_notification: Failed to save app state");
        }
    }

    /// Gets the last used microphone device ID.
    pub fn last_used_mic(&self) -> Option<String> {
        let _lock = self.lock.lock().unwrap();
        self.state.last_used_mic.clone()
    }

    /// Updates the last used microphone setting and saves to disk.
    pub fn set_last_used_mic(&mut self, mic: String) {
        log::info!("set_last_used_mic: {mic}");
        let _lock = self.lock.lock().unwrap();
        self.state.last_used_mic = Some(mic);
        if !self.save() {
            log::error!("set_last_used_mic: Failed to save app state");
        }
    }

    /// Checks if this is the user's first time running the application.
    pub fn first_run(&self) -> bool {
        let _lock = self.lock.lock().unwrap();
        self.state.first_run
    }

    /// Updates the first-run flag and saves to disk.
    pub fn set_first_run(&mut self, value: bool) {
        let _lock = self.lock.lock().unwrap();
        self.state.first_run = value;
        if !self.save() {
            log::error!("set_first_run: Failed to save app state");
        }
    }

    /// Saves the current state to disk.
    ///
    /// # Returns
    ///
    /// `true` if the save was successful, `false` if an error occurred
    ///
    /// # Thread Safety
    ///
    /// This method assumes the caller already holds the internal lock.
    /// It should only be called from other methods that have acquired the lock.
    fn save(&self) -> bool {
        let app_state_path = self.root_folder.join(APP_STATE_FILE);
        Self::write_file(&app_state_path, &self.state)
    }

    /// Writes the state data to a file in JSON format.
    ///
    /// # Arguments
    ///
    /// * `path` - Path where the state file should be written
    /// * `state` - State data to serialize and write
    ///
    /// # Returns
    ///
    /// `true` if the write was successful, `false` if an error occurred
    ///
    /// # Error Handling
    ///
    /// Logs serialization and file write errors but does not panic.
    fn write_file(path: &PathBuf, state: &AppStateInternal) -> bool {
        match serde_json::to_string_pretty(state) {
            Ok(serialized) => {
                return fs::write(path, serialized).is_ok();
            }
            Err(e) => log::error!("Failed to serialize app state: {e}"),
        }
        false
    }
}
