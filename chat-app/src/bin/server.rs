

use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};




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

