use std::io::ErrorKind;
use std::io::prelude::*;
use std::net::SocketAddr;
use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::mpsc;
use std::thread;

use emilhul_sockets::ThreadPool;

const SERVER_ADDR: &str = "127.0.0.1:6000";
const MSG_SIZE: usize = 1024;

const CHANGE_NICK: &[u8; 6] = b"/nick ";

fn main() {
    let listener = match TcpListener::bind(SERVER_ADDR) {
        Ok(_listener) => {
            println!("Opened server at: {}", SERVER_ADDR);
            _listener
        },
        Err(_) => {
            println!("Failed to open server at: {}", SERVER_ADDR);
            std::process::exit(1)
        }
    };

    // Sets non-blocking to true.
    listener.set_nonblocking(true).expect("Failed to initiate non-blocking!");

    // Initialze vector that will hold our clients.
    let mut clients = Vec::new();

    // Initialize server_index
    let mut server_index: usize = 0;

    // Initialize a threadpool with size 10. Allowing up to 10 clients to connect simultanously
    let pool = ThreadPool::new(10);

    // Set up channel for sending messages from workers to main thread
    let (msg_sender, msg_reciever) = mpsc::channel::<(SocketAddr, String)>();

    // Start server loop
    loop {
        if let Ok((socket, addr)) = listener.accept() {
            println!("Client {} connected.", addr);

            let sender = msg_sender.clone();

            // Address is for client id and socket to write data back
            clients.push(
                (addr, socket.try_clone().expect("Failed to clone client! Client wont recieve messages!"))
            );

            // Set start nick to server index
            let nick: String = format!("{}", server_index);
            server_index += 1;

            // Get a worker from our thread pool to run handle client.
            pool.execute(move || {
              handle_client(socket, addr, nick, sender);  
            })
        }

        if let Ok(msg) = msg_reciever.try_recv() {
            if msg.1.len() > 0 {
                // Iterate over clients
                clients = clients.into_iter().filter_map(|mut client| {
                    match msg.1.as_str() {
                        _ if client.0 != msg.0 => { // If it's not the sender
                            let buffer = msg.1.clone().into_bytes();
                            client.1.write(&buffer).map(|_| client).ok()
                        },
                        _ => { Some(client) }   // Otherwise don't send anything
                    }
                }).collect::<Vec<_>>();
            }
        }
        sleep(); // The program sleeps shortly before next loop
    }
}

fn handle_client(mut socket: TcpStream, addr: SocketAddr, mut nick: String, sender: mpsc::Sender<(SocketAddr, String)>) {
    loop {
        let mut buffer = [0; MSG_SIZE];

        match socket.read(&mut buffer) {
            Ok(0) => {
                println!("Closing connection with: {}", addr);
                sender.send((addr, format!("SERVER: {} has disconnected", nick))).expect("Failed to relay message!");
                break;
            }
            Ok(len) => {
                if buffer.starts_with(CHANGE_NICK) {
                    let _nick = std::str::from_utf8(&buffer[5..len])
                        .unwrap().trim()
                        .trim_matches(char::from(0));
                    println!("{}/{} changed nickname to {}", addr, nick, _nick);
                    sender.send((addr, format!("SERVER: {} is now known as {}", nick, _nick))).expect("Failed to relay message!");
                    nick = format!("{}", _nick);
                } else {
                    let _msg = std::str::from_utf8(&buffer[..len])
                        .expect("Invalid UTF-8 message!");
                    
                        
                    let msg = format!("{}: {}", nick, _msg.trim_matches(char::from(0)));
                    
                    println!("{}/{}", addr, msg);
                    sender.send((addr, msg)).expect("Failed to relay message!");
                }
            },
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
            Err(_) => {
                println!("Closing connection with: {}", addr);
                sender.send((addr, format!("SERVER: {} has disconnected", nick))).expect("Failed to relay message!");
                break;
            }
        }

        sleep();    
    }
}

fn sleep() {
    thread::sleep(::std::time::Duration::from_millis(100));
}