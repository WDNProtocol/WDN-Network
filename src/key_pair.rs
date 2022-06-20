use crypto::digest::Digest;
use crypto::md5::Md5;
use rand::rngs::OsRng;
use rand::RngCore;
use std::error::Error;
use std::io::{stdin, stdout, Read, Write};
use std::{fmt, fs};
use std::{path::Path, result};
use termion::input::TermRead;

use libp2p::identity::Keypair;
use serde::{Deserialize, Serialize};

use crate::{encrypt, module_quick_from};

const KEY_FILENAME: &str = "key";

pub type Result<T> = result::Result<T, KeyPairError>;

#[derive(Serialize, Debug, Default)]
pub struct KeyPairError {
    pub message: String,
}

impl fmt::Display for KeyPairError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for KeyPairError {}

module_quick_from!(String, KeyPairError);
module_quick_from!(std::io::Error, KeyPairError);
module_quick_from!(serde_json::Error, KeyPairError);
module_quick_from!(libp2p::identity::error::DecodingError, KeyPairError);
module_quick_from!(crypto::symmetriccipher::SymmetricCipherError, KeyPairError);

pub fn new(base_path: String) -> Result<Keypair> {
    let p = Path::new(&base_path).join(KEY_FILENAME);
    if p.exists() {
        load_from_file(&p)
    } else {
        create(&p)
    }
}

fn load_from_file(p: &Path) -> Result<Keypair> {
    println!("found node key, please input password");

    let file_data = fs::read_to_string(p)?;
    let local_key_data: LocalKey = serde_json::from_str(&file_data)?;

    let mut decrypted_key = vec![];
    for i in 0..2 {
        let password = read_password()?;
        let md5_password = md5(&password);

        let decrypted_key_res =
            encrypt::decrypt(&local_key_data.local_key, &md5_password, &local_key_data.iv);
        match decrypted_key_res {
            Ok(dk) => {
                decrypted_key = dk;
                break;
            }
            Err(_) => println!("Password incorrect! You have {} more times!", 2 - i),
        }
    }
    if decrypted_key.is_empty() {
        return Err("Invalid password".to_string().into());
    }

    let local_key = Keypair::from_protobuf_encoding(&mut decrypted_key)?;
    log::info!("Publick key : {:?}", &local_key.public());

    Ok(local_key)
}

fn create(p: &Path) -> Result<Keypair> {
    println!("Node key not found, create new node key. please set password");

    let password_first = read_password()?;
    println!("Please enter your password again.");
    let password_second = read_password()?;

    if password_first != password_second {
        println!("Entered passwords differ");
        return Err("Entered passwords differ".to_string().into());
    }

    let local_key = Keypair::generate_ed25519();

    // encrypt and save file
    let md5_password = md5(&password_first);
    let mut iv = [0u8; 16];
    OsRng.fill_bytes(&mut iv);
    log::info!("publick key : {:?}", local_key.public());
    let local_key_vec = local_key.to_protobuf_encoding()?;
    let encrypt_data = encrypt::encrypt(&local_key_vec, &md5_password, &iv)?;
    let local_key_data = LocalKey {
        iv: iv.to_vec(),
        local_key: encrypt_data,
    };
    let local_key_data_json = serde_json::to_string(&local_key_data).unwrap();
    fs::write(p, local_key_data_json)?;

    Ok(local_key)
}

fn read_password() -> Result<String> {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let stdin = stdin();
    let mut stdin = stdin.lock();

    stdout.write_all(b"password: ")?;
    stdout.flush()?;

    let pass = stdin.read_passwd(&mut stdout)?;
    match pass {
        Some(p) => Ok(p),
        None => return Err("no password".to_string().into()),
    }
}

fn md5(data: &str) -> [u8; 32] {
    let mut md5 = Md5::new();
    md5.input_str(data);
    let mut key = [0u8; 32];
    md5.result(&mut key);
    key
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocalKey {
    iv: Vec<u8>,
    local_key: Vec<u8>,
}
