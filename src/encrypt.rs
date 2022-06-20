use crypto::buffer::{BufferResult, ReadBuffer, WriteBuffer};
use crypto::{aes, blockmodes, buffer, symmetriccipher};

/// encrypt data
pub fn encrypt(
    data: &[u8],
    key: &[u8],
    iv: &[u8],
) -> Result<Vec<u8>, symmetriccipher::SymmetricCipherError> {
    let mut encryptor =
        aes::cbc_encryptor(aes::KeySize::KeySize256, key, iv, blockmodes::PkcsPadding);

    let mut final_result = Vec::<u8>::new();
    let mut read_buffer = buffer::RefReadBuffer::new(data);
    let mut buffer = [0; 4096];
    let mut write_buffer = buffer::RefWriteBuffer::new(&mut buffer);

    loop {
        let result = encryptor.encrypt(&mut read_buffer, &mut write_buffer, true)?;

        final_result.extend(
            write_buffer
                .take_read_buffer()
                .take_remaining()
                .iter()
                .map(|&i| i),
        );

        match result {
            BufferResult::BufferUnderflow => break,
            BufferResult::BufferOverflow => {}
        }
    }

    Ok(final_result)
}

/// decrypt data
pub fn decrypt(
    encrypted_data: &[u8],
    key: &[u8],
    iv: &[u8],
) -> Result<Vec<u8>, symmetriccipher::SymmetricCipherError> {
    let mut decryptor =
        aes::cbc_decryptor(aes::KeySize::KeySize256, key, iv, blockmodes::PkcsPadding);

    let mut final_result = Vec::<u8>::new();
    let mut read_buffer = buffer::RefReadBuffer::new(encrypted_data);
    let mut buffer = [0; 4096];
    let mut write_buffer = buffer::RefWriteBuffer::new(&mut buffer);

    loop {
        let result = decryptor.decrypt(&mut read_buffer, &mut write_buffer, true)?;
        final_result.extend(
            write_buffer
                .take_read_buffer()
                .take_remaining()
                .iter()
                .map(|&i| i),
        );
        match result {
            BufferResult::BufferUnderflow => break,
            BufferResult::BufferOverflow => {}
        }
    }

    Ok(final_result)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
        // let sleep_seconds = time::Duration::from_secs(1000);
        // let message = "I love Rust,Julia & Python, they are so cool! ";

        // let mut key: [u8; 32] = [0; 32];
        // let mut iv: [u8; 16] = [0; 16];

        // let mut rng = OsRng::new().ok().unwrap();
        // rng.fill_bytes(&mut key);
        // rng.fill_bytes(&mut iv);
        // println!("key:{:?}", key);
        // println!("iv:{:?}", iv);

        // let encrypted_data = encrypt(message.as_bytes(), &key, &iv).ok().unwrap();
        // let message_bytes = message.as_bytes();
        // println!(
        //     "message->as_bytes:{:?}, byte_len:{}",
        //     message_bytes,
        //     message_bytes.len()
        // );
        // println!(
        //     "message->encrypted:{:?} byte_len:{}",
        //     encrypted_data,
        //     encrypted_data.len()
        // );

        // let decrypted_data = decrypt(&encrypted_data[..], &key, &iv).ok().unwrap();

        // let the_string = str::from_utf8(&decrypted_data).expect("not UTF-8");

        // assert!(message_bytes == &decrypted_data[..]);

        // assert!(message == the_string);

        // println!("the_string:{:?}", the_string);

        // thread::sleep(sleep_seconds);
    }
}
