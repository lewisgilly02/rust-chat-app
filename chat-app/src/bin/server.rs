

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


/*
* Ok had to use chat gpt to explain this to me
* previously I was using Box< dyn std::error::Error> as the result of async functions
* but the issue was tokio::spawn requires whatever is in it is safe to move
* to another OS thread but box dyn error isn't marked as thread safe so will just define it here
* and use anyerror and hope it doesn't cause issues down the line
*/
type AnyError = Box<dyn std::error::Error + Send + Sync>;


#[tokio::main]  
async fn main() -> Result<(), AnyError> {
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












async fn handle_client(mut stream: TcpStream, serverState: Arc<Mutex<ServerState>>) -> Result<(), AnyError>{

    let (reader, mut writer) = stream.into_split();

    let mut reader = BufReader::new(reader);

    let state_clone = &serverState.clone();
    
    match login_phase(reader, writer, state_clone).await?{
        LoginResult::Success(user_id , reader) => {
            main_message_loop(user_id, reader).await?;
        }
        LoginResult::disconnected => {
            return Ok(())
        }
    }

    Ok(())
}

/* =========================================
*
* ====== BELOW LIES HELPER FUNCTIONS =======
*
* ==========================================
*/ 

async fn login_phase(mut reader: BufReader<OwnedReadHalf>,mut writer: OwnedWriteHalf, state_clone:  &Arc<Mutex<ServerState>>) -> Result<LoginResult, AnyError>{
    println!("a client has entered the login phase");
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
            id
        };


        

        {
            let mut state = state_clone.lock().await;

            // creates new user & add to server state connections
            let user = User {id: user_id, username: username.clone()};
            
            state.users.insert(user_id, user);
            
            state.connections.insert(
                user_id,
                ClientConnection { user_id: Some(user_id), writer }
            ); 
            
            // this caused a crash as it unwrapped a none value as we insert the client details to state below here.
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


async fn main_message_loop(user_id: UserId, mut reader: BufReader<OwnedReadHalf>) -> Result<(), AnyError>{
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

