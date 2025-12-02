

use tokio::io::AsyncBufReadExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::tcp::OwnedWriteHalf;
use std::collections::HashMap;
use tokio::io::BufReader;
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
    active_users: Vec<UserId>, // prob wont be used until post front end intergration
    
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
    // inserting channels for testing
    create_test_channels_and_add_to_state(shared_state.clone()).await;

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

async fn create_test_channels_and_add_to_state(state: Arc<Mutex<ServerState>>){
   let channel_1 = Channel{
       id: 1,
       name: String::from("First"),
       kind: ChannelKind::Broadcast,
       members: Vec::new(),
       active_users: Vec::new()
   };


    let channel_2 = Channel{
       id: 2,
       name: String::from("Second_Channel!"),
       kind: ChannelKind::Broadcast,
       members: Vec::new(),
       active_users: Vec::new()
   };

   {
    let mut state = state.lock().await;
    state.channels.insert(channel_1.id, channel_1);
    state.channels.insert(channel_2.id, channel_2);
   }
}





async fn handle_client(stream: TcpStream, server_state: Arc<Mutex<ServerState>>) -> Result<(), AnyError>{

    let (reader, writer) = stream.into_split();

    let reader = BufReader::new(reader);

    let state_clone = &server_state.clone();
    
    match login_phase(reader, writer, state_clone).await?{
        LoginResult::Success(user_id , reader) => {
            session_loop(user_id, reader, state_clone).await?;
        }
        LoginResult::Disconnected => {
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
        return Ok(LoginResult::Disconnected)
        // client disconnects immediately
    }

    let line = line.trim_end(); // removes \n
    if let Some(rest) = line.strip_prefix("LOGIN ") {

        let username = rest.to_string();

        let user_id = {

            let mut state = state_clone.lock().await;

            let id = state.next_user_id;

            state.next_user_id += 1;

            id
        };

    
        let available_channels = get_available_channels(state_clone).await.unwrap();


        {
            let mut state = state_clone.lock().await;

            
            let user = User {id: user_id, username: username.clone()};
            
            state.users.insert(user_id, user);
            
            state.connections.insert(
                user_id,
                ClientConnection { user_id: Some(user_id), writer }
            ); 

            let username = state.users.get(&user_id).unwrap().username.clone();
            
            if let Some(conn) = state.connections.get_mut(&user_id){

                
                conn.writer.write_all(format!("Welcome, {username}, you successfully logged in!\n").as_bytes()).await?;
                conn.writer.write_all(format!("Available channels: {available_channels}").as_bytes()).await?;

            }
        }
        return Ok(LoginResult::Success(user_id, reader));
        
    } else {
        writer.write_all("expected: LOGIN - dropping connection. Please try again!".as_bytes()).await?;
        return Ok(LoginResult::Disconnected);
    }


}
enum LoginResult{
    Success(UserId, BufReader<OwnedReadHalf>),
    Disconnected,
}


// this cannot be called within a block where the ARC MUTEX is locked
// it is best to do so; slow work like this should be 
async fn get_available_channels(state: &Arc<Mutex<ServerState>>) -> Result<String, AnyError>{
    
    
    let names = {
        let state = state.lock().await;
        state.channels.values().map(|c| c.name.clone()).collect::<Vec<_>>()
    };

    let mut output = String::new();

    for name in names {
        output.push('\n');
        output.push_str(&name);
        output.push('\n');
    }

        
    
    Ok(output)
}


async fn session_loop(user_id: UserId, mut reader: BufReader<OwnedReadHalf>, state: &Arc<Mutex<ServerState>>) -> Result<(), AnyError>{
    println!("a client has reached the session loop");
    let mut line = String::new();
    loop {
        // pretty much each command uses the state mutex so we will just clone per request - cheap enough unless I figure out a better way.
        let state_clone = state.clone();

        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            println!("client disconnected");
            break;
        }

        let command = parse_command(line.as_str());
        

        match command{
            Command::Join(name) =>
            {
                join_channel(name, user_id, state_clone).await?;
            },
            Command::Active(name) =>
            {

            },
            Command::Inactive => todo!(),
            Command::Message(message) => todo!(),
            Command::Quit => todo!(),
            Command::Unknown => send_message_to_client(&state_clone, user_id, "unknown command".to_string()).await?,
        }
        
    }
    Ok(())
}

