use futures_util::{SinkExt, StreamExt};
use local_ip_address::local_ip;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::Mutex;
use tauri::State;
use tauri::{Emitter, Manager};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::StatusCode;
use tokio_tungstenite::tungstenite::Message;
use tts::Tts;

#[derive(serde::Serialize, Clone)]
struct EtatFile {
    compteur: i32,
    guichet: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Device {
    id: i32,
    name: String,
    token: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Annonce {
    id: i32,
    message: String,
    active: bool,
}

#[derive(Deserialize, Debug)]
struct EspMessage {
    message: String,
}

struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    fn init() -> Self {
        let conn = Connection::open("qms.db").expect("Impossible d'ouvrir la DB");

        conn.execute(
            "CREATE TABLE IF NOT EXISTS etat_courant (
                id INTEGER PRIMARY KEY,
                valeur_compteur INTEGER NOT NULL,
                dernier_guichet TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS devices (
                id INTEGER PRIMARY KEY,
                name TEXT UNIQUE NOT NULL,
                token TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS annonces (
                id INTEGER PRIMARY KEY,
                message TEXT NOT NULL,
                active BOOLEAN DEFAULT 1
            )",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT OR IGNORE INTO etat_courant (id, valeur_compteur, dernier_guichet) VALUES (1, 0, 'None')",
            [],
        ).unwrap();

        Database {
            conn: Mutex::new(conn),
        }
    }

    fn incrementer(&self, nom_guichet: &str) -> EtatFile {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE etat_courant SET valeur_compteur = valeur_compteur + 1, dernier_guichet = ?1 WHERE id = 1",
            params![nom_guichet],
        ).unwrap();
        self.lire_etat(&conn)
    }

    fn reset(&self) -> EtatFile {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE etat_courant SET valeur_compteur = 0, dernier_guichet = 'Reset' WHERE id = 1",
            [],
        )
        .unwrap();
        self.lire_etat(&conn)
    }

    fn get_current(&self) -> EtatFile {
        let conn = self.conn.lock().unwrap();
        self.lire_etat(&conn)
    }

    fn lire_etat(&self, conn: &Connection) -> EtatFile {
        conn.query_row(
            "SELECT valeur_compteur, dernier_guichet FROM etat_courant WHERE id = 1",
            [],
            |row| {
                Ok(EtatFile {
                    compteur: row.get(0)?,
                    guichet: row.get(1)?,
                })
            },
        )
        .unwrap()
    }

    fn register_device(&self, name: String) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();

        let token: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();

        conn.execute(
            "INSERT OR IGNORE INTO devices (name, token) VALUES (?1, ?2)",
            params![name, token],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_device_info(&self, token: &str) -> Option<(i32, String)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, name FROM devices WHERE token = ?1")
            .unwrap();

        let result = stmt.query_row(params![token], |row| Ok((row.get(0)?, row.get(1)?)));

        result.ok()
    }

    fn get_all_devices(&self) -> Vec<Device> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM devices").unwrap();

        let devices_iter = stmt
            .query_map([], |row| {
                Ok(Device {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    token: row.get(2)?,
                })
            })
            .unwrap();

