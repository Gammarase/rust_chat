use std::{
    collections::{HashMap, HashSet},
    io,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    }, usize,
};


use crate::datasource::{self, Datasource};
use rand::{thread_rng, Rng as _};
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
enum Command {
    Connect {
        conn_tx: mpsc::UnboundedSender<String>,
        res_tx: oneshot::Sender<usize>,
    },

    Disconnect {
        conn: usize,
    },

    List {
        res_tx: oneshot::Sender<Vec<String>>,
    },

    Join {
        conn: usize,
        room: String,
        res_tx: oneshot::Sender<()>,
    },

    Message {
        msg: String,
        conn: usize,
        res_tx: oneshot::Sender<()>,
    },
}

#[derive(Debug)]
pub struct ChatServer {
    sessions: HashMap<usize, mpsc::UnboundedSender<String>>,
    rooms: HashMap<String, HashSet<usize>>,
    visitor_count: Arc<AtomicUsize>,
    cmd_rx: mpsc::UnboundedReceiver<Command>,
}

impl ChatServer {
    pub fn new() -> (Self, ChatServerHandle) {
        let mut rooms = HashMap::with_capacity(4);

        rooms.insert("main".to_owned(), HashSet::new());

        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();

        (
            Self {
                sessions: HashMap::new(),
                rooms,
                visitor_count: Arc::new(AtomicUsize::new(0)),
                cmd_rx,
            },
            ChatServerHandle { cmd_tx },
        )
    }

    async fn send_system_message(&self, room: &str, skip: usize, msg: impl Into<String>) {
        if let Some(sessions) = self.rooms.get(room) {
            let msg = msg.into();

            for conn_id in sessions {
                if *conn_id != skip {
                    if let Some(tx) = self.sessions.get(conn_id) {
                        let _ = tx.send(msg.clone());
                    }
                }
            }
        }
    }
   
    async fn send_message(&self, conn: usize, msg: impl Into<String>) {
    let msg = msg.into(); 
        if let Some(room) = self
            .rooms
            .iter()
            .find_map(|(room, participants)| participants.contains(&conn).then_some(room))
        {
            self.send_system_message(room, conn, msg.clone()).await;
            match Datasource::new() {
                Ok(datasource) => {
                    datasource.store_message(room, &msg).unwrap();
                },
                Err(_) => {
                    log::error!("Failed to store message");
                }
            }
        };
    }

   
    async fn connect(&mut self, tx: mpsc::UnboundedSender<String>) -> usize {
        log::info!("Someone joined");
        self.send_system_message("main", 0, "Someone joined").await;
       
        let id = thread_rng().gen::<usize>();
        self.sessions.insert(id, tx);
       
        self.rooms.entry("main".to_owned()).or_default().insert(id);

        let count = self.visitor_count.fetch_add(1, Ordering::SeqCst);
        self.send_system_message("main", 0, format!("Total visitors {count}"))
            .await;
        id
    }

   
    async fn disconnect(&mut self, conn_id: usize) {
        println!("Someone disconnected");

        let mut rooms: Vec<String> = Vec::new();
        if self.sessions.remove(&conn_id).is_some() {
            for (name, sessions) in &mut self.rooms {
                if sessions.remove(&conn_id) {
                    rooms.push(name.to_owned());
                }
            }
        }
        for room in rooms {
            self.send_system_message(&room, 0, "Someone disconnected")
                .await;
        }
    }

   
    fn list_rooms(&mut self) -> Vec<String> {
        self.rooms.keys().cloned().collect()
    }

   
    async fn join_room(&mut self, conn_id: usize, room: String) {
        let mut rooms = Vec::new();
        for (n, sessions) in &mut self.rooms {
            if sessions.remove(&conn_id) {
                rooms.push(n.to_owned());
            }
        }
        for room in rooms {
            self.send_system_message(&room, 0, "Someone disconnected")
                .await;
        }

        self.rooms.entry(room.clone()).or_default().insert(conn_id);

        self.send_system_message(&room, conn_id, "Someone connected")
            .await;
    }

    pub async fn run(mut self) -> io::Result<()> {
        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                Command::Connect { conn_tx, res_tx } => {
                    let conn_id = self.connect(conn_tx).await;
                    let _ = res_tx.send(conn_id);
                }

                Command::Disconnect { conn } => {
                    self.disconnect(conn).await;
                }

                Command::List { res_tx } => {
                    let _ = res_tx.send(self.list_rooms());
                }

                Command::Join { conn, room, res_tx } => {
                    self.join_room(conn, room).await;
                    let _ = res_tx.send(());
                }

                Command::Message { conn, msg, res_tx } => {
                    self.send_message(conn, msg).await;
                    let _ = res_tx.send(());
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ChatServerHandle {
    cmd_tx: mpsc::UnboundedSender<Command>,
}

impl ChatServerHandle {
    pub async fn connect(&self, conn_tx: mpsc::UnboundedSender<String>) -> usize {
        let (res_tx, res_rx) = oneshot::channel();

        self.cmd_tx
            .send(Command::Connect { conn_tx, res_tx })
            .unwrap();
        res_rx.await.unwrap()
    }

    pub async fn list_rooms(&self) -> Vec<String> {
        let (res_tx, res_rx) = oneshot::channel();

        self.cmd_tx.send(Command::List { res_tx }).unwrap();

        res_rx.await.unwrap()
    }

    pub async fn join_room(&self, conn: usize, room: impl Into<String>) {
        let (res_tx, res_rx) = oneshot::channel();

        self.cmd_tx
            .send(Command::Join {
                conn,
                room: room.into(),
                res_tx,
            })
            .unwrap();

        res_rx.await.unwrap();
    }

    pub async fn send_message(&self, conn: usize, msg: impl Into<String>) {
        let (res_tx, res_rx) = oneshot::channel();

        self.cmd_tx
            .send(Command::Message {
                msg: msg.into(),
                conn,
                res_tx,
            })
            .unwrap();

        res_rx.await.unwrap();
    }

    pub fn disconnect(&self, conn: usize) {
        self.cmd_tx.send(Command::Disconnect { conn }).unwrap();
    }
}
