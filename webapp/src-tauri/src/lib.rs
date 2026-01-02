use tauri::{Emitter, Manager};
use futures_util::StreamExt;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_hdr_async; 
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response, ErrorResponse};
use tokio_tungstenite::tungstenite::http::StatusCode;
use std::sync::Mutex;
use rusqlite::{params, Connection};
use local_ip_address::local_ip;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

// Structure pour renvoyer les données au JS
#[derive(serde::Serialize, Clone)]
struct EtatFile {
    compteur: i32,
    guichet: String,
}

struct Database {
    // On protège la connexion avec un Mutex car plusieurs threads (Websocket) vont l'utiliser
    conn: Mutex<Connection>,
}

impl Database {
    // 1. Initialisation : Crée le fichier .db et la table si elle n'existe pas
    fn init() -> Self {
        let conn = Connection::open("qms.db").expect("Impossible d'ouvrir la DB");

        // On crée une table simple qui ne contiendra QU'UNE SEULE LIGNE pour l'état actuel
        conn.execute(
            "CREATE TABLE IF NOT EXISTS etat_courant (
                id INTEGER PRIMARY KEY,
                valeur_compteur INTEGER NOT NULL,
                dernier_guichet TEXT NOT NULL
            )",
            [],
        ).unwrap();

        // On s'assure qu'il y a une ligne initialisée à 0 (si la table est vide)
        conn.execute(
            "INSERT OR IGNORE INTO etat_courant (id, valeur_compteur, dernier_guichet) 
             VALUES (1, 0, 'Aucun')",
            [],
        ).unwrap();

        Database {
            conn: Mutex::new(conn),
        }
    }

    // 2. Fonction INCREMENTER
    fn incrementer(&self, nom_guichet: String) -> EtatFile {
        let conn = self.conn.lock().unwrap();
        
        // On met à jour le compteur (+1)
        conn.execute(
            "UPDATE etat_courant SET valeur_compteur = valeur_compteur + 1, dernier_guichet = ?1 WHERE id = 1",
            params![nom_guichet],
        ).unwrap();

        self.lire_etat(&conn)
    }

    // 3. Fonction RESET (Remise à zéro)
    fn reset(&self) -> EtatFile {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "UPDATE etat_courant SET valeur_compteur = 0, dernier_guichet = 'Reset' WHERE id = 1",
            [],
        ).unwrap();

        self.lire_etat(&conn)
    }

    // Petite fonction utilitaire pour lire l'état actuel
    fn lire_etat(&self, conn: &Connection) -> EtatFile {
        let mut stmt = conn.prepare("SELECT valeur_compteur, dernier_guichet FROM etat_courant WHERE id = 1").unwrap();
        
        let etat = stmt.query_row([], |row| {
            Ok(EtatFile {
                compteur: row.get(0)?,
                guichet: row.get(1)?,
            })
        }).unwrap();

        etat
    }
    
    // Fonction publique pour récupérer l'état (utile au chargement de l'app)
    fn get_current(&self) -> EtatFile {
        let conn = self.conn.lock().unwrap();
        self.lire_etat(&conn)
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();
            
            // On initialise la DB
            let db = std::sync::Arc::new(Database::init());
            
            // On rend la DB accessible aux commandes Tauri (pour le bouton Reset du frontend)
            app.manage(db.clone());

            tauri::async_runtime::spawn(async move {
                let addr = "0.0.0.0:8765";
                let listener = TcpListener::bind(&addr).await.expect("Impossible de lancer le serveur");
                
                println!("🚀 Serveur WebSocket sécurisé en écoute sur : ws://{}", addr);

                while let Ok((stream, _)) = listener.accept().await {
                    let app_handle = app_handle.clone();
                    
                    tokio::spawn(async move {
                        // C'est ICI que tout change.
                        // On définit une "callback" qui reçoit la requête HTTP
                        let callback = |req: &Request, response: Response| {
                            let uri = req.uri();
                            println!("Tentative de connexion avec l'URI : {:?}", uri);

                            // 1. On récupère la partie après le '?' (ex: "id=user_123")
                            let query = uri.query().unwrap_or("");

                            // 2. LOGIQUE DE VÉRIFICATION
                            // Ici, on vérifie si la query contient l'ID attendu.
                            // Dans un vrai cas, tu ferais une vérification plus poussée (base de données, liste, etc.)
                            let id_attendu = "id=user_123"; 

                            if query.contains(id_attendu) {
                                println!("✅ ID validé ! Connexion autorisée.");
                                Ok(response) // On accepte la connexion
                            } else {
                                println!("⛔ ID invalide ou manquant. Connexion rejetée.");
                                
                                // On crée une réponse d'erreur (Code 403 Forbidden)
                                let mut error_resp = ErrorResponse::new(Some("ID Invalide".to_string()));
                                *error_resp.status_mut() = StatusCode::FORBIDDEN;
                                Err(error_resp)
                            }
                        };

                        // 3. On tente l'upgrade WebSocket avec notre callback de sécurité
                        match accept_hdr_async(stream, callback).await {
                            Ok(ws_stream) => {
                                println!("Client connecté et authentifié !");
                                let (_, mut read) = ws_stream.split();

                                while let Some(msg) = read.next().await {
                                    match msg {
                                        Ok(message) => {
                                            if message.is_text() {
                                                let text = message.to_string();
                                                app_handle.emit("nouveau-message", text).unwrap(); 
                                            }
                                        }
                                        Err(_) => break,  
                                    }
                                }
                                println!("Client déconnecté");
                            },
                            Err(e) => {
                                // C'est ici qu'on arrive si l'ID était faux
                                println!("La connexion a échoué (Probablement mauvais ID) : {}", e);
                            }
                        }
                    });
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
          get_machine_ip, reset_counter, get_counter_state, generate_token
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

// Commande Tauri pour le bouton RESET du frontend (optionnel)
#[tauri::command]
fn reset_counter(state: tauri::State<std::sync::Arc<Database>>) -> EtatFile {
    state.reset()
}

// Commande pour charger l'état au démarrage du frontend
#[tauri::command]
fn get_counter_state(state: tauri::State<std::sync::Arc<Database>>) -> EtatFile {
    state.get_current()
}

#[tauri::command]
fn generate_token(device: String) -> String {
  println!("device {}", device);

  thread_rng()
      .sample_iter(&Alphanumeric)
      .take(16)
      .map(char::from)
      .collect()
}
