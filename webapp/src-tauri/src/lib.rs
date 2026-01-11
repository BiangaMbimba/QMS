use axum::{
    extract::{Query, State},
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
    Router
};
use futures::stream::Stream;
use local_ip_address::local_ip;
use uuid::Uuid;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{Emitter, Manager};
use tokio::sync::broadcast;
use tts::Tts;

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
    status: Option<String>,
    ip_address: Option<String>
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Annonce {
    id: i32,
    message: String,
    active: bool,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct HistoryItem {
    id: i32,
    ticket_number: i32,
    desk_name: String,
    created_at: String,
}

#[derive(Serialize, Debug)]
pub struct TicketStats {
    ticket_number: i32,
    desk_name: String,
    start_time: String,            // "HH:MM:SS"
    end_time: Option<String>,      // Option because the last client has no end time
    duration_minutes: Option<f64>, // Option because the last client has no duration
}

async fn next_handler(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // 1. Authentication: Extract Token
    let token = match headers.get("Authorization") {
        Some(value) => value.to_str().unwrap_or("").replace("Bearer ", ""),
        None => return (StatusCode::UNAUTHORIZED, "Missing Token").into_response(),
    };

    // 2. Verify Device against DB
    // We use a match statement to handle both Success (Some) and Failure (None)
    match state.db.get_device_info(&token) {
        Some((_id, device_name)) => {
            println!("🟢 Button pressed by: {}", device_name);

            // A. Logic (Increment DB)
            let nouveau_numero = state.db.incrementer(&device_name).compteur;

            // B. Emit to Tauri Frontend (Main Window)
            let event_payload = EtatFile {
                guichet: device_name.clone(),
                compteur: nouveau_numero,
            };
            let _ = state.app_handle.emit("nouveau-message", &event_payload);

            // C. TTS Speak
            let text_to_speak = format!("Client {}, to {}", nouveau_numero, device_name);
            if let Ok(mut tts_guard) = state.tts.lock() {
                if let Some(tts) = tts_guard.as_mut() {
                    let _ = tts.speak(text_to_speak, true);
                }
            }

            // D. Prepare JSON Data
            // Create a JSON Value, not a String, so we can reuse it easily
            let response_json = serde_json::json!({
                "guichet": device_name,
                "compteur": nouveau_numero
            });

            // E. Broadcast update to SSE Screens
            // (Convert to string only for the channel transmission)
            let _ = state.tx.send(response_json.to_string());

            // F. Response to Button (ESP32)
            // Return the JSON object directly.
            (StatusCode::OK, Json(response_json)).into_response()
        }
        
        None => {
            // G. Handle Invalid Token
            println!("🔴 Login attempt with invalid token: {}", token);
            (StatusCode::UNAUTHORIZED, "Invalid Token").into_response()
        }
    }
}

// --- HANDLER 2: SCREENS (SSE GET /events) ---
#[derive(serde::Deserialize)]
struct SseParams {
    token: String,
}

async fn sse_handler(
    Query(params): Query<SseParams>,    // Extract ?token=...
    State(state): State<Arc<AppState>>,
) -> Result<Sse<impl Stream<Item = Result<Event, axum::Error>>>, (StatusCode, String)> {
    
    // 3. Verify Token
    let is_valid = {
        let conn = state.db.conn.lock().unwrap();
        // Check if token exists in 'devices' table
        let mut stmt = conn.prepare("SELECT count(*) FROM devices WHERE token = ?1").unwrap();
        let count: i32 = stmt.query_row(params![params.token], |row| row.get(0)).unwrap_or(0);
        count > 0
    };

    if !is_valid {
        println!("🔴 SSE Connection rejected: Invalid Token");
        // Return 401 Unauthorized
        return Err((StatusCode::UNAUTHORIZED, "Invalid Token".to_string()));
    }

    println!("✅ New Authorized Screen Connected (Token verified)");

    // 4. Set up the Stream (Same as before)
    let mut rx = state.tx.subscribe();

    let stream = async_stream::stream! {
        yield Ok(Event::default().data("connected"));

        while let Ok(msg) = rx.recv().await {
            yield Ok(Event::default().data(msg));
        }
    };

    // Return the stream wrapped in Ok()
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
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
                token TEXT UNIQUE NOT NULL
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

        conn.execute(
            "CREATE TABLE IF NOT EXISTS historique (
                id INTEGER PRIMARY KEY,
                ticket_number INTEGER NOT NULL,
                desk_name TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )
        .unwrap();

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

        let etat = self.lire_etat(&conn);

        // 3. NEW: Save to History
        conn.execute(
            "INSERT INTO historique (ticket_number, desk_name) VALUES (?1, ?2)",
            params![etat.compteur, nom_guichet],
        )
        .unwrap();

        etat
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

        let token = Uuid::new_v4().to_string();

        conn.execute(
            "INSERT OR IGNORE INTO devices (name, token) VALUES (?1, ?2)",
            params![name, token],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn get_device_info(&self, token: &str) -> Option<(i32, String)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, name FROM devices WHERE token = ?1")
            .unwrap();

        let result = stmt.query_row(params![token], |row| Ok((row.get(0)?, row.get(1)?)));

        result.ok()
    }

    fn delete_device(&self, id: i32) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM devices WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
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
                    ip_address: None,
                    status: None
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

    fn update_annonce_message(&self, id: i32, new_message: String) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "UPDATE annonces SET message = ?1 WHERE id = ?2",
            params![new_message, id],
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    fn set_annonce_active(&self, id: i32, is_active: bool) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "UPDATE annonces SET active = ?1 WHERE id = ?2",
            params![is_active, id],
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    fn delete_annonce(&self, id: i32) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM annonces WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_history(&self) -> Vec<HistoryItem> {
        let conn = self.conn.lock().unwrap();

        // Logic:
        // 1. Find the ID of the last "Reset" (-2). If none, use 0.
        // 2. Fetch valid tickets (>= 0) that came AFTER that ID.
        // 3. Skip the first one (OFFSET 1) because it is the "Current" ticket on screen.
        // 4. Take the next 5.
        let sql = "
            WITH LastReset AS (
                SELECT COALESCE(MAX(id), 0) as reset_id 
                FROM historique 
                WHERE ticket_number = -2
            )
            SELECT id, ticket_number, desk_name, created_at 
            FROM historique, LastReset
            WHERE id > LastReset.reset_id 
            AND ticket_number >= 0  -- Ignored -1 (Close) and -2 (Reset)
            ORDER BY id DESC 
            LIMIT 5 OFFSET 1;
            ";

        let mut stmt = conn.prepare(sql).unwrap();

        let iter = stmt
            .query_map([], |row| {
                Ok(HistoryItem {
                    id: row.get(0)?,
                    ticket_number: row.get(1)?,
                    desk_name: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })
            .unwrap();

        iter.map(|h| h.unwrap()).collect()
    }

    // inside impl Database { ... }

    pub fn get_desk_statistics(&self, desk_name: &str) -> Vec<TicketStats> {
        let conn = self.conn.lock().unwrap();

        let sql = "
            -- 1. Find the Global Reset Time (-2)
            -- We NO LONGER look for the local desk close (-1) here.
            WITH SessionStart AS (
                SELECT COALESCE(MAX(created_at), '1970-01-01') as start_time
                FROM historique
                WHERE ticket_number = -2 
            ),
            
            -- 2. Fetch ALL events (Tickets AND Breaks) since the reset
            -- We need the '-1' rows here so the Window Function knows when a ticket 'stopped'.
            RawEvents AS (
                SELECT id, ticket_number, desk_name, created_at
                FROM historique, SessionStart
                WHERE desk_name = ?1 
                AND created_at > SessionStart.start_time
                AND ticket_number >= -1 -- Include Tickets AND Breaks
            ),

            -- 3. Calculate Durations using Window Functions
            CalculatedEvents AS (
                SELECT 
                    ticket_number,
                    desk_name,
                    id, -- Needed for sorting
                    time(created_at) as start_time,
                    -- Look at the NEXT event (whether it's a ticket or a break)
                    LEAD(time(created_at)) OVER (ORDER BY id) as end_time,
                    (julianday(LEAD(created_at) OVER (ORDER BY id)) - julianday(created_at)) * 24 * 60 as duration_minutes
                FROM RawEvents
            )

            -- 4. Final Display: Remove the '-1' rows
            SELECT 
                ticket_number, desk_name, start_time, end_time, duration_minutes
            FROM CalculatedEvents
            WHERE ticket_number >= 0 -- Only show real tickets
            ORDER BY id DESC;
            ";

        let mut stmt = conn.prepare(sql).unwrap();

        // We pass 'desk' twice: once for the Subquery, once for the Main Query
        let iter = stmt
            .query_map(params![desk_name], |row| {
                Ok(TicketStats {
                    ticket_number: row.get(0)?,
                    desk_name: row.get(1)?,
                    start_time: row.get(2)?,
                    end_time: row.get(3).ok(),
                    duration_minutes: row.get(4).ok(),
                })
            })
            .unwrap();

        iter.filter_map(Result::ok).collect()
    }

    pub fn close_desk(&self, desk_name: String) -> Result<String, String> {
        let conn = self.conn.lock().unwrap();

        let last_ticket_result: Result<i32, rusqlite::Error> = conn.query_row(
            "SELECT ticket_number FROM historique WHERE desk_name = ?1 ORDER BY id DESC LIMIT 1",
            params![desk_name],
            |row| row.get(0),
        );

        match last_ticket_result {
            // Case A: Already Closed
            Ok(-1) => {
                println!("⚠️ Desk '{}' is already closed.", desk_name);
                Ok("ALREADY_CLOSED".to_string()) // Send this code to JS
            }

            // Case B: No History
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                println!("⚠️ Desk '{}' has no history.", desk_name);
                Ok("NO_HISTORY".to_string()) // Send this code to JS
            }

            // Case C: Success (We try to close it)
            Ok(_ticket_num) => {
                let insert_result = conn.execute(
                    "INSERT INTO historique (ticket_number, desk_name) VALUES (-1, ?1)",
                    params![desk_name],
                );

                match insert_result {
                    Ok(_) => {
                        println!("✅ Desk '{}' closed.", desk_name);
                        Ok("SUCCESS".to_string()) // Send this code to JS
                    }
                    Err(e) => {
                        eprintln!("❌ DB Error: {}", e);
                        Err(e.to_string()) // Send actual error to JS (Promise reject)
                    }
                }
            }

            // Case D: Database Error during check
            Err(e) => {
                eprintln!("❌ DB Error: {}", e);
                Err(e.to_string())
            }
        }
    }

    fn reset_display_history(&self) -> EtatFile {
        let conn = self.conn.lock().unwrap();

        // Insert the -2 marker.
        // We can use a generic name like "Admin" or "System" for the desk_name.
        conn.execute(
            "INSERT INTO historique (ticket_number, desk_name) VALUES (-2, 'System')",
            [],
        )
        .unwrap();

        println!("History display reset marker (-2) added.");

        conn.execute(
            "UPDATE etat_courant SET valeur_compteur = 0, dernier_guichet = 'Reset' WHERE id = 1",
            [],
        )
        .unwrap();
        self.lire_etat(&conn)
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
            get_annonces,
            get_history,
            get_stats,
            update_annonce_message,
            set_annonce_active,
            delete_device
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


/**
 * COUNTER ***************************************************************
 */

#[tauri::command]
fn reset_counter(state: tauri::State<std::sync::Arc<Database>>) -> EtatFile {
    state.reset_display_history()
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
fn delete_device(state: tauri::State<Arc<Database>>, id: i32) -> Result<(), String> {
    state.delete_device(id)
}

/**
 * ANNOUNCEMENT *********************************************************
 */

#[tauri::command]
fn get_annonces(state: tauri::State<Arc<Database>>) -> Vec<Annonce> {
    state.get_annonces()
}

#[tauri::command]
fn add_annonce(state: tauri::State<Arc<Database>>, message: String) -> Result<(), String> {
    state.add_annonce(message)
}

#[tauri::command]
fn update_annonce_message(state: tauri::State<Arc<Database>>, id: i32, message: String) -> Result<(), String> {
    state.update_annonce_message(id, message)
}

#[tauri::command]
fn set_annonce_active(state: tauri::State<Arc<Database>>, id: i32, is_active: bool) -> Result<(), String> {
    state.set_annonce_active(id, is_active)
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

#[tauri::command]
fn get_history(state: tauri::State<Arc<Database>>) -> Vec<HistoryItem> {
    state.get_history()
}

#[tauri::command]
fn get_stats(desk_name: String, state: tauri::State<std::sync::Arc<Database>>) -> Vec<TicketStats> {
    state.get_desk_statistics(&desk_name)
}
