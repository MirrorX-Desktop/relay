use std::io;

pub struct RedisClient{
    
}

impl RedisClient{
    pub fn new() -> io::Result<Self>{
        let client = redis::Client::open("redis://127.0.0.1")?;
        client.getcon
    }
}