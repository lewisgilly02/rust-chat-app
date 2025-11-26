

use std::error::Error;
use std::ptr::read;
use tokio::io::AsyncBufReadExt;
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::OwnedWriteHalf;

/*
    the client (in this iteration) connects to the server and sends / reads messages. I think a CLI command based client is fine until front end.


*/

#[tokio::main]


async fn main() -> Result<(), Box<dyn Error>> {
    const PORT: &str = "127.0.0.1:7878";
    let mut stream = TcpStream::connect(PORT).await?;
    // de structure the tuple returned by into split to seperate the two stream halves
    let (mut reader, mut writer) = stream.into_split();

    tokio::select! {
        _ = listen_for_messages(reader) => {
            println!("Server closed connection");
        }

        _ = write_messages(writer) => {
            println!("client closed connection")

        }

    }

    


    //end of main
    Ok(())
}

async fn listen_for_messages(mut stream: OwnedReadHalf) -> Result<(), Box<dyn Error>>  {
    let mut buffer = [0u8; 1024];

    loop{

        let n = stream.read(&mut buffer).await?;
        if n == 0 {
            break;
        }

        println!("received {}", String::from_utf8_lossy(&buffer[..n]))
    }

    Ok(())
}   


// loop to keep prompting to write a message
async fn write_messages(mut stream: OwnedWriteHalf) -> Result<(), Box<dyn Error>> {
        // stdin is the reusable buffer each typed message is stored
        let mut stdin = BufReader::new(io::stdin());
        let mut buffer = String::new();


        
        loop {
            println!("enter your message: ");
            buffer.clear();
            let bytes_read = stdin.read_line(&mut buffer).await?;

            // sending an empty message is the exit in place of a button or whatever
            if bytes_read == 0 {
                break;
            }
            stream.write_all(buffer.as_bytes()).await?;
    }
    Ok(())
}