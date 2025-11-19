

use tokio::io::AsyncBufReadExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::OwnedWriteHalf;
use std::collections::HashMap;
use std::hash::Hash;
use std::mem;
use tokio::io::BufReader;
use std::ptr::read;
use std::time::SystemTime;
use std::sync::Arc;
use tokio::sync::Mutex;




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


    // this is where lies the backend of this server
    let state = ServerState {
        next_user_id: 1,
        users: HashMap::new(),
        connections: HashMap::new(),
        channels: HashMap::new(),
        messages: HashMap::new()
    };
    // wrap so tasks may share a server state
    let shared_state = Arc::new(Mutex::new(state));

    let listener = TcpListener::bind(PORT).await?;

    /*
        main runs a loop awaiting new listeners and for each opens up an async task to handle each user
        this is far more performant
    
     */
    loop {
        let (socket, address) = listener.accept().await?;

        println!("client connected from {}", address);
        let state_for_task = shared_state.clone();

        tokio::spawn(async move {
            // handle client inside
            if let Err(e) = handle_client(socket, state_for_task).await{
                eprintln!("error connecting client... {}", e)
            }
        });
    }
    // end of main
    Ok(())
}












async fn handle_client(mut stream: TcpStream, serverState: Arc<Mutex<ServerState>>) -> Result<(), Box<dyn std::error::Error>>{
    let (reader, mut writer) = stream.into_split();


    // this will later be redeclared to the user ID after login
    let mut client_id: Option<u64> = None;

    let mut reader = BufReader::new(reader);
    
    let mut line = String::new();

    // first command - expect a LOGIN this will probably end up being wrapped to a function for readability

    line.clear();
    let n = reader.read_line(&mut line).await?;
    if n == 0 {
        return Ok(())
        // client disconnects immediately
    }

    let line = line.trim_end(); // removes \n
    if let Some(rest) = line.strip_prefix("LOGIN ") {

        let username = rest.to_string();

        let user_id = {
            let mut state = serverState.lock().await;
            let id = state.next_user_id;
            state.next_user_id += 1;


            // creates new user & add to server state connections
            let user = User {id, username: username.clone()};



            state.users.insert(id, user);

            state.connections.insert(
                id,
                ClientConnection { user_id: Some(id), writer }
            ); // GOT UP TO HERE
            /*
                log - im thinking about how i access the username and shit
                globally - do I maybe have to declare them
                and re-designate them at the start of handle client maybe?
             */
            id
        };

        client_id = Some(user_id);
        let client_id = client_id.unwrap();
        

        {
            let mut state = serverState.lock().await;

            let username = state.users.get(&user_id).unwrap().username.clone();
            
            if let Some(conn) = state.connections.get_mut(&client_id){
            
                conn.writer.write_all(format!("Welcome, {username}, you successfully logged in!\n").as_bytes()).await?;
            }
        }
        
    } else {
        writer.write_all("expected: LOGIN - dropping connection. Please try again!".as_bytes()).await?;
        return Ok(());
    }
    

    // main message loop

    let mut line = String::new();
    loop {

        line.clear();
        
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            println!("client disconnected");
            break;
        }

        let message = line.trim_end();
        

        println!("{}", message);
        
    }
    Ok(())
}