enum Command {
    Join(String),
    Active(String),
    Inactive,
    Message(String),
    Quit,
    Unknown,
}

fn parse_command(line: &str) -> Command {
    if let Some(name) = line.strip_prefix("JOIN ")
    {
        Command::Join(name.to_string())
    }
    else if let Some(message) = line.strip_prefix("MESSAGE ")
    {
        Command::Message(message.to_string())
    }
    else if let Some(name) = line.strip_prefix("ACTIVE ")
    {
        Command::Active(name.to_string())
    }
    else if line == "QUIT" 
    {
        Command::Quit
    }
    else 
    {
        Command::Unknown
    }
}

async fn join_channel(channel_name: String, user_id: UserId, state: Arc<Mutex<ServerState>>) -> Result<(), AnyError>{

    println!("join channel function activated for {}", &channel_name);
    // the function triggered when a client wants to join a channel
    // of course we have yet to introduce permissions etc, so client will be able to just say any channel name that exists.
    // needed from state = state.servers to modify
    // flow - match the paramater channel_name to the channel

    // go over each channel in state.channels and see which channelid matches the channel_name
    // save the channel id
    //  go back into state and append user id to the members of state.channels,get_mut(channel_id)
    let state_clone_for_match = state.clone();

    let state_clone_for_send_message = state.clone();

    match match_channel_name_to_id(channel_name, &state_clone_for_match).await {
        // this some wont trigger indicating the name is getting manipulated incorrectly perhaps or that this function doesn't work in the intended way
        Some(channel_id) => {
            let mut channel_name = String::new();
            {
                
                let mut state = state.lock().await;
    
                //unwrap is fine here as we verify the channel to exist in match name to id
                let channel_to_join = state.channels.get_mut(&channel_id).unwrap();
                
                channel_name = channel_to_join.name.clone();

                channel_to_join.members.push(user_id);
                
            }
            // issue - we cant mutable borrow state twice, but I need to mutable borrow in order to use conn.writer
            // okay, technically, we dont NEED to do this below bit. Perhaps if we wrap it in another function?
            let message = format!("you have successfully joined {channel_name}!");

            send_message_to_client(&state_clone_for_send_message, user_id, message).await?;
            
            // following this the client will receive a message saying like "you joined {channel name}"
            // begs the question do i need to establish some kinda function for message a specific client (although in the future the client will likely also receive
            // commands to display these as like a banner)
        },
        None => {
            send_message_to_client(&state_clone_for_send_message, user_id, "Requested server not found.".to_string()).await?;
        }
    }
    Ok(())
}


async fn match_channel_name_to_id(target_channel_name: String, state_clone: &Arc<Mutex<ServerState>>) -> Option<ChannelId>{
    println!("attempting to join client to {}", &target_channel_name);
    let mut target_channel_id: Option<ChannelId> = None;

    let state = state_clone.lock().await;
    

    for(channel_id, channel) in state.channels.iter() {
        if channel.name == target_channel_name {
            target_channel_id = Some(*channel_id);
            break;
        }
    }
    target_channel_id
}

async fn send_message_to_client(state_clone:  &Arc<Mutex<ServerState>>, user_id: UserId, message: String) -> Result<(), AnyError>{
    // lock state - find user connection based on user id - send them the message

    let mut state = state_clone.lock().await;

    if let Some(conn) = state.connections.get_mut(&user_id){
        conn.writer.write_all(message.as_bytes()).await?;
    }
    Ok(())
}