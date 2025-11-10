

use std::error::Error;
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/*
    the client (in this iteration) connects to the server and sends / reads messages to it to be relayed to the other client


*/

#[tokio::main]
/*
    BEFORE CONTINUING HERE, LEARN ABOUT RUNNING CONCURRENTLY 
    do a deep dive on tokio and async runtime


*/


async fn main() -> Result<(), Box<dyn Error>> {
    const PORT: &str = "127.0.0.1:7878";
    let mut stream = TcpStream::connect(PORT).await?;
    // de structure the tuple returned by into split to seperate the two stream halves
    let (mut reader, mut writer) = stream.into_split();
    listen(reader).await;
    writer.write("hello, world!".as_bytes()).await?;
    


    //end of main
    Ok(())
}

async fn listen(mut stream: OwnedReadHalf) -> Result<(), Box<dyn Error>>  {
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