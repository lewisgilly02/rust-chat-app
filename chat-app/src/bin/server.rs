

use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::OwnedWriteHalf;
use std::collections::HashMap;
use std::time::SystemTime;




/*
user represents one logical user of the app
it will be created upon login / connection of a client
stored in serverstate.users


*/
type UserId = u64;
struct User{
    id: UserId,
    username: String
}

/*
    represents a message and its associated data

*/
struct Message {
    author_id: UserId,
    content: String,
    timestamp: SystemTime
}

/*
    represents a channel - its various types and associated data

*/
enum ChannelKind {
    Public,
    GroupChat,
    DirectMessage,
    Broadcast
}

type ChannelId = u64;
struct Channel {
    id: ChannelId,
    name: String,
    kind: ChannelKind,
    members: Vec<UserId>,
    
}
/*
    represents an active TCP connection to the server
    created as soon as a client connects (before login)
    holds the writer half so the server can send messages to this client
    stored in ServerState.connections

*/
struct ClientConnection {
    user_id: Option<UserId>,
    writer: OwnedWriteHalf
}
/*
    global in memory storage for all app data
    created once in main before accepting any connections
    shared across all async tasks via arc mutex

*/
struct ServerState {
    next_user_id: UserId,
    users: HashMap<UserId, User>,
    connections: HashMap<UserId, ClientConnection>,
    channels: HashMap<ChannelId, Channel>,
    messages: HashMap<ChannelId, Vec<Message>>
}

#[tokio::main]  
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    const PORT: &str = "127.0.0.1:7878";
    let listener = TcpListener::bind(PORT).await?;

    /*
        main runs a loop awaiting new listeners and for each opens up an async task to handle each user
        this is far more performant
    
     */
    loop {
        let (socket, address) = listener.accept().await?;
        println!("client connected from {}", address);
        tokio::spawn(async move {
            // handle client inside
            handle_client(socket).await;
        });
    }
    // end of main
    Ok(())
}

// the function that listens for messages
async fn handle_client(mut stream: TcpStream) -> Result<(), Box<dyn std::error::Error>>{
    let mut buffer = [0u8 ;1024];
    loop {


        match stream.read(&mut buffer).await? {
            0 => {
                println!("client has disconnected");
                break;
            }
            // n here being the length / amount of bytes received via message (max size 1024)
            n => {
                println!("Message received: {}", String::from_utf8_lossy(&buffer[..n]));
                stream.write_all(b"thank you for messaging!").await?;
            }
        }
        /* old version may be useful to re read
        *
        *
        *    match stream.read(&mut buffer).await{
        *       Ok(0) => {
        *            println!("client disconnected");
        *            break;
        *        }
        *
        *        Ok(n) => {
        *            println!("Message received: {}", String::from_utf8_lossy(&buffer[..n]));
        *            stream.write("Thank you for messaging!".as_bytes()).await;
        *            
        *        }
        *        Err(e) => {
        *            eprintln!("error reading from client {}", e);
        *        }
        *    };
        * 
        *
        *
        */

       
        
    }
    Ok(())
}

