use server_fns::server;

fn main() {}

#[server]
pub async fn example() -> Result<(), ()> {
    // body
    Ok(())
}
