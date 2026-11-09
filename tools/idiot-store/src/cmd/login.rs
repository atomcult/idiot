use idiot_store::store::login::login;
use idiot_store::StoreError;
use std::error::Error;
use std::io::{self, Write};

pub async fn run() -> Result<(), Box<dyn Error>> {
    let email = prompt_email()?;
    let password = rpassword::prompt_password("Password: ")?;

    let credential = match login(email.clone(), password.clone(), None).await {
        Ok(c) => c,
        Err(e) => match e.downcast::<StoreError>() {
            Ok(store_err) if matches!(*store_err, StoreError::TwoFactorRequired) => {
                let otp = rpassword::prompt_password("One-time password: ")?;
                login(email, password, Some(otp)).await?
            }
            Ok(store_err) => return Err(store_err),
            Err(other) => return Err(other),
        },
    };

    println!("{credential}");
    Ok(())
}

fn prompt_email() -> Result<String, Box<dyn Error>> {
    eprint!("Email: ");
    io::stderr().flush()?;
    let mut email = String::new();
    io::stdin().read_line(&mut email)?;
    Ok(email.trim().to_string())
}