        devices_iter.map(|d| d.unwrap()).collect()
    }

    // --- GESTION DES ANNONCES (NOUVEAU) ---

    fn add_annonce(&self, message: String) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO annonces (message) VALUES (?1)",
            params![message],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn get_annonces(&self) -> Vec<Annonce> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM annonces").unwrap();
        let iter = stmt
            .query_map([], |row| {
                Ok(Annonce {
                    id: row.get(0)?,
                    message: row.get(1)?,
                    active: row.get(2)?,
                })
            })
            .unwrap();
        iter.map(|a| a.unwrap()).collect()
    }

    fn delete_annonce(&self, id: i32) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM annonces WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();

            let db = std::sync::Arc::new(Database::init());
            app.manage(db.clone());

            tauri::async_runtime::spawn(async move {
                let addr = "0.0.0.0:8765";
                let listener = TcpListener::bind(&addr)
                    .await
                    .expect("Impossible de lancer le serveur");

                println!(
                    "🚀 Serveur WebSocket sécurisé en écoute sur : ws://{}",
                    addr
                );

                while let Ok((stream, _)) = listener.accept().await {
                    let app_handle = app_handle.clone();
                    let db_clone = db.clone();

                    tokio::spawn(async move {
                        let device_name_captured = std::sync::Arc::new(Mutex::new(None));
                        let device_name_writer = device_name_captured.clone();

                        let db_callback_clone = db_clone.clone();

                        let callback = move |req: &Request, response: Response| {
                            let uri = req.uri();
                            let query = uri.query().unwrap_or("");

                            let token = query
                                .split('&')
                                .find(|p| p.starts_with("token="))
                                .map(|p| p.trim_start_matches("token="))
                                .unwrap_or("");

                            if let Some((_id, name)) = db_callback_clone.get_device_info(token) {
                                println!("✅ Connexion validée pour : {}", name);

                                *device_name_writer.lock().unwrap() = Some(name);

                                Ok(response)
                            } else {
                                let mut err = ErrorResponse::new(Some("Token Invalide".into()));
                                *err.status_mut() = StatusCode::FORBIDDEN;
                                Err(err)
                            }
                        };

                        let mut tts = match Tts::default() {
                            Ok(t) => Some(t),
                            Err(e) => {
                                eprintln!("Error initializing TTS: {}", e);
                                None
                            }
                        };

                        // --- CONNEXION ---
                        match accept_hdr_async(stream, callback).await {
                            Ok(ws_stream) => {
                                let device_name = {
                                    let lock = device_name_captured.lock().unwrap();
                                    lock.clone()
                                };

                                if device_name.is_none() {
                                    return;
                                }
                                let current_device_name = device_name.unwrap();
                                println!("💾 Session active pour : {}", current_device_name);

                                let (mut write, mut read) = ws_stream.split();

                                while let Some(msg) = read.next().await {
                                    match msg {
                                        Ok(Message::Text(text)) => {
                                            if let Ok(payload) =
                                                serde_json::from_str::<EspMessage>(&text)
                                            {
                                                if payload.message == "NEXT" {
                                                    println!(
                                                        "🟢 Action NEXT reçue de {}",
                                                        current_device_name
                                                    );

                                                    let nouveau_numero = db_clone
                                                        .incrementer(&current_device_name)
                                                        .compteur;

                                                    let event_payload = EtatFile {
                                                        guichet: current_device_name.clone(),
                                                        compteur: nouveau_numero,
                                                    };
                                                    app_handle
                                                        .emit("nouveau-message", event_payload)
                                                        .unwrap();

                                                    let text_to_speak = format!(
                                                        "Client numéro {}, guichet {}",
                                                        nouveau_numero, current_device_name
                                                    );

                                                    if let Some(ref mut speech_engine) = tts {
                                                        // .speak(text, interrupt_current_speech)
                                                        let _ = speech_engine
                                                            .speak(text_to_speak, true);
                                                    }

                                                    let reponse =
                                                        format!("{}", current_device_name);
                                                    if let Err(_) =
                                                        write.send(Message::Text(reponse)).await
                                                    {
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                        Ok(Message::Close(_)) => break,
                                        Err(_) => break,
                                        _ => {}
                                    }
                                }
                            }
                            Err(e) => println!("Echec connexion : {}", e),
                        }
                    });
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_machine_ip,
            reset_counter,
            get_counter_state,
            get_all_devices,
            add_annonce,
            delete_annonce,
            register_device,
            get_annonces
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn get_machine_ip() -> String {
    match local_ip() {
        Ok(ip) => ip.to_string(),
        Err(e) => {
            println!("Erreur IP: {}", e);
            "127.0.0.1".to_string()
        }
    }
}

#[tauri::command]
fn reset_counter(state: tauri::State<std::sync::Arc<Database>>) -> EtatFile {
    state.reset()
}

#[tauri::command]
fn get_counter_state(state: tauri::State<std::sync::Arc<Database>>) -> EtatFile {
    state.get_current()
}

#[tauri::command]
fn get_all_devices(state: State<Arc<Database>>) -> Vec<Device> {
    state.get_all_devices()
}

#[tauri::command]
fn get_annonces(state: State<Arc<Database>>) -> Vec<Annonce> {
    state.get_annonces()
}

#[tauri::command]
fn add_annonce(state: State<Arc<Database>>, message: String) -> Result<(), String> {
    state.add_annonce(message)
}

#[tauri::command]
fn delete_annonce(state: State<Arc<Database>>, id: i32) -> Result<(), String> {
    state.delete_annonce(id)
}

#[tauri::command]
fn register_device(state: State<Arc<Database>>, name: String) -> Result<(), String> {
    println!("{}", name);
    state.register_device(name)
}
