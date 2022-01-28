use std::io::{self, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::Duration;

/// Server addred
const SERVER_ADDR: &str = "127.0.0.1:6000";

/// Max length of message
const MSG_SIZE: usize = 1024;

fn main() {

    // Try to connect to server address.
    let mut client = match TcpStream::connect(SERVER_ADDR) {
        Ok(_client) => {
            println!("Connected to server at: {}", SERVER_ADDR);
            _client
        },
        Err(_) => {
            println!("Failed to connect to server at: {}", SERVER_ADDR);
            std::process::exit(1)
        }
    };

    // Sets nonblocking to true
    client.set_nonblocking(true).expect("Failed to initiate non-blocking!");

    let (sender, receiver) = mpsc::channel::<String>();

    /* Start thread that listens to server. */
    thread::spawn(move || loop {
        let mut msg_buffer = vec![0; MSG_SIZE];

        match client.read(&mut msg_buffer) {
            Ok(0) => {
                println!("Lost connection with server!");
                break;
            }
            // received message
            Ok(len) => {
                if len > 1 {
                    // read until end-of-message (zero character)
                    let msg = std::str::from_utf8(&msg_buffer[..len]).expect("Invalid UTF-8 message!");
                    let msg = msg.trim().trim_matches(char::from(0));
                    println!("{}", msg);
                }
            },
            // no message in stream
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
            // connection error
            Err(_) => {
                println!("Lost connection with server!");
                break;
            }
        }

        /* Send message in channel to server. */
        match receiver.try_recv() {
            // received message from channel
            Ok(msg) => {
                let mut msg_buffer = msg.clone().into_bytes();
                // add zero character to mark end of message
                msg_buffer.resize(MSG_SIZE, 0);

                if client.write_all(&msg_buffer).is_err() {
                    println!("Failed to send message!")
                }
            }, 
            // no message in channel
            Err(TryRecvError::Empty) => (),
            // channel has been disconnected (main thread has terminated)
            Err(TryRecvError::Disconnected) => break
        }
        thread::sleep(Duration::from_millis(100));
    });

    /* Listen for and act on user messages. */
    println!("Chat open:\nType /help for info on commands\n");
    loop {
        let mut msg_buffer = String::new();

        // wait for user to write message
        io::stdin().read_line(&mut msg_buffer).expect("Failed to read user message!");

        let msg = msg_buffer.trim().to_string();
        match msg.as_str() {
            "/help" => {
                println!("\nAvailable commands are:\n/quit Disconnects from chat\n/nick <x> Change nickname on server to x")
            }
            "/quit" => {
                break;
            }
            _ => {
                if sender.send(msg).is_err() {break}
            }
        }
    }

    println!("\nClosing chat...");
}