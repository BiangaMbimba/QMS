use local_ip_address::local_ip;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};
use tts::Tts;
use axum::{
    extract::{Query, State},
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post},
    Json, Router,
};
use std::{collections::HashMap, sync::{Arc, Mutex}, time::Duration};
use tokio::sync::broadcast;
use futures::stream::Stream;

struct AppState {
    db: Arc<Database>,
    app_handle: tauri::AppHandle,
    tx: broadcast::Sender<String>,
    tts: Arc<Mutex<Option<Tts>>>,
}


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

async fn next_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<String> {
    
    // 1. Authentication
    let token = params.get("token").map(|s| s.as_str()).unwrap_or("");
    let device_info = state.db.get_device_info(token);

    if let Some((_id, device_name)) = device_info {
        println!("🟢 Button pressed by: {}", device_name);

        // 2. Logic (Increment DB)
        let nouveau_numero = state.db.incrementer(&device_name).compteur;

        // 3. Emit to Tauri Frontend
        let event_payload = EtatFile {
            guichet: device_name.clone(),
            compteur: nouveau_numero,
        };
        let _ = state.app_handle.emit("nouveau-message", &event_payload);

        // 4. TTS Speak
        let text_to_speak = format!("Client numéro {}, guichet {}", nouveau_numero, device_name);
        if let Ok(mut tts_guard) = state.tts.lock() {
            if let Some(tts) = tts_guard.as_mut() {
                let _ = tts.speak(text_to_speak, true);
            }
        }

        // 5. Broadcast update to SSE Screens
        // We send a JSON string so screens can parse it
        let broadcast_msg = serde_json::json!({
            "guichet": device_name,
            "compteur": nouveau_numero
        }).to_string();
        
        let _ = state.tx.send(broadcast_msg);

        // 6. Response to Button (ESP32)
        return Json("OK".into());
    }

    Json("FORBIDDEN".into())
}

// --- HANDLER 2: SCREENS (SSE GET /events) ---
// usage: GET http://ip:8765/events
async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, axum::Error>>> {
    
    println!("✅ New Screen Connected to SSE");

    // Subscribe to the broadcast channel
    let mut rx = state.tx.subscribe();

    // Create a stream that listens to the channel
    let stream = async_stream::stream! {
        // Send initial "Keep Alive" or Welcome message
        yield Ok(Event::default().data("connected"));

        while let Ok(msg) = rx.recv().await {
            yield Ok(Event::default().data(msg));
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
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

            // Initialize TTS once (Shared across threads)
            let tts_instance = match Tts::default() {
                Ok(t) => Some(t),
                Err(e) => {
                    eprintln!("Error initializing TTS: {}", e);
                    None
                }
            };
            let tts = Arc::new(Mutex::new(tts_instance));

            // Create Broadcast Channel (Capacity 100)
            let (tx, _rx) = broadcast::channel(100);

            let heartbeat_tx = tx.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    // We send a "PING" message. 
                    // The ESP32 will see: "data: PING"
                    if let Err(e) = heartbeat_tx.send("PING".to_string()) {
                        // If no clients are connected, this might error, which is fine
                        eprintln!("Heartbeat skipped (no listeners): {}", e);
                    }
                }
            });

            // Create State to pass to handlers
            let state = Arc::new(AppState {
                db: db.clone(),
                app_handle,
                tx,
                tts,
            });

            // Spawn the Web Server
            tauri::async_runtime::spawn(async move {
                // Route Definition
                let app = Router::new()
                    .route("/events", get(sse_handler)) // For SCREENS (SSE)
                    .route("/next", post(next_handler)) // For BUTTONS (POST)
                    .with_state(state);

                let addr = "0.0.0.0:8765";
                let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
                
                println!("🚀 Server SSE/HTTP ready on http://{}", addr);
                
                axum::serve(listener, app).await.unwrap();
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
fn get_all_devices(state: tauri::State<Arc<Database>>) -> Vec<Device> {
    state.get_all_devices()
}

#[tauri::command]
fn get_annonces(state: tauri::State<Arc<Database>>) -> Vec<Annonce> {
    state.get_annonces()
}

#[tauri::command]
fn add_annonce(state: tauri::State<Arc<Database>>, message: String) -> Result<(), String> {
    state.add_annonce(message)
}

#[tauri::command]
fn delete_annonce(state: tauri::State<Arc<Database>>, id: i32) -> Result<(), String> {
    state.delete_annonce(id)
}

#[tauri::command]
fn register_device(state: tauri::State<Arc<Database>>, name: String) -> Result<(), String> {
    println!("{}", name);
    state.register_device(name)
}
