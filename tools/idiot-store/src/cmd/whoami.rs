use idiot_store::store::whoami::whoami;
use std::error::Error;
use std::io;

pub async fn run() -> Result<(), Box<dyn Error>> {
    let credential = std::env::var("UBUNTU_STORE_AUTH").map_err(|e| {
        io::Error::new(io::ErrorKind::NotFound, format!("UBUNTU_STORE_AUTH: {e}"))
    })?;

    let info = whoami(&credential).await?;

    let permissions = if info.permissions.is_empty() {
        "no restrictions".to_string()
    } else {
        info.permissions.join(", ")
    };

    let channels = if info.channels.is_empty() {
        "no restrictions".to_string()
    } else {
        info.channels.join(", ")
    };

    let expires = info.expires.as_deref().unwrap_or("N/A");

    println!("email:       {}", info.account.email);
    println!("username:    {}", info.account.username);
    println!("id:          {}", info.account.id);
    println!("permissions: {permissions}");
    println!("channels:    {channels}");
    println!("expires:     {expires}");

    Ok(())
}
