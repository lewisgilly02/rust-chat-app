

use tokio::io::AsyncBufReadExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::OwnedReadHalf;
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
    active_users: Vec<UserId>,
    
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
        this is far more performant than threaded.
    
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


    // this will later be redeclared to the user ID after login. Maybe I will eventually find a cleaner way of getting this global to handle_client
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
            ); 
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
    // main message loop end


    Ok(())
}

/* =========================================
*
* ====== BELOW LIES HELPER FUNCTIONS =======
*
* ==========================================
*/ 

async fn login_phase(mut reader: BufReader<OwnedReadHalf>,writer: OwnedWriteHalf, state_clone:  &Arc<Mutex<ServerState>>) -> Result<LoginResult, Box<dyn std::error::Error>>{
    
    let mut line = String::new();

    

    // first command - expect a LOGIN this will probably end up being wrapped to a function for readability

    line.clear();
    let n = reader.read_line(&mut line).await?;
    if n == 0 {
        return Ok((LoginResult::disconnected))
        // client disconnects immediately
    }

    let line = line.trim_end(); // removes \n
    if let Some(rest) = line.strip_prefix("LOGIN ") {

        let username = rest.to_string();

        let user_id = {
            let mut state = state_clone.lock().await;
            let id = state.next_user_id;
            state.next_user_id += 1;
            // so after the above line the user's id is locked in here and it can't be affected by anything else until incremented because arc mutex so its fine now
            // it is impossible to end up with mix ups.
            
            /* ok so the issue is this - writer is passed to server state within this block causing ownership issues at the else clause
            * the solution to this, is to do the below insertions after the welcome message is printed
            * (it also makes sense to add to state once the login is actually finished) but im gonna take a break for sanity.
            * also pondering perhaps we could do all the user_id block at the end as it doesnt really matter if users have an ID in the order they join
            * plus it may make this shitshow more readable
            *
            * Extra thought on 26/11/25, yeah its completely redundant -
            * realistically everything after state.nextuserid doesnt actually need to be done here - The user id is sorted anyways so yeah we can just do all the shit after at the end
            * an issue i can see arising is now the state insertions cant use id, but rather will have to use user_id
            */

            // creates new user & add to server state connections
            let user = User {id, username: username.clone()};

            state.users.insert(id, user);

            state.connections.insert(
                id,
                ClientConnection { user_id: Some(id), writer }
            ); 
            id
        };


        

        {
            let mut state = state_clone.lock().await;

            let username = state.users.get(&user_id).unwrap().username.clone();

            if let Some(conn) = state.connections.get_mut(&user_id){
            
                conn.writer.write_all(format!("Welcome, {username}, you successfully logged in!\n").as_bytes()).await?;

            }
        }
        return Ok(LoginResult::Success(user_id, reader));
        
    } else {
        writer.write_all("expected: LOGIN - dropping connection. Please try again!".as_bytes()).await?;
        return Ok((LoginResult::disconnected));
    }


}

enum LoginResult{
    Success(UserId, BufReader<OwnedReadHalf>),
    disconnected,
}

